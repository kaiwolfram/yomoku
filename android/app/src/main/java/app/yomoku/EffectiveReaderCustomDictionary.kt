package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.EffectiveReaderWordState
import app.yomoku.shared.EffectiveWordDetails
import app.yomoku.shared.Event

/** Literal drafts belong to the compact selection; native state owns accepted writes/retries. */
@Composable
internal fun EffectiveReaderCustomDictionary(
    details: EffectiveWordDetails,
    state: EffectiveReaderWordState,
    enabled: Boolean,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
) {
    key(details.selection) {
        val entry = details.customEntry
        var editing by remember { mutableStateOf(false) }
        var meaning by remember { mutableStateOf("") }
        var reading by remember { mutableStateOf("") }
        var submitted by remember { mutableStateOf<Pair<String, String>?>(null) }
        LaunchedEffect(entry, state.customEntryPending, submitted) {
            val saved = submitted
            if (
                !state.customEntryPending &&
                    saved != null &&
                    entry?.englishMeaning == saved.first &&
                    entry.reading == saved.second
            ) {
                editing = false
                submitted = null
            }
        }
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                stringResource(R.string.word_custom_title),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.testTag("word_custom_entry"),
            )
            if (entry != null) {
                Text(
                    stringResource(R.string.word_reading, entry.reading),
                    Modifier.testTag("word_custom_reading"),
                )
                Text(entry.englishMeaning, Modifier.testTag("word_custom_meaning"))
            }
            if (editing) {
                OutlinedTextField(
                    value = meaning,
                    onValueChange = {
                        onInteraction()
                        meaning = it
                    },
                    label = { Text(stringResource(R.string.word_custom_meaning_label)) },
                    enabled = enabled,
                    modifier = Modifier.fillMaxWidth().testTag("word_custom_meaning_input"),
                )
                OutlinedTextField(
                    value = reading,
                    onValueChange = {
                        onInteraction()
                        reading = it
                    },
                    label = { Text(stringResource(R.string.word_custom_reading_label)) },
                    enabled = enabled,
                    modifier = Modifier.fillMaxWidth().testTag("word_custom_reading_input"),
                )
                Row {
                    TextButton(
                        onClick = {
                            onInteraction()
                            submitted = meaning to reading
                            dispatch(
                                Event.SaveEffectiveReaderCustomEntry(
                                    details.selection,
                                    meaning,
                                    reading,
                                )
                            )
                        },
                        enabled = enabled && validCustomField(meaning) && validCustomField(reading),
                        modifier = Modifier.testTag("word_custom_save"),
                    ) {
                        Text(stringResource(R.string.word_custom_save))
                    }
                    TextButton(
                        onClick = {
                            onInteraction()
                            editing = false
                            submitted = null
                        },
                        enabled = enabled,
                        modifier = Modifier.testTag("word_custom_cancel"),
                    ) {
                        Text(stringResource(R.string.cancel))
                    }
                }
            } else {
                Row {
                    TextButton(
                        onClick = {
                            onInteraction()
                            meaning = entry?.englishMeaning.orEmpty()
                            reading = entry?.reading.orEmpty()
                            submitted = null
                            editing = true
                        },
                        enabled = enabled,
                        modifier = Modifier.testTag("word_custom_edit"),
                    ) {
                        Text(
                            stringResource(
                                if (entry == null) R.string.word_custom_add
                                else R.string.word_custom_edit
                            )
                        )
                    }
                    if (entry != null) {
                        TextButton(
                            onClick = {
                                onInteraction()
                                dispatch(Event.DeleteEffectiveReaderCustomEntry(details.selection))
                            },
                            enabled = enabled,
                            modifier = Modifier.testTag("word_custom_delete"),
                        ) {
                            Text(stringResource(R.string.word_custom_delete))
                        }
                    }
                }
            }
        }
    }
}

private fun validCustomField(value: String): Boolean =
    utf8Boundaries(value) != null && value.any { !rustWhitespace(it) }

// Match Rust str::trim/Surface::new (Unicode White_Space). Kotlin's isBlank also
// includes U+001C..U+001F and omits U+0085 on the JVM, changing valid literal parts.
internal fun rustWhitespace(char: Char): Boolean =
    char in '\u0009'..'\u000D' ||
        char == '\u0020' ||
        char == '\u0085' ||
        char == '\u00A0' ||
        char == '\u1680' ||
        char in '\u2000'..'\u200A' ||
        char == '\u2028' ||
        char == '\u2029' ||
        char == '\u202F' ||
        char == '\u205F' ||
        char == '\u3000'
