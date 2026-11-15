package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.InputStream
import java.util.concurrent.CancellationException
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FileImportMemoryTest {
    private class GeneratedStream(val length: Long, val reportedAvailable: Int) : InputStream() {
        var bytesRead = 0L
        var readCalls = 0
        var availableCalls = 0
        var afterRead: () -> Unit = {}

        override fun available(): Int {
            availableCalls++
            return reportedAvailable
        }

        override fun read(): Int = error("Use bounded bulk reads")

        override fun read(bytes: ByteArray, offset: Int, count: Int): Int {
            readCalls++
            val size = minOf(count.toLong(), length - bytesRead).toInt()
            if (size == 0) return -1
            repeat(size) { bytes[offset + it] = (bytesRead + it).toByte() }
            bytesRead += size
            afterRead()
            return size
        }
    }

    @Test
    fun admissionIgnoresMissingAndIncorrectLengthsAndChecksActualBoundary() {
        val limit = 19_001 // Exercise growth and a partial final block.
        for (metadata in listOf(0, 1, limit, Int.MAX_VALUE)) {
            for (length in listOf(0L, limit - 1L, limit.toLong(), 64L * 1024 * 1024)) {
                val stream = GeneratedStream(length, metadata)
                if (length <= limit) {
                    val bytes = FileImportMemory.read(stream, limit) { true }
                    assertEquals(length.toInt(), bytes.size)
                    bytes.forEachIndexed { index, byte -> assertEquals(index.toByte(), byte) }
                } else {
                    assertThrows(ImportMemoryExceeded::class.java) {
                        FileImportMemory.read(stream, limit) { true }
                    }
                }
                assertEquals(minOf(length, limit + 1L), stream.bytesRead)
                assertEquals(0, stream.availableCalls)
                assertTrue(stream.readCalls <= 4)
            }
        }
        assertArrayEquals(ByteArray(0), FileImportMemory.read(GeneratedStream(0, 0), 0) { true })
        assertThrows(ImportMemoryExceeded::class.java) {
            FileImportMemory.read(GeneratedStream(1, 1), 0) { true }
        }
    }

    @Test
    fun cancellationStopsBeforeReadingAndImmediatelyAfterAnObsoleteReadReturns() {
        val untouched = GeneratedStream(64L * 1024 * 1024, 0)
        assertThrows(CancellationException::class.java) {
            FileImportMemory.read(untouched, 32_768) { false }
        }
        assertEquals(0, untouched.readCalls)
        var current = true
        val cancelled = GeneratedStream(64L * 1024 * 1024, 0)
        cancelled.afterRead = { current = false }
        assertThrows(CancellationException::class.java) {
            FileImportMemory.read(cancelled, 32_768) { current }
        }
        assertEquals(1, cancelled.readCalls)
        assertEquals(8192L, cancelled.bytesRead)
    }

    @Test
    fun memoryAccountingIncludesRetainedSourceAndTransportHeadroom() {
        assertEquals(0, FileImportMemory.sourceLimit(-1))
        assertEquals(1024, FileImportMemory.sourceLimit(8192))
        assertEquals(256 * 1024 * 1024, FileImportMemory.sourceLimit(Long.MAX_VALUE))
        val filename = "本😀.txt"
        val required = (1024 + filename.length * 3L + 64) * 8
        FileImportMemory.requireSubmissionRoom(1024, filename, required)
        assertThrows(ImportMemoryExceeded::class.java) {
            FileImportMemory.requireSubmissionRoom(1024, filename, required - 1)
        }
    }
}
