package app.yomoku

import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import app.yomoku.shared.AnkiExportView
import app.yomoku.shared.AnkiSelection
import app.yomoku.shared.AnkiSettingsPhase
import app.yomoku.shared.AnkiSettingsView
import app.yomoku.shared.Event

/** Presentation only: the native owner authenticates selections and freezes fields/deck. */
@Composable
internal fun ReaderAnkiPanel(
    selection: AnkiSelection?,
    settings: AnkiSettingsView,
    export: AnkiExportView?,
    discovery: AnkiDeckDiscoveryState,
    enabled: Boolean,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
    ankiBrowser: AnkiNoteBrowser? = null,
) {
    val interact: (Event) -> Unit = {
        onInteraction()
        dispatch(it)
    }
    if (export != null) {
        // A different word/page or preference must never relabel or retarget accepted work.
        AnkiExportPanel(export, interact, ankiBrowser, onInteraction)
    } else if (selection != null) {
        val configured =
            settings.phase == AnkiSettingsPhase.READY &&
                settings.defaultDeckId != null &&
                discovery is AnkiDeckDiscoveryState.Ready &&
                discovery.decks.any { it.id == settings.defaultDeckId && !it.filtered }
        var attempted by remember(selection) { mutableStateOf(false) }
        if (configured) {
            TextButton(
                onClick = {
                    attempted = true
                    interact(Event.PrepareReaderAnkiExport(selection))
                },
                enabled = enabled,
                modifier = Modifier.testTag("word_anki_add"),
            ) {
                Text(stringResource(R.string.anki_export_add))
            }
            // Preparation is synchronous and performs no I/O. A rejected selection/gate grants
            // no export projection; leave configuration/recovery reachable without retrying.
            if (attempted) {
                Text(
                    stringResource(R.string.word_anki_not_prepared),
                    Modifier.testTag("word_anki_not_prepared"),
                )
            }
        } else {
            Text(stringResource(R.string.word_anki_setup), Modifier.testTag("word_anki_setup"))
        }
    }
    TextButton(
        onClick = { interact(Event.OpenSettings) },
        enabled = enabled,
        modifier = Modifier.testTag("word_anki_settings"),
    ) {
        Text(stringResource(R.string.word_anki_settings))
    }
}
