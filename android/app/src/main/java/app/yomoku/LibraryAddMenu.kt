package app.yomoku

import androidx.compose.foundation.layout.Box
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryAddMenu(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    onImportFiles: (() -> Unit)?,
    enabled: Boolean,
) {
    val canAct =
        enabled && !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    var expanded by remember(canAct) { mutableStateOf(false) }
    Box {
        IconButton(
            onClick = { expanded = true },
            enabled = canAct,
            modifier = Modifier.testTag("library_add"),
        ) {
            Icon(
                painterResource(R.drawable.ic_add),
                contentDescription = stringResource(R.string.library_add),
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            DropdownMenuItem(
                text = { Text(stringResource(R.string.paste_title)) },
                onClick = {
                    expanded = false
                    dispatch(Event.OpenPaste)
                },
            )
            if (onImportFiles != null) {
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.import_files)) },
                    onClick = {
                        expanded = false
                        onImportFiles()
                    },
                )
            }
        }
    }
}
