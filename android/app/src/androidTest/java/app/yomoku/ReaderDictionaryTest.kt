package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderDictionaryTest {
    @Test
    fun applicableViewsKeepAllLicensedReadingsAndHideOtherSpellingsAndSenses() {
        val sense =
            Sense(
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
                emptyList(),
            )
        val entry =
            WordEntry(
                listOf(
                    Spelling("生", emptyList(), emptyList()),
                    Spelling("別", emptyList(), emptyList()),
                ),
                listOf(
                    Reading("せい", false, emptyList(), emptyList(), emptyList()),
                    Reading("なま", false, emptyList(), emptyList(), emptyList()),
                    Reading("べつ", false, emptyList(), emptyList(), emptyList()),
                    Reading("かな", true, emptyList(), emptyList(), emptyList()),
                ),
                listOf(sense, sense, sense),
            )
        val candidate =
            WordCandidate(
                1,
                entry,
                listOf(
                    WordFormMatch(
                        "生",
                        listOf(
                            WordMatch(ReadingMatch(0UL, 0UL), listOf(0UL, 1UL)),
                            WordMatch(ReadingMatch(0UL, 1UL), listOf(0UL)),
                        ),
                    ),
                    // A surface/base union can repeat the same applicable pair.
                    WordFormMatch(
                        "せい",
                        listOf(WordMatch(ReadingMatch(0UL, 0UL), listOf(0UL, 1UL))),
                    ),
                ),
            )
        assertEquals(listOf("生【せい】", "生【なま】"), applicableSenseReadings(candidate, 0))
        assertEquals(listOf("生【せい】"), applicableSenseReadings(candidate, 1))
        assertEquals(emptyList<String>(), applicableSenseReadings(candidate, 2))
        assertEquals(
            "かな",
            dictionaryReadingLabel(entry.kanji, entry.readings, ReadingMatch(null, 3UL)),
        )
    }

    @Test
    fun grammaticalLabelsAreCompactAndUnknownEnglishLabelsSurvive() {
        val labels =
            mapOf(
                "noun (common) (futsuumeishi)" to "noun",
                "Ichidan verb" to "Ichidan verb",
                "Godan verb with 'ru' ending" to "Godan verb",
                "adjective (keiyoushi)" to "i-adjective",
                "adjectival nouns or quasi-adjectives (keiyodoshi)" to "na-adjective",
                "transitive verb" to "transitive verb",
                "unclassified English label" to "unclassified English label",
            )
        for ((source, expected) in labels) assertEquals(expected, compactPartOfSpeech(source))
    }
}
