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
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderWordDetails
import app.yomoku.shared.ReaderWordSelection

/** Drafts and retry payloads belong to one original token, never to a later selection. */
@Composable
internal fun ReaderCustomDictionary(
    details: ReaderWordDetails,
    enabled: Boolean,
    error: DocumentError?,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
) {
    val selection = ReaderWordSelection(details.key, details.token.tokenIndex)
    key(selection) {
        val entry = details.customEntry
        var editing by remember { mutableStateOf(false) }
        var meaning by remember { mutableStateOf("") }
        var reading by remember { mutableStateOf("") }
        // Store the exact submitted command. A retry must not silently use edited fields.
        var attempted by remember { mutableStateOf<Event?>(null) }
        LaunchedEffect(entry, enabled, error, attempted) {
            if (enabled && error == null) {
                val committed =
                    when (val action = attempted) {
                        is Event.SaveReaderCustomEntry ->
                            entry?.englishMeaning == action.englishMeaning &&
                                entry.reading == action.reading
                        is Event.DeleteReaderCustomEntry -> entry == null
                        else -> false
                    }
                if (committed) {
                    attempted = null
                    editing = false
                }
            }
        }
        val submit: (Event) -> Unit = {
            onInteraction()
            attempted = it
            dispatch(it)
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
                            submit(Event.SaveReaderCustomEntry(selection, meaning, reading))
                        },
                        enabled = enabled && meaning.isNotBlank() && reading.isNotBlank(),
                        modifier = Modifier.testTag("word_custom_save"),
                    ) {
                        Text(stringResource(R.string.word_custom_save))
                    }
                    TextButton(
                        onClick = {
                            onInteraction()
                            editing = false
                            attempted = null
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
                            attempted = null
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
                            onClick = { submit(Event.DeleteReaderCustomEntry(selection)) },
                            enabled = enabled,
                            modifier = Modifier.testTag("word_custom_delete"),
                        ) {
                            Text(stringResource(R.string.word_custom_delete))
                        }
                    }
                }
            }
            val action = attempted
            if (error != null && action != null) {
                val deleting = action is Event.DeleteReaderCustomEntry
                Text(
                    stringResource(
                        if (deleting) R.string.word_custom_delete_failed
                        else R.string.word_custom_save_failed
                    ),
                    modifier = Modifier.testTag("word_custom_error"),
                )
                TextButton(
                    onClick = { submit(action) },
                    enabled = enabled,
                    modifier = Modifier.testTag("word_custom_retry"),
                ) {
                    Text(
                        stringResource(
                            if (deleting) R.string.word_custom_retry_delete
                            else R.string.word_custom_retry_save
                        )
                    )
                }
            }
        }
    }
}
