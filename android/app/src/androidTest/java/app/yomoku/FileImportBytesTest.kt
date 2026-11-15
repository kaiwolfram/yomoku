package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import java.security.MessageDigest
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FileImportBytesTest {
    @Test
    fun largeFileRetainsExactUnsignedBytesThroughTypedImportEncodingAndRetry() {
        // The former boxed list exceeds the normal 192 MiB emulator heap at this size.
        val bytes = ByteArray(18 * 1024 * 1024) { it.toByte() }
        val input = FileImportBytes(bytes)
        val event = Event.ImportEpub(EpubImport(input, "literal-😀𠮷.epub"))
        val expectedHash = MessageDigest.getInstance("SHA-256").digest(bytes)
        repeat(2) {
            val encoded = event.bincodeSerialize()
            // Event tag + sequence length precede the original file. Check every byte,
            // including NUL and unsigned values above 127, without a second boxed list.
            assertEquals(43, encoded[0].toInt())
            assertEquals(
                bytes.size + 4 + 8 + 8 + "literal-😀𠮷.epub".toByteArray().size,
                encoded.size,
            )
            val hash = MessageDigest.getInstance("SHA-256")
            hash.update(encoded, 12, bytes.size)
            assertArrayEquals(expectedHash, hash.digest())
        }
        val small = FileImportBytes(bytes.copyOfRange(0, 256))
        for (event in
            listOf(
                Event.ImportTxt(TxtImport(small, "literal.txt", TextEncoding.SHIFTJIS)),
                Event.ImportSrt(SrtImport(small, "literal.srt", TextEncoding.UTF8)),
                Event.ImportEpub(EpubImport(small, "literal.epub")),
            )) {
            assertEquals(event, Event.bincodeDeserialize(event.bincodeSerialize()))
        }
    }
}
