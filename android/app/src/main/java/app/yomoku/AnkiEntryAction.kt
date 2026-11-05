package app.yomoku

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiEntryKey
import app.yomoku.shared.AnkiEntryPhase
import app.yomoku.shared.AnkiEntryView
import app.yomoku.shared.AnkiExportPhase
import app.yomoku.shared.AnkiSelection
import app.yomoku.shared.AnkiSetupPhase
import app.yomoku.shared.Event

@Composable
internal fun ObserveReaderAnkiEntries(
    selection: AnkiSelection,
    setup: AnkiSetupPhase,
    enabled: Boolean,
    dispatch: (Event) -> Unit,
) {
    var requested by remember(selection, setup) { mutableStateOf(false) }
    LaunchedEffect(selection, setup, enabled) {
        if (setup == AnkiSetupPhase.Ready && enabled && !requested) {
            requested = true
            dispatch(Event.LoadReaderAnkiEntries(selection))
        }
    }
}

@Composable
internal fun AnkiEntryAction(
    entry: AnkiEntryView?,
    enabled: Boolean,
    dispatch: (Event) -> Unit,
    browser: AnkiNoteBrowser? = null,
) {
    if (entry == null) return
    val phase = entry.phase
    val existing = phase is AnkiEntryPhase.Existing
    val export = (phase as? AnkiEntryPhase.Export)?.value
    val failed = phase is AnkiEntryPhase.Failed || export is AnkiExportPhase.ProviderFailed
    val uncertain =
        export is AnkiExportPhase.NoteUncertain || export is AnkiExportPhase.ModelUncertain
    val busy =
        phase == AnkiEntryPhase.Checking ||
            phase == AnkiEntryPhase.Busy ||
            (export != null && !failed && !uncertain)
    val dark = isSystemInDarkTheme()
    val label =
        stringResource(
            if (existing) R.string.anki_entry_open
            else if (failed) R.string.anki_entry_retry else R.string.anki_entry_add,
            entry.headword,
        )
    val entryTag =
        when (val key = entry.key) {
            is AnkiEntryKey.Word -> "word_${key.value}"
            is AnkiEntryKey.Name -> "name_${key.value}"
        }
    Column {
        FilledIconButton(
            onClick = {
                when {
                    existing -> browser?.openEntry(entry)
                    failed -> dispatch(Event.LoadReaderAnkiEntries(entry.selection))
                    phase == AnkiEntryPhase.Add ->
                        dispatch(Event.AddReaderAnkiEntry(entry.selection, entry.key))
                }
            },
            enabled = enabled && !busy && !uncertain && (!existing || browser != null),
            colors =
                IconButtonDefaults.filledIconButtonColors(
                    containerColor =
                        when {
                            existing -> Color(if (dark) 0xFF744052 else 0xFFF7C9D9)
                            failed || uncertain -> MaterialTheme.colorScheme.errorContainer
                            else -> Color(if (dark) 0xFF285C3B else 0xFFB6E6BE)
                        },
                    contentColor =
                        if (existing) Color(if (dark) 0xFF91BBFF else 0xFF245DB2)
                        else MaterialTheme.colorScheme.onSurface,
                ),
            modifier = Modifier.size(48.dp).testTag("anki_entry_$entryTag"),
        ) {
            if (busy)
                CircularProgressIndicator(
                    Modifier.size(18.dp).semantics { contentDescription = label },
                    strokeWidth = 2.dp,
                )
            else
                Icon(
                    painterResource(
                        if (existing) R.drawable.ic_anki_book else R.drawable.ic_anki_add
                    ),
                    contentDescription = label,
                    modifier = Modifier.size(20.dp),
                )
        }
    }
}

@Composable
internal fun AnkiEntryNotice(entry: AnkiEntryView?, browser: AnkiNoteBrowser? = null) {
    if (entry == null) return
    val export = (entry.phase as? AnkiEntryPhase.Export)?.value
    val error =
        (entry.phase as? AnkiEntryPhase.Failed)?.value
            ?: (export as? AnkiExportPhase.ProviderFailed)?.value
    if (error != null)
        Text(
            stringResource(error.messageResource()),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
        )
    if (export is AnkiExportPhase.NoteUncertain || export is AnkiExportPhase.ModelUncertain) {
        Text(
            stringResource(R.string.anki_entry_uncertain),
            style = MaterialTheme.typography.bodySmall,
        )
    }
    val attempt = browser?.state?.takeIf { it.entry == entry }
    if (attempt?.phase == AnkiBrowserPhase.Unavailable)
        Text(
            stringResource(R.string.anki_note_browser_unavailable),
            style = MaterialTheme.typography.bodySmall,
        )
    if (attempt?.phase is AnkiBrowserPhase.Failed)
        Text(
            stringResource(attempt.phase.error.exportError().messageResource()),
            style = MaterialTheme.typography.bodySmall,
        )
}
