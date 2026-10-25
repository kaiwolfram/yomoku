package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.ReaderWordDetails
import app.yomoku.shared.Reading
import app.yomoku.shared.ReadingMatch
import app.yomoku.shared.Spelling
import app.yomoku.shared.Translation
import app.yomoku.shared.WordCandidate

/** Source labels are already expanded English, so unknown labels remain readable. */
internal fun compactPartOfSpeech(label: String): String =
    when {
        label == "noun (common) (futsuumeishi)" -> "noun"
        label == "noun (temporal) (jisoumeishi)" -> "temporal noun"
        label == "adjectival nouns or quasi-adjectives (keiyodoshi)" -> "na-adjective"
        label == "adjective (keiyoushi)" -> "i-adjective"
        label == "adverb (fukushi)" -> "adverb"
        label == "adverb taking the 'to' particle" -> "to-adverb"
        label.startsWith("Godan verb") ->
            label.substringBefore(" with").substringBefore(" (irregular")
        label.startsWith("Ichidan verb") -> "Ichidan verb"
        label == "suru verb - included" -> "suru verb"
        label == "suru verb - special class" -> "suru verb (special)"
        label == "Kuru verb - special class" -> "kuru verb"
        else -> label
    }

internal fun dictionaryReadingLabel(
    kanji: List<Spelling>,
    readings: List<Reading>,
    match: ReadingMatch,
): String {
    val reading = readings[match.readingIndex.toInt()].text
    val spelling = match.spellingIndex?.let { kanji[it.toInt()].text }
    return if (spelling == null || spelling == reading) reading else "$spelling【$reading】"
}

/** Union only licensed pairs for this sense, retaining source order and all readings. */
internal fun applicableSenseReadings(candidate: WordCandidate, senseIndex: Int): List<String> =
    candidate.forms
        .flatMap { it.matches }
        .filter { senseIndex.toULong() in it.senseIndices }
        .map { dictionaryReadingLabel(candidate.entry.kanji, candidate.entry.readings, it.reading) }
        .distinct()

private fun translationText(translation: Translation): String {
    val qualifiers = listOfNotNull(translation.gType, translation.gGend)
    return if (qualifiers.isEmpty()) translation.text
    else "${translation.text} (${qualifiers.joinToString(", ")})"
}

@Composable
internal fun ReaderDictionary(details: ReaderWordDetails) {
    Text(
        stringResource(R.string.word_encountered, details.encountered.toString()),
        modifier = Modifier.testTag("word_encountered"),
    )
    val candidates = details.candidates
    if (candidates.words.isEmpty() && candidates.names.isEmpty()) {
        Text(stringResource(R.string.word_no_match), Modifier.testTag("word_no_match"))
    }
    if (candidates.words.isNotEmpty()) {
        Text(
            stringResource(R.string.word_definitions),
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.testTag("word_definitions"),
        )
        for (candidate in candidates.words) {
            HorizontalDivider()
            var number = 0
            for ((index, sense) in candidate.entry.senses.withIndex()) {
                val readings = applicableSenseReadings(candidate, index)
                if (readings.isEmpty()) continue
                number++
                Column(
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                    modifier = Modifier.testTag("word_sense_${candidate.entryId}_$index"),
                ) {
                    Text(readings.joinToString("、"), style = MaterialTheme.typography.titleSmall)
                    val labels =
                        (sense.pos.map(::compactPartOfSpeech) +
                                sense.field +
                                sense.misc +
                                sense.dial)
                            .distinct()
                    if (labels.isNotEmpty()) {
                        Text(
                            labels.joinToString(" · "),
                            style = MaterialTheme.typography.labelMedium,
                        )
                    }
                    Text(
                        "$number. ${sense.glosses.joinToString("; ", transform = ::translationText)}"
                    )
                    for (note in sense.sInf) Text(note, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
    if (candidates.names.isNotEmpty()) {
        Text(
            stringResource(R.string.word_names),
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.testTag("word_names"),
        )
        for (candidate in candidates.names) {
            HorizontalDivider()
            val readings =
                candidate.forms
                    .flatMap { it.matches }
                    .map {
                        dictionaryReadingLabel(candidate.entry.kanji, candidate.entry.readings, it)
                    }
                    .distinct()
            Text(readings.joinToString("、"), style = MaterialTheme.typography.titleSmall)
            for ((index, group) in candidate.entry.names.withIndex()) {
                Column(
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                    modifier = Modifier.testTag("word_name_${candidate.entryId}_$index"),
                ) {
                    if (group.nameType.isNotEmpty()) {
                        Text(
                            group.nameType.joinToString(" · "),
                            style = MaterialTheme.typography.labelMedium,
                        )
                    }
                    Text(group.translations.joinToString("; ", transform = ::translationText))
                }
            }
        }
    }
}
