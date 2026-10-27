package app.yomoku

import android.graphics.Color
import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

private fun imageUri(bytes: ByteArray, media: String = "image/svg+xml") =
    "data:$media;base64," + Base64.encodeToString(bytes, Base64.NO_WRAP)

private fun vectorImage(uri: String) =
    "<svg width='64' height='16'><image width='64' height='16' href='$uri'/></svg>"

internal fun nestedSvgImageBytes(color: Int = Color.BLUE): ByteArray {
    // The nested vector includes both a raster resource and an independently scoped ID.
    val child = embeddedSvgImageBytes(color)
    return vectorImage(imageUri(child)).toByteArray()
}

@RunWith(AndroidJUnit4::class)
class EpubSvgNestedImagesTest {
    private fun decode(source: String, budget: Int = EPUB_PAGE_PIXELS) =
        decodeEpubImage(source.toByteArray(), budget, "image/svg+xml")

    @Test
    fun nestedVectorsAndRastersKeepPixelsUtf16GeometryIdsAndIndependentStyles() {
        val child = svgImageBytes("160.5", "80.25")
        val uri = imageUri(child.toString(Charsets.UTF_8).toByteArray(Charsets.UTF_16))
        val source =
            """<svg width="80" height="80"><defs><g id="box"/>
            <clipPath id="clip"><rect width="64" height="64"/></clipPath></defs>
            <image x="8" y="8" width="64" height="64" href="$uri"
                transform="translate(0 0)" clip-path="url(#clip)" opacity="0.5"/>
            </svg>"""
        val prepared = checkNotNull(prepareSvgImages(source.toByteArray()))
        assertEquals(1, prepared.sources.size)
        assertEquals(160.5f, prepared.sources.single().size.width)
        assertEquals(80.25f, prepared.sources.single().size.height)
        val image = checkNotNull(decode(source, 1024))
        val center = image.bitmap.getPixel(image.bitmap.width / 2, image.bitmap.height / 2)
        assertTrue(Color.alpha(center) in 126..129)
        assertEquals(255, Color.blue(center))
        assertEquals(0, Color.alpha(image.bitmap.getPixel(image.bitmap.width / 2, 1)))
        image.bitmap.recycle()
        checkNotNull(decodeEpubImage(nestedSvgImageBytes(), 128, "image/svg+xml")).also {
            assertEquals(Color.BLUE, it.bitmap.getPixel(it.bitmap.width / 2, it.bitmap.height / 2))
            it.bitmap.recycle()
        }
        val viewBox = "<svg viewBox='0 0 64 16'><path fill='blue' d='M0 0h64v16H0z'/></svg>"
        checkNotNull(decode(vectorImage(imageUri(viewBox.toByteArray())), 128)).also {
            assertEquals(Color.BLUE, it.bitmap.getPixel(0, 0))
            it.bitmap.recycle()
        }
    }

    @Test
    fun repeatedUrisShareAcrossNestedDocumentsAndAllTemporaryPixelsShareOneBudget() {
        val raster = imageUri(rasterImageBytes(4096, 1024), "image/png")
        val first = vectorImage(raster)
        val second = first.replace("<svg ", "<svg id='other' ")
        val root =
            """<svg width="128" height="16"><defs>
            <image id="tile" width="64" height="16" href="${imageUri(first.toByteArray())}"/>
            </defs><use href="#tile"/><use x="64" href="#tile"/>
            <image width="64" height="16" href="${imageUri(second.toByteArray())}"/>
            <image width="64" height="16" href="$raster"/></svg>"""
        val prepared = checkNotNull(prepareSvgImages(root.toByteArray()))
        assertEquals(3, prepared.sources.size)
        assertEquals(1, prepared.sources.count { it is SvgRasterSource })
        for (budget in listOf(3, 128, 16384, EPUB_PAGE_PIXELS)) {
            val resources = checkNotNull(SvgImageResources.decode(prepared.sources, budget))
            try {
                assertEquals(3, resources.images.size)
                assertTrue(
                    resources.images.sumOf { it.allocationByteCount.toLong() } <= budget * 4L
                )
                for (bitmap in resources.images) {
                    assertTrue(bitmap.width <= EPUB_IMAGE_EDGE && bitmap.height <= EPUB_IMAGE_EDGE)
                    assertEquals(Color.BLUE, bitmap.getPixel(0, 0))
                }
                assertSame(resources.images[2], resources.resolveImage("yomoku-svg-image:2"))
                assertNull(resources.resolveImage("data:image/svg+xml;base64,AA=="))
                assertTrue(resources.isFormatSupported("image/svg+xml"))
            } finally {
                resources.close()
            }
            assertTrue(resources.images.all { it.isRecycled })
        }
        assertNull(SvgImageResources.decode(prepared.sources, 2))
        assertNull(decode(root, 2))
    }

    @Test
    fun singlePixelNestedVectorsKeepOriginalRatioAndShareRepeatedUses() {
        val unused =
            (0 until 63).joinToString("") {
                val child =
                    "<svg width='1' height='1'><rect width='1' height='1' fill='rgb($it,0,0)'/></svg>"
                "<image width='1' height='1' href='${imageUri(child.toByteArray())}'/>"
            }
        val child = "<svg width='64' height='16'><path fill='blue' d='M0 0h64v16H0z'/></svg>"
        val uri = imageUri(child.toByteArray())
        val source =
            """<svg width="64" height="64"><defs>$unused
            <image id="tile" width="64" height="64" href="$uri"/></defs>
            <use href="#tile" opacity="0.5"/></svg>"""
        val image = checkNotNull(decode(source, 64))
        assertEquals(8, image.bitmap.width)
        assertEquals(0, Color.alpha(image.bitmap.getPixel(4, 1)))
        val center = image.bitmap.getPixel(4, 4)
        assertTrue(Color.alpha(center) in 126..129)
        assertEquals(255, Color.blue(center))
        image.bitmap.recycle()
        val repeated =
            "<svg width='64' height='16'>" +
                "<image width='64' height='16' href='$uri'/>".repeat(10) +
                "</svg>"
        checkNotNull(decode(repeated, 1)).also {
            assertEquals(Color.BLUE, it.bitmap.getPixel(0, 0))
            it.bitmap.recycle()
        }
    }

    @Test
    fun nestedMalformedExternalActiveCyclicAndStyledCompoundInputsFallBackWithoutPoisoningRetry() {
        val children =
            listOf(
                "<!DOCTYPE svg [<!ENTITY x 'expanded'>]><svg width='1' height='1'><text>&x;</text></svg>",
                "<?xml-stylesheet href='file:///tmp/style.css'?><svg width='1' height='1'/>",
                "<svg width='1' height='1'><use href='#missing'/></svg>",
                "<svg width='1' height='1'><g id='a'><use href='#a'/></g></svg>",
                "<svg width='1' height='1'><script/></svg>",
                "<svg width='1' height='1'><pattern/></svg>",
                vectorImage("file:///tmp/image.svg"),
                vectorImage("../image.svg"),
                vectorImage("https://example.org/image.svg"),
                vectorImage("#local"),
                vectorImage("data:image/svg+xml;base64,!!!"),
                vectorImage("data:image/svg+xml;base64,AA=="),
                vectorImage("yomoku-svg-image:0"),
                vectorImage(imageUri(svgImageBytes()))
                    .replace("<image ", "<style>image {opacity:0.5}</style><image "),
                "<svg width='32769' height='1'/>",
                "<svg width='1' height='1'>",
            )
        for (child in children) {
            assertNull(child.take(100), decode(vectorImage(imageUri(child.toByteArray()))))
        }
        checkNotNull(decodeEpubImage(nestedSvgImageBytes(), 128, "image/svg+xml")).bitmap.recycle()
        checkNotNull(decodeEpubImage(svgImageBytes(), 128, "image/svg+xml")).bitmap.recycle()
    }

    @Test
    fun nestingAndAggregateXmlNodeResourceAndExpansionLimitsApplyAcrossAllChildren() {
        var nested = "<svg width='64' height='16'><path fill='blue' d='M0 0h64v16H0z'/></svg>"
        repeat(8) { nested = vectorImage(imageUri(nested.toByteArray())) }
        checkNotNull(decode(nested, 8)).bitmap.recycle()
        assertNull(decode(vectorImage(imageUri(nested.toByteArray())), 9))
        val manyNodes = "<svg width='1' height='1'>" + "<path/>".repeat(2100) + "</svg>"
        val many =
            "<svg width='1' height='1'>" +
                listOf(manyNodes, manyNodes.replace("<svg ", "<svg id='other' ")).joinToString("") {
                    "<image width='1' height='1' href='${imageUri(it.toByteArray())}'/>"
                } +
                "</svg>"
        assertNull(decode(many))
        val sixtyFive =
            "<svg width='1' height='1'>" +
                (0..64).joinToString("") {
                    "<image width='1' height='1' href='${imageUri("<svg id='v$it' width='1' height='1'/>".toByteArray())}'/>"
                } +
                "</svg>"
        assertNull(decode(sixtyFive))
        val padded = "<svg width='1' height='1'>" + " ".repeat(950_000) + "</svg>"
        val aggregateBytes = vectorImage(imageUri(padded.toByteArray()))
        assertTrue(aggregateBytes.length < EPUB_SVG_BYTES)
        assertNull(decode(aggregateBytes))
        // Each document is individually below the expansion cap; their combined cost is not.
        val expanded =
            "<svg width='1' height='1'><defs><g id='g0'/></defs>" +
                (1..12).joinToString("") {
                    "<g id='g$it'><use href='#g${it - 1}'/><use href='#g${it - 1}'/></g>"
                } +
                "</svg>"
        checkNotNull(decode(expanded)).bitmap.recycle()
        val aggregateExpansion =
            vectorImage(imageUri(expanded.toByteArray()))
                .replace("</svg>", "<use href='#top'/></svg>")
                .replace("<image ", "<image id='top' ")
        // Duplicate uses of a pre-rendered child should NOT multiply its vector expansion.
        checkNotNull(decode(aggregateExpansion, 128)).bitmap.recycle()
        val twoExpanded =
            "<svg width='1' height='1'>" +
                listOf(expanded, expanded.replace("<svg ", "<svg id='other' ")).joinToString("") {
                    "<image width='1' height='1' href='${imageUri(it.toByteArray())}'/>"
                } +
                "</svg>"
        assertNull(decode(twoExpanded))
    }
}
