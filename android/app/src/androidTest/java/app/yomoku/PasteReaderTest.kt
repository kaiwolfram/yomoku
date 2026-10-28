package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToKey
import androidx.compose.ui.test.performTextReplacement
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import app.yomoku.shared.TextOffset
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertSame
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
        compose.waitUntil(30_000) { !core().view.storageBusy }
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

    private fun awaitDictionaryMapping(expected: Boolean) {
        val deadline = System.nanoTime() + 30_000_000_000L
        while (File("/proc/self/maps").readText().contains("/sudachi/full-") != expected) {
            check(System.nanoTime() < deadline) { "Unexpected production dictionary mapping" }
            Thread.sleep(10)
        }
    }

    @Test
    fun readerClockPausesInBackgroundAndRetainsDurationsThroughRecreation() {
        awaitStorage()
        val title = "Reader clock ${System.nanoTime()}"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste("$title\n猫と犬。")) }
        awaitStorage()
        val id = core().view.documents.single { it.title == title }.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        compose.waitUntil(30_000) { !core().view.storageBusy && core().view.readerTime != null }
        val retained = core()
        Thread.sleep(150)
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        val instrumentation =
            androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        fun saved(): Long {
            var result = -1L
            instrumentation.runOnMainSync {
                if (!retained.view.storageBusy && !retained.view.readingTimeSavePending) {
                    result = retained.view.readerTime!!.pending.value
                }
            }
            return result
        }
        val deadline = System.nanoTime() + 30_000_000_000L
        var paused = saved()
        while (paused < 0) {
            check(System.nanoTime() < deadline)
            Thread.sleep(10)
            paused = saved()
        }
        assertTrue(paused >= 150L)
        Thread.sleep(200)
        assertEquals(paused, saved())
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        compose.activityRule.scenario.recreate()
        assertSame(retained, core())
        Thread.sleep(150)
        compose.onNodeWithText("Settings").performClick()
        awaitStorage()
        val after = core().view.readerTime!!.pending.value
        assertTrue(after >= paused + 150L)
        Thread.sleep(200)
        assertEquals(after, core().view.readerTime!!.pending.value)
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithText("Back").performClick()
        awaitStorage()
    }

    @Test
    fun readerAndSettingsOfferRetryForFailedTimeRestoreAndFinalSave() {
        awaitStorage()
        val title = "Reader retry ${System.nanoTime()}"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste("$title\n猫と犬。")) }
        awaitStorage()
        val id = core().view.documents.single { it.title == title }.documentId
        val database = File(compose.activity.filesDir, "yomoku.sqlite3")
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use { db ->
            try {
                db.execSQL(
                    "CREATE TRIGGER fail_clock_restore BEFORE INSERT ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'test restore'); END"
                )
                compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && core().view.documentError != null
                }
                compose.onNodeWithTag("document_error").assertIsDisplayed()
                db.execSQL("DROP TRIGGER fail_clock_restore")
                compose.onNodeWithText("Retry").performClick()
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && core().view.readerTime != null
                }
                db.execSQL(
                    "CREATE TRIGGER fail_clock_save BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'test save'); END"
                )
                Thread.sleep(100)
                compose.onNodeWithText("Settings").performClick()
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && core().view.documentError != null
                }
                compose.onNodeWithTag("page_length_250").assertIsNotEnabled()
                db.execSQL("DROP TRIGGER fail_clock_save")
                compose.onNodeWithText("Retry").performClick()
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && !core().view.readingTimeSavePending
                }
                compose.onNodeWithTag("document_error").assertDoesNotExist()
                compose.onNodeWithText("Back").performClick()
                compose.onNodeWithText("Back").performClick()
                awaitStorage()
            } finally {
                db.execSQL("DROP TRIGGER IF EXISTS fail_clock_restore")
                db.execSQL("DROP TRIGGER IF EXISTS fail_clock_save")
            }
        }
    }

    @Test
    fun pageLengthSettingsReflowWithoutMovingTheExactSavedPosition() {
        awaitStorage()
        val original = core().view.pageLength!!
        try {
            compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
            compose.onNodeWithTag("page_length_500").performScrollTo().performClick()
            awaitStorage()
            compose.onNodeWithTag("page_length_500").assertIsSelected()
            compose.onNodeWithText("Back").performClick()

            val title = "Reflow ${System.nanoTime()}"
            val paragraph = "${"𠮷".repeat(249)}。\n"
            val text = "$title\n" + paragraph.repeat(10)
            compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
            awaitStorage()
            val id = core().view.documents.single { it.title == title }.documentId
            compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
            compose.onNodeWithTag("document_${id.value}").performScrollTo().performClick()
            awaitStorage()
            val position =
                core()
                    .view
                    .reader!!
                    .position
                    .copy(
                        offset =
                            TextOffset(
                                ("$title\n" + paragraph).toByteArray(Charsets.UTF_8).size.toLong() +
                                    4
                            )
                    )
            compose.runOnUiThread { core().dispatch(Event.SaveReadingPosition(position)) }
            awaitStorage()
            val progress = core().view.documents.single { it.documentId == id }.progressPercent
            assertEquals(1UL, core().view.reader!!.pageNumber)
            compose.onNodeWithTag("page_counter").performScrollTo()
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("page_length_250").performScrollTo().performClick()
            awaitStorage()
            compose.onNodeWithTag("page_length_250").assertIsSelected()
            compose.activityRule.scenario.recreate()
            compose.onNodeWithTag("page_length_250").assertIsSelected()
            compose.onNodeWithText("Back").performClick()
            assertEquals(position, core().view.reader!!.position)
            assertEquals(2UL, core().view.reader!!.pageNumber)
            compose.onNodeWithTag("reader_text").assertTextEquals(paragraph)
            assertReaderAtTop()

            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("page_length_1500").performScrollTo().performClick()
            awaitStorage()
            compose.onNodeWithText("Back").performClick()
            assertEquals(position, core().view.reader!!.position)
            assertEquals(1UL, core().view.reader!!.pageNumber)
            compose.onNodeWithTag("reader_text").assertTextEquals("$title\n" + paragraph.repeat(6))
            assertReaderAtTop()
            assertEquals(
                progress,
                core().view.documents.single { it.documentId == id }.progressPercent,
            )
            compose.onNodeWithText("Back").performClick()
            compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
            compose.onNodeWithTag("document_${id.value}").performScrollTo().performClick()
            awaitStorage()
            assertEquals(position, core().view.reader!!.position)
            assertEquals(PageLength.CHARACTERS1500, core().view.pageLength)
            assertReaderAtTop()
        } finally {
            compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
            compose.runOnUiThread { core().dispatch(Event.SetPageLength(original)) }
            awaitStorage()
            compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
        }
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
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
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
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
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
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
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
        compose.onNodeWithTag("import_summary_message").assertTextEquals("Imported: $title")
        awaitDictionaryMapping(true)
        compose.activityRule.scenario.recreate()
        awaitDictionaryMapping(true) // warm cache survives configuration changes
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        awaitDictionaryMapping(false)
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        awaitDictionaryMapping(false) // returning to Library never eagerly loads Full
        // Reimport through the actual form: same row, visible duplicate result, no reader.
        compose.onNodeWithText("Paste text").performClick()
        compose.onNodeWithTag("paste_text").performTextReplacement(text)
        compose.onNodeWithText("Save to Library").performClick()
        awaitStorage()
        compose
            .onNodeWithTag("import_summary_message")
            .assertTextEquals("Already in Library: $title")
        awaitDictionaryMapping(true)
        assertEquals(1, core().view.documents.count { it.title == title })
        compose.onNodeWithTag("reader_text").assertDoesNotExist()
        compose.activityRule.scenario.recreate()
        compose
            .onNodeWithTag("import_summary_message")
            .assertTextEquals("Already in Library: $title")
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Back").performClick()
        compose
            .onNodeWithTag("import_summary_message")
            .assertTextEquals("Already in Library: $title")
        compose.onNodeWithText("Close").performClick()
        compose.onNodeWithTag("import_summary").assertDoesNotExist()
        compose.activityRule.scenario.recreate()
        compose.onNodeWithTag("import_summary").assertDoesNotExist()
        var documentTag = ""
        compose.runOnUiThread {
            val view = core().view
            assertEquals(ContentRoute.MAIN, view.contentRoute)
            documentTag = "document_${view.documents.single { it.title == title }.documentId.value}"
        }
        compose
            .onNodeWithTag("library_documents")
            .performScrollToKey(core().view.documents.single { it.title == title }.documentId.value)
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
        compose
            .onNodeWithTag("library_documents")
            .performScrollToKey(core().view.documents.single { it.title == title }.documentId.value)
        compose.onNodeWithTag(documentTag).performScrollTo().performClick()
        awaitStorage()
        compose.onNodeWithTag("reader_text").assertIsDisplayed().assertTextEquals(firstPage)
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
    }
}
