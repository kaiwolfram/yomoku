package app.yomoku

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Handler
import android.os.Looper
import androidx.compose.ui.geometry.Size
import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubAssetListener
import app.yomoku.shared.ReaderImageAsset
import java.util.ArrayDeque
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

// Shared assets decode once per page, with a 16 MiB ARGB page budget. Published images use GC.
internal const val EPUB_PAGE_PIXELS = 4 * 1024 * 1024
internal const val EPUB_IMAGE_EDGE = 2048

internal data class EpubImageKey(val documentId: Long, val asset: ReaderImageAsset)

internal data class DecodedEpubImage(
    val bitmap: Bitmap,
    val width: Int,
    val height: Int,
    val sourceSize: Size = Size(width.toFloat(), height.toFloat()),
)

/** Bounds-first raster or preflighted SVG decode; source geometry stays separate from pixels. */
internal fun decodeEpubImage(
    bytes: ByteArray,
    pixelBudget: Int,
    mediaType: String = "image/png",
): DecodedEpubImage? {
    check(Looper.myLooper() != Looper.getMainLooper())
    if (bytes.isEmpty() || bytes.size > 32 * 1024 * 1024 || pixelBudget < 1) return null
    if (mediaType == "image/svg+xml") return decodeEpubSvg(bytes, pixelBudget)
    return try {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        val width = bounds.outWidth
        val height = bounds.outHeight
        if (width !in 1..32768 || height !in 1..32768 || width.toLong() * height > 100_000_000)
            return null
        var sample = 1
        fun sampled(edge: Int) = (edge.toLong() + sample - 1) / sample
        while (
            sampled(width) > EPUB_IMAGE_EDGE ||
                sampled(height) > EPUB_IMAGE_EDGE ||
                sampled(width) * sampled(height) > pixelBudget
        ) sample *= 2
        val options =
            BitmapFactory.Options().apply {
                inSampleSize = sample
                inPreferredConfig = Bitmap.Config.ARGB_8888
                inScaled = false
            }
        val bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options) ?: return null
        if (
            bitmap.allocationByteCount.toLong() > pixelBudget.toLong() * 4 ||
                bitmap.width > EPUB_IMAGE_EDGE ||
                bitmap.height > EPUB_IMAGE_EDGE
        ) {
            bitmap.recycle()
            null
        } else DecodedEpubImage(bitmap, width, height)
    } catch (_: RuntimeException) {
        null
    } catch (_: OutOfMemoryError) {
        null
    }
}

/** One encoded asset/decode at a time, including across disposed pages. Main-thread owner. */
internal class EpubImageLoader(
    private val read: (EpubImageKey, EpubAssetListener) -> Unit,
    private val executor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val decode: (ByteArray, Int, String) -> DecodedEpubImage? = ::decodeEpubImage,
) : AutoCloseable {
    private val main = Handler(Looper.getMainLooper())
    private val queue = ArrayDeque<Request>()
    private var active: Request? = null
    private var closed = false

    private class Request(
        val key: EpubImageKey,
        val pixelBudget: Int,
        var deliver: ((DecodedEpubImage?) -> Unit)?,
    ) {
        val cancelled = AtomicBoolean(false)
        val responded = AtomicBoolean(false)

        fun cancel() {
            cancelled.set(true)
            deliver = null
        }
    }

    fun load(
        key: EpubImageKey,
        pixelBudget: Int,
        deliver: (DecodedEpubImage?) -> Unit,
    ): AutoCloseable {
        check(Looper.myLooper() == Looper.getMainLooper())
        val request = Request(key, pixelBudget, deliver)
        if (closed) {
            deliver(null)
            request.cancel()
        } else {
            queue.add(request)
            startNext()
        }
        return AutoCloseable {
            check(Looper.myLooper() == Looper.getMainLooper())
            request.cancel()
            queue.remove(request)
        }
    }

    private fun startNext() {
        if (closed || active != null || queue.isEmpty()) return
        val request = queue.removeFirst()
        active = request
        val listener =
            object : EpubAssetListener {
                override fun onAsset(mediaType: String, bytes: ByteArray) {
                    if (!request.responded.compareAndSet(false, true)) return
                    // FIFO callback only hands off; neither decoding nor waiting runs on Rust's
                    // worker.
                    if (request.cancelled.get() || mediaType != request.key.asset.mediaType) {
                        main.post { finish(request, null) }
                        return
                    }
                    try {
                        executor.execute {
                            val image =
                                if (request.cancelled.get()) null
                                else decode(bytes, request.pixelBudget, mediaType)
                            main.post { finish(request, image) }
                        }
                    } catch (_: java.util.concurrent.RejectedExecutionException) {
                        main.post { finish(request, null) }
                    }
                }

                override fun onError(error: EpubAssetError) {
                    if (request.responded.compareAndSet(false, true))
                        main.post { finish(request, null) }
                }
            }
        try {
            read(request.key, listener)
        } catch (_: Exception) {
            listener.onError(EpubAssetError.Unavailable)
        }
    }

    private fun finish(request: Request, image: DecodedEpubImage?) {
        if (request.cancelled.get() || closed) {
            // Never recycle a published bitmap: Compose/RenderThread may still hold it.
            image?.bitmap?.recycle()
        } else request.deliver?.invoke(image)
        request.deliver = null
        active = null
        startNext()
    }

    override fun close() {
        check(Looper.myLooper() == Looper.getMainLooper())
        closed = true
        queue.forEach { it.cancel() }
        queue.clear()
        active?.cancel()
        executor.shutdown()
    }
}
