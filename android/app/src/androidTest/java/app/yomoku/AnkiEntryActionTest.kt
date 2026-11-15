package app.yomoku

import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
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
            ),
            1UL,
        )

    @Test
    fun actionsAreEntrySpecificAndCheckingFailuresDoNotOfferAdd() {
        val entry = mutableStateOf<AnkiEntryView?>(null)
        val events = mutableListOf<Event>()
        compose.setContent {
            YomokuTheme {
                Column {
                    Text("Dictionary")
                    AnkiEntryAction(entry.value, true, events::add)
                }
            }
        }
        compose.onNodeWithContentDescription("Add 猫【ねこ】 to Anki").assertDoesNotExist()
        compose.runOnIdle {
            entry.value =
                AnkiEntryView(
                    selection,
                    AnkiEntryKey.Word(Long.MAX_VALUE, ReadingMatch(0UL, 0UL)),
                    "猫",
                    "ねこ",
                    AnkiEntryPhase.Add,
                )
        }
        compose.onNodeWithContentDescription("Add 猫【ねこ】 to Anki").performClick()
        compose.runOnIdle {
            assertEquals(
                listOf(
                    Event.AddReaderAnkiEntry(
                        selection,
                        AnkiEntryKey.Word(Long.MAX_VALUE, ReadingMatch(0UL, 0UL)),
                    )
                ),
                events,
            )
        }
        compose.runOnIdle { entry.value = entry.value!!.copy(phase = AnkiEntryPhase.Checking) }
        compose.onNodeWithTag("anki_entry_word_${Long.MAX_VALUE}_0_0").assertIsNotEnabled()
        compose.runOnIdle {
            entry.value =
                entry.value!!.copy(phase = AnkiEntryPhase.Failed(AnkiExportError.PERMISSIONDENIED))
        }
        compose.onNodeWithContentDescription("Retry checking 猫【ねこ】 in Anki").performClick()
        compose.runOnIdle { assertEquals(Event.LoadReaderAnkiEntries(selection), events.last()) }
        compose.runOnIdle {
            entry.value = entry.value!!.copy(phase = AnkiEntryPhase.Existing(listOf(17L)))
        }
        compose.onNodeWithContentDescription("Open 猫【ねこ】 in Anki").assertExists()
        compose.onNodeWithContentDescription("Add 猫【ねこ】 to Anki").assertDoesNotExist()
    }

    @Test
    fun dictionaryReadingsHaveSeparateActionsAndOnlyThePriorityPairHasAStar() {
        val entry =
            WordEntry(
                listOf(Spelling("生", emptyList(), listOf("spec1"))),
                listOf(
                    Reading("せい", false, emptyList(), emptyList(), listOf("spec1")),
                    Reading("なま", false, emptyList(), emptyList(), emptyList()),
                ),
                listOf(
                    Sense(
                        emptyList(),
                        emptyList(),
                        listOf("noun"),
                        emptyList(),
                        emptyList(),
                        emptyList(),
                        emptyList(),
                        emptyList(),
                        emptyList(),
                        emptyList(),
                        listOf(Translation("life; living", "eng", null, null)),
                    )
                ),
            )
        val first = ReadingMatch(0UL, 0UL)
        val second = ReadingMatch(0UL, 1UL)
        val matches = listOf(WordMatch(first, listOf(0UL)), WordMatch(second, listOf(0UL)))
        val candidates =
            DictionaryCandidates(
                "生",
                null,
                listOf(
                    WordCandidate(
                        1,
                        entry,
                        listOf(WordFormMatch("生", matches), WordFormMatch("生", matches)),
                    )
                ),
                emptyList(),
            )
        val events = mutableListOf<Event>()
        val firstPhase = mutableStateOf<AnkiEntryPhase>(AnkiEntryPhase.Add)
        compose.setContent {
            YomokuTheme {
                Surface {
                    Column(Modifier.testTag("dictionary_preview")) {
                        ReaderDictionary(
                            0UL,
                            candidates,
                            action = { key ->
                                val reading = (key as AnkiEntryKey.Word).reading
                                AnkiEntryAction(
                                    AnkiEntryView(
                                        selection,
                                        key,
                                        "生",
                                        entry.readings[reading.readingIndex.toInt()].text,
                                        if (reading == first) firstPhase.value
                                        else AnkiEntryPhase.Add,
                                    ),
                                    true,
                                    events::add,
                                )
                            },
                        )
                    }
                }
            }
        }
        compose.onAllNodesWithContentDescription("High priority entry").assertCountEquals(1)
        compose.onNodeWithContentDescription("Add 生【せい】 to Anki").performClick()
        compose.onNodeWithContentDescription("Add 生【なま】 to Anki").performClick()
        compose.runOnIdle {
            assertEquals(
                listOf(
                    Event.AddReaderAnkiEntry(selection, AnkiEntryKey.Word(1, first)),
                    Event.AddReaderAnkiEntry(selection, AnkiEntryKey.Word(1, second)),
                ),
                events,
            )
        }
        compose.runOnIdle { firstPhase.value = AnkiEntryPhase.Existing(listOf(17L)) }
        compose.onNodeWithContentDescription("Open 生【せい】 in Anki").assertExists()
        compose.onNodeWithContentDescription("Add 生【なま】 to Anki").assertIsEnabled()
        val bitmap = compose.onNodeWithTag("dictionary_preview").captureToImage().asAndroidBitmap()
        File(compose.activity.cacheDir, "anki-dictionary.png").outputStream().use {
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
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
