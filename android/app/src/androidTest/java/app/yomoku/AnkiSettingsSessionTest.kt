package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
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

    private fun open() {
        directory.mkdirs()
        main {
            core =
                CoreSession(
                    database.path,
                    sudachiDictionaryProvider(instrumentation.targetContext),
                    readAnkiDecks = {
                        assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                        reads.incrementAndGet()
                        AnkiReadResult.Value(
                            listOf(
                                AnkiProviderDeck(Long.MAX_VALUE, "日本語", false),
                                AnkiProviderDeck(42, "Filtered", true),
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
        await { !it.storageBusy }
    }

    private fun close() {
        main { core.close() }
        compose.waitUntil(30_000) { core.isClosed }
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun explicitConfigurationPersistsClearsAndRetainsAcrossCompositionAndNavigation() {
        open()
        try {
            assertEquals(0, reads.get())
            assertEquals(AnkiSettingsPhase.UNLOADED, core.view.ankiSettings.phase)
            click("anki_settings_load")
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
            assertEquals(before.ankiRecovery, core.view.ankiRecovery)
            assertEquals(before.documents, core.view.documents)
            close()
            open()
            assertEquals(AnkiSettingsPhase.UNLOADED, core.view.ankiSettings.phase)
            assertEquals(AnkiDeckDiscoveryState.Unchecked, core.ankiDecks.state)
            assertEquals(1, reads.get())
            click("anki_settings_load")
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertEquals(Long.MAX_VALUE, core.view.ankiSettings.defaultDeckId)
            click("anki_settings_clear")
            await {
                it.ankiSettings.phase == AnkiSettingsPhase.READY &&
                    it.ankiSettings.defaultDeckId == null
            }
            close()
            open()
            click("anki_settings_load")
            await { it.ankiSettings.phase == AnkiSettingsPhase.READY }
            assertNull(core.view.ankiSettings.defaultDeckId)
        } finally {
            close()
            directory.deleteRecursively()
        }
    }

    @Test
    fun failedSaveRequiresAuthoritativeReloadAndDoesNotRepeatUpdate() {
        open()
        try {
            click("anki_settings_load")
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
