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
internal fun VocabularyWordSheet(view: ViewModel, dispatch: (Event) -> Unit) {
    val state = view.vocabulary
    val word = state.word
    val enabled = !view.storageBusy && state.wordStatus == null
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
    ) {
        val candidates = word.candidates
        if (candidates != null) ReaderDictionary(word.row.encounteredCount, candidates)
        else
            ReaderWordLookupState(word.error != null, enabled) {
                dispatch(Event.RetryVocabularyWord)
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
