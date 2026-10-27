package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.font.FontWeight
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderImagesTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private val textNodes =
        SemanticsMatcher("reader text fragments") {
            it.config.getOrElse(SemanticsProperties.TestTag) { "" }.startsWith("reader_text_")
        }

    private fun awaitReader() {
        compose.waitUntil(60_000) {
            val view = core().view
            !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending &&
                view.readerTime != null &&
                view.readerTokens != null
        }
        compose.waitForIdle()
    }

    private fun awaitImages(count: Int) {
        compose.waitUntil(30_000) {
            (0 until count).all {
                compose.onAllNodesWithTag("reader_image_$it").fetchSemanticsNodes().size == 1
            }
        }
        compose.waitForIdle()
    }

    private fun fragments() =
        compose.onAllNodes(textNodes).fetchSemanticsNodes().map {
            it.config[SemanticsProperties.Text].single()
        }

    private fun tapWord(word: String) {
        val node = compose.onNode(textNodes and hasText(word, substring = true)).performScrollTo()
        val text = node.fetchSemanticsNode().config[SemanticsProperties.Text].single()
        val layouts = mutableListOf<androidx.compose.ui.text.TextLayoutResult>()
        node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action!!(layouts)
        val bounds = layouts.single().getBoundingBox(text.text.indexOf(word))
        node.performTouchInput { click(bounds.center) }
    }

    @Test
    fun realEpubSvgImagesKeepSourceOrderRatioStylesWordIdentityRecreationAndChapterNavigation() =
        verifyInlineImages(svgImageBytes())

    @Test
    fun embeddedSvgRastersKeepWordAndLinkIdentityViewerRecreationAndNavigation() =
        verifyInlineImages(embeddedSvgImageBytes())

    @Test
    fun nestedSvgImagesKeepWordAndLinkIdentityViewerRecreationAndNavigation() =
        verifyInlineImages(nestedSvgImageBytes())

    private fun verifyInlineImages(svgBytes: ByteArray) {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "inline-images-${System.nanoTime()}"
        val path = "images/挿絵.svg"
        val bytes =
            FileImportFixtures.epub(
                title,
                """<p><strong>😀𠮷前<img src='$path' alt='一枚目'/>後</strong> 猫。</p>
            <p><a href='https://example.org/'>外部<img src='$path' alt='二枚目'/>site</a> <a href='two.xhtml'>note</a></p>""",
                images = mapOf(path to ("image/svg+xml" to svgBytes)),
                secondBody = "<img src='$path' alt='絵だけの章'/>",
            )
        compose.runOnUiThread {
            core().dispatch(Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub")))
        }
        compose.waitUntil(90_000) { !core().view.storageBusy }
        val id = core().view.importSummary!!.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitReader()
        awaitImages(2)
        val first = core().view.reader!!
        assertEquals(2, first.images.size)
        assertEquals(first.images[0].asset, first.images[1].asset)
        assertEquals(first.text.replace("\uFFFC", ""), fragments().joinToString("") { it.text })
        assertTrue(fragments().first().spanStyles.any { it.item.fontWeight == FontWeight.Bold })
        assertTrue(
            fragments().drop(1).any { text ->
                text.spanStyles.any { it.item.fontWeight == FontWeight.Bold }
            }
        )
        assertEquals(
            2,
            fragments().sumOf {
                it.getLinkAnnotations(0, it.length).count { span ->
                    span.item is LinkAnnotation.Url
                }
            },
        )
        val sourceOrder =
            compose.onAllNodes(textNodes).fetchSemanticsNodes().map { it.positionInRoot.y }
        val image0 = compose.onNodeWithTag("reader_image_0").fetchSemanticsNode().positionInRoot.y
        val image1 = compose.onNodeWithTag("reader_image_1").fetchSemanticsNode().positionInRoot.y
        assertTrue(sourceOrder[0] < image0)
        assertTrue(image0 < sourceOrder[1])
        assertTrue(sourceOrder[1] < image1)
        assertTrue(image1 < sourceOrder[2])
        for (index in 0..1) {
            val node = compose.onNodeWithTag("reader_image_$index").performScrollTo()
            node.assertContentDescriptionEquals(if (index == 0) "一枚目" else "二枚目")
            val bounds = node.getUnclippedBoundsInRoot()
            assertEquals(
                4f,
                (bounds.right - bounds.left).value / (bounds.bottom - bounds.top).value,
                0.03f,
            )
            val pixels = node.captureToImage().toPixelMap()
            val center = pixels[pixels.width / 2, pixels.height / 2]
            assertEquals(1f, center.blue, 0.01f)
            assertEquals(0f, center.red, 0.01f)
        }
        assertEquals(first.position, core().view.reader!!.position)
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val monitor =
            instrumentation.addMonitor(
                IntentFilter(Intent.ACTION_VIEW).apply {
                    addCategory(Intent.CATEGORY_BROWSABLE)
                    addDataScheme("https")
                },
                ActivityResult(Activity.RESULT_OK, null),
                true,
            )
        try {
            // An SVG's own anchor and the surrounding EPUB anchor stay inert on image taps.
            compose.onNodeWithTag("reader_image_1").performScrollTo().performClick()
            compose.onNodeWithTag("image_viewer_canvas").assertContentDescriptionEquals("二枚目")
            compose.onNodeWithTag("image_viewer_zoom_in").performClick()
            val pixels = compose.onNodeWithTag("image_viewer_canvas").captureToImage().toPixelMap()
            assertEquals(1f, pixels[pixels.width / 2, pixels.height / 2].blue, 0.01f)
            assertEquals(0, monitor.hits)
            compose.onNodeWithTag("image_viewer_close").performClick()
            tapWord("site")
            assertEquals(1, monitor.hits)
            tapWord("note")
            assertEquals(1, monitor.hits)
            assertEquals(first.position, core().view.reader!!.position)
            assertNull(core().view.readerWordSelection)
        } finally {
            instrumentation.removeMonitor(monitor)
        }
        tapWord("猫")
        compose.onNodeWithTag("word_surface").assertTextEquals("猫")
        compose.onNodeWithText("Close").performClick()
        compose.activityRule.scenario.recreate()
        awaitReader()
        awaitImages(2)
        assertEquals(first.position, core().view.reader!!.position)
        compose.onNodeWithTag("next_page").performScrollTo().performClick()
        compose.waitUntil(30_000) { core().view.reader?.pageNumber == 2UL }
        awaitReader()
        awaitImages(1)
        assertEquals("\uFFFC", core().view.reader!!.text.trim())
        compose.onNodeWithTag("reader_image_0").assertContentDescriptionEquals("絵だけの章")
        compose.onNodeWithTag("previous_page").performScrollTo().performClick()
        compose.waitUntil(30_000) { core().view.reader?.pageNumber == 1UL }
        awaitReader()
        awaitImages(2)
        assertEquals(first.anchor, core().view.reader!!.anchor)
        compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
        compose.waitUntil(30_000) {
            !core().view.storageBusy && !core().view.readingTimeSavePending
        }
        compose.onNodeWithTag("page_length_250").performClick()
        compose.waitUntil(30_000) { !core().view.storageBusy }
        compose.onNodeWithText("SVG renderer credits").performScrollTo().performClick()
        compose.waitUntil(30_000) {
            compose.onAllNodesWithTag("svg_notices_text").fetchSemanticsNodes().any {
                it.config[SemanticsProperties.Text]
                    .single()
                    .text
                    .contains("END OF TERMS AND CONDITIONS")
            }
        }
        compose.activityRule.scenario.recreate()
        compose.waitUntil(30_000) {
            compose.onAllNodesWithTag("svg_notices_text").fetchSemanticsNodes().any {
                it.config[SemanticsProperties.Text]
                    .single()
                    .text
                    .contains("Copyright 2013 Paul LeBeau")
            }
        }
        compose.onNodeWithText("Close").performClick()
        compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
        awaitReader()
        awaitImages(2)
        assertEquals(first.position, core().view.reader!!.position)
    }

    @Test
    fun unsupportedImageLeavesAltAndRetryWhileCompletionStillWorks() {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "unsupported-image-${System.nanoTime()}"
        val bytes =
            FileImportFixtures.epub(
                title,
                "<img src='image.svg' alt='図の説明'/>",
                images =
                    mapOf(
                        "image.svg" to
                            ("image/svg+xml" to
                                "<svg xmlns='http://www.w3.org/2000/svg'/>".toByteArray())
                    ),
            )
        compose.runOnUiThread {
            core().dispatch(Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub")))
        }
        compose.waitUntil(90_000) { !core().view.storageBusy }
        val id = core().view.importSummary!!.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitReader()
        compose.waitUntil(30_000) {
            compose.onAllNodesWithText("Image unavailable.").fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText("図の説明").assertIsDisplayed()
        val position = core().view.reader!!.position
        compose.onNodeWithText("Retry").performClick()
        compose.waitUntil(30_000) {
            compose.onAllNodesWithText("Image unavailable.").fetchSemanticsNodes().isNotEmpty()
        }
        assertEquals(position, core().view.reader!!.position)
        compose.onNodeWithTag("next_page").performScrollTo().performClick()
        compose.waitUntil(30_000) { core().view.reader?.pageNumber == 2UL }
        awaitReader()
        compose.onNodeWithTag("reader_image_content").assertDoesNotExist()
    }
}
