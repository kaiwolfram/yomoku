package app.yomoku

import android.graphics.Color
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

internal fun svgImageBytes(width: String = "64", height: String = "16"): ByteArray =
    """<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        width="$width" height="$height" viewBox="0 0 64 16">
        <defs><path id="box" d="M0 0h64v16H0z"/></defs>
        <style>.blue { fill: #0000ff; }</style>
        <a xlink:href="https://example.org/svg"><use xlink:href="#box" class="blue"/></a>
    </svg>"""
        .toByteArray()

@RunWith(AndroidJUnit4::class)
class EpubSvgDecodeTest {
    private fun decode(source: String, budget: Int = EPUB_PAGE_PIXELS) =
        decodeEpubImage(source.toByteArray(), budget, "image/svg+xml")

    @Test
    fun pathsTransformsLocalUsesGradientsAndColorCssRenderIntoTransparentBoundedPixels() {
        val image = checkNotNull(decodeEpubImage(svgImageBytes(), 1024, "image/svg+xml"))
        assertEquals(64, image.width)
        assertEquals(16, image.height)
        assertEquals(Color.BLUE, image.bitmap.getPixel(32, 8))
        image.bitmap.recycle()
        val source =
            """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 40">
          <defs><linearGradient id="g"><stop stop-color="red"/>
          <stop offset="1" stop-color="blue"/></linearGradient></defs>
          <g transform="translate(10 10)"><path d="M0 0h60v20H0z" fill="url(#g)"/></g>
        </svg>"""
        val gradient = checkNotNull(decode(source))
        assertEquals(0, Color.alpha(gradient.bitmap.getPixel(0, 0)))
        val left = gradient.bitmap.getPixel(12, 20)
        val right = gradient.bitmap.getPixel(68, 20)
        assertTrue(Color.red(left) > 200 && Color.blue(left) < 30)
        assertTrue(Color.blue(right) > 200 && Color.red(right) < 30)
        gradient.bitmap.recycle()
        val utf16 = source.toByteArray(Charsets.UTF_16)
        checkNotNull(decodeEpubImage(utf16, 1024, "image/svg+xml")).bitmap.recycle()
        // Media type selects SVG deliberately; raster sniffing never invokes its XML parser.
        assertNull(decodeEpubImage(svgImageBytes(), 1024, "image/png"))
        assertNull(decodeEpubImage(rasterImageBytes(), 1024, "image/svg+xml"))
    }

    @Test
    fun viewBoxPhysicalUnitsFractionalSizesAndAllPixelBudgetsPreserveSourceGeometry() {
        for (budget in listOf(1, 128, 128 * 128, EPUB_PAGE_PIXELS)) {
            val image =
                checkNotNull(
                    decodeEpubImage(svgImageBytes("4096", "1024"), budget, "image/svg+xml")
                )
            assertEquals(4096, image.width)
            assertEquals(1024, image.height)
            assertTrue(image.bitmap.width <= EPUB_IMAGE_EDGE)
            assertTrue(image.bitmap.height <= EPUB_IMAGE_EDGE)
            assertTrue(image.bitmap.allocationByteCount.toLong() <= budget.toLong() * 4)
            assertEquals(Color.BLUE, image.bitmap.getPixel(0, 0))
            image.bitmap.recycle()
        }
        val fractional =
            checkNotNull(decodeEpubImage(svgImageBytes("160.5", "80.25"), 100, "image/svg+xml"))
        assertEquals(160.5f, fractional.sourceSize.width)
        assertEquals(80.25f, fractional.sourceSize.height)
        fractional.bitmap.recycle()
        val units =
            checkNotNull(decodeEpubImage(svgImageBytes("1in", "0.5in"), 100, "image/svg+xml"))
        assertEquals(96f, units.sourceSize.width)
        assertEquals(48f, units.sourceSize.height)
        units.bitmap.recycle()
        checkNotNull(decode("<svg viewBox='0 0 64 16' width='100%' height='100%'/>"))
            .bitmap
            .recycle()
        for (size in
            listOf(
                "width='32769' height='1'",
                "width='10001' height='10000'",
                "width='0' height='10'",
                "width='-1' height='10'",
                "viewBox='0 0 0 10'",
                "",
                "width='NaN' height='1'",
            )) {
            assertNull(size, decode("<svg $size/>"))
        }
        assertNull(decodeEpubImage(svgImageBytes(), 0, "image/svg+xml"))
        assertNull(decodeEpubImage(ByteArray(EPUB_SVG_BYTES + 1), 100, "image/svg+xml"))
    }

    @Test
    fun malformedEntitiesResourcesAndCyclicOrExplosiveReferencesFailBeforeRendering() {
        val rejected =
            listOf(
                "<!DOCTYPE svg [<!ENTITY x 'expanded'>]><svg width='10' height='10'><text>&x;</text></svg>",
                "<!DOCTYPE svg SYSTEM 'file:///proc/self/environ'><svg width='10' height='10'/>",
                "<?xml-stylesheet href='https://example.org/style.css'?><svg width='10' height='10'/>",
                "<svg width='10' height='10'><image href='data:image/png;base64,AA=='/></svg>",
                "<svg width='10' height='10'><image href='file:///tmp/image.png'/></svg>",
                "<svg width='10' height='10'><g fill='url(#p)'><pattern id='p' width='1' height='1'><path d='M0 0h1v1z'/></pattern><path d='M0 0h10v10z'/></g></svg>",
                "<svg width='10' height='10'><use href='https://example.org/a.svg#x'/></svg>",
                "<svg width='10' height='10'><g id='x'><use href='#x'/></g></svg>",
                "<svg width='10' height='10'><use id='x' href='#y'/><use id='y' href='#x'/></svg>",
                "<svg width='10' height='10'><g id='x' style='clip-path:url(#x)'><path d='M0 0h1'/></g></svg>",
                "<svg width='10' height='10'><g id='x' clip-path='url(#x'><path d='M0 0h1'/></g></svg>",
                "<svg width='10' height='10'><style><style/>.x{fill:url(#g)}</style><g id='g'/></svg>",
                "<svg width='10' height='10'><style>.x{fill:url(#g)}</style><g id='g'/></svg>",
                "<svg width='10' height='10'><style>@import 'https://example.org/a.css';</style></svg>",
                "<svg width='10' height='10'><g id='x'/><g id='x'/></svg>",
                "<svg width='10' height='10'><use href='#missing'/></svg>",
                "<svg width='10' height='10'><g>",
                "<svg width='10' height='10'>" + "<g>".repeat(65) + "</g>".repeat(65) + "</svg>",
                "<svg width='10' height='10'>" + "<path/>".repeat(4096) + "</svg>",
                "<svg width='10' height='10'><defs><g id='g0'/></defs>" +
                    (1..20).joinToString("") {
                        "<g id='g$it'><use href='#g${it - 1}'/><use href='#g${it - 1}'/></g>"
                    } +
                    "</svg>",
            )
        for (source in rejected) assertNull(source.take(160), decode(source))
        // A rejected asset does not poison the next decode on the shared worker.
        checkNotNull(decodeEpubImage(svgImageBytes(), 1024, "image/svg+xml")).bitmap.recycle()
    }
}
