package app.yomoku

import android.graphics.Bitmap
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

internal fun rasterImageBytes(
    width: Int = 64,
    height: Int = 32,
    format: Bitmap.CompressFormat = Bitmap.CompressFormat.PNG,
    color: Int = android.graphics.Color.BLUE,
): ByteArray {
    val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
    bitmap.eraseColor(color)
    return ByteArrayOutputStream().use {
        assertTrue(bitmap.compress(format, 100, it))
        bitmap.recycle()
        it.toByteArray()
    }
}

@RunWith(AndroidJUnit4::class)
class EpubImageDecodeTest {
    @Test
    fun boundsSamplingPreservesSourceRatioAndRejectsUnsupportedOrBrokenBytes() {
        val bytes = rasterImageBytes(4096, 2049)
        val image = checkNotNull(decodeEpubImage(bytes, 128 * 128))
        assertEquals(4096, image.width)
        assertEquals(2049, image.height)
        assertTrue(image.bitmap.width <= 128)
        assertTrue(image.bitmap.height <= 128)
        assertTrue(image.bitmap.allocationByteCount <= 128 * 128 * 4)
        image.bitmap.recycle()
        val edge = checkNotNull(decodeEpubImage(bytes, EPUB_PAGE_PIXELS))
        assertTrue(edge.bitmap.width <= EPUB_IMAGE_EDGE)
        assertTrue(edge.bitmap.height <= EPUB_IMAGE_EDGE)
        edge.bitmap.recycle()
        val tiny = checkNotNull(decodeEpubImage(bytes, 1))
        assertEquals(1, tiny.bitmap.width)
        assertEquals(1, tiny.bitmap.height)
        tiny.bitmap.recycle()
        val full = checkNotNull(decodeEpubImage(rasterImageBytes(), EPUB_PAGE_PIXELS))
        assertEquals(64, full.bitmap.width)
        assertEquals(32, full.bitmap.height)
        full.bitmap.recycle()
        val jpeg =
            checkNotNull(
                decodeEpubImage(
                    rasterImageBytes(120, 81, Bitmap.CompressFormat.JPEG),
                    EPUB_PAGE_PIXELS,
                )
            )
        assertEquals(120, jpeg.width)
        assertEquals(81, jpeg.height)
        jpeg.bitmap.recycle()
        assertNull(decodeEpubImage(bytes, 0))
        assertNull(decodeEpubImage(byteArrayOf(), 100))
        assertNull(decodeEpubImage(ByteArray(128) { it.toByte() }, 100))
        assertNull(decodeEpubImage("<svg xmlns='http://www.w3.org/2000/svg'/>".toByteArray(), 100))
        assertNull(decodeEpubImage(ByteArray(32 * 1024 * 1024 + 1), 100))
        // A forged huge PNG header must fail before allocating the source dimensions.
        val forged = bytes.copyOf()
        java.nio.ByteBuffer.wrap(forged).putInt(16, 32769).putInt(20, 1)
        val crc = java.util.zip.CRC32().apply { update(forged, 12, 17) }
        java.nio.ByteBuffer.wrap(forged).putInt(29, crc.value.toInt())
        assertNull(decodeEpubImage(forged, 100))
    }

    @Test
    fun unicodeImagePartsPreserveTextAndRejectPartialOverlappingOrForeignScalars() {
        val text = "😀𠮷前\uFFFC\uFFFC後"
        val asset = ReaderImageAsset("images/挿絵.png", "image/png")
        fun image(start: Long, end: Long) =
            ReaderInlineImage(TextRange(TextOffset(start), TextOffset(end)), asset, "絵")
        val images = listOf(image(11, 14), image(14, 17))
        assertEquals(
            listOf(
                ReaderImagePart.Text(0, 5),
                ReaderImagePart.Image(0),
                ReaderImagePart.Image(1),
                ReaderImagePart.Text(7, 8),
            ),
            readerImageParts(text, images),
        )
        assertNull(readerImageParts(text, listOf(image(12, 14))))
        assertNull(readerImageParts(text, listOf(image(11, 13))))
        assertNull(readerImageParts(text, listOf(image(8, 11))))
        assertNull(readerImageParts(text, images.reversed()))
        assertNull(readerImageParts(text, listOf(images[0], images[0])))
        assertNull(readerImageParts("\uD800", emptyList()))
        assertEquals(
            listOf(ReaderImagePart.Image(0)),
            readerImageParts("\uFFFC", listOf(image(0, 3))),
        )
    }
}
