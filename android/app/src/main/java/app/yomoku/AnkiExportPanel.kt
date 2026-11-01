package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiExportPhase
import app.yomoku.shared.AnkiExportView
import app.yomoku.shared.Event

/** Frozen native intent, independent of the currently open dictionary selection. */
@Composable
internal fun AnkiExportPanel(
    state: AnkiExportView,
    dispatch: (Event) -> Unit,
    browser: AnkiNoteBrowser? = null,
    onInteraction: () -> Unit = {},
) {
    val phase = state.phase
    Column(Modifier.testTag("anki_export"), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            stringResource(R.string.anki_export_title),
            style = MaterialTheme.typography.titleMedium,
        )
        Text(state.surface, Modifier.testTag("anki_export_surface"))
        Text(stringResource(R.string.anki_export_deck, state.deckId.toString()))
        val message =
            when (phase) {
                AnkiExportPhase.Prepared -> stringResource(R.string.anki_export_prepared)
                AnkiExportPhase.Checking -> stringResource(R.string.anki_export_checking)
                AnkiExportPhase.CreateModel -> stringResource(R.string.anki_export_create_model)
                is AnkiExportPhase.ReuseModel -> stringResource(R.string.anki_export_reuse_model)
                is AnkiExportPhase.Existing ->
                    stringResource(R.string.anki_export_existing, phase.value.joinToString(", "))
                AnkiExportPhase.Beginning,
                AnkiExportPhase.Ready -> stringResource(R.string.anki_export_beginning)
                AnkiExportPhase.Invoking -> stringResource(R.string.anki_export_invoking)
                AnkiExportPhase.Saving -> stringResource(R.string.anki_export_saving)
                is AnkiExportPhase.NoteAdded -> stringResource(R.string.anki_export_added)
                is AnkiExportPhase.ModelCreated ->
                    stringResource(R.string.anki_export_model_created)
                AnkiExportPhase.NotSubmitted -> stringResource(R.string.anki_export_not_submitted)
                is AnkiExportPhase.NoteUncertain ->
                    stringResource(R.string.anki_export_note_uncertain)
                is AnkiExportPhase.ModelUncertain ->
                    stringResource(R.string.anki_export_model_uncertain)
                is AnkiExportPhase.ProviderFailed -> stringResource(phase.value.messageResource())
                is AnkiExportPhase.StorageFailed ->
                    stringResource(R.string.anki_export_storage_failed)
            }
        Text(message, Modifier.testTag("anki_export_status"))
        if (browser != null) {
            val attempt = browser.state?.takeIf { it.export == state }
            for (id in state.openableNoteIds()) {
                TextButton(
                    onClick = {
                        onInteraction()
                        browser.open(state, id)
                    },
                    enabled = attempt?.phase != AnkiBrowserPhase.Resolving,
                    modifier = Modifier.testTag("anki_note_open_$id"),
                ) {
                    Text(stringResource(R.string.anki_note_open, id.toString()))
                }
            }
            attempt?.let {
                val status =
                    when (val value = it.phase) {
                        AnkiBrowserPhase.Resolving -> R.string.anki_note_opening
                        AnkiBrowserPhase.Unavailable -> R.string.anki_note_browser_unavailable
                        is AnkiBrowserPhase.Failed -> value.error.exportError().messageResource()
                    }
                Text(stringResource(status), Modifier.testTag("anki_note_browser_status"))
            }
        }
        when (phase) {
            AnkiExportPhase.Prepared ->
                TextButton(
                    onClick = { dispatch(Event.BeginAnkiExportPreflight) },
                    modifier = Modifier.testTag("anki_export_check"),
                ) {
                    Text(stringResource(R.string.anki_export_check))
                }
            is AnkiExportPhase.ProviderFailed ->
                TextButton(
                    onClick = { dispatch(Event.RetryAnkiExportPreflight) },
                    modifier = Modifier.testTag("anki_export_check"),
                ) {
                    Text(stringResource(R.string.anki_export_check_again))
                }
            AnkiExportPhase.CreateModel,
            is AnkiExportPhase.ReuseModel ->
                TextButton(
                    onClick = {
                        dispatch(Event.BeginPreparedAnkiExport(state.preparationId, phase))
                    },
                    modifier = Modifier.testTag("anki_export_begin"),
                ) {
                    Text(
                        stringResource(
                            if (phase == AnkiExportPhase.CreateModel) R.string.anki_export_create
                            else R.string.anki_export_add
                        )
                    )
                }
            else -> Unit
        }
        // A failed live commit must be restored through recovery. Never offer a write Retry.
        when (phase) {
            AnkiExportPhase.Beginning,
            AnkiExportPhase.Ready,
            AnkiExportPhase.Invoking,
            AnkiExportPhase.Saving,
            is AnkiExportPhase.StorageFailed -> Unit
            else ->
                TextButton(
                    onClick = { dispatch(Event.ClearAnkiExport(state.preparationId)) },
                    modifier = Modifier.testTag("anki_export_dismiss"),
                ) {
                    Text(stringResource(R.string.close))
                }
        }
    }
}
