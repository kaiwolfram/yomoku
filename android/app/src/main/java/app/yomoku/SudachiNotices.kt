package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

@Composable
internal fun SudachiNotices(onDismiss: () -> Unit) {
    val assets = LocalContext.current.applicationContext.assets
    val unavailable = stringResource(R.string.notices_unavailable)
    val notices by
        produceState<String?>(initialValue = null, assets, unavailable) {
            val loaded =
                withContext(Dispatchers.IO) {
                    try {
                        listOf("LEGAL", "LICENSE-2.0.txt").joinToString("\n\n") { name ->
                            "$name\n\n" +
                                assets.open("sudachi/$name").bufferedReader().use { it.readText() }
                        }
                    } catch (_: java.io.IOException) {
                        unavailable
                    }
                }
            // Publish on Android's UI dispatcher even when the composition's
            // effect context resumes on the I/O thread (e.g. under UI tests).
            withContext(Dispatchers.Main.immediate) { value = loaded }
        }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.sudachi_notices_title)) },
        text = {
            Column(
                Modifier.heightIn(max = 480.dp).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(stringResource(R.string.sudachi_attribution))
                SelectionContainer {
                    Text(
                        notices ?: stringResource(R.string.notices_loading),
                        Modifier.testTag("sudachi_notices_text"),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.close)) }
        },
    )
}
