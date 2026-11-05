package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class VocabularyProductionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core().view
            !view.storageBusy &&
                !view.completionSavePending &&
                (view.destination != Destination.VOCABULARY ||
                    view.settingsOpen ||
                    view.contentRoute != ContentRoute.MAIN ||
                    view.vocabulary.matchingCount != null ||
                    view.vocabulary.error != null)
        }
    }

    private fun visit() {
        compose.onNode(hasText("Vocabulary") and hasClickAction()).performClick()
        idle()
    }

    private fun search(prefix: String) {
        compose.onNodeWithTag("vocabulary_scroll").performScrollToIndex(0)
        compose.onNodeWithTag("vocabulary_search").performTextReplacement(prefix)
        idle()
        assertEquals(prefix, core().view.vocabulary.query.prefix)
    }

    private fun sql(action: (SQLiteDatabase) -> Unit) {
        SQLiteDatabase.openDatabase(
                java.io.File(compose.activity.filesDir, "yomoku.sqlite3").path,
                null,
                SQLiteDatabase.OPEN_READWRITE,
            )
            .use { db ->
                db.setForeignKeyConstraintsEnabled(true)
                action(db)
            }
    }

    private fun insert(db: SQLiteDatabase, surface: String): VocabularyId {
        db.execSQL(
            "INSERT INTO vocabulary(surface,status,registered_at_unix_ms) VALUES (?, 'new', 1)",
            arrayOf(surface),
        )
        val id =
            db.rawQuery("SELECT last_insert_rowid()", null).use {
                it.moveToFirst()
                it.getLong(0)
            }
        db.execSQL(
            "INSERT INTO vocabulary_status_events(semantics_version,vocabulary_id,old_status,new_status,cause,occurred_at_unix_ms) VALUES(1,?,NULL,'new','registration',1)",
            arrayOf(id),
        )
        return VocabularyId(id)
    }

    private fun requestStatus(status: VocabularyStatus) {
        compose.onNodeWithTag("vocabulary_change_status").performClick()
        compose.onNodeWithTag("vocabulary_bulk_status_${status.name}").performClick()
        compose.onNodeWithTag("vocabulary_bulk_dialog").assertIsDisplayed()
    }

    @Test
    fun unloadedAndHiddenSelectionsFrozenStatusAndDeleteRetrySurviveRecreation() {
        idle()
        val prefix = "選-${System.nanoTime()}-"
        val ids = mutableListOf<VocabularyId>()
        lateinit var hidden: VocabularyId
        sql { db ->
            hidden = insert(db, prefix + "hidden")
            repeat(205) { ids.add(insert(db, prefix + "match-" + it.toString().padStart(4, '0'))) }
        }
        try {
            visit()
            search(prefix + "hidden")
            compose.onNodeWithTag("vocabulary_row_${hidden.value}").performTouchInput {
                longClick()
            }
            compose.onNodeWithTag("vocabulary_selected_count").assertTextEquals("Selected words: 1")
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithText("Back").performClick()
            idle()
            assertEquals(listOf(hidden), core().view.vocabulary.selection)
            search(prefix + "match-")
            assertEquals(100, core().view.vocabulary.rows.size)
            compose.onNodeWithTag("vocabulary_select_all").performClick()
            idle()
            val selected = (ids + hidden).sortedBy { it.value }
            assertEquals(selected, core().view.vocabulary.selection)
            assertEquals(100, core().view.vocabulary.rows.size)
            compose
                .onNodeWithTag("vocabulary_selected_count")
                .assertTextEquals("Selected words: 206")
            val session = core()
            compose.activityRule.scenario.recreate()
            idle()
            assertSame(session, core())
            assertEquals(selected, core().view.vocabulary.selection)
            requestStatus(VocabularyStatus.FAMILIAR)
            val cancelled = checkNotNull(core().view.vocabulary.confirmation)
            compose.activityRule.scenario.recreate()
            idle()
            assertEquals(cancelled, core().view.vocabulary.confirmation)
            compose.onNodeWithTag("vocabulary_cancel_bulk").performClick()
            assertEquals(selected, core().view.vocabulary.selection)
            requestStatus(VocabularyStatus.KNOWN)
            val frozen = checkNotNull(core().view.vocabulary.confirmation)
            sql {
                it.execSQL(
                    "CREATE TRIGGER fail_ui_vocabulary_status BEFORE UPDATE ON vocabulary WHEN OLD.id = ${ids.last().value} BEGIN SELECT RAISE(ABORT, 'last word'); END"
                )
            }
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            idle()
            assertNotNull(core().view.vocabulary.bulkError)
            sql { db ->
                db.rawQuery(
                        "SELECT COUNT(*) FROM vocabulary WHERE surface >= ? AND surface < ? AND status != 'new'",
                        arrayOf(prefix, prefix + "\uffff"),
                    )
                    .use {
                        it.moveToFirst()
                        assertEquals(0, it.getInt(0))
                    }
                insert(db, prefix + "match-later")
                db.execSQL("DROP TRIGGER fail_ui_vocabulary_status")
            }
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.SetVocabularyQuery(
                            core().view.vocabulary.query.copy(prefix = "ignored")
                        )
                    )
                core().dispatch(Event.SelectDestination(Destination.LIBRARY))
                core().dispatch(Event.OpenSettings)
            }
            compose.activityRule.scenario.recreate()
            idle()
            assertEquals(Destination.VOCABULARY, core().view.destination)
            assertFalse(core().view.settingsOpen)
            assertEquals(prefix + "match-", core().view.vocabulary.query.prefix)
            assertEquals(frozen, core().view.vocabulary.confirmation)
            compose.onNodeWithTag("vocabulary_bulk_error").assertIsDisplayed()
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            idle()
            assertNull(core().view.vocabulary.selection)
            assertNull(core().view.vocabulary.confirmation)
            assertNull(core().view.vocabulary.bulkError)
            assertEquals(206UL, core().view.vocabulary.matchingCount)
            sql { db ->
                db.rawQuery(
                        "SELECT COUNT(*), MIN(occurred_at_unix_ms), MAX(occurred_at_unix_ms) FROM vocabulary_status_events WHERE cause = 'manual' AND vocabulary_id IN (${selected.joinToString { it.value.toString() }})",
                        null,
                    )
                    .use {
                        it.moveToFirst()
                        assertEquals(206, it.getInt(0))
                        val date =
                            (frozen.action as VocabularyBulkAction.SetStatus).occurredAtUnixMs
                        assertEquals(date, it.getLong(1))
                        assertEquals(date, it.getLong(2))
                    }
            }
            compose.onNodeWithTag("vocabulary_select_all").performClick()
            idle()
            assertEquals(206, core().view.vocabulary.selection?.size)
            compose.onNodeWithTag("vocabulary_delete_selected").performClick()
            val deletion = checkNotNull(core().view.vocabulary.confirmation)
            sql {
                it.execSQL(
                    "CREATE TRIGGER fail_ui_vocabulary_delete BEFORE DELETE ON vocabulary WHEN OLD.id = ${ids.last().value} BEGIN SELECT RAISE(ABORT, 'last word'); END"
                )
            }
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            idle()
            assertNotNull(core().view.vocabulary.bulkError)
            sql { db ->
                db.rawQuery(
                        "SELECT COUNT(*) FROM vocabulary WHERE surface >= ? AND surface < ?",
                        arrayOf(prefix, prefix + "\uffff"),
                    )
                    .use {
                        it.moveToFirst()
                        assertEquals(207, it.getInt(0))
                    }
                db.execSQL("DROP TRIGGER fail_ui_vocabulary_delete")
                insert(db, prefix + "match-after-delete-prompt")
            }
            compose.activityRule.scenario.recreate()
            idle()
            assertEquals(deletion, core().view.vocabulary.confirmation)
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            idle()
            assertEquals(1UL, core().view.vocabulary.matchingCount)
            assertEquals(
                prefix + "match-after-delete-prompt",
                core().view.vocabulary.rows.single().surface,
            )
            search(prefix + "hidden")
            assertEquals(VocabularyStatus.KNOWN, core().view.vocabulary.rows.single().status)
            compose.onNodeWithTag("vocabulary_row_${hidden.value}").performClick()
            compose.onNodeWithTag("vocabulary_selected_count").assertTextEquals("Selected words: 1")
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            idle()
            assertNull(core().view.vocabulary.selection)
        } finally {
            sql { db ->
                db.execSQL("DROP TRIGGER IF EXISTS fail_ui_vocabulary_status")
                db.execSQL("DROP TRIGGER IF EXISTS fail_ui_vocabulary_delete")
                db.execSQL(
                    "DELETE FROM vocabulary WHERE surface >= ? AND surface < ?",
                    arrayOf(prefix, prefix + "\uffff"),
                )
            }
        }
    }

    @Test
    fun failedCaptureRetriesItsQueryAndBackAbandonsItWithoutAutomaticRetry() {
        idle()
        val prefix = "捕-${System.nanoTime()}-"
        sql { db -> repeat(3) { insert(db, prefix + it) } }
        var renamed = false
        try {
            visit()
            search(prefix)
            sql { it.execSQL("ALTER TABLE vocabulary RENAME TO vocabulary_capture_unavailable") }
            renamed = true
            compose.onNodeWithTag("vocabulary_select_all").performClick()
            idle()
            assertNotNull(core().view.vocabulary.bulkError)
            assertNull(core().view.vocabulary.confirmation)
            sql { it.execSQL("ALTER TABLE vocabulary_capture_unavailable RENAME TO vocabulary") }
            renamed = false
            compose.activityRule.scenario.recreate()
            idle()
            assertNotNull(core().view.vocabulary.bulkError)
            compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
            idle()
            assertNull(core().view.vocabulary.bulkError)
            assertEquals(3, core().view.vocabulary.selection?.size)
            sql { it.execSQL("ALTER TABLE vocabulary RENAME TO vocabulary_capture_unavailable") }
            renamed = true
            compose.onNodeWithTag("vocabulary_select_all").performClick()
            idle()
            assertNotNull(core().view.vocabulary.bulkError)
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            idle()
            assertNull(core().view.vocabulary.bulkError)
            assertNull(core().view.vocabulary.selection)
            compose.onNodeWithTag("vocabulary_bulk_dialog").assertDoesNotExist()
        } finally {
            sql { db ->
                if (renamed)
                    db.execSQL("ALTER TABLE vocabulary_capture_unavailable RENAME TO vocabulary")
                db.execSQL(
                    "DELETE FROM vocabulary WHERE surface >= ? AND surface < ?",
                    arrayOf(prefix, prefix + "\uffff"),
                )
            }
        }
    }

    @Test
    fun nativePagesUnloadedPrefixExplicitRetrySettingsAndRecreation() {
        idle()
        val prefix = "頁-${System.nanoTime()}-"
        val surfaces = (0..204).map { prefix + it.toString().padStart(4, '0') }
        sql { db ->
            db.beginTransaction()
            try {
                surfaces.forEach { surface ->
                    db.execSQL(
                        "INSERT INTO vocabulary(surface,status,registered_at_unix_ms) VALUES (?, 'new', 1)",
                        arrayOf(surface),
                    )
                    db.execSQL(
                        "INSERT INTO vocabulary_status_events(semantics_version,vocabulary_id,old_status,new_status,cause,occurred_at_unix_ms) VALUES(1,last_insert_rowid(),NULL,'new','registration',1)"
                    )
                }
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
            }
        }
        var renamed = false
        try {
            visit()
            search(prefix)
            assertEquals(205UL, core().view.vocabulary.matchingCount)
            assertEquals(100, core().view.vocabulary.rows.size)
            assertEquals(100UL, core().view.vocabulary.nextOffset)
            val firstLast = core().view.vocabulary.rows.last().id.value
            compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(firstLast)
            idle()
            assertEquals(200, core().view.vocabulary.rows.size)
            val secondLast = core().view.vocabulary.rows.last().id.value
            compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(secondLast)
            idle()
            assertEquals(205, core().view.vocabulary.rows.size)
            assertNull(core().view.vocabulary.nextOffset)
            val last = core().view.vocabulary.rows.last().id.value
            compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(last)
            val session = core()
            compose.activityRule.scenario.recreate()
            idle()
            assertSame(session, core())
            assertEquals(205, core().view.vocabulary.rows.size)
            compose.onNodeWithTag("vocabulary_row_$last").assertIsDisplayed()
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("vocabulary_scroll").assertDoesNotExist()
            compose.onNodeWithText("Back").performClick()
            idle()
            compose.onNodeWithTag("vocabulary_row_$last").assertIsDisplayed()

            // This surface was absent from the first 100 newest registrations.
            search(surfaces.first())
            assertEquals(1UL, core().view.vocabulary.matchingCount)
            assertEquals(surfaces.first(), core().view.vocabulary.rows.single().surface)
            compose.onNodeWithTag("vocabulary_status_NEW").performClick()
            idle()
            assertEquals(0UL, core().view.vocabulary.matchingCount)
            compose.onNodeWithText("No matching words.").assertIsDisplayed()
            compose.onNodeWithTag("vocabulary_status_NEW").performClick()
            idle()
            VocabularySort.entries.forEach { sort ->
                compose.onNodeWithTag("vocabulary_sort").performClick()
                compose.onNodeWithTag("vocabulary_sort_${sort.name}").performClick()
                idle()
                assertEquals(sort, core().view.vocabulary.query.sort)
                assertEquals(surfaces.first(), core().view.vocabulary.rows.single().surface)
            }
            compose.onNodeWithText("Settings").performClick()
            sql { it.execSQL("ALTER TABLE vocabulary RENAME TO vocabulary_browser_unavailable") }
            renamed = true
            compose.onNodeWithText("Back").performClick()
            idle()
            // A query change starts a new read, which fails while the table is absent.
            search(prefix)
            assertNotNull(core().view.vocabulary.error)
            compose.onNodeWithTag("vocabulary_error").assertIsDisplayed()
            sql { it.execSQL("ALTER TABLE vocabulary_browser_unavailable RENAME TO vocabulary") }
            renamed = false
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithText("Back").performClick()
            idle()
            assertNotNull(core().view.vocabulary.error)
            compose.onNodeWithTag("vocabulary_retry").performClick()
            idle()
            assertNull(core().view.vocabulary.error)
            assertEquals(205UL, core().view.vocabulary.matchingCount)
            assertEquals(100, core().view.vocabulary.rows.size)
            assertEquals(VocabularySort.ALPHABETICAL, core().view.vocabulary.query.sort)
        } finally {
            sql { db ->
                if (renamed)
                    db.execSQL("ALTER TABLE vocabulary_browser_unavailable RENAME TO vocabulary")
                db.execSQL(
                    "DELETE FROM vocabulary WHERE surface >= ? AND surface < ?",
                    arrayOf(prefix, prefix + "\uffff"),
                )
            }
        }
    }

    private fun open(id: DocumentId): EffectiveReaderPage {
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        compose.waitUntil(90_000) {
            core().view.effectiveReaderTokens?.identity?.base?.documentId == id &&
                !core().view.storageBusy
        }
        return checkNotNull(core().view.effectiveReaderTokens)
    }

    private fun statistics(): StatisticsValues {
        compose.onNode(hasText("Statistics") and hasClickAction()).performClick()
        compose.waitUntil(90_000) {
            !core().view.storageBusy && core().view.statistics.data != null
        }
        assertNull(core().view.statistics.error)
        return checkNotNull(core().view.statistics.data).buckets.last().values
    }

    private fun library() {
        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        compose.waitUntil(90_000) {
            !core().view.storageBusy && core().view.libraryWordDistributions != null
        }
        assertNull(core().view.libraryWordError)
    }

    @Test
    fun composeBulkChangesRefreshStatisticsLibraryAndEffectiveReaderWithoutRescanning() {
        idle()
        val title = "Bulk-refresh-${System.nanoTime()}"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste("$title\n猫 猫 犬")) }
        idle()
        val id = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        open(id)
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        visit()
        search("猫")
        val cat = core().view.vocabulary.rows.single { it.surface == "猫" }
        compose.onNodeWithTag("vocabulary_row_${cat.id.value}").performClick()
        requestStatus(VocabularyStatus.LEARNING)
        compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
        idle()
        assertEquals(
            VocabularyStatus.LEARNING,
            core().view.vocabulary.rows.single { it.id == cat.id }.status,
        )
        library()
        assertEquals(
            WordStatusCounts(0UL, 1UL, 0UL, 1UL, 0UL),
            core().view.libraryWordDistributions!!.single { it.documentId == id }.counts,
        )
        var page = open(id)
        assertTrue(
            page.tokens
                .filter { it.surface == "猫" }
                .all { it.vocabulary?.status == VocabularyStatus.LEARNING }
        )
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        val before = statistics()
        visit()
        search("猫")
        compose.onNodeWithTag("vocabulary_row_${cat.id.value}").performClick()
        compose.onNodeWithTag("vocabulary_delete_selected").performClick()
        compose.onNodeWithTag("vocabulary_confirm_bulk").performClick()
        idle()
        assertFalse(core().view.vocabulary.rows.any { it.id == cat.id })
        val after = statistics()
        assertEquals(
            before.reading.japaneseOccurrences - cat.encounteredCount,
            after.reading.japaneseOccurrences,
        )
        assertEquals(before.statuses.learning - 1UL, after.statuses.learning)
        library()
        assertEquals(
            WordStatusCounts(0UL, 0UL, 0UL, 1UL, 0UL),
            core().view.libraryWordDistributions!!.single { it.documentId == id }.counts,
        )
        page = open(id)
        assertEquals(listOf("猫", "猫", "犬"), page.tokens.map { it.surface })
        assertTrue(page.tokens.filter { it.surface == "猫" }.all { it.vocabulary == null })
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        visit()
        search("猫")
        assertFalse(core().view.vocabulary.rows.any { it.surface == "猫" })
        library()
        compose.runOnUiThread { core().dispatch(Event.CreatePaste("$title-fresh\n猫")) }
        idle()
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        visit()
        search("猫")
        val fresh = core().view.vocabulary.rows.single { it.surface == "猫" }
        assertNotEquals(cat.id, fresh.id)
        assertEquals(VocabularyStatus.NEW, fresh.status)
        assertEquals(0UL, fresh.encounteredCount)
    }

    @Test
    fun realImportManualStatusAndCompletionRefreshCountsWithoutLookupEncounters() {
        idle()
        visit()
        search("猫")
        val before = core().view.vocabulary.rows.find { it.surface == "猫" }
        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("Vocabulary-${System.nanoTime()}\n猫 猫 犬"))
        }
        idle()
        val id = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        visit()
        search("猫")
        val imported = core().view.vocabulary.rows.single { it.surface == "猫" }
        assertEquals(before?.encounteredCount ?: 0UL, imported.encounteredCount)
        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        var page = open(id)
        fun setStatus(status: VocabularyStatus) {
            val token = page.tokens.first { it.surface == "猫" }
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.SetEffectiveReaderStatus(
                            EffectiveReaderSelection(page.identity, token.segmentIndex),
                            checkNotNull(token.vocabulary).id,
                            status,
                            System.currentTimeMillis(),
                        )
                    )
            }
            idle()
            assertNull(core().view.effectiveReaderStatus?.error)
            assertEquals(
                status,
                core()
                    .view
                    .effectiveReaderTokens
                    ?.tokens
                    ?.first { it.surface == "猫" }
                    ?.vocabulary
                    ?.status,
            )
        }
        setStatus(VocabularyStatus.LEARNING)
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        visit()
        val changed = core().view.vocabulary.rows.single { it.surface == "猫" }
        assertEquals(VocabularyStatus.LEARNING, changed.status)
        assertEquals(imported.encounteredCount, changed.encounteredCount)
        assertEquals(imported.lastSeenUnixMs, changed.lastSeenUnixMs)
        compose.onNode(hasText("Library") and hasClickAction()).performClick()
        idle()
        page = open(id)
        setStatus(VocabularyStatus.NEW)
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        visit()
        val completed = core().view.vocabulary.rows.single { it.surface == "猫" }
        assertEquals(VocabularyStatus.KNOWN, completed.status)
        assertEquals(imported.encounteredCount + 2UL, completed.encounteredCount)
        assertNotNull(completed.lastSeenUnixMs)
        val query = core().view.vocabulary.query
        compose.activityRule.scenario.recreate()
        idle()
        assertEquals(query, core().view.vocabulary.query)
        assertEquals(completed, core().view.vocabulary.rows.single { it.surface == "猫" })
        compose.onNodeWithTag("vocabulary_row_${completed.id.value}").assertIsDisplayed()
        compose.onNodeWithTag("vocabulary_search").assertTextContains("猫")
    }
}
