package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DocumentRenameTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core().view
            !view.storageBusy &&
                !view.completionSavePending &&
                (view.libraryWordDistributions != null || view.libraryWordError != null)
        }
    }

    private fun db() =
        SQLiteDatabase.openDatabase(
            File(compose.activity.filesDir, "yomoku.sqlite3").path,
            null,
            SQLiteDatabase.OPEN_READWRITE,
        )

    private fun sourceAndHistory(id: Long): List<String> =
        db().use { database ->
            database
                .rawQuery(
                    "SELECT s.original_title, t.text, (SELECT COUNT(*) FROM reading_completions WHERE document_id = d.id), (SELECT COUNT(*) FROM reading_occurrences o JOIN reading_completions r ON r.command_id = o.command_id WHERE r.document_id = d.id) FROM documents d JOIN sources s ON s.id = d.source_id JOIN chapters c ON c.source_id = s.id JOIN chapter_texts t ON t.chapter_id = c.id WHERE d.id = ?",
                    arrayOf(id.toString()),
                )
                .use { cursor ->
                    assertTrue(cursor.moveToFirst())
                    (0..3).map { cursor.getString(it) }
                }
        }

    @Test
    fun renameRetainsLiteralDraftAcrossRecreationCancelAndFailureRetryWithoutResettingBook() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
        }
        idle()
        val originalTitle = "Rename-${System.nanoTime()}"
        val text = "$originalTitle\n猫 猫 犬"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val id = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread {
            core().dispatch(Event.DismissImportSummary)
        }
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
        compose.onNodeWithTag("document_${id.value}").performClick()
        compose.waitUntil(90_000) {
            core().view.effectiveReaderTokens != null && !core().view.storageBusy
        }
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        val before = core().view.documents.single { it.documentId == id }
        val history = sourceAndHistory(id.value)
        val counts = core().view.libraryWordDistributions
        assertTrue(before.isRead)
        assertEquals("1", history[2])
        val title = "  改題𠮷 か\u3099  "
        compose.onNodeWithTag("document_${id.value}").performTouchInput { longClick() }
        compose.onNodeWithTag("rename_document_${id.value}").performClick()
        compose.onNodeWithTag("document_title_draft").assertTextContains(originalTitle)
        compose.onNodeWithTag("document_title_draft").performTextReplacement(" \n\t")
        compose.onNodeWithTag("save_document_title").assertIsNotEnabled()
        compose.onNodeWithTag("document_title_draft").performTextReplacement(title)
        val session = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertSame(session, core())
        compose.onNodeWithTag("document_title_draft").assertTextContains(title)
        compose.onNodeWithTag("cancel_document_rename").performClick()
        assertNull(core().view.libraryRename)
        assertEquals(before, core().view.documents.single { it.documentId == id })
        assertEquals(history, sourceAndHistory(id.value))
        db().use {
            it.execSQL(
                "CREATE TRIGGER fail_ui_document_rename BEFORE UPDATE OF title ON documents WHEN OLD.id = ${id.value} BEGIN SELECT RAISE(ABORT, 'fixture'); END"
            )
        }
        try {
            compose.onNodeWithTag("document_${id.value}").performTouchInput { longClick() }
            compose.onNodeWithTag("rename_document_${id.value}").performClick()
            compose.onNodeWithTag("document_title_draft").performTextReplacement(title)
            compose.onNodeWithTag("save_document_title").performClick()
            compose.waitUntil(90_000) { core().view.libraryRename?.error != null }
            compose
                .onNodeWithText("Could not rename this document. Retry or cancel.")
                .assertIsDisplayed()
            assertEquals(before, core().view.documents.single { it.documentId == id })
            compose.activityRule.scenario.recreate()
            compose.waitForIdle()
            compose.onNodeWithTag("document_title_draft").assertTextContains(title)
            compose.onNodeWithTag("save_document_title").assertTextEquals("Retry")
        } finally {
            db().use { it.execSQL("DROP TRIGGER fail_ui_document_rename") }
        }
        compose.onNodeWithTag("save_document_title").performClick()
        compose.waitUntil(90_000) { core().view.libraryRename == null }
        idle()
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
        compose.onNodeWithTag("document_${id.value}").assertIsDisplayed()
        idle()
        val renamed = core().view.documents.single { it.documentId == id }
        assertEquals(before.copy(title = title), renamed)
        assertEquals(counts, core().view.libraryWordDistributions)
        assertEquals(history, sourceAndHistory(id.value))
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val summary = checkNotNull(core().view.importSummary)
        assertEquals(ImportOutcome.EXISTING, summary.outcome)
        assertEquals(id, summary.documentId)
        assertEquals(title, summary.title)
        compose.runOnUiThread {
            core().dispatch(Event.DismissImportSummary)
        }
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
        compose.onNodeWithTag("document_${id.value}").performClick()
        compose.waitUntil(90_000) { core().view.reader != null && !core().view.storageBusy }
        assertEquals(title, core().view.reader?.title)
        assertEquals(text, core().view.reader?.text)
        assertEquals(history, sourceAndHistory(id.value))
    }
}
