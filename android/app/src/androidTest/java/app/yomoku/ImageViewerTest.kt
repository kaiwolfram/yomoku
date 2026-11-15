package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import android.graphics.Bitmap
import android.os.ParcelFileDescriptor
import android.view.KeyEvent
import androidx.activity.compose.setContent
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ImageViewerTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitReader() {
        compose.waitUntil(60_000) {
            val view = core().view
            !view.storageBusy && !view.completionSavePending && view.effectiveReaderTokens != null
        }
        compose.waitUntil(30_000) {
            compose.onAllNodesWithTag("reader_image_0").fetchSemanticsNodes().size == 1
        }
        compose.waitForIdle()
    }

    private fun importAndOpen() {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val bitmap = Bitmap.createBitmap(160, 80, Bitmap.Config.ARGB_8888)
        for (x in 0 until 160) for (y in 0 until 80) {
            bitmap.setPixel(
                x,
                y,
                if (x < 80) android.graphics.Color.RED else android.graphics.Color.BLUE,
            )
        }
        val bytes =
            ByteArrayOutputStream().use {
                assertTrue(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it))
                bitmap.recycle()
                it.toByteArray()
            }
        val title = "image-viewer-${System.nanoTime()}"
        val epub =
            FileImportFixtures.epub(
                title,
                "<a href='https://example.org/'><img src='image.png' alt='赤と青'/></a><p>猫。</p>",
                images = mapOf("image.png" to ("image/png" to bytes)),
                secondBody = "<img src='image.png' alt='次の絵'/>",
            )
        compose.runOnUiThread {
            core().dispatch(Event.ImportEpub(EpubImport(epub.map { it.toUByte() }, "$title.epub")))
        }
        compose.waitUntil(90_000) { !core().view.storageBusy }
        val id = core().view.importSummary!!.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitReader()
    }

    private fun openViewer() {
        compose.onNodeWithTag("reader_image_0").performScrollTo().performClick()
        compose.waitUntil(30_000) {
            compose.onAllNodesWithTag("image_viewer_canvas").fetchSemanticsNodes().size == 1
        }
        compose.onNodeWithTag("image_viewer").assertIsDisplayed()
        val bounds = compose.onNodeWithTag("image_viewer").fetchSemanticsNode().boundsInRoot
        val window = compose.activity.windowManager.currentWindowMetrics.bounds
        assertEquals(window.width().toFloat(), bounds.width, 2f)
        assertEquals(window.height().toFloat(), bounds.height, 2f)
    }

    private fun zoom() =
        compose
            .onNodeWithTag("image_viewer_canvas")
            .fetchSemanticsNode()
            .config[SemanticsProperties.StateDescription]
            .removeSuffix("%")
            .toInt()

    private fun shell(command: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(
                instrumentation.uiAutomation.executeShellCommand(command)
            )
            .bufferedReader()
            .use { it.readText().trim() }

    @Test
    fun linkedImageOpensViewerPinchPanAndBackNeverNavigateOrCountReading() {
        importAndOpen()
        val before = core().view.reader!!
        val tokens = core().view.effectiveReaderTokens
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
            openViewer()
            compose.onNodeWithTag("image_viewer_canvas").assertContentDescriptionEquals("赤と青")
            compose.onNodeWithTag("image_viewer_zoom_out").assertIsNotEnabled()
            compose.onNodeWithTag("image_viewer_canvas").performTouchInput {
                pinch(
                    start0 = Offset(width * .4f, height * .5f),
                    end0 = Offset(width * .2f, height * .5f),
                    start1 = Offset(width * .6f, height * .5f),
                    end1 = Offset(width * .8f, height * .5f),
                )
            }
            assertTrue("Pinch must increase zoom", zoom() > 150)
            compose.onNodeWithTag("image_viewer_reset").performClick()
            assertEquals(100, zoom())
            compose.onNodeWithTag("image_viewer_zoom_in").performClick()
            assertEquals(200, zoom())
            compose.onNodeWithTag("image_viewer_canvas").performTouchInput { swipeLeft() }
            val pixels = compose.onNodeWithTag("image_viewer_canvas").captureToImage().toPixelMap()
            val sample = pixels[(pixels.width * .4f).toInt(), pixels.height / 2]
            assertEquals("Panning should reveal the blue half", 1f, sample.blue, 0.02f)
            assertEquals(0f, sample.red, 0.02f)
            assertEquals(before.position, core().view.reader!!.position)
            assertEquals(tokens, core().view.effectiveReaderTokens)
            assertFalse(core().view.reader!!.finishedAtLeastOnce)
            assertEquals(0, monitor.hits)
            instrumentation.sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
            compose.onNodeWithTag("image_viewer").assertDoesNotExist()
            assertEquals(ContentRoute.READER, core().view.contentRoute)
            assertEquals(before.position, core().view.reader!!.position)
            openViewer()
            assertEquals(100, zoom())
            compose.onNodeWithTag("image_viewer_close").performClick()
            compose.onNodeWithTag("image_viewer").assertDoesNotExist()
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }

    @Test
    fun viewerSelectionAndZoomSurviveRotationAndRecreationThenClearOnChapterChange() {
        val rotation = shell("wm user-rotation")
        try {
            shell("wm user-rotation lock 0")
            importAndOpen()
            val before = core().view.reader!!.position
            openViewer()
            repeat(2) { compose.onNodeWithTag("image_viewer_zoom_in").performClick() }
            assertEquals(400, zoom())
            compose.onNodeWithTag("image_viewer_canvas").performTouchInput { swipeLeft() }
            val orientation = compose.activity.resources.configuration.orientation
            shell("wm user-rotation lock 1")
            compose.waitUntil(30_000) {
                compose.activity.resources.configuration.orientation != orientation
            }
            awaitReader()
            compose.waitUntil(30_000) {
                compose.onAllNodesWithTag("image_viewer_canvas").fetchSemanticsNodes().size == 1
            }
            assertEquals(400, zoom())
            assertEquals(before, core().view.reader!!.position)
            assertBlueCenter()
            compose.activityRule.scenario.recreate()
            awaitReader()
            compose.waitUntil(30_000) {
                compose.onAllNodesWithTag("image_viewer_canvas").fetchSemanticsNodes().size == 1
            }
            assertEquals(400, zoom())
            assertEquals(before, core().view.reader!!.position)
            assertBlueCenter()
            // An external page change must dispose the modal and its source identity too.
            val page = core().view.reader!!
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.CompleteEffectiveReaderPage(
                            core().view.effectiveReaderTokens!!.identity,
                            page.position,
                            System.currentTimeMillis(),
                        )
                    )
            }
            compose.waitUntil(30_000) { core().view.reader?.pageNumber == 2UL }
            awaitReader()
            compose.onNodeWithTag("image_viewer").assertDoesNotExist()
            openViewer()
            assertEquals(100, zoom())
            compose.onNodeWithTag("image_viewer_canvas").assertContentDescriptionEquals("次の絵")
            compose.onNodeWithTag("image_viewer_close").performClick()
        } finally {
            shell("wm user-rotation $rotation")
        }
    }

    private fun assertBlueCenter() {
        val pixels = compose.onNodeWithTag("image_viewer_canvas").captureToImage().toPixelMap()
        val center = pixels[pixels.width / 2, pixels.height / 2]
        assertEquals("Panned source focus must survive recreation", 1f, center.blue, 0.02f)
        assertEquals(0f, center.red, 0.02f)
    }

    @Test
    fun sharedViewerKeepsLoadingAndFailureModalAllowsRetry() {
        val image = mutableStateOf<androidx.compose.ui.graphics.ImageBitmap?>(null)
        val loading = mutableStateOf(true)
        var retries = 0
        var closes = 0
        compose.runOnUiThread {
            compose.activity.setContent {
                ImageViewer(
                    image.value,
                    Size(64f, 32f),
                    "shared image",
                    loading.value,
                    { retries++ },
                    { closes++ },
                )
            }
        }
        compose.onNodeWithText("Loading image…").assertIsDisplayed()
        compose.onNodeWithTag("image_viewer_zoom_in").assertIsNotEnabled()
        compose.runOnUiThread { loading.value = false }
        compose.onNodeWithText("Image unavailable.").assertIsDisplayed()
        compose.onNodeWithText("Retry").performClick()
        assertEquals(1, retries)
        val bitmap = Bitmap.createBitmap(64, 32, Bitmap.Config.ARGB_8888)
        compose.runOnUiThread { image.value = bitmap.asImageBitmap() }
        compose.onNodeWithTag("image_viewer_zoom_in").performClick()
        compose.onNodeWithTag("image_viewer_canvas").performTouchInput { swipeLeft() }
        repeat(8) { compose.onNodeWithTag("image_viewer_zoom_in").performClick() }
        assertEquals(800, zoom())
        compose.onNodeWithTag("image_viewer_zoom_in").assertIsNotEnabled()
        compose.onNodeWithTag("image_viewer_reset").performClick()
        assertEquals(100, zoom())
        compose.onNodeWithTag("image_viewer_close").performClick()
        assertEquals(1, closes)
    }
}
