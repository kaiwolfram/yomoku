package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
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
internal fun LibrarySelectionActions(view: ViewModel, dispatch: (Event) -> Unit, enabled: Boolean) {
    val selection = view.librarySelection ?: return
    val canAct =
        enabled && !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    Column {
        Text(
            pluralStringResource(R.plurals.library_selected_count, selection.size, selection.size),
            modifier = Modifier.testTag("library_selected_count"),
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(
                onClick = { dispatch(Event.SelectAllLibraryDocuments) },
                enabled = canAct && view.documents.isNotEmpty(),
                modifier = Modifier.testTag("select_all_library_documents"),
            ) {
                Text(stringResource(R.string.library_select_all))
            }
            TextButton(
                onClick = { dispatch(Event.RequestSelectedDocumentDeletion) },
                enabled = canAct && selection.isNotEmpty(),
                modifier = Modifier.testTag("delete_selected_documents"),
            ) {
                Text(stringResource(R.string.library_delete_selected))
            }
            TextButton(
                onClick = { dispatch(Event.CancelLibrarySelection) },
                enabled = !view.storageBusy,
                modifier = Modifier.testTag("cancel_library_selection"),
            ) {
                Text(stringResource(R.string.cancel))
            }
        }
    }
}
