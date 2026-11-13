package app.yomoku

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryRenameDialog(view: ViewModel, dispatch: (Event) -> Unit, enabled: Boolean) {
    val draft = view.libraryRename ?: return
    val canAct = enabled && !view.storageBusy && !view.completionSavePending
    AlertDialog(
        onDismissRequest = { if (!view.storageBusy) dispatch(Event.CancelDocumentRename) },
        title = { Text(stringResource(R.string.library_rename)) },
        text = {
            Column {
                OutlinedTextField(
                    value = draft.title,
                    onValueChange = { dispatch(Event.EditDocumentTitle(draft.documentId, it)) },
                    label = { Text(stringResource(R.string.library_document_title)) },
                    enabled = canAct,
                    modifier = Modifier.testTag("document_title_draft"),
                )
                if (draft.error != null) {
                    Text(stringResource(R.string.library_rename_error))
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { dispatch(Event.SaveDocumentTitle(draft.documentId)) },
                enabled = canAct && draft.title.isNotBlank(),
                modifier = Modifier.testTag("save_document_title"),
            ) {
                Text(stringResource(if (draft.error == null) R.string.save else R.string.retry))
            }
        },
        dismissButton = {
            TextButton(
                onClick = { dispatch(Event.CancelDocumentRename) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("cancel_document_rename"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        },
        modifier = Modifier.testTag("document_rename_dialog"),
    )
}
