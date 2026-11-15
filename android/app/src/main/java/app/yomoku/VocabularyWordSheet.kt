package app.yomoku

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import app.yomoku.shared.*

@Composable
internal fun VocabularyWordSheet(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    ankiBrowser: AnkiNoteBrowser? = null,
) {
    val state = view.vocabulary
    val word = state.word
    val enabled = !view.storageBusy && state.wordStatus == null
    val selection = word?.selection?.let { AnkiSelection.Vocabulary(it) }
    ObserveReaderAnkiEntries(
        selection,
        view.ankiSetup,
        enabled && word?.candidates != null && !word.lookup.loading && word.lookup.error == null,
        dispatch,
    )
    if (word == null) return
    ReaderWordPanel(
        ReaderWordHeader(
            word.row.surface,
            null,
            null,
            ReaderVocabulary(word.row.id, word.row.status),
        ),
        enabled,
        onClose = { dispatch(Event.DismissVocabularyWord) },
        onStatus = {
            dispatch(
                Event.SetVocabularyWordStatus(
                    VocabularyWordStatusCommand(word.row.id, it, System.currentTimeMillis())
                )
            )
        },
        choices = {
            DictionaryPartChoices(word.lookup, enabled) {
                dispatch(Event.ChooseDictionaryPart(checkNotNull(selection), it))
            }
        },
    ) {
        val candidates = word.candidates
        if (candidates != null && !word.lookup.loading && word.lookup.error == null)
            ReaderDictionary(
                word.row.encounteredCount,
                candidates,
                action = { key ->
                    AnkiEntryAction(
                        view.ankiEntries.find { it.selection == selection && it.key == key },
                        enabled,
                        dispatch,
                        ankiBrowser,
                    )
                },
                notice = { key ->
                    AnkiEntryNotice(
                        view.ankiEntries.find { it.selection == selection && it.key == key },
                        ankiBrowser,
                    )
                },
            )
        else
            ReaderWordLookupState(word.error != null || word.lookup.error != null, enabled) {
                if (candidates != null) dispatch(Event.RetryDictionaryPart(checkNotNull(selection)))
                else dispatch(Event.RetryVocabularyWord)
            }
        VocabularyWordStatusRetry(view, dispatch)
    }
}

@Composable
internal fun VocabularyWordStatusRetry(view: ViewModel, dispatch: (Event) -> Unit) {
    val state = view.vocabulary
    if (state.wordStatusError == null) return
    Column {
        Text(
            stringResource(R.string.word_status_pending_failed),
            Modifier.testTag("vocabulary_word_status_error"),
        )
        Row {
            TextButton(
                onClick = { dispatch(Event.RetryVocabularyWordStatus) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("vocabulary_word_status_retry"),
            ) {
                Text(stringResource(R.string.retry))
            }
            TextButton(
                onClick = { dispatch(Event.CancelVocabularyWordStatus) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("vocabulary_word_status_cancel"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        }
    }
}
