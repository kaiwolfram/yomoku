package app.yomoku.tests

import app.yomoku.nativebridge.EpubAssetError
import app.yomoku.nativebridge.EpubAssetListener
import app.yomoku.nativebridge.EpubCoverListener
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.DocumentId
import java.security.MessageDigest
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal class CoverResponse : EpubCoverListener {
    private val responses = LinkedBlockingQueue<Pair<Thread, Any>>()

    override fun onCover(path: String, mediaType: String, sha256: ByteArray) {
        responses.add(Thread.currentThread() to Triple(path, mediaType, sha256))
    }

    override fun onNoCover() {
        responses.add(Thread.currentThread() to Unit)
    }

    override fun onError(error: EpubAssetError) {
        responses.add(Thread.currentThread() to error)
    }

    fun await(): Any {
        val (worker, result) = checkNotNull(responses.poll(60, TimeUnit.SECONDS))
        check(worker != Thread.currentThread())
        check(responses.isEmpty())
        return result
    }
}

internal class AssetResponse : EpubAssetListener {
    private val responses = LinkedBlockingQueue<Triple<Thread, String?, Any>>()

    override fun onAsset(mediaType: String, bytes: ByteArray) {
        responses.add(Triple(Thread.currentThread(), mediaType, bytes))
    }

    override fun onError(error: EpubAssetError) {
        responses.add(Triple(Thread.currentThread(), null, error))
    }

    fun awaitBytes(expected: ByteArray) {
        val (worker, media, result) = checkNotNull(responses.poll(60, TimeUnit.SECONDS))
        check(worker != Thread.currentThread())
        check(media == "image/png" && result is ByteArray && result.contentEquals(expected))
        check(responses.isEmpty())
    }

    fun awaitError(expected: EpubAssetError) {
        val (worker, media, result) = checkNotNull(responses.poll(60, TimeUnit.SECONDS))
        check(worker != Thread.currentThread() && media == null && result == expected)
        check(responses.isEmpty())
    }
}

internal fun verifyEpubAssetReads(bridge: NativeBridge, id: DocumentId) {
    val before = bridge.view()
    repeat(2) {
        val cover = CoverResponse()
        bridge.loadEpubCoverReference(id.value, cover)
        val result = cover.await() as Triple<*, *, *>
        check(result.first == "OPS/images/cover.png" && result.second == "image/png")
        check(
            (result.third as ByteArray).contentEquals(
                MessageDigest.getInstance("SHA-256").digest("fixture cover bytes".toByteArray())
            )
        )
    }
    val absent = CoverResponse()
    bridge.loadEpubCoverReference(Long.MAX_VALUE, absent)
    check(absent.await() == Unit)
    for (invalid in listOf(0L, -1L)) {
        check(
            runCatching { bridge.loadEpubCoverReference(invalid, CoverResponse()) }
                .exceptionOrNull() == EpubAssetError.InvalidRequest
        )
    }
    for ((path, expected) in
        listOf(
            "OPS/images/挿絵.png" to "fixture image bytes",
            "OPS/images/cover.png" to "fixture cover bytes",
            "OPS/images/挿絵.png" to "fixture image bytes",
        )) {
        val response = AssetResponse()
        bridge.loadEpubImageAsset(id.value, path, "image/png", response)
        response.awaitBytes(expected.toByteArray())
    }
    for ((document, path, media, error) in
        listOf(
            AssetFailure(id.value, "OPS/missing.png", "image/png", EpubAssetError.NotFound),
            AssetFailure(Long.MAX_VALUE, "OPS/images/挿絵.png", "image/png", EpubAssetError.NotFound),
            AssetFailure(
                id.value,
                "OPS/images/挿絵.png",
                "image/jpeg",
                EpubAssetError.InvalidDocument,
            ),
        )) {
        val response = AssetResponse()
        bridge.loadEpubImageAsset(document, path, media, response)
        response.awaitError(error)
    }
    for ((document, path, media) in
        listOf(
            Triple(0L, "OPS/images/挿絵.png", "image/png"),
            Triple(-1L, "OPS/images/挿絵.png", "image/png"),
            Triple(id.value, "../OPS/images/挿絵.png", "image/png"),
            Triple(id.value, "https://example.org/image.png", "image/png"),
            Triple(id.value, "OPS/images/挿絵.png", "text/plain"),
        )) {
        val error = runCatching {
            bridge.loadEpubImageAsset(document, path, media, AssetResponse())
        }
            .exceptionOrNull()
        check(error == EpubAssetError.InvalidRequest)
    }
    check(bridge.view().contentEquals(before))
}

private data class AssetFailure(
    val id: Long,
    val path: String,
    val media: String,
    val error: EpubAssetError,
)
