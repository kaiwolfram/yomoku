package app.yomoku

import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class LibraryCoversTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() =
        compose.waitUntil(90_000) {
            !core().view.storageBusy && !core().view.completionSavePending
        }

    private fun show(id: Long) {
        compose.onNodeWithTag("library_documents").performScrollToNode(hasTestTag("document_$id"))
    }

    private fun pixels(id: Long, ratio: Float) {
        show(id)
        compose.waitUntil(30_000) {
            compose
                .onAllNodesWithTag("library_cover_$id", useUnmergedTree = true)
                .fetchSemanticsNodes()
                .size == 1
        }
        val node = compose.onNodeWithTag("library_cover_$id", useUnmergedTree = true)
        val bounds = node.getUnclippedBoundsInRoot()
        assertEquals(
            ratio,
            (bounds.right - bounds.left).value / (bounds.bottom - bounds.top).value,
            0.12f,
        )
        // Returning from the reader can retain the row's finishing click ripple.
        compose.mainClock.advanceTimeBy(500)
        compose.waitForIdle()
        val pixels = node.captureToImage().toPixelMap()
        val center = pixels[pixels.width / 2, pixels.height / 2]
        assertEquals(1f, center.blue, 0.02f)
        assertEquals(0f, center.red, 0.02f)
    }

    @Test
    fun realRasterAndSvgCoversPreserveRatioRecreateAndOpenReaderWithoutChangingProgress() {
        idle()
        compose.runOnUiThread {
            if (core().view.settingsOpen) core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
        }
        idle()
        val images =
            listOf(
                Triple("png", "image/png", rasterImageBytes(600, 1200)),
                Triple("svg", "image/svg+xml", svgImageBytes()),
                Triple("embedded.svg", "image/svg+xml", embeddedSvgImageBytes()),
                Triple("nested.svg", "image/svg+xml", nestedSvgImageBytes()),
            )
        for ((extension, media, image) in images) {
            val title = "library-cover-$extension-${System.nanoTime()}"
            val path = "images/表紙.$extension"
            val bytes =
                FileImportFixtures.epub(
                    title,
                    images = mapOf(path to (media to image)),
                    coverPath = path,
                )
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub"))
                    )
            }
            idle()
            val id = checkNotNull(core().view.importSummary).documentId
            val reference = core().view.documents.single { it.documentId == id }
            val ratio = if (extension == "png") 0.5f else 4f
            compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
            pixels(id.value, ratio)
            compose.activityRule.scenario.recreate()
            pixels(id.value, ratio)
            assertEquals(reference, core().view.documents.single { it.documentId == id })
            compose.onNodeWithTag("document_${id.value}").performClick()
            compose.waitUntil(60_000) {
                core().view.reader?.documentId == id
            }
            assertEquals(0L, core().view.reader!!.position.offset.value)
            compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
            idle()
            pixels(id.value, ratio)
            assertEquals(
                reference.progressPercent,
                core().view.documents.single { it.documentId == id }.progressPercent,
            )
            assertFalse(core().view.documents.single { it.documentId == id }.isRead)
        }
    }

    @Test
    fun absentAndUnsupportedCoversUseGenericDocumentIconAndRemainOpenable() {
        idle()
        compose.runOnUiThread {
            if (core().view.settingsOpen) core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
        }
        idle()
        for (hasCover in listOf(false, true)) {
            val title = "library-fallback-$hasCover-${System.nanoTime()}"
            val bytes =
                FileImportFixtures.epub(
                    title,
                    images =
                        if (hasCover) mapOf("broken.png" to ("image/png" to byteArrayOf(1, 2, 3)))
                        else emptyMap(),
                    coverPath = if (hasCover) "broken.png" else null,
                )
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub"))
                    )
            }
            idle()
            val id = checkNotNull(core().view.importSummary).documentId
            compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
            show(id.value)
            compose
                .onNodeWithTag("library_cover_fallback_${id.value}", useUnmergedTree = true)
                .assertExists()
            compose
                .onNodeWithTag("library_cover_${id.value}", useUnmergedTree = true)
                .assertDoesNotExist()
            compose.onNodeWithTag("document_${id.value}").performClick()
            compose.waitUntil(60_000) {
                core().view.reader?.documentId == id
            }
            assertNull(core().view.documentError)
            compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
            idle()
        }
    }
}
