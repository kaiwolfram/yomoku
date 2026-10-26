package app.yomoku

import android.graphics.Bitmap
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubAssetListener
import app.yomoku.nativebridge.EpubCoverListener
import java.security.MessageDigest
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EpubCoverLoaderTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun <T> LinkedBlockingQueue<T>.await(): T = checkNotNull(poll(10, TimeUnit.SECONDS))

    private fun digest(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes)

    private class Outcome(val image: DecodedEpubImage?)

    @Test
    fun lruBoundsBytesAndEntriesWithoutRecyclingPublishedPixels() {
        val images =
            (0..2).map {
                DecodedEpubImage(Bitmap.createBitmap(2, 1, Bitmap.Config.ARGB_8888), 2, 1)
            }
        main {
            val size = images[0].bitmap.allocationByteCount
            val cache = EpubCoverCache(size * 2)
            cache.put("a", images[0])
            cache.put("b", images[1])
            assertSame(images[0], cache["a"])
            cache.put("c", images[2])
            assertNull(cache["b"])
            assertSame(images[0], cache["a"])
            assertSame(images[2], cache["c"])
            assertEquals(size * 2, cache.byteCount)
            cache.put("a", images[1])
            assertEquals(size * 2, cache.byteCount)
            val tooSmall = EpubCoverCache(size - 1)
            tooSmall.put("a", images[0])
            assertEquals(0, tooSmall.byteCount)
            cache.clear()
            assertEquals(0, cache.byteCount)
            assertTrue(images.none { it.bitmap.isRecycled })
            val capped = EpubCoverCache()
            repeat(65) { capped.put(it.toString(), images[0]) }
            assertNull(capped["0"])
            assertSame(images[0], capped["1"])
            assertEquals(size * 64, capped.byteCount)
        }
    }

    @Test
    fun cacheHitsResolveLiveMetadataAndSharePixelsAcrossDocumentsWithoutAnotherBlob() {
        val metadata = LinkedBlockingQueue<Pair<Long, EpubCoverListener>>()
        val assets = LinkedBlockingQueue<EpubAssetListener>()
        val delivered = LinkedBlockingQueue<Outcome>()
        val images = EpubImageLoader(read = { _, listener -> assets.add(listener) })
        val cache = EpubCoverCache()
        val loader =
            EpubCoverLoader({ id, listener -> metadata.add(id to listener) }, images, cache)
        val bytes = rasterImageBytes(4096, 1024)
        fun request(id: Long) = main {
            loader.load(id) {
                check(Looper.myLooper() == Looper.getMainLooper())
                delivered.add(Outcome(it))
            }
        }
        try {
            request(1)
            request(2)
            val first = metadata.await()
            assertEquals(1L, first.first)
            assertTrue(metadata.isEmpty())
            first.second.onCover("挿絵.png", "image/png", digest(bytes))
            assets.await().onAsset("image/png", bytes)
            val image = checkNotNull(delivered.await().image)
            assertEquals(4096, image.width)
            assertTrue(image.bitmap.allocationByteCount <= EPUB_COVER_PIXELS * 4)
            val second = metadata.await()
            assertEquals(2L, second.first)
            second.second.onCover("different/path.png", "image/png", digest(bytes))
            assertSame(image, delivered.await().image)
            assertTrue(assets.isEmpty())
            request(1)
            metadata.await().second.onNoCover() // Deleted/absent ownership must not reuse pixels.
            assertNull(delivered.await().image)
            request(2)
            metadata.await().second.onError(EpubAssetError.InvalidDocument)
            assertNull(delivered.await().image)
            assertTrue(assets.isEmpty())
            main {
                loader.close()
                images.close()
            }
            assertEquals(0, cache.byteCount)
            assertFalse(image.bitmap.isRecycled)
        } finally {
            main {
                loader.close()
                images.close()
            }
        }
    }

    @Test
    fun changedDigestAndMediaNeverReuseCacheAndFailedDecodeCanRetry() {
        val metadata = LinkedBlockingQueue<EpubCoverListener>()
        val assets = LinkedBlockingQueue<EpubAssetListener>()
        val delivered = LinkedBlockingQueue<Outcome>()
        val images = EpubImageLoader(read = { _, listener -> assets.add(listener) })
        val cache = EpubCoverCache()
        val loader = EpubCoverLoader({ _, listener -> metadata.add(listener) }, images, cache)
        val bytes = rasterImageBytes()
        fun request(media: String, hash: ByteArray) {
            main { loader.load(1) { delivered.add(Outcome(it)) } }
            metadata.await().onCover("cover", media, hash)
        }
        try {
            request("image/png", digest(byteArrayOf(0)))
            assets.await().onAsset("image/png", bytes) // Asset changed after metadata read.
            assertNull(delivered.await().image)
            main { assertEquals(0, cache.byteCount) }
            request("image/png", digest(bytes))
            assets.await().onAsset("image/png", bytes)
            assertNotNull(delivered.await().image)
            request("image/svg+xml", digest(bytes))
            assets.await().onAsset("image/svg+xml", bytes)
            assertNull(delivered.await().image)
            request("image/png", ByteArray(2))
            assertNull(delivered.await().image)
            assertTrue(assets.isEmpty())
        } finally {
            main {
                loader.close()
                images.close()
            }
        }
    }

    @Test
    fun cancelledQueuedMetadataAndAssetRequestsDrainAndCloseSuppressesLateDelivery() {
        val metadata = LinkedBlockingQueue<Pair<Long, EpubCoverListener>>()
        val assets = LinkedBlockingQueue<EpubAssetListener>()
        val delivered = LinkedBlockingQueue<Outcome>()
        val images = EpubImageLoader(read = { _, listener -> assets.add(listener) })
        val loader = EpubCoverLoader({ id, listener -> metadata.add(id to listener) }, images)
        val bytes = rasterImageBytes()
        lateinit var first: AutoCloseable
        lateinit var third: AutoCloseable
        try {
            main {
                first = loader.load(1) { fail("cancelled metadata delivered") }
                loader.load(2) { fail("cancelled queue delivered") }.close()
                third = loader.load(3) { fail("cancelled asset delivered") }
            }
            val late = metadata.await()
            main { first.close() }
            assertTrue(metadata.isEmpty())
            late.second.onCover("cover", "image/png", digest(bytes))
            val next = metadata.await()
            assertEquals(3L, next.first)
            assertTrue(assets.isEmpty())
            next.second.onCover("cover", "image/png", digest(bytes))
            val cancelledAsset = assets.await()
            main {
                third.close()
                loader.load(4) { delivered.add(Outcome(it)) }
            }
            metadata.await().second.onCover("cover", "image/png", digest(bytes))
            cancelledAsset.onAsset("image/png", bytes)
            assets.await().onAsset("image/png", bytes)
            assertNotNull(delivered.await().image)
            main { loader.load(5) { fail("closed request delivered") } }
            val closing = metadata.await()
            main {
                loader.close()
                images.close()
            }
            closing.second.onCover("cover", "image/png", digest(bytes))
            instrumentation.waitForIdleSync()
            assertTrue(assets.isEmpty())
            main { loader.load(6) { delivered.add(Outcome(it)) } }
            assertNull(delivered.await().image)
            assertTrue(metadata.isEmpty())
        } finally {
            main {
                loader.close()
                images.close()
            }
        }
    }
}
