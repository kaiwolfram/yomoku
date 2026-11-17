package app.yomoku

import android.database.sqlite.SQLiteDatabase
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
class VocabularyProductionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core().view
            !view.storageBusy &&
                !view.completionSavePending &&
                (view.destination != Destination.LIBRARY ||
                    view.settingsOpen ||
                    view.contentRoute != ContentRoute.MAIN ||
                    view.libraryWordDistributions != null ||
                    view.libraryWordError != null) &&
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
                java.io
                    .File(
                        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
                            .targetContext
                            .filesDir,
                        "yomoku.sqlite3",
                    )
                    .path,
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
            // Settle the v2 dispatcher's queued scroll before saving recreation state.
            idle()
            val session = core()
            compose.activityRule.scenario.recreate()
            idle()
            assertSame(session, core())
            assertEquals(205, core().view.vocabulary.rows.size)
            compose.onNodeWithTag("vocabulary_row_$last").assertIsDisplayed()
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("vocabulary_scroll").assertDoesNotExist()
            compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            compose.onNodeWithTag("screen_title").assertTextEquals("Vocabulary")
            idle()
            compose.onNodeWithTag("vocabulary_row_$last").assertIsDisplayed()

            // This surface was absent from the first 100 newest registrations.
            search(surfaces.first())
            assertEquals(1UL, core().view.vocabulary.matchingCount)
            assertEquals(surfaces.first(), core().view.vocabulary.rows.single().surface)
            compose.onNodeWithTag("vocabulary_status_NEW").assertIsNotSelected().performClick()
            idle()
            assertEquals(1UL, core().view.vocabulary.matchingCount)
            compose.onNodeWithTag("vocabulary_status_NEW").assertIsSelected().performClick()
            idle()
            assertTrue(core().view.vocabulary.query.statuses.isEmpty())
            assertEquals(1UL, core().view.vocabulary.matchingCount)
            assertEquals(surfaces.first(), core().view.vocabulary.rows.single().surface)
            listOf(
                    VocabularySort.NEWEST,
                    VocabularySort.FREQUENCY,
                    VocabularySort.LASTSEEN,
                    VocabularySort.ALPHABETICAL,
                )
                .forEach { sort ->
                    compose.onNodeWithTag("vocabulary_sort").performClick()
                    compose.onNodeWithTag("vocabulary_sort_${sort.name}").performClick()
                    idle()
                    assertEquals(sort, core().view.vocabulary.query.sort)
                    assertEquals(surfaces.first(), core().view.vocabulary.rows.single().surface)
                }
            compose.onNodeWithText("Settings").performClick()
            sql { it.execSQL("ALTER TABLE vocabulary RENAME TO vocabulary_browser_unavailable") }
            renamed = true
            compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            compose.onNodeWithTag("screen_title").assertTextEquals("Vocabulary")
            idle()
            // A query change starts a new read, which fails while the table is absent.
            search(prefix)
            assertNotNull(core().view.vocabulary.error)
            compose.onNodeWithTag("vocabulary_error").assertIsDisplayed()
            sql { it.execSQL("ALTER TABLE vocabulary_browser_unavailable RENAME TO vocabulary") }
            renamed = false
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            compose.onNodeWithTag("screen_title").assertTextEquals("Vocabulary")
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

    @Test
    fun statusRemovalRetryAndReentryKeepLoadedPrefixAndScrollPastFirstPage() {
        idle()
        val prefix = "状態-${System.nanoTime()}-"
        sql { db ->
            db.beginTransaction()
            try {
                (0..304).forEach { insert(db, prefix + it.toString().padStart(4, '0')) }
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
            }
        }
        try {
            visit()
            search(prefix)
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.SetVocabularyQuery(
                            VocabularyQuery(
                                prefix,
                                listOf(VocabularyStatus.NEW),
                                VocabularySort.ALPHABETICAL,
                            )
                        )
                    )
            }
            idle()
            repeat(2) {
                compose.runOnUiThread { core().dispatch(Event.LoadMoreVocabulary) }
                idle()
            }
            val original = core().view.vocabulary
            assertEquals(300, original.rows.size)
            val row = original.rows[150]
            val next = original.rows[151]
            compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(row.id.value)
            val top =
                compose
                    .onNodeWithTag("vocabulary_row_${row.id.value}")
                    .getUnclippedBoundsInRoot()
                    .top
                    .value
            compose.onNodeWithTag("vocabulary_row_${row.id.value}").performClick()
            idle()
            val candidates = core().view.vocabulary.word?.candidates
            sql {
                it.execSQL(
                    "CREATE TRIGGER fail_polish_status BEFORE UPDATE OF status ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture'); END"
                )
            }
            compose.onNodeWithTag("word_status_LEARNING").performClick()
            idle()
            assertNotNull(core().view.vocabulary.wordStatusError)
            assertEquals(original.rows, core().view.vocabulary.rows)
            sql { it.execSQL("DROP TRIGGER fail_polish_status") }
            compose.onNodeWithTag("vocabulary_word_status_retry").performClick()
            idle()
            val removed = core().view.vocabulary
            assertEquals(304UL, removed.matchingCount)
            assertEquals(300, removed.rows.size)
            assertEquals(300UL, removed.nextOffset)
            assertFalse(removed.rows.any { it.id == row.id })
            assertEquals(candidates, removed.word?.candidates)
            compose.onNodeWithTag("word_status_NEW").performClick()
            idle()
            assertEquals(original.rows, core().view.vocabulary.rows)
            assertEquals(305UL, core().view.vocabulary.matchingCount)
            assertEquals(300UL, core().view.vocabulary.nextOffset)
            assertEquals(candidates, core().view.vocabulary.word?.candidates)
            compose.runOnUiThread { core().dispatch(Event.DismissVocabularyWord) }
            idle()
            val nextTop =
                compose
                    .onNodeWithTag("vocabulary_row_${next.id.value}")
                    .getUnclippedBoundsInRoot()
                    .top
                    .value
            assertEquals(top, nextTop, 0.5f)
            compose.activityRule.scenario.recreate()
            idle()
            assertEquals(300, core().view.vocabulary.rows.size)
            assertEquals(
                nextTop,
                compose
                    .onNodeWithTag("vocabulary_row_${next.id.value}")
                    .getUnclippedBoundsInRoot()
                    .top
                    .value,
                0.5f,
            )
            compose.runOnUiThread { core().dispatch(Event.LoadMoreVocabulary) }
            idle()
            assertEquals(305, core().view.vocabulary.rows.size)
            assertNull(core().view.vocabulary.nextOffset)
        } finally {
            sql { db ->
                db.execSQL("DROP TRIGGER IF EXISTS fail_polish_status")
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
    fun composeStatusChangesRefreshStatisticsLibraryAndEffectiveReaderWithoutRescanning() {
        idle()
        val title = "Status-refresh-${System.nanoTime()}"
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
        idle()
        // Vocabulary is global; ensure Learning is a transition, not a dismissal.
        if (cat.status == VocabularyStatus.LEARNING) {
            compose.onNodeWithTag("word_status_KNOWN").performClick()
            idle()
        }
        compose.onNodeWithTag("word_status_LEARNING").performClick()
        idle()
        compose.onNodeWithTag("word_status_LEARNING").performClick()
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
        idle()
        compose.onNodeWithTag("word_surface").assertTextEquals("猫")
        assertDictionaryEntries(compose)
        compose
            .onNodeWithTag("word_encountered")
            .assertTextEquals(seenHistory(cat.encounteredCount, cat.lastSeenUnixMs))
        compose.onNodeWithTag("vocabulary_delete_selected").assertDoesNotExist()
        compose.onNodeWithTag("word_status_IGNORED").performClick()
        idle()
        assertEquals(VocabularyStatus.IGNORED, core().view.vocabulary.word?.row?.status)
        compose.onNodeWithTag("reader_word_panel").assertExists()
        compose.onNodeWithTag("word_status_IGNORED").performClick()
        idle()
        assertNull(core().view.vocabulary.word)
        val after = statistics()
        assertEquals(before.reading.japaneseOccurrences, after.reading.japaneseOccurrences)
        assertEquals(before.statuses.learning - 1UL, after.statuses.learning)
        assertEquals(before.statuses.ignored + 1UL, after.statuses.ignored)
        library()
        assertEquals(
            WordStatusCounts(0UL, 0UL, 0UL, 1UL, 1UL),
            core().view.libraryWordDistributions!!.single { it.documentId == id }.counts,
        )
        page = open(id)
        assertEquals(listOf("猫", "猫", "犬"), page.tokens.map { it.surface })
        assertTrue(
            page.tokens
                .filter { it.surface == "猫" }
                .all { it.vocabulary?.status == VocabularyStatus.IGNORED }
        )
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        visit()
        search("猫")
        val retained = core().view.vocabulary.rows.single { it.surface == "猫" }
        assertEquals(cat.id, retained.id)
        assertEquals(cat.encounteredCount, retained.encounteredCount)
    }

    @Test
    fun wordSheetRemainsOpenAfterStatusFilterRemovalAndBookDeletion() {
        idle()
        library()
        compose.runOnUiThread {
            core().dispatch(Event.CreatePaste("Vocabulary-sheet-${System.nanoTime()}\n食べました 東京都"))
        }
        idle()
        val book = checkNotNull(core().view.importSummary).documentId
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        visit()
        search("食べました")
        val row = core().view.vocabulary.rows.single { it.surface == "食べました" }
        val target =
            if (row.status == VocabularyStatus.LEARNING) VocabularyStatus.KNOWN
            else VocabularyStatus.LEARNING
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.SetVocabularyQuery(
                        VocabularyQuery(row.surface, listOf(row.status), VocabularySort.NEWEST)
                    )
                )
        }
        idle()
        compose.onNodeWithTag("vocabulary_row_${row.id.value}").performClick()
        idle()
        compose.onNodeWithTag("word_surface").assertTextEquals(row.surface)
        assertDictionaryEntries(compose)
        compose.onNodeWithTag("word_status_${target.name}").performClick()
        idle()
        assertEquals(0UL, core().view.vocabulary.matchingCount)
        assertEquals(target, core().view.vocabulary.word?.row?.status)
        compose.onNodeWithTag("reader_word_panel").assertExists()
        compose.onNodeWithTag("word_surface").assertTextEquals(row.surface)
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        idle()
        assertNull(core().view.vocabulary.word)
        library()
        compose.runOnUiThread { core().dispatch(Event.RequestDocumentDeletion(book)) }
        compose.runOnUiThread { core().dispatch(Event.ConfirmDocumentDeletion(book)) }
        idle()
        visit()
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.SetVocabularyQuery(
                        VocabularyQuery(
                            row.surface,
                            VocabularyStatus.entries.toList(),
                            VocabularySort.NEWEST,
                        )
                    )
                )
        }
        idle()
        compose.onNodeWithTag("vocabulary_row_${row.id.value}").performClick()
        idle()
        assertDictionaryEntries(compose)
        assertEquals(row.encounteredCount, core().view.vocabulary.word?.row?.encounteredCount)
        compose.onNodeWithTag("word_status_${target.name}").performClick()
        idle()
        assertNull(core().view.vocabulary.word)
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
