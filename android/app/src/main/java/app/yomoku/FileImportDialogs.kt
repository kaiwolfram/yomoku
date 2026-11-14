package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.TextEncoding

@Composable
internal fun FileImportDialogs(imports: FileImports) {
    if (imports.active) {
        AlertDialog(
            onDismissRequest = {},
            properties =
                DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
            title = { Text(stringResource(R.string.importing_book)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(imports.filename)
                    if (imports.cancelling) Text(stringResource(R.string.import_cancelling))
                    if (imports.choosingEncoding) {
                        Text(stringResource(checkNotNull(imports.encodingError).messageResource()))
                        TextButton(onClick = { imports.selectEncoding(TextEncoding.UTF8) }) {
                            Text("UTF-8")
                        }
                        TextButton(onClick = { imports.selectEncoding(TextEncoding.SHIFTJIS) }) {
                            Text("Shift-JIS")
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = imports::cancel, enabled = !imports.cancelling) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    } else if (imports.result != null) {
        val result = checkNotNull(imports.result)
        AlertDialog(
            onDismissRequest = imports::dismissSummary,
            title = { Text(stringResource(R.string.import_summary_title)) },
            text = {
                Column(
                    Modifier.verticalScroll(rememberScrollState()).testTag("file_import_summary"),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(result.filename)
                    Text(
                        when {
                            result.error != null -> stringResource(result.error.messageResource())
                            result.message != null -> stringResource(result.message)
                            result.outcome == ImportOutcome.EXISTING ->
                                stringResource(
                                    R.string.import_existing,
                                    result.title ?: result.filename,
                                )
                            result.outcome == ImportOutcome.CREATED ->
                                stringResource(R.string.import_file_created)
                            else -> stringResource(R.string.error_storage_unavailable)
                        }
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = imports::dismissSummary) {
                    Text(stringResource(R.string.close))
                }
            },
        )
    }
}
