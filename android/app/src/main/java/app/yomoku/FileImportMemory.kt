package app.yomoku

import java.io.InputStream
import java.util.concurrent.CancellationException

internal class ImportMemoryExceeded : Exception()

internal object FileImportMemory {
    // Matches the native EPUB source ceiling; Android applies it to all file formats.
    private const val MAX_SOURCE_BYTES = 256 * 1024 * 1024

    fun availableBytes(): Long =
        Runtime.getRuntime().run { maxMemory() - (totalMemory() - freeMemory()) }

    // Leave room for retained retry bytes, acquisition growth/copy, serializer growth/copy,
    // JNI/native source copies and other app work. This budgets transport, not parser expansion.
    fun sourceLimit(availableBytes: Long): Int =
        (availableBytes.coerceAtLeast(0) / 8).coerceAtMost(MAX_SOURCE_BYTES.toLong()).toInt()

    fun requireSubmissionRoom(size: Int, filename: String, availableBytes: Long) {
        // Bincode includes the filename and small typed fields as well as the source bytes.
        val encodedSize = size.toLong() + filename.length * 3L + 64
        if (encodedSize > sourceLimit(availableBytes)) throw ImportMemoryExceeded()
    }

    /** Never trust provider length/available(); read at most the budget plus one probe byte. */
    fun read(input: InputStream, limit: Int, isCurrent: () -> Boolean): ByteArray {
        require(limit >= 0)
        val block = ByteArray(8192)
        var buffer = ByteArray(0)
        var size = 0
        fun checkCurrent() {
            if (!isCurrent()) throw CancellationException()
        }
        while (true) {
            checkCurrent()
            val count =
                input.read(block, 0, minOf(block.size.toLong(), limit.toLong() - size + 1).toInt())
            checkCurrent()
            if (count < 0) break
            if (count == 0) continue
            if (count > limit - size) throw ImportMemoryExceeded()
            val required = size + count
            if (required > buffer.size) {
                val capacity =
                    maxOf(required.toLong(), buffer.size.toLong() * 2)
                        .coerceAtMost(limit.toLong())
                        .toInt()
                buffer = buffer.copyOf(capacity)
            }
            block.copyInto(buffer, size, 0, count)
            size = required
        }
        checkCurrent()
        return if (size == buffer.size) buffer else buffer.copyOf(size)
    }
}
