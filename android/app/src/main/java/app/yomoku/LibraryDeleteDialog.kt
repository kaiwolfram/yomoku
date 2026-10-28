package app.yomoku

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryDeleteDialog(view: ViewModel, dispatch: (Event) -> Unit, enabled: Boolean) {
    val bulk = view.libraryBulkDeleteTargets
    val targets = bulk ?: view.libraryDeleteTarget?.let { listOf(it) } ?: return
    val multiple = targets.size > 1
    val canAct =
        enabled && !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    AlertDialog(
        onDismissRequest = { if (!view.storageBusy) dispatch(Event.CancelDocumentDeletion) },
        title = {
            Text(
                if (!multiple) stringResource(R.string.library_delete_title)
                else
                    pluralStringResource(
                        R.plurals.library_bulk_delete_title,
                        targets.size,
                        targets.size,
                    )
            )
        },
        text = {
            Column {
                Text(
                    if (!multiple)
                        stringResource(R.string.library_delete_confirmation, targets.single().title)
                    else
                        pluralStringResource(
                            R.plurals.library_bulk_delete_confirmation,
                            targets.size,
                            targets.size,
                        )
                )
                if (multiple) {
                    Column(Modifier.heightIn(max = 240.dp).verticalScroll(rememberScrollState())) {
                        targets.forEach { target -> Text(target.title) }
                    }
                }
                if (view.libraryDeleteError != null) {
                    Text(
                        stringResource(
                            if (!multiple) R.string.library_delete_error
                            else R.string.library_bulk_delete_error
                        )
                    )
                }
                if (view.storageBusy) LinearProgressIndicator()
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    dispatch(
                        if (bulk == null) Event.ConfirmDocumentDeletion(targets.single().documentId)
                        else Event.ConfirmSelectedDocumentDeletion(targets.map { it.documentId })
                    )
                },
                enabled = canAct,
                modifier = Modifier.testTag("confirm_document_deletion"),
            ) {
                Text(
                    stringResource(
                        if (view.libraryDeleteError == null) R.string.library_delete
                        else R.string.retry
                    )
                )
            }
        },
        dismissButton = {
            TextButton(
                onClick = { dispatch(Event.CancelDocumentDeletion) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("cancel_document_deletion"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        },
        modifier = Modifier.testTag("document_deletion_dialog"),
    )
}
