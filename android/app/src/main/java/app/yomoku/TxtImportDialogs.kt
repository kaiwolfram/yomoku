package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
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
internal fun TxtImportDialogs(imports: TxtImports) {
    if (imports.active) {
        AlertDialog(
            onDismissRequest = {},
            properties =
                DialogProperties(dismissOnBackPress = false, dismissOnClickOutside = false),
            title = { Text(stringResource(R.string.import_txt)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(
                        stringResource(
                            R.string.import_current_file,
                            imports.currentFileNumber,
                            imports.totalFiles,
                        )
                    )
                    Text(imports.filename)
                    if (imports.cancelling) Text(stringResource(R.string.import_cancelling))
                    Text(
                        stringResource(
                            R.string.import_files_processed,
                            imports.results.size,
                            imports.totalFiles,
                        )
                    )
                    LinearProgressIndicator(
                        progress = { imports.progress },
                        modifier = Modifier.fillMaxWidth().testTag("txt_import_progress"),
                    )
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
            confirmButton = {
                if (imports.choosingEncoding) {
                    TextButton(onClick = imports::skipEncoding) {
                        Text(stringResource(R.string.import_skip_file))
                    }
                }
            },
            dismissButton = {
                TextButton(onClick = imports::cancel, enabled = !imports.cancelling) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    } else if (imports.results.isNotEmpty()) {
        AlertDialog(
            onDismissRequest = imports::dismissSummary,
            title = { Text(stringResource(R.string.import_summary_title)) },
            text = {
                LazyColumn(
                    Modifier.heightIn(max = 400.dp).testTag("txt_import_summary"),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    items(imports.results) { result ->
                        Column {
                            Text(result.filename)
                            Text(
                                when {
                                    result.error != null ->
                                        stringResource(result.error.messageResource())
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
                    }
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
