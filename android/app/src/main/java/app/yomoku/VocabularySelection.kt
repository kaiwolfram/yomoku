package app.yomoku

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*

@Composable
internal fun VocabularySelectAll(view: ViewModel, dispatch: (Event) -> Unit, enabled: Boolean) {
    TextButton(
        onClick = { dispatch(Event.SelectAllVocabulary) },
        enabled = enabled && view.vocabulary.matchingCount != null && view.vocabulary.error == null,
        modifier = Modifier.testTag("vocabulary_select_all"),
    ) {
        Text(stringResource(R.string.vocabulary_select_all))
    }
}

@Composable
internal fun VocabularySelectionActions(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    enabled: Boolean,
) {
    val selection = view.vocabulary.selection ?: return
    var statusMenu by remember { mutableStateOf(false) }
    Column(Modifier.padding(horizontal = 16.dp)) {
        Text(
            stringResource(R.string.vocabulary_selected, selection.size.toString()),
            modifier = Modifier.testTag("vocabulary_selected_count"),
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            VocabularySelectAll(view, dispatch, enabled)
            Box {
                TextButton(
                    onClick = { statusMenu = true },
                    enabled = enabled && selection.isNotEmpty(),
                    modifier = Modifier.testTag("vocabulary_change_status"),
                ) {
                    Text(stringResource(R.string.vocabulary_change_status))
                }
                DropdownMenu(expanded = statusMenu, onDismissRequest = { statusMenu = false }) {
                    VocabularyStatus.entries.forEach { status ->
                        DropdownMenuItem(
                            text = { Text(stringResource(status.labelResource())) },
                            onClick = {
                                statusMenu = false
                                dispatch(
                                    Event.RequestVocabularyBulkAction(
                                        VocabularyBulkAction.SetStatus(
                                            status,
                                            System.currentTimeMillis(),
                                        )
                                    )
                                )
                            },
                            enabled = enabled,
                            modifier = Modifier.testTag("vocabulary_bulk_status_${status.name}"),
                        )
                    }
                }
            }
            TextButton(
                onClick = {
                    dispatch(Event.RequestVocabularyBulkAction(VocabularyBulkAction.Delete))
                },
                enabled = enabled && selection.isNotEmpty(),
                modifier = Modifier.testTag("vocabulary_delete_selected"),
            ) {
                Text(stringResource(R.string.library_delete_selected))
            }
            TextButton(
                onClick = { dispatch(Event.CancelVocabularySelection) },
                enabled = enabled,
                modifier = Modifier.testTag("vocabulary_cancel_selection"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        }
    }
}

@Composable
internal fun VocabularyBulkDialog(view: ViewModel, dispatch: (Event) -> Unit) {
    val state = view.vocabulary
    val command = state.confirmation
    val capturing = state.selectingAll || (state.bulkError != null && command == null)
    if (command == null && !capturing) return
    val cancel =
        if (command != null) Event.CancelVocabularyBulkAction else Event.CancelVocabularySelection
    AlertDialog(
        onDismissRequest = { if (!view.storageBusy) dispatch(cancel) },
        title = {
            Text(
                stringResource(
                    if (capturing) R.string.vocabulary_select_all
                    else R.string.vocabulary_confirm_title
                )
            )
        },
        text = {
            Column {
                if (command != null) {
                    when (val action = command.action) {
                        VocabularyBulkAction.Delete ->
                            Text(
                                stringResource(
                                    R.string.vocabulary_delete_confirmation,
                                    command.ids.size.toString(),
                                )
                            )
                        is VocabularyBulkAction.SetStatus ->
                            Text(
                                stringResource(
                                    R.string.vocabulary_status_confirmation,
                                    command.ids.size.toString(),
                                    stringResource(action.status.labelResource()),
                                )
                            )
                    }
                }
                if (state.bulkError != null)
                    Text(
                        stringResource(
                            if (capturing) R.string.vocabulary_capture_error
                            else R.string.vocabulary_bulk_error
                        ),
                        Modifier.testTag("vocabulary_bulk_error"),
                    )
                if (view.storageBusy)
                    LinearProgressIndicator(
                        Modifier.fillMaxWidth().testTag("vocabulary_bulk_loading")
                    )
            }
        },
        confirmButton = {
            if (!capturing || state.bulkError != null)
                TextButton(
                    onClick = {
                        // Retry refers to the retained native command/query. Never
                        // rebuild its selection or timestamp from current shell state.
                        dispatch(
                            when {
                                capturing -> Event.RetryVocabularySelection
                                state.bulkError != null -> Event.RetryVocabularyBulkAction
                                else -> Event.ConfirmVocabularyBulkAction(checkNotNull(command))
                            }
                        )
                    },
                    enabled = !view.storageBusy,
                    modifier = Modifier.testTag("vocabulary_confirm_bulk"),
                ) {
                    Text(
                        stringResource(
                            if (state.bulkError != null) R.string.retry
                            else R.string.vocabulary_confirm
                        )
                    )
                }
        },
        dismissButton = {
            TextButton(
                onClick = { dispatch(cancel) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("vocabulary_cancel_bulk"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        },
        modifier = Modifier.testTag("vocabulary_bulk_dialog"),
    )
}
