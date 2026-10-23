package app.yomoku

import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextReplacement
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.shared.TextOffset
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PasteReaderTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core(): CoreSession =
        ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitStorage() {
        compose.waitUntil(10_000) { !core().view.storageBusy }
        compose.waitForIdle()
    }

    private fun assertReaderAtTop() {
        val range =
            compose
                .onNodeWithTag("reader_scroll")
                .fetchSemanticsNode()
                .config[SemanticsProperties.VerticalScrollAxisRange]
        assertEquals(0f, range.value(), 0f)
    }

    @Test
    fun savedOffsetsUpdateLibraryAndReopenContainingPageAtTop() {
        awaitStorage()
        val title = "位置 ${System.nanoTime()}"
        val first = "$title\n${"𠮷".repeat(3000)}\n"
        val second = "${"次".repeat(3000)}\n"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(first + second + "終。")) }
        awaitStorage()
        val id = core().view.documents.single { it.title == title }.documentId
        compose.onNodeWithTag("progress_${id.value}", useUnmergedTree = true).assertTextEquals("0%")
        compose.onNodeWithTag("document_${id.value}").performScrollTo().performClick()
        awaitStorage()
        val start = core().view.reader!!.position
        compose.onNodeWithTag("page_counter").performScrollTo()
        val scroll =
            compose
                .onNodeWithTag("reader_scroll")
                .fetchSemanticsNode()
                .config[SemanticsProperties.VerticalScrollAxisRange]
        assertTrue(scroll.value() > 0f)
        val position =
            start.copy(offset = TextOffset(first.toByteArray(Charsets.UTF_8).size.toLong() + 3))
        compose.runOnUiThread { core().dispatch(Event.SaveReadingPosition(position)) }
        awaitStorage()
        compose.onNodeWithTag("reader_text").assertTextEquals(second)
        assertReaderAtTop()
        val percent = core().view.documents.single { it.documentId == id }.progressPercent
        compose.onNodeWithText("Back").performClick()
        compose
            .onNodeWithTag("progress_${id.value}", useUnmergedTree = true)
            .assertTextEquals("$percent%")
        compose.onNodeWithTag("document_${id.value}").performScrollTo().performClick()
        awaitStorage()
        assertEquals(position, core().view.reader!!.position)
        assertReaderAtTop()
        compose.onNodeWithTag("page_counter").performScrollTo().assertTextEquals("2 / 3")
        compose.activityRule.scenario.recreate()
        assertReaderAtTop()
        compose.runOnUiThread { core().dispatch(Event.SaveReadingPosition(start)) }
        awaitStorage()
        compose.onNodeWithTag("reader_text").assertTextEquals(first)
        assertReaderAtTop()
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithTag("progress_${id.value}", useUnmergedTree = true).assertTextEquals("0%")
    }

    @Test
    fun pasteReturnsToLibraryAndOpensFirstLogicalPageWithBackAndRecreation() {
        awaitStorage()
        val title = "題𠮷 ${System.nanoTime()}"
        val firstPage = "$title\n${"𠮷".repeat(500)}\n"
        val text = firstPage + "次の段落。"
        compose.onNodeWithText("Paste text").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Paste text")
        compose.onNodeWithText("Save to Library").assertIsNotEnabled()
        compose.onNodeWithTag("paste_text").performTextReplacement(text)

        // Retain an unsaved draft through Settings and Activity recreation.
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Back").performClick()
        compose.activityRule.scenario.recreate()
        compose.onNodeWithTag("paste_text").assertTextEquals("Text to import", text)

        // A typed repository error leaves the draft available for retry.
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(" \n")) }
        awaitStorage()
        compose.onNodeWithTag("document_error").assertTextEquals("Enter some text to import.")
        compose.onNodeWithTag("paste_text").assertTextEquals("Text to import", text)
        compose.onNodeWithText("Save to Library").performClick()
        awaitStorage()
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
        compose.onNodeWithTag("reader_text").assertDoesNotExist()
        var documentTag = ""
        compose.runOnUiThread {
            val view = core().view
            assertEquals(ContentRoute.MAIN, view.contentRoute)
            documentTag = "document_${view.documents.single { it.title == title }.documentId.value}"
        }
        compose.onNodeWithTag(documentTag).performScrollTo().performClick()
        awaitStorage()
        compose.onNodeWithTag("screen_title").assertTextEquals(title)
        compose.onNodeWithTag("reader_text").assertTextEquals(firstPage)
        compose.onNodeWithTag("bottom_navigation").assertDoesNotExist()
        compose.onNodeWithTag("navigation_rail").assertDoesNotExist()
        compose.onNodeWithTag("page_counter").performScrollTo().assertTextEquals("1 / 2")
        compose.activityRule.scenario.recreate()
        compose.onNodeWithTag("reader_text").assertTextEquals(firstPage)
        compose.runOnUiThread { assertNotNull(core().view.reader) }
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals(title)
        compose.waitForIdle()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
        compose.onNodeWithTag(documentTag).performScrollTo().performClick()
        awaitStorage()
        compose.onNodeWithTag("reader_text").assertIsDisplayed().assertTextEquals(firstPage)
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
    }
}
