package app.yomoku

import android.graphics.Bitmap
import android.graphics.Color
import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

private fun rasterUri(bytes: ByteArray, media: String = "image/png") =
    "data:$media;base64," + Base64.encodeToString(bytes, Base64.NO_WRAP)

internal fun embeddedSvgImageBytes(color: Int = Color.BLUE): ByteArray {
    val uri = rasterUri(rasterImageBytes(64, 16, color = color))
    return """<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
        width="64" height="16" viewBox="0 0 64 16">
        <defs><image id="tile" width="32" height="16" preserveAspectRatio="none" xlink:href="$uri"/></defs>
        <a xlink:href="https://example.org/svg"><use xlink:href="#tile"/>
        <image x="32" width="32" height="16" preserveAspectRatio="none" href="$uri"/></a>
    </svg>"""
        .toByteArray()
}

@RunWith(AndroidJUnit4::class)
class EpubSvgEmbeddedImagesTest {
    private fun decode(source: String, budget: Int = EPUB_PAGE_PIXELS) =
        decodeEpubImage(source.toByteArray(), budget, "image/svg+xml")

    @Test
    fun embeddedPngJpegLocalUsesTransformsClipsAndUtf16RenderWithOriginalRasterRatio() {
        for (format in listOf(Bitmap.CompressFormat.PNG, Bitmap.CompressFormat.JPEG)) {
            val media = if (format == Bitmap.CompressFormat.PNG) "image/png" else "image/jpeg"
            // Sampling 4096x1024 down to 1x1 must still fit as a 4:1 image, not a square.
            val uri = rasterUri(rasterImageBytes(4096, 1024, format), media)
            val source =
                """<svg xmlns="http://www.w3.org/2000/svg" width="80" height="80">
                <defs><clipPath id="clip"><rect width="64" height="64"/></clipPath></defs>
                <image x="8" y="8" width="64" height="64" transform="translate(0 0)"
                    clip-path="url(#clip)" href="$uri"/>
            </svg>"""
            // The resource decoder also supports a one-pixel share without source allocation.
            val raster = checkNotNull(SvgRasterSource.read(uri))
            val resources = checkNotNull(SvgImageResources.decode(listOf(raster), 1))
            try {
                assertEquals(1, resources.images.single().width)
                assertEquals(1, resources.images.single().height)
            } finally {
                resources.close()
            }
            val image = checkNotNull(decode(source, 128))
            assertEquals(80, image.width)
            assertEquals(80, image.height)
            val bitmap = image.bitmap
            assertEquals(0, Color.alpha(bitmap.getPixel(bitmap.width / 2, 1)))
            assertTrue(Color.blue(bitmap.getPixel(bitmap.width / 2, bitmap.height / 2)) > 240)
            assertEquals(0, Color.alpha(bitmap.getPixel(0, bitmap.height / 2)))
            bitmap.recycle()
            checkNotNull(decodeEpubImage(source.toByteArray(Charsets.UTF_16), 128, "image/svg+xml"))
                .bitmap
                .recycle()
        }
        // Exact URI repeats share one resource even when there is only a one-pixel budget.
        val repeated = checkNotNull(decodeEpubImage(embeddedSvgImageBytes(), 1, "image/svg+xml"))
        assertEquals(Color.BLUE, repeated.bitmap.getPixel(0, 0))
        repeated.bitmap.recycle()
    }

    @Test
    fun singlePixelRastersKeepOriginalLetterboxingAndImageOpacity() {
        val uri = rasterUri(rasterImageBytes(64, 16))
        val unused =
            (0 until 63).joinToString("") {
                "<image width='1' height='1' href='${rasterUri(rasterImageBytes(1, 1, color = Color.rgb(it, 0, 0)))}'/>"
            }
        // 64 unique sources share 64 temporary pixels. The visible 4:1 raster is 1x1,
        // while the final 8x8 output must still have transparent letterboxing.
        val source =
            """<svg width="64" height="64"><defs>$unused</defs>
            <image width="64" height="64" href="$uri" opacity="0.5"/></svg>"""
        val image = checkNotNull(decode(source, 64))
        assertEquals(8, image.bitmap.width)
        assertEquals(8, image.bitmap.height)
        assertEquals(0, Color.alpha(image.bitmap.getPixel(4, 1)))
        val center = image.bitmap.getPixel(4, 4)
        assertTrue(Color.alpha(center) in 126..129)
        assertEquals(255, Color.blue(center))
        image.bitmap.recycle()
    }

    @Test
    fun temporaryRastersShareAggregateBudgetResolveOnlyPrivateKeysAndRecycleOnClose() {
        val sources =
            listOf(
                checkNotNull(SvgRasterSource.read(rasterUri(rasterImageBytes(4096, 1024)))),
                checkNotNull(SvgRasterSource.read(rasterUri(rasterImageBytes(2048, 2048)))),
            )
        for (budget in listOf(2, 128, 16384, EPUB_PAGE_PIXELS)) {
            val resources = checkNotNull(SvgImageResources.decode(sources, budget))
            try {
                assertTrue(
                    resources.images.sumOf { it.allocationByteCount.toLong() } <= budget * 4L
                )
                assertTrue(
                    resources.images.all {
                        it.width <= EPUB_IMAGE_EDGE && it.height <= EPUB_IMAGE_EDGE
                    }
                )
                assertSame(resources.images[0], resources.resolveImage("yomoku-svg-image:0"))
                assertSame(
                    resources.resolveImage("yomoku-svg-image:0"),
                    resources.resolveImage("yomoku-svg-image:0"),
                )
                for (key in
                    listOf(
                        "0",
                        "yomoku-svg-image:00",
                        "yomoku-svg-image:-1",
                        "yomoku-svg-image:2",
                        "file:///tmp/a.png",
                        "https://example.org/a.png",
                        "data:image/png;base64,AA==",
                    )) {
                    assertNull(key, resources.resolveImage(key))
                }
                assertNull(resources.resolveCSSStyleSheet("file:///tmp/style.css"))
                assertNull(resources.resolveFont("file:///tmp/font.ttf", 400, "normal"))
            } finally {
                resources.close()
            }
            assertTrue(resources.images.all { it.isRecycled })
        }
        assertNull(SvgImageResources.decode(sources, 1))
        assertNull(SvgImageResources.decode(List(65) { sources[0] }, 100))
        val bad = SvgRasterSource(byteArrayOf(1, 2, 3), "image/png", 1, 1)
        assertNull(SvgImageResources.decode(sources + bad, 128))
    }

    @Test
    fun malformedExternalNestedHugeAmbiguousAndOverBudgetResourcesFailThenRetryWorks() {
        val png = rasterImageBytes()
        val uri = rasterUri(png)
        val huge = png.copyOf()
        java.nio.ByteBuffer.wrap(huge).putInt(16, 32769).putInt(20, 1)
        val crc = java.util.zip.CRC32().apply { update(huge, 12, 17) }
        java.nio.ByteBuffer.wrap(huge).putInt(29, crc.value.toInt())
        val hrefs =
            listOf(
                "",
                "file:///tmp/a.png",
                "https://example.org/a.png",
                "../a.png",
                "#a",
                "yomoku-svg-image:0",
                "data:image/png;base64,AA==",
                "data:image/png;base64,!!!",
                "data:image/png;base64,A===",
                "data:image/png,percent-encoded",
                rasterUri(rasterImageBytes(), "image/svg+xml"),
                rasterUri(png, "image/jpeg"),
                rasterUri(huge),
            )
        for (href in hrefs) {
            assertNull(
                href.take(100),
                decode(
                    "<svg width='64' height='32'><image width='64' height='32' href='$href'/></svg>"
                ),
            )
        }
        for (image in
            listOf(
                "<image width='64' height='32'/>",
                "<image href='$uri'/>",
                "<style>image.blue { opacity:0.5 }</style><image width='64' height='32' class='blue' href='$uri'/>",
                "<image width='64' height='32' href='$uri' xlink:href='$uri'/>",
                "<image width='64' height='32' bad:href='$uri'/>",
                "<image width='64' height='32' href='$uri'><image href='$uri'/></image>",
            )) {
            assertNull(
                decode(
                    "<svg xmlns:xlink='http://www.w3.org/1999/xlink' xmlns:bad='urn:bad' width='64' height='32'>$image</svg>"
                )
            )
        }
        val different = rasterUri(rasterImageBytes(color = Color.RED))
        assertNull(
            decode(
                "<svg width='64' height='32'><image width='64' height='32' href='$uri'/><image width='64' height='32' href='$different'/></svg>",
                1,
            )
        )
        val many =
            (0..64).joinToString("") {
                "<image width='1' height='1' href='${rasterUri(rasterImageBytes(1, 1, color = Color.rgb(it, 0, 0)))}'/>"
            }
        assertNull(decode("<svg width='64' height='32'>$many</svg>"))
        checkNotNull(decodeEpubImage(embeddedSvgImageBytes(), 128, "image/svg+xml"))
            .bitmap
            .recycle()
        checkNotNull(decodeEpubImage(svgImageBytes(), 128, "image/svg+xml")).bitmap.recycle()
    }

    @Test
    fun concurrentSessionsCannotExchangeGlobalResolversOrRecycledRasterPixels() {
        val executor = Executors.newFixedThreadPool(2)
        try {
            val jobs =
                listOf(Color.BLUE, Color.RED).map { color ->
                    Callable {
                        val source = embeddedSvgImageBytes(color)
                        repeat(12) {
                            val bytes = if (it % 2 == 0) source else nestedSvgImageBytes(color)
                            val image = checkNotNull(decodeEpubImage(bytes, 1024, "image/svg+xml"))
                            assertEquals(color, image.bitmap.getPixel(32, 8))
                            image.bitmap.recycle()
                            assertNull(
                                decode(
                                    "<svg width='10' height='10'><image href='data:image/png;base64,AA=='/></svg>"
                                )
                            )
                        }
                    }
                }
            executor.invokeAll(jobs).forEach { it.get(30, TimeUnit.SECONDS) }
        } finally {
            executor.shutdownNow()
        }
    }
}
