package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import app.yomoku.shared.EffectiveOverrideAction
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ViewModel

/** Owns selection only; exact prompt/write/retry ownership remains in the native session. */
@Composable
internal fun EffectiveReaderBoundaries(
    view: ViewModel,
    sessionKey: String,
    enabled: Boolean,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
) {
    val page = view.reader ?: return
    val state = view.effectiveReaderOverride ?: return
    val snapshot = view.effectiveReaderTokens
    val base = view.readerAnalysis?.let { ReaderTokenKey(page.documentId, page.anchor, it) }
    val boundaries =
        remember(page, snapshot, base) {
            snapshot
                ?.takeIf {
                    it.identity.base == base &&
                        base.analysis is ReaderAnalysis.Available &&
                        effectiveDisplayTokens(page, it.identity, it) != null
                }
                ?.let { originalDisplayBoundaries(page, it.identity, it) }
        }
    // Saveable state explicitly includes its owner; native revisions may repeat in a new session.
    val owner = "$sessionKey:${snapshot?.identity}"
    var selectionOwner by rememberSaveable { mutableStateOf(owner) }
    var selecting by rememberSaveable { mutableStateOf(false) }
    var first by rememberSaveable { mutableStateOf(-1) }
    var last by rememberSaveable { mutableStateOf(-1) }
    val currentOwner by rememberUpdatedState(owner)
    val currentEnabled by rememberUpdatedState(enabled)
    val canSelect = enabled && boundaries?.isNotEmpty() == true
    TextButton(
        onClick = {
            if (canSelect && currentEnabled && currentOwner == owner) {
                onInteraction()
                dispatch(Event.DismissReaderWord)
                selectionOwner = owner
                first = -1
                last = -1
                selecting = true
            }
        },
        enabled = canSelect,
        modifier = Modifier.testTag("boundary_select"),
    ) {
        Text(stringResource(R.string.boundary_select))
    }
    if (selecting && selectionOwner == owner && boundaries != null && snapshot != null) {
        val anchor = originalBoundaryAnchor(boundaries, first, last)
        Dialog(onDismissRequest = { selecting = false }) {
            Surface(shape = MaterialTheme.shapes.large) {
                Column(
                    Modifier.verticalScroll(rememberScrollState())
                        .padding(20.dp)
                        .testTag("boundary_selection"),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(stringResource(R.string.boundary_select_help))
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        boundaries.forEachIndexed { index, boundary ->
                            FilterChip(
                                selected =
                                    first >= 0 && index in minOf(first, last)..maxOf(first, last),
                                onClick = {
                                    if (currentEnabled && currentOwner == owner) {
                                        onInteraction()
                                        if (first < 0 || last != first) {
                                            first = index
                                            last = index
                                        } else last = index
                                    }
                                },
                                enabled = enabled,
                                label = { Text(boundary.token.surface) },
                                modifier = Modifier.testTag("boundary_token_$index"),
                            )
                        }
                    }
                    if (first >= 0 && anchor == null) {
                        Text(
                            stringResource(R.string.boundary_selection_gap),
                            Modifier.testTag("boundary_selection_gap"),
                        )
                    }
                    TextButton(
                        onClick = {
                            if (anchor != null && currentEnabled && currentOwner == owner) {
                                onInteraction()
                                selecting = false
                                dispatch(
                                    Event.EffectiveReaderOverride(
                                        EffectiveOverrideAction.Open(snapshot.identity, anchor)
                                    )
                                )
                            }
                        },
                        enabled = enabled && anchor != null,
                        modifier = Modifier.testTag("boundary_open"),
                    ) {
                        Text(stringResource(R.string.boundary_title))
                    }
                    TextButton(
                        onClick = {
                            onInteraction()
                            selecting = false
                        },
                        modifier = Modifier.testTag("boundary_selection_close"),
                    ) {
                        Text(stringResource(R.string.cancel))
                    }
                }
            }
        }
    }
    // Failed accepted commands remain reachable even after the page is retired.
    if (state.identity != null) {
        val idle = !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
        ReaderBoundaryEditor(
            state,
            sessionKey,
            idle &&
                view.effectiveReaderWord?.busy != true &&
                view.effectiveReaderWord?.customEntryPending != true &&
                view.effectiveReaderStatus?.busy != true &&
                view.effectiveReaderStatus?.pending != true,
            dispatch = { dispatch(Event.EffectiveReaderOverride(it)) },
            onReload = {
                state.identity.let {
                    dispatch(Event.EffectiveReaderOverride(EffectiveOverrideAction.Dismiss(it)))
                }
                base?.let { dispatch(Event.LoadReaderTokens(it)) }
            },
            onClose = {},
            onInteraction = onInteraction,
        )
    }
}
