package app.yomoku

import android.content.res.AssetManager
import android.text.Html
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
    OfflineNotices(
        title = stringResource(R.string.sudachi_notices_title),
        attribution = stringResource(R.string.sudachi_attribution),
        root = "sudachi",
        names = listOf("LEGAL", "LICENSE-2.0.txt"),
        tag = "sudachi_notices_text",
        onDismiss = onDismiss,
    )
}

internal val wordDictionaryNoticeNames =
    listOf(
        "ATTRIBUTION.txt",
        "CC-BY-SA-4.0.txt",
        "edrdg-licence.html",
        "jmdict-project.html",
        "jmnedict.html",
    )

@Composable
internal fun SvgNotices(onDismiss: () -> Unit) {
    OfflineNotices(
        title = stringResource(R.string.svg_notices_title),
        attribution = stringResource(R.string.svg_attribution),
        root = "androidsvg",
        names = listOf("ATTRIBUTION.txt", "LICENSE-2.0.txt"),
        tag = "svg_notices_text",
        onDismiss = onDismiss,
    )
}

@Composable
internal fun WordDictionaryNotices(onDismiss: () -> Unit) {
    OfflineNotices(
        title = stringResource(R.string.word_dictionary_notices_title),
        attribution = stringResource(R.string.word_dictionary_attribution),
        root = "dictionaries/notices",
        names = wordDictionaryNoticeNames,
        tag = "word_dictionary_notices_text",
        onDismiss = onDismiss,
    )
}

internal fun loadOfflineNotices(assets: AssetManager, root: String, names: List<String>): String =
    names.joinToString("\n\n") { name ->
        val source = assets.open("$root/$name").bufferedReader().use { it.readText() }
        val text =
            if (name.endsWith(".html")) Html.fromHtml(source, Html.FROM_HTML_MODE_LEGACY).toString()
            else source
        "$name\n\n$text"
    }

@Composable
private fun OfflineNotices(
    title: String,
    attribution: String,
    root: String,
    names: List<String>,
    tag: String,
    onDismiss: () -> Unit,
) {
    val assets = LocalContext.current.applicationContext.assets
    val unavailable = stringResource(R.string.notices_unavailable)
    val notices by
        produceState<String?>(initialValue = null, assets, unavailable, root, names) {
            val loaded =
                withContext(Dispatchers.IO) {
                    try {
                        loadOfflineNotices(assets, root, names)
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
        title = { Text(title) },
        text = {
            Column(
                Modifier.heightIn(max = 480.dp).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(attribution)
                SelectionContainer {
                    Text(
                        notices ?: stringResource(R.string.notices_loading),
                        Modifier.testTag(tag),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.close)) }
        },
    )
}
