package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Visible controls through actual CoreSession/JNI/FIFO/SQLite with read-only fake discovery. */
@RunWith(AndroidJUnit4::class)
class AnkiSettingsSessionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val directory =
        File(instrumentation.targetContext.noBackupFilesDir, "anki-settings-${System.nanoTime()}")
    private val database
        get() = File(directory, "db.sqlite3")

    private val reads = AtomicInteger()
    private lateinit var core: CoreSession

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun await(predicate: (ViewModel) -> Boolean) {
        compose.waitUntil(30_000) {
            var result = false
            main { result = predicate(core.view) }
            result
        }
    }

    private fun open(needsPermission: () -> Boolean = { false }) {
        directory.mkdirs()
        main {
            core =
                CoreSession(
                    database.path,
                    sudachiDictionaryProvider(instrumentation.targetContext),
                    needsAnkiPermission = needsPermission,
                    readAnkiDecks = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        reads.incrementAndGet()
                        AnkiReadResult.Value(
                            listOf(
                                AnkiProviderDeck(Long.MAX_VALUE, "日本語", false),
                                AnkiProviderDeck(42, "Filtered", true),
                                AnkiProviderDeck(99, "Another deck", false),
                            )
                        )
                    },
                )
            core.checkAnkiDecks() // No discovery outside Settings.
            core.dispatch(Event.OpenSettings)
            compose.activity.setContent {
                YomokuTheme {
                    SettingsScreen(
                        core.view,
                        core::dispatch,
                        core.ankiDecks.state,
                        core::checkAnkiDecks,
                    )
                }
            }
        }
        await { !it.storageBusy && it.ankiSettings.phase == AnkiSettingsPhase.READY }
    }

    private fun close() {
        main { core.close() }
        compose.waitUntil(30_000) { core.isClosed }
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun explicitConfigurationPersistsSelectionAndRetainsAcrossCompositionAndNavigation() {
        open()
        try {
            assertEquals(0, reads.get())
            assertEquals(AnkiSettingsPhase.READY, core.view.ankiSettings.phase)
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertNull(core.view.ankiSettings.defaultDeckId)
            click("anki_decks_check")
            compose.waitUntil(30_000) { core.ankiDecks.state is AnkiDeckDiscoveryState.Ready }
            compose.onNodeWithTag("anki_deck_42").assertDoesNotExist()
            click("anki_deck_${Long.MAX_VALUE}")
            await {
                it.ankiSettings.phase == AnkiSettingsPhase.READY &&
                    it.ankiSettings.defaultDeckId == Long.MAX_VALUE
            }
            val before = core.view
            main {
                core.dispatch(Event.CloseSettings)
                core.checkAnkiDecks()
                core.dispatch(Event.OpenSettings)
                // Replacing the composition consumes the same retained session, as recreation does.
                compose.activity.setContent {
                    YomokuTheme {
                        SettingsScreen(
                            core.view,
                            core::dispatch,
                            core.ankiDecks.state,
                            core::checkAnkiDecks,
                        )
                    }
                }
            }
            compose.waitForIdle()
            assertEquals(1, reads.get())
            assertEquals(before.ankiSettings, core.view.ankiSettings)
            assertEquals(before.ankiExport, core.view.ankiExport)
            assertEquals(before.documents, core.view.documents)
            close()
            open()
            assertEquals(AnkiSettingsPhase.READY, core.view.ankiSettings.phase)
            assertEquals(AnkiDeckDiscoveryState.Unchecked, core.ankiDecks.state)
            assertEquals(1, reads.get())
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertEquals(Long.MAX_VALUE, core.view.ankiSettings.defaultDeckId)
            compose.onNodeWithTag("anki_settings_clear").assertDoesNotExist()
            click("anki_decks_check")
            compose.waitUntil(30_000) { core.ankiDecks.state is AnkiDeckDiscoveryState.Ready }
            click("anki_deck_99")
            await {
                it.ankiSettings.phase == AnkiSettingsPhase.READY &&
                    it.ankiSettings.defaultDeckId == 99L
            }
            close()
            open()
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertEquals(99L, core.view.ankiSettings.defaultDeckId)
        } finally {
            close()
            directory.deleteRecursively()
        }
    }

    @Test
    fun permissionApprovalContinuesOnceAndDenialWaitsForExplicitRetry() {
        var permissionMissing = true
        var prompts = 0
        open { permissionMissing }
        try {
            main {
                core.configureAnki { prompts++ }
                core.configureAnki { prompts++ }
            }
            assertEquals(1, prompts)
            assertEquals(0, reads.get())
            main { core.ankiPermissionResult(false) }
            assertEquals(
                AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied),
                core.ankiDecks.state,
            )
            main { core.ankiPermissionResult(true) } // unsolicited/stale callback
            assertEquals(0, reads.get())
            assertEquals(1, prompts)
            main { core.configureAnki { prompts++ } }
            assertEquals(2, prompts)
            // Replacing the composition does not restart the retained permission request.
            main {
                compose.activity.setContent { SettingsScreen(core.view, core::dispatch) }
                core.configureAnki { prompts++ }
                permissionMissing = false
                core.ankiPermissionResult(true)
                core.ankiPermissionResult(true)
            }
            compose.waitUntil(30_000) { core.ankiDecks.state is AnkiDeckDiscoveryState.Ready }
            assertEquals(1, reads.get())
            assertEquals(2, prompts)
            main { core.configureAnki { prompts++ } }
            compose.waitUntil(30_000) {
                reads.get() == 2 && core.ankiDecks.state is AnkiDeckDiscoveryState.Ready
            }
            assertEquals(2, prompts)
        } finally {
            close()
            directory.deleteRecursively()
        }
    }

    @Test
    fun permissionApprovalAfterLeavingSettingsDoesNotRestartSetup() {
        open { true }
        try {
            var prompts = 0
            main {
                core.configureAnki { prompts++ }
                core.dispatch(Event.CloseSettings)
                core.ankiPermissionResult(true)
                core.dispatch(Event.OpenSettings)
            }
            assertEquals(1, prompts)
            assertEquals(0, reads.get())
            assertEquals(AnkiDeckDiscoveryState.Unchecked, core.ankiDecks.state)
        } finally {
            close()
            directory.deleteRecursively()
        }
    }

    @Test
    fun failedSaveRequiresAuthoritativeReloadAndDoesNotRepeatUpdate() {
        open()
        try {
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            click("anki_decks_check")
            compose.waitUntil(30_000) { core.ankiDecks.state is AnkiDeckDiscoveryState.Ready }
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
                it.execSQL(
                    "CREATE TRIGGER fail_anki_settings BEFORE UPDATE ON anki_settings BEGIN SELECT RAISE(ABORT, 'test'); END"
                )
            }
            click("anki_deck_${Long.MAX_VALUE}")
            await { it.ankiSettings.phase == AnkiSettingsPhase.FAILED }
            assertNull(core.view.ankiSettings.defaultDeckId)
            compose.onNodeWithTag("anki_deck_${Long.MAX_VALUE}").assertIsNotEnabled()
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use {
                it.execSQL("DROP TRIGGER fail_anki_settings")
            }
            click("anki_settings_load")
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertNull(core.view.ankiSettings.defaultDeckId)
            assertEquals(1, reads.get())
            click("anki_deck_${Long.MAX_VALUE}")
            await {
                it.ankiSettings.phase == AnkiSettingsPhase.READY &&
                    it.ankiSettings.defaultDeckId == Long.MAX_VALUE
            }
        } finally {
            close()
            directory.deleteRecursively()
        }
    }
}
