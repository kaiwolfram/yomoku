package app.yomoku

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryDeleteDialog(view: ViewModel, dispatch: (Event) -> Unit, enabled: Boolean) {
    val target = view.libraryDeleteTarget ?: return
    val canAct =
        enabled && !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    AlertDialog(
        onDismissRequest = { if (!view.storageBusy) dispatch(Event.CancelDocumentDeletion) },
        title = { Text(stringResource(R.string.library_delete_title)) },
        text = {
            Column {
                Text(stringResource(R.string.library_delete_confirmation, target.title))
                if (view.libraryDeleteError != null) {
                    Text(stringResource(R.string.library_delete_error))
                }
                if (view.storageBusy) LinearProgressIndicator()
            }
        },
        confirmButton = {
            TextButton(
                onClick = { dispatch(Event.ConfirmDocumentDeletion(target.documentId)) },
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
