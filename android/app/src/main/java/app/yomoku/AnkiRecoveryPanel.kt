package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiExportError
import app.yomoku.shared.AnkiRecoveryPhase
import app.yomoku.shared.AnkiRecoveryReadTarget
import app.yomoku.shared.AnkiRecoveryView
import app.yomoku.shared.Event

@Composable
internal fun AnkiRecoveryPanel(state: AnkiRecoveryView, dispatch: (Event) -> Unit) {
    Column(Modifier.testTag("anki_recovery"), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            stringResource(R.string.anki_recovery_title),
            style = MaterialTheme.typography.titleMedium,
        )
        Text(
            stringResource(
                when (state.phase) {
                    AnkiRecoveryPhase.LOADING -> R.string.anki_recovery_loading
                    AnkiRecoveryPhase.READY -> R.string.anki_recovery_ready
                    AnkiRecoveryPhase.UNRESOLVED -> R.string.anki_recovery_unresolved
                    AnkiRecoveryPhase.CHECKING -> R.string.anki_recovery_checking
                    AnkiRecoveryPhase.SAVING -> R.string.anki_recovery_saving
                    AnkiRecoveryPhase.BUSY -> R.string.anki_recovery_busy
                    AnkiRecoveryPhase.FAILED -> R.string.anki_recovery_failed
                }
            ),
            Modifier.testTag("anki_recovery_status"),
        )
        if (state.notePending || state.modelPending) {
            Text(stringResource(R.string.anki_recovery_read_only))
        }
        state.storageError?.let {
            Text(stringResource(it.messageResource()), color = MaterialTheme.colorScheme.error)
        }
        state.providerError?.let {
            Text(stringResource(it.messageResource()), color = MaterialTheme.colorScheme.error)
        }
        // Each action is authorized again against native state; opening Settings never dispatches
        // it.
        Row {
            if (state.notePending) {
                TextButton(
                    onClick = {
                        dispatch(Event.BeginAnkiRecoveryRead(AnkiRecoveryReadTarget.NOTE))
                    },
                    enabled = state.canCheckNote,
                    modifier = Modifier.testTag("anki_recovery_note"),
                ) {
                    Text(stringResource(R.string.anki_recovery_check_note))
                }
            }
            if (state.modelPending) {
                TextButton(
                    onClick = {
                        dispatch(Event.BeginAnkiRecoveryRead(AnkiRecoveryReadTarget.MODEL))
                    },
                    enabled = state.canCheckModel,
                    modifier = Modifier.testTag("anki_recovery_model"),
                ) {
                    Text(stringResource(R.string.anki_recovery_check_model))
                }
            }
        }
        if (state.canRetry) {
            TextButton(
                onClick = { dispatch(Event.RetryAnkiRecovery) },
                modifier = Modifier.testTag("anki_recovery_retry"),
            ) {
                Text(stringResource(R.string.retry))
            }
        }
    }
}

private fun AnkiExportError.messageResource(): Int =
    when (this) {
        AnkiExportError.PROVIDERUNAVAILABLE -> R.string.anki_error_unavailable
        AnkiExportError.UNSUPPORTEDPROVIDER -> R.string.anki_error_unsupported
        AnkiExportError.PERMISSIONDENIED -> R.string.anki_error_permission
        AnkiExportError.COLLECTIONUNAVAILABLE -> R.string.anki_error_collection
        AnkiExportError.DECKUNAVAILABLE -> R.string.anki_error_deck
        AnkiExportError.INCOMPATIBLEMODEL -> R.string.anki_error_model
        AnkiExportError.DUPLICATELOOKUPFAILED -> R.string.anki_error_duplicates
        AnkiExportError.PROVIDERFAILURE -> R.string.anki_error_provider
        AnkiExportError.INVALIDRESPONSE -> R.string.anki_error_response
    }
