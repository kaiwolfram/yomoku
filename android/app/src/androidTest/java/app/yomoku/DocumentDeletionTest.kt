package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DocumentDeletionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core().view
            !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending &&
                (view.contentRoute != ContentRoute.MAIN ||
                    view.destination != Destination.LIBRARY ||
                    view.libraryWordDistributions != null ||
                    view.libraryWordError != null)
        }
    }

    private fun db() =
        SQLiteDatabase.openDatabase(
            File(compose.activity.filesDir, "yomoku.sqlite3").path,
            null,
            SQLiteDatabase.OPEN_READWRITE,
        )

    private fun history(db: SQLiteDatabase, id: Long): List<Long> =
        db.rawQuery(
                "SELECT COUNT(*), COALESCE(SUM(active_time_ms), 0), (SELECT COUNT(*) FROM reading_occurrences o JOIN reading_completions c ON c.command_id = o.command_id WHERE c.document_id = ?) FROM reading_completions WHERE document_id = ?",
                arrayOf(id.toString(), id.toString()),
            )
            .use {
                assertTrue(it.moveToFirst())
                listOf(it.getLong(0), it.getLong(1), it.getLong(2))
            }

    @Test
    fun cancelRecreateFailureRetryPreserveHistoryAndReimportStartsFresh() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.SearchLibraryTitles(""))
        }
        idle()
        val title = "Deletion-${System.nanoTime()}"
        val text = "$title\n猫 猫 犬 𠮷"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val id = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("Remaining-${System.nanoTime()}\n猫"))
        }
        idle()
        val remainingId = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread {
            core().dispatch(Event.DismissImportSummary)
            core().dispatch(Event.SearchLibraryTitles(title))
        }
        compose.onNodeWithTag("delete_document_${id.value}").performClick()
        compose.onNodeWithTag("document_deletion_dialog").assertIsDisplayed()
        compose
            .onNodeWithText(
                "Delete “$title” from your Library? This removes its content and reading progress. Your vocabulary and reading history are kept. This cannot be undone."
            )
            .assertIsDisplayed()
        val session = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertSame(session, core())
        compose.onNodeWithTag("document_deletion_dialog").assertIsDisplayed()
        compose.onNodeWithTag("cancel_document_deletion").performClick()
        compose.onNodeWithTag("document_deletion_dialog").assertDoesNotExist()
        compose.onNodeWithTag("document_${id.value}").performClick()
        compose.waitUntil(90_000) {
            core().view.readerTokens != null &&
                core().view.readerTime != null &&
                !core().view.storageBusy
        }
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        compose.onNodeWithTag("finished_${id.value}", useUnmergedTree = true).assertExists()
        val before = db().use { history(it, id.value) }
        assertEquals(1L, before[0])
        assertTrue(before[2] >= 3L)
        db().use {
            it.execSQL(
                "CREATE TRIGGER fail_ui_document_delete BEFORE DELETE ON documents WHEN OLD.id = ${id.value} BEGIN SELECT RAISE(ABORT, 'fixture'); END"
            )
        }
        try {
            compose.onNodeWithTag("delete_document_${id.value}").performClick()
            compose.onNodeWithTag("confirm_document_deletion").performClick()
            compose.waitUntil(90_000) { core().view.libraryDeleteError != null }
            compose
                .onNodeWithText("Could not delete this document. Retry or cancel.")
                .assertIsDisplayed()
            assertEquals(before, db().use { history(it, id.value) })
            compose.activityRule.scenario.recreate()
            compose.waitForIdle()
            compose.onNodeWithTag("confirm_document_deletion").assertTextEquals("Retry")
        } finally {
            db().use { it.execSQL("DROP TRIGGER fail_ui_document_delete") }
        }
        compose.onNodeWithTag("confirm_document_deletion").performClick()
        compose.waitUntil(90_000) { core().view.libraryDeleteTarget == null }
        idle()
        assertEquals(title, core().view.libraryTitleQuery)
        compose.onNodeWithTag("document_${id.value}").assertDoesNotExist()
        compose.onNodeWithTag("library_empty_message").assertTextEquals("No matching titles.")
        assertEquals(before, db().use { history(it, id.value) })
        db().use {
            it.rawQuery(
                    "SELECT id FROM documents WHERE id = ?",
                    arrayOf(remainingId.value.toString()),
                )
                .use { cursor ->
                    assertTrue(cursor.moveToFirst())
                }
        }
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val newId = checkNotNull(core().view.importSummary).documentId
        assertTrue(newId.value > id.value)
        val newDocument = core().view.documents.single()
        assertEquals(0UL, newDocument.progressPercent)
        assertFalse(newDocument.finishedAtLeastOnce)
        assertNull(newDocument.lastReadSequence)
        compose
            .onNodeWithTag("finished_${newId.value}", useUnmergedTree = true)
            .assertDoesNotExist()
        assertEquals(before, db().use { history(it, id.value) })
    }
}
