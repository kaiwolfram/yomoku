package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiEntryActionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val selection =
        AnkiSelection.Effective(
            EffectiveReaderSelection(
                EffectivePageIdentity(
                    1UL,
                    ReaderTokenKey(
                        DocumentId(1),
                        TextAnchor(
                            SourceId(2),
                            ChapterId(3),
                            TextRange(TextOffset(0), TextOffset(6)),
                        ),
                        ReaderAnalysis.Available(AnalysisId(4)),
                    ),
                ),
                0UL,
            )
        )

    @Test
    fun actionsAreEntrySpecificAndCheckingFailuresDoNotOfferAdd() {
        val entry = mutableStateOf<AnkiEntryView?>(null)
        val events = mutableListOf<Event>()
        compose.setContent {
            YomokuTheme {
                Column {
                    Text("Dictionary")
                    AnkiEntryAction(entry.value, true, events::add, {})
                }
            }
        }
        compose.onNodeWithContentDescription("Add 猫 to Anki").assertDoesNotExist()
        compose.runOnIdle {
            entry.value =
                AnkiEntryView(selection, AnkiEntryKey.Word(Long.MAX_VALUE), "猫", AnkiEntryPhase.Add)
        }
        compose.onNodeWithContentDescription("Add 猫 to Anki").performClick()
        compose.runOnIdle {
            assertEquals(
                listOf(Event.AddReaderAnkiEntry(selection, AnkiEntryKey.Word(Long.MAX_VALUE))),
                events,
            )
        }
        compose.runOnIdle { entry.value = entry.value!!.copy(phase = AnkiEntryPhase.Checking) }
        compose.onNodeWithTag("anki_entry_word_${Long.MAX_VALUE}").assertIsNotEnabled()
        compose.runOnIdle {
            entry.value =
                entry.value!!.copy(phase = AnkiEntryPhase.Failed(AnkiExportError.PERMISSIONDENIED))
        }
        compose.onNodeWithContentDescription("Retry checking 猫 in Anki").performClick()
        compose.runOnIdle { assertEquals(Event.LoadReaderAnkiEntries(selection), events.last()) }
        compose.runOnIdle {
            entry.value = entry.value!!.copy(phase = AnkiEntryPhase.Existing(listOf(17L)))
        }
        compose.onNodeWithContentDescription("Open 猫 in Anki").assertExists()
        compose.onNodeWithContentDescription("Add 猫 to Anki").assertDoesNotExist()
    }

    @Test
    fun sheetHasSegmentedStatusChoicesWithoutRedundantHeaderOrAnkiEditor() {
        val status = mutableStateOf(VocabularyStatus.LEARNING)
        val events = mutableListOf<VocabularyStatus>()
        compose.setContent {
            YomokuTheme {
                ReaderWordPanel(
                    ReaderWordHeader(
                        "食べた",
                        "タベタ",
                        "食べる",
                        ReaderVocabulary(VocabularyId(5), status.value),
                    ),
                    true,
                    {},
                    {},
                    {
                        events.add(it)
                        status.value = it
                    },
                ) {
                    Text("Dictionary entries")
                }
            }
        }
        compose.onNodeWithTag("word_status_pill").assertExists()
        compose.onNodeWithText("Reading: タベタ").assertDoesNotExist()
        compose.onNodeWithText("Dictionary form: 食べる").assertDoesNotExist()
        compose.onNodeWithTag("anki_export").assertDoesNotExist()
        compose.onNodeWithTag("word_status_LEARNING").assertIsSelected()
        compose.onNodeWithTag("word_status_KNOWN").performClick().assertIsSelected()
        compose.runOnIdle { assertEquals(listOf(VocabularyStatus.KNOWN), events) }
    }
}
