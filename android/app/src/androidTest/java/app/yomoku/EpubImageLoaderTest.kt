package app.yomoku

import android.graphics.Bitmap
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubAssetListener
import app.yomoku.shared.ReaderImageAsset
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EpubImageLoaderTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val asset = ReaderImageAsset("images/挿絵.png", "image/png")

    private fun key(id: Long) = EpubImageKey(id, asset)

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun <T> LinkedBlockingQueue<T>.await(): T = checkNotNull(poll(10, TimeUnit.SECONDS))

    @Test
    fun serialPipelineDropsQueuedAndLateCancelledResultsAndRecyclesOnlyUnpublishedBitmap() {
        val reads = LinkedBlockingQueue<Pair<EpubImageKey, EpubAssetListener>>()
        val delivered = LinkedBlockingQueue<DecodedEpubImage>()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val staleBitmap = Bitmap.createBitmap(2, 1, Bitmap.Config.ARGB_8888)
        val decodeCount = AtomicInteger()
        val loader =
            EpubImageLoader(
                read = { key, listener -> reads.add(key to listener) },
                decode = { bytes, budget ->
                    check(Looper.myLooper() != Looper.getMainLooper())
                    if (decodeCount.incrementAndGet() == 1) {
                        entered.countDown()
                        check(release.await(10, TimeUnit.SECONDS))
                        DecodedEpubImage(staleBitmap, 2, 1)
                    } else decodeEpubImage(bytes, budget)
                },
            )
        try {
            lateinit var first: AutoCloseable
            main {
                first = loader.load(key(1), 100) { fail("disposed page received pixels") }
                val queued = loader.load(key(2), 100) { fail("cancelled queue delivered") }
                queued.close()
                loader.load(key(3), 100) {
                    check(Looper.myLooper() == Looper.getMainLooper())
                    delivered.add(checkNotNull(it))
                }
            }
            val firstRead = reads.await()
            assertEquals(key(1), firstRead.first)
            firstRead.second.onAsset("image/png", rasterImageBytes(2, 1))
            assertTrue(entered.await(10, TimeUnit.SECONDS))
            assertTrue(reads.isEmpty()) // SQLite must not copy another BLOB while decode is busy.
            main { first.close() }
            release.countDown()
            val nextRead = reads.await()
            assertTrue(staleBitmap.isRecycled)
            assertEquals(key(3), nextRead.first)
            nextRead.second.onAsset("image/png", rasterImageBytes(2, 1))
            val result = delivered.await()
            assertFalse(result.bitmap.isRecycled)
            main { loader.close() }
            assertFalse(result.bitmap.isRecycled) // May already be held by RenderThread.
            result.bitmap.recycle()
        } finally {
            release.countDown()
            main { loader.close() }
        }
    }

    @Test
    fun errorsRetryExplicitlyAndCallbacksAfterDisposalOrCloseNeverDecode() {
        val reads = LinkedBlockingQueue<EpubAssetListener>()
        val outcomes = LinkedBlockingQueue<Boolean>()
        val decodeCount = AtomicInteger()
        val loader =
            EpubImageLoader(
                read = { _, listener -> reads.add(listener) },
                decode = { _, _ ->
                    decodeCount.incrementAndGet()
                    error("cancelled decode")
                },
            )
        fun request(): AutoCloseable {
            lateinit var result: AutoCloseable
            main { result = loader.load(key(1), 100) { outcomes.add(it == null) } }
            return result
        }
        try {
            request()
            reads.await().onError(EpubAssetError.NotFound)
            assertTrue(outcomes.await())
            assertTrue(reads.isEmpty())
            request()
            reads.await().onAsset("image/jpeg", rasterImageBytes())
            assertTrue(outcomes.await())
            val disposed = request()
            val late = reads.await()
            main { disposed.close() }
            late.onAsset("image/png", rasterImageBytes())
            instrumentation.waitForIdleSync()
            request()
            val closing = reads.await()
            main { loader.close() }
            closing.onAsset("image/png", rasterImageBytes())
            instrumentation.waitForIdleSync()
            assertEquals(0, decodeCount.get())
            assertTrue(outcomes.isEmpty())
            request()
            assertTrue(outcomes.await())
            assertTrue(reads.isEmpty())
        } finally {
            main { loader.close() }
        }
    }
}
