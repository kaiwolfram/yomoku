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
                !view.readingTimeSavePending &&
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
                core().view.readerTime != null &&
                !core().view.storageBusy
        }
        return checkNotNull(core().view.effectiveReaderTokens)
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
