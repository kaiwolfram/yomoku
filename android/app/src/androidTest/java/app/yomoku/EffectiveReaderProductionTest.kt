package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Actual production NativeBridge, Full, SQLite and app controls; each book fits one short page. */
@RunWith(AndroidJUnit4::class)
class EffectiveReaderProductionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private var owner by mutableStateOf<CoreSession?>(null)
    private var core: CoreSession
        get() = checkNotNull(owner)
        set(value) {
            owner = value
        }

    private lateinit var database: File
    private val fullCalls = AtomicInteger()

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) {
            val view = core.view
            !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending &&
                (view.reader == null ||
                    view.effectiveReaderTokens != null ||
                    view.effectiveReaderOverride?.reloadRequired == true ||
                    view.effectiveReaderOverride?.pending == true ||
                    view.documentError != null)
        }
        compose.waitForIdle()
    }

    private fun dispatch(event: Event) {
        compose.runOnUiThread { core.dispatch(event) }
        idle()
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    private fun show(provider: DictionaryProvider) {
        compose.runOnUiThread {
            core = CoreSession(database.path, provider, { 0L })
            core.setReaderActive(true)
        }
        compose.setContent {
            YomokuTheme {
                YomokuApp(core.view, core.sessionKey, core::dispatch, core::onReaderInteraction)
            }
        }
        idle()
    }

    private fun edit() {
        click("boundary_select")
        click("boundary_token_0")
        click("boundary_token_1")
        click("boundary_open")
        idle()
        compose.onNodeWithTag("boundary_surface").assertTextEquals("国家公務員")
    }

    private fun reload() {
        compose.onNodeWithTag("retry_word_lookup").assertTextEquals("Reload")
        click("retry_word_lookup")
        idle()
        assertNotNull(core.view.effectiveReaderTokens)
        assertNull(core.view.readerTokens)
    }

    private fun surfaces() = core.view.effectiveReaderTokens!!.tokens.map { it.surface }

    private fun sql(action: (SQLiteDatabase) -> Unit) {
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use(action)
    }

    private fun count(table: String): Long {
        var result = -1L
        sql { db ->
            db.rawQuery("SELECT count(*) FROM $table", null).use {
                it.moveToFirst()
                result = it.getLong(0)
            }
        }
        return result
    }

    private fun withBook(action: (DocumentId, DictionaryProvider) -> Unit) {
        val directory =
            File(compose.activity.noBackupFilesDir, "effective-production-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        database = File(directory, "reader.sqlite3")
        val actual = sudachiDictionaryProvider(compose.activity)
        val provider =
            object : DictionaryProvider by actual {
                override fun openVerifiedDictionary(): Long {
                    fullCalls.incrementAndGet()
                    return actual.openVerifiedDictionary()
                }
            }
        show(provider)
        try {
            dispatch(Event.CreatePaste("国家公務員。国家公務員。ヌヌヌヌヌヌ"))
            val id = core.view.importSummary!!.documentId
            dispatch(Event.OpenReader(id))
            assertEquals(listOf("国家", "公務員", "国家", "公務員", "ヌヌヌヌヌヌ"), surfaces())
            action(id, provider)
        } finally {
            compose.runOnUiThread {
                compose.activity.setContent {}
                core.close()
            }
            compose.waitUntil(30_000) { core.isClosed }
            assertTrue(directory.deleteRecursively())
        }
    }

    @Test
    fun realModesLocalAndGlobalEditsPersistReverseAndRefreshLibraryWithoutRescanning() =
        withBook { id, provider ->
            for (mode in listOf(SplitMode.A, SplitMode.C)) {
                edit()
                click("boundary_mode_$mode")
                idle()
                assertTrue(core.view.effectiveReaderOverride!!.reloadRequired)
                reload()
            }
            edit()
            val prompt = core.view.effectiveReaderOverride!!.prompt!!
            assertEquals(
                listOf("国家", "公務", "員"),
                prompt.choices.single { it.mode == SplitMode.A }.parts,
            )
            assertEquals(listOf("国家公務員"), prompt.choices.single { it.mode == SplitMode.C }.parts)
            compose.onNodeWithTag("boundary_scope_ALLOCCURRENCES").assertIsSelected()
            click("boundary_scope_THISOCCURRENCE")
            click("boundary_mode_C")
            idle()
            reload()
            assertEquals(listOf("国家公務員", "国家", "公務員"), surfaces().take(3))
            edit()
            val local = core.view.effectiveReaderOverride!!.prompt!!.winningRules.single()
            click("boundary_reverse_${local.id.value}")
            idle()
            reload()
            edit()
            click("boundary_merge_all")
            click("boundary_cut_1")
            compose.onNodeWithTag("boundary_preview").assertTextEquals("国 | 家公務員")
            click("boundary_save")
            idle()
            reload()
            assertEquals(listOf("国", "家公務員", "国", "家公務員"), surfaces().take(4))
            assertTrue(
                core.view.effectiveReaderTokens!!.tokens.take(4).all {
                    it.reading == null && it.dictionaryForm == null
                }
            )
            dispatch(Event.CloseDocument)
            compose.waitUntil(30_000) { core.view.libraryWordDistributions != null }
            assertEquals(
                WordStatusCounts(3UL, 0UL, 0UL, 0UL, 0UL),
                core.view.libraryWordDistributions!!.single().counts,
            )
            assertEquals(0L, count("reading_completions"))
            assertEquals(1, fullCalls.get())
            compose.runOnUiThread { core.close() }
            compose.waitUntil(30_000) { core.isClosed }
            // Reopen production with Full deliberately unavailable; A/C and rules are durable.
            val noFull =
                object : DictionaryProvider by provider {
                    override fun openVerifiedDictionary(): Long =
                        error("Stored reads must not load Full")
                }
            compose.runOnUiThread {
                core = CoreSession(database.path, noFull, { 0L })
                core.setReaderActive(true)
            }
            idle()
            dispatch(Event.OpenReader(id))
            assertEquals(listOf("国", "家公務員", "国", "家公務員"), surfaces().take(4))
            edit()
            assertEquals(3, core.view.effectiveReaderOverride!!.prompt!!.choices.size)
            val global = core.view.effectiveReaderOverride!!.prompt!!.winningRules.single()
            assertEquals(OverrideScope.ALLOCCURRENCES, global.scope)
            click("boundary_reverse_${global.id.value}")
            idle()
            reload()
            assertEquals(listOf("国家", "公務員", "国家", "公務員"), surfaces().take(4))
            assertEquals(0L, count("reading_occurrences"))
        }

    @Test
    fun failedEditsAndCompletionRetryExactlyWithFrozenEffectiveOccurrencesAndTime() =
        withBook { id, _ ->
            edit()
            click("boundary_merge_all")
            click("boundary_cut_1")
            sql {
                it.execSQL(
                    "CREATE TRIGGER fail_override BEFORE INSERT ON segmentation_overrides BEGIN SELECT RAISE(ABORT,'fixture'); END"
                )
            }
            click("boundary_save")
            idle()
            assertTrue(core.view.effectiveReaderOverride!!.pending)
            assertEquals(0L, count("segmentation_overrides"))
            sql { it.execSQL("DROP TRIGGER fail_override") }
            click("boundary_retry")
            idle()
            reload()
            assertEquals(listOf("国", "家公務員", "国", "家公務員"), surfaces().take(4))
            assertEquals(1L, count("segmentation_overrides"))
            dispatch(Event.OpenSettings)
            dispatch(Event.SetReaderTypography(core.view.readerTypography!!.copy(furigana = true)))
            dispatch(Event.CloseSettings)
            idle()
            assertTrue(core.view.readerFurigana.none { it.range.start.value < 30 })
            // Disable the real clock, then persist a deterministic absolute duration.
            compose.runOnUiThread { core.setReaderActive(false) }
            idle()
            dispatch(
                Event.CheckpointReadingTime(
                    core.view.readerTime!!.copy(pending = PendingReadingTime(321))
                )
            )
            sql {
                it.execSQL(
                    "CREATE TRIGGER fail_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT,'fixture'); END"
                )
            }
            click("next_page")
            compose.waitUntil(30_000) { !core.view.storageBusy && core.view.documentError != null }
            assertTrue(core.view.completionSavePending)
            assertEquals(0L, count("reading_occurrences"))
            sql {
                it.execSQL("UPDATE segmentation_overrides SET active=0")
                it.execSQL("DROP TRIGGER fail_completion")
            }
            compose.onNodeWithText("Retry").performScrollTo().performClick()
            idle()
            assertEquals(ContentRoute.MAIN, core.view.contentRoute)
            assertEquals(5L, count("reading_occurrences"))
            assertEquals(1L, count("reading_completions"))
            sql { db ->
                db.rawQuery("SELECT active_time_ms FROM reading_completions", null).use {
                    it.moveToFirst()
                    assertEquals(321L, it.getLong(0))
                }
            }
            dispatch(Event.RetryReaderCompletion)
            assertEquals(1L, count("reading_completions"))
            dispatch(Event.OpenReader(id))
            assertEquals(0L, core.view.reader!!.position.offset.value)
            assertEquals(listOf("国家", "公務員", "国家", "公務員"), surfaces().take(4))
            assertTrue(core.view.reader!!.finishedAtLeastOnce)
        }
}
