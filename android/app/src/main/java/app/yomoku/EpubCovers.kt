package app.yomoku

import android.os.Handler
import android.os.Looper
import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubCoverListener
import app.yomoku.shared.ReaderImageAsset
import java.util.ArrayDeque
import java.util.concurrent.atomic.AtomicBoolean

internal const val EPUB_COVER_PIXELS = 128 * 128
internal const val EPUB_COVER_CACHE_BYTES = 4 * 1024 * 1024

/** Main-owned, content-addressed LRU. Eviction never recycles published/visible pixels. */
internal class EpubCoverCache(private val byteLimit: Int = EPUB_COVER_CACHE_BYTES) {
    private val entries = LinkedHashMap<String, DecodedEpubImage>(16, 0.75f, true)
    var byteCount = 0
        private set

    operator fun get(key: String): DecodedEpubImage? = entries[key]

    fun put(key: String, image: DecodedEpubImage) {
        val size = image.bitmap.allocationByteCount
        if (size > byteLimit) return
        entries.put(key, image)?.let { byteCount -= it.bitmap.allocationByteCount }
        byteCount += size
        while (byteCount > byteLimit || entries.size > 64) {
            val oldest = entries.entries.iterator()
            byteCount -= oldest.next().value.bitmap.allocationByteCount
            oldest.remove()
        }
    }

    fun clear() {
        entries.clear()
        byteCount = 0
    }
}

/** Resolve live ownership before every cache hit; populate only from verified asset bytes. */
internal class EpubCoverLoader(
    private val read: (Long, EpubCoverListener) -> Unit,
    private val images: EpubImageLoader,
    private val cache: EpubCoverCache = EpubCoverCache(),
) : AutoCloseable {
    private val main = Handler(Looper.getMainLooper())
    private val queue = ArrayDeque<Request>()
    private var active: Request? = null
    private var closed = false

    private class Request(val id: Long, var deliver: ((DecodedEpubImage?) -> Unit)?) {
        var imageRequest: AutoCloseable? = null
    }

    fun load(id: Long, deliver: (DecodedEpubImage?) -> Unit): AutoCloseable {
        check(Looper.myLooper() == Looper.getMainLooper())
        val request = Request(id, deliver)
        if (closed) deliver(null)
        else {
            queue.add(request)
            startNext()
        }
        return AutoCloseable {
            check(Looper.myLooper() == Looper.getMainLooper())
            request.deliver = null
            queue.remove(request)
            if (active === request && request.imageRequest != null) {
                request.imageRequest?.close()
                finish(request, null)
            }
            // An accepted metadata callback must drain before starting the next lookup.
        }
    }

    private fun startNext() {
        if (closed || active != null || queue.isEmpty()) return
        val request = queue.removeFirst()
        active = request
        val responded = AtomicBoolean()
        val listener =
            object : EpubCoverListener {
                override fun onCover(path: String, mediaType: String, sha256: ByteArray) {
                    if (!responded.compareAndSet(false, true)) return
                    val digest = sha256.copyOf()
                    main.post {
                        if (closed || request.deliver == null || digest.size != 32) {
                            finish(request, null)
                        } else {
                            val key =
                                mediaType + ":" + digest.joinToString("") { "%02x".format(it) }
                            val cached = cache[key]
                            if (cached != null) finish(request, cached)
                            else
                                request.imageRequest =
                                    images.load(
                                        EpubImageKey(request.id, ReaderImageAsset(path, mediaType)),
                                        EPUB_COVER_PIXELS,
                                        expectedSha256 = digest,
                                    ) { image ->
                                        if (image != null) cache.put(key, image)
                                        finish(request, image)
                                    }
                        }
                    }
                }

                override fun onNoCover() = absent()

                override fun onError(error: EpubAssetError) = absent()

                private fun absent() {
                    if (responded.compareAndSet(false, true)) main.post { finish(request, null) }
                }
            }
        try {
            read(request.id, listener)
        } catch (_: Exception) {
            listener.onError(EpubAssetError.Unavailable)
        }
    }

    private fun finish(request: Request, image: DecodedEpubImage?) {
        if (active !== request) return
        if (!closed) request.deliver?.invoke(image)
        request.deliver = null
        active = null
        startNext()
    }

    override fun close() {
        check(Looper.myLooper() == Looper.getMainLooper())
        closed = true
        queue.forEach { it.deliver = null }
        queue.clear()
        active?.let {
            it.deliver = null
            it.imageRequest?.close()
        }
        cache.clear()
    }
}
