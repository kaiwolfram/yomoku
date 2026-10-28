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

    @Test
    fun longPressSelectAllIncludesOffscreenRowsAndBulkRetryIsAtomicAcrossRecreation() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.SearchLibraryTitles(""))
        }
        idle()
        val prefix = "Bulk-${System.nanoTime()}"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste("Survivor-$prefix\n犬")) }
        idle()
        val survivor = checkNotNull(core().view.importSummary).documentId
        val text = "$prefix-0\n猫 猫 𠮷"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val first = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(first)) }
        compose.waitUntil(90_000) {
            core().view.readerTokens != null &&
                core().view.readerTime != null &&
                !core().view.storageBusy
        }
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        val before = db().use { history(it, first.value) }
        val catStatus =
            db().use {
                it.rawQuery("SELECT status FROM vocabulary WHERE surface = '猫'", null).use { cursor
                    ->
                    assertTrue(cursor.moveToFirst())
                    cursor.getString(0)
                }
            }
        assertEquals(1L, before[0])
        assertTrue(before[2] >= 3)
        val ids = mutableListOf(first)
        repeat(15) { index ->
            compose.runOnUiThread {
                core().dispatch(Event.CreatePaste("$prefix-${index + 1}\n猫 犬"))
            }
            idle()
            ids.add(checkNotNull(core().view.importSummary).documentId)
        }
        compose.runOnUiThread {
            core().dispatch(Event.DismissImportSummary)
            core().dispatch(Event.SearchLibraryTitles("$prefix-"))
            core().dispatch(Event.SortLibrary(LibrarySort.IMPORTDATE))
        }
        compose.waitForIdle()
        compose.onNodeWithTag("library_documents").performScrollToIndex(0)
        val row = "document_${ids.last().value}"
        compose.onNodeWithTag("document_${first.value}").assertIsNotDisplayed()
        compose.onNodeWithTag(row).performTouchInput { longClick() }
        compose.onNodeWithTag("library_selected_count").assertTextEquals("1 selected")
        compose.onNodeWithTag(row).assertIsSelected().performClick()
        compose.onNodeWithTag("library_selected_count").assertTextEquals("0 selected")
        compose.onNodeWithTag("delete_selected_documents").assertIsNotEnabled()
        compose.onNodeWithTag(row).performClick()
        compose.onNodeWithTag("select_all_library_documents").performClick()
        compose.onNodeWithTag("library_selected_count").assertTextEquals("16 selected")
        assertEquals(ids, core().view.librarySelection)
        assertEquals(ContentRoute.MAIN, core().view.contentRoute)
        compose.runOnUiThread { core().dispatch(Event.SearchLibraryTitles("$prefix-0")) }
        compose.onNodeWithTag("library_selected_count").assertTextEquals("16 selected")
        compose.onNodeWithTag("delete_selected_documents").performClick()
        compose.onNodeWithText("Delete 16 documents?").assertIsDisplayed()
        compose
            .onNodeWithText(
                "Delete these 16 documents from your Library? This removes their content and reading progress. Your vocabulary and reading history are kept. This cannot be undone."
            )
            .assertIsDisplayed()
        val session = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertSame(session, core())
        compose.onNodeWithTag("cancel_document_deletion").performClick()
        compose.onNodeWithTag("library_selected_count").assertTextEquals("16 selected")
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.waitForIdle()
        assertNull(core().view.librarySelection)
        assertEquals(before, db().use { history(it, first.value) })

        compose.runOnUiThread { core().dispatch(Event.SearchLibraryTitles("$prefix-")) }
        compose.onNodeWithTag("library_documents").performScrollToIndex(0)
        compose.onNodeWithTag(row).performTouchInput { longClick() }
        compose.onNodeWithTag("select_all_library_documents").performClick()
        compose.onNodeWithTag("delete_selected_documents").performClick()
        db().use {
            it.execSQL(
                "CREATE TRIGGER fail_ui_bulk_delete BEFORE DELETE ON documents WHEN OLD.id = ${ids.last().value} BEGIN SELECT RAISE(ABORT, 'last document'); END"
            )
        }
        try {
            compose.onNodeWithTag("confirm_document_deletion").performClick()
            compose.waitUntil(90_000) { core().view.libraryDeleteError != null }
            compose
                .onNodeWithText("Could not delete these documents. Retry or cancel.")
                .assertIsDisplayed()
            db().use {
                it.rawQuery(
                        "SELECT COUNT(*) FROM documents WHERE id IN (${ids.joinToString { id -> id.value.toString() }})",
                        null,
                    )
                    .use { cursor ->
                        assertTrue(cursor.moveToFirst())
                        assertEquals(16L, cursor.getLong(0))
                    }
                assertEquals(before, history(it, first.value))
            }
            compose.activityRule.scenario.recreate()
            compose.waitForIdle()
            compose.onNodeWithTag("confirm_document_deletion").assertTextEquals("Retry")
            assertEquals(ids, core().view.librarySelection)
        } finally {
            db().use { it.execSQL("DROP TRIGGER fail_ui_bulk_delete") }
        }
        compose.onNodeWithTag("confirm_document_deletion").performClick()
        compose.waitUntil(90_000) { core().view.libraryBulkDeleteTargets == null }
        idle()
        assertNull(core().view.librarySelection)
        assertEquals("$prefix-", core().view.libraryTitleQuery)
        assertEquals(LibrarySort.IMPORTDATE, core().view.librarySort)
        compose.onNodeWithTag("library_empty_message").assertTextEquals("No matching titles.")
        assertEquals(before, db().use { history(it, first.value) })
        db().use {
            it.rawQuery("SELECT id FROM documents WHERE id = ?", arrayOf(survivor.value.toString()))
                .use { cursor -> assertTrue(cursor.moveToFirst()) }
            it.rawQuery("SELECT status FROM vocabulary WHERE surface = '猫'", null).use { cursor ->
                assertTrue(cursor.moveToFirst())
                assertEquals(catStatus, cursor.getString(0))
            }
        }
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        idle()
        val fresh = core().view.documents.single()
        assertTrue(fresh.documentId.value > ids.last().value)
        assertEquals(0UL, fresh.progressPercent)
        assertFalse(fresh.finishedAtLeastOnce)
        assertNull(fresh.lastReadSequence)
        assertEquals(before, db().use { history(it, first.value) })
    }
}
