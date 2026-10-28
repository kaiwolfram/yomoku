package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import app.yomoku.shared.EffectiveOverrideAction
import app.yomoku.shared.EffectiveOverrideFailure
import app.yomoku.shared.EffectiveOverridePrompt
import app.yomoku.shared.EffectiveReaderOverrideState
import app.yomoku.shared.OverrideScope
import app.yomoku.shared.SplitMode

/**
 * A compact prompt renderer; its owner opens the validated span and applies FIFO/time gates.
 * sessionKey must identify the retained native session across rotation and change on replacement.
 * Exact pending writes/retries stay in Rust. This component never reconstructs their payload.
 */
@Composable
internal fun ReaderBoundaryEditor(
    state: EffectiveReaderOverrideState,
    sessionKey: String,
    enabled: Boolean,
    dispatch: (EffectiveOverrideAction) -> Unit,
    onReload: () -> Unit,
    onClose: () -> Unit,
    onInteraction: () -> Unit,
    now: () -> Long = System::currentTimeMillis,
) {
    val identity = state.identity
    val prompt = state.prompt?.takeIf { it.identity == identity }
    val text = prompt?.let { ReaderBoundaryText.parse(it.surface) }
    // Keep saveable ownership above the dialog's separate Android view registry.
    val draft =
        if (prompt != null && text != null) {
            rememberBoundaryDraft(sessionKey, prompt, text)
        } else null
    val idle = enabled && !state.busy
    val canClose = idle && !state.pending
    val close = {
        if (canClose) {
            onInteraction()
            identity?.let { dispatch(EffectiveOverrideAction.Dismiss(it)) }
            onClose()
        }
    }
    val submit: (EffectiveOverrideAction) -> Unit = {
        onInteraction()
        dispatch(it)
    }
    val interaction by rememberUpdatedState(onInteraction)
    Dialog(onDismissRequest = close) {
        Surface(shape = MaterialTheme.shapes.large) {
            Column(
                Modifier.widthIn(max = 560.dp)
                    .fillMaxWidth()
                    .pointerInput(Unit) {
                        awaitPointerEventScope {
                            while (true) {
                                awaitPointerEvent(PointerEventPass.Initial)
                                interaction()
                            }
                        }
                    }
                    .onPreviewKeyEvent {
                        interaction()
                        false
                    }
                    .verticalScroll(rememberScrollState())
                    .padding(20.dp)
                    .testTag("boundary_editor"),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    stringResource(R.string.boundary_title),
                    style = MaterialTheme.typography.titleLarge,
                )
                val retired =
                    state.reloadRequired ||
                        state.error == EffectiveOverrideFailure.StaleContext ||
                        state.error == EffectiveOverrideFailure.InvalidSelection ||
                        (state.prompt != null && (prompt == null || text == null))
                if (prompt != null && text != null && draft != null && !retired) {
                    BoundaryChoices(
                        prompt,
                        text,
                        draft.value,
                        { draft.value = it },
                        idle && !state.pending,
                        submit,
                        onInteraction,
                        now,
                    )
                }
                if (state.busy) {
                    Text(stringResource(R.string.boundary_busy), Modifier.testTag("boundary_busy"))
                } else if (state.pending && identity != null) {
                    Text(
                        stringResource(R.string.boundary_write_failed),
                        Modifier.testTag("boundary_error"),
                    )
                    Row {
                        TextButton(
                            onClick = { submit(EffectiveOverrideAction.Retry(identity)) },
                            enabled = idle,
                            modifier = Modifier.testTag("boundary_retry"),
                        ) {
                            Text(stringResource(R.string.retry))
                        }
                        TextButton(
                            onClick = { submit(EffectiveOverrideAction.Cancel(identity)) },
                            enabled = idle,
                            modifier = Modifier.testTag("boundary_cancel"),
                        ) {
                            Text(stringResource(R.string.cancel))
                        }
                    }
                } else if (retired || state.error != null) {
                    Text(
                        stringResource(
                            if (retired) R.string.boundary_reload_required
                            else R.string.boundary_read_failed
                        ),
                        Modifier.testTag("boundary_error"),
                    )
                    TextButton(
                        onClick = {
                            onInteraction()
                            onReload()
                        },
                        enabled = idle,
                        modifier = Modifier.testTag("boundary_reload"),
                    ) {
                        Text(stringResource(R.string.boundary_reload))
                    }
                }
                TextButton(
                    onClick = close,
                    enabled = canClose,
                    modifier = Modifier.testTag("boundary_close"),
                ) {
                    Text(stringResource(R.string.close))
                }
            }
        }
    }
}

@Composable
private fun rememberBoundaryDraft(
    sessionKey: String,
    prompt: EffectiveOverridePrompt,
    text: ReaderBoundaryText,
): MutableState<BoundaryDraft> =
    rememberSaveable(
        sessionKey,
        prompt.identity,
        saver = boundaryDraftStateSaver(sessionKey, prompt, text),
    ) {
        mutableStateOf(
            BoundaryDraft(
                prompt.defaultScope,
                prompt.choices
                    .firstOrNull { it.mode == SplitMode.B }
                    ?.parts
                    ?.let(text::cuts)
                    .orEmpty(),
            )
        )
    }

@Composable
private fun BoundaryChoices(
    prompt: EffectiveOverridePrompt,
    text: ReaderBoundaryText,
    draft: BoundaryDraft,
    onDraftChange: (BoundaryDraft) -> Unit,
    enabled: Boolean,
    submit: (EffectiveOverrideAction) -> Unit,
    onInteraction: () -> Unit,
    now: () -> Long,
) {
    Text(prompt.surface, Modifier.testTag("boundary_surface"))
    Row(Modifier.fillMaxWidth().selectableGroup()) {
        for (scope in OverrideScope.entries) {
            Row(
                Modifier.weight(1f)
                    .selectable(
                        selected = draft.scope == scope,
                        enabled = enabled,
                        role = Role.RadioButton,
                    ) {
                        onInteraction()
                        onDraftChange(draft.copy(scope = scope))
                    }
                    .heightIn(min = 48.dp)
                    .padding(8.dp)
                    .testTag("boundary_scope_$scope"),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                RadioButton(selected = draft.scope == scope, onClick = null, enabled = enabled)
                Text(stringResource(scope.label()))
            }
        }
    }
    for (mode in SplitMode.entries) {
        val choice = prompt.choices.singleOrNull { it.mode == mode }
        val available = choice != null && text.cuts(choice.parts) != null
        val canGenerate = mode != SplitMode.B && prompt.choices.none { it.mode == mode }
        Row {
            TextButton(
                onClick = {
                    if (available) {
                        submit(
                            EffectiveOverrideAction.Choose(
                                prompt.identity,
                                mode,
                                draft.scope,
                                now(),
                            )
                        )
                    } else {
                        submit(EffectiveOverrideAction.Generate(prompt.identity, mode))
                    }
                },
                enabled = enabled && (available || canGenerate),
                modifier = Modifier.testTag("boundary_mode_$mode"),
            ) {
                Text(
                    stringResource(
                        when {
                            available -> R.string.boundary_use_mode
                            canGenerate -> R.string.boundary_load_mode
                            else -> R.string.boundary_unavailable_mode
                        },
                        mode.name,
                    )
                )
            }
            if (available) {
                Text(choice.parts.joinToString(" | "), Modifier.padding(8.dp))
            }
        }
    }
    Text(stringResource(R.string.boundary_manual_help))
    FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        for ((index, edges) in text.offsets.zipWithNext().withIndex()) {
            val scalar = text.surface.substring(edges.first, edges.second)
            if (edges.second == text.surface.length) {
                Text(scalar, Modifier.padding(12.dp))
            } else {
                val selected = edges.second in draft.cuts
                val description =
                    stringResource(
                        if (selected) R.string.boundary_merge_after
                        else R.string.boundary_split_after,
                        index + 1,
                    )
                FilterChip(
                    selected = selected,
                    onClick = {
                        onInteraction()
                        onDraftChange(
                            draft.copy(
                                cuts =
                                    if (selected) draft.cuts - edges.second
                                    else draft.cuts + edges.second
                            )
                        )
                    },
                    label = { Text(if (selected) "$scalar |" else scalar) },
                    enabled = enabled,
                    modifier =
                        Modifier.heightIn(min = 48.dp)
                            .semantics { contentDescription = description }
                            .testTag("boundary_cut_${edges.second}"),
                )
            }
        }
    }
    val parts = text.parts(draft.cuts)
    if (parts != null) {
        Text(parts.joinToString(" | "), Modifier.testTag("boundary_preview"))
    } else {
        Text(stringResource(R.string.boundary_invalid_parts), Modifier.testTag("boundary_invalid"))
    }
    Row {
        TextButton(
            onClick = {
                onInteraction()
                onDraftChange(draft.copy(cuts = emptySet()))
            },
            enabled = enabled,
            modifier = Modifier.testTag("boundary_merge_all"),
        ) {
            Text(stringResource(R.string.boundary_merge_all))
        }
        TextButton(
            onClick = {
                parts?.let {
                    submit(EffectiveOverrideAction.Save(prompt.identity, draft.scope, it, now()))
                }
            },
            enabled = enabled && parts != null,
            modifier = Modifier.testTag("boundary_save"),
        ) {
            Text(stringResource(R.string.boundary_save))
        }
    }
    for (rule in prompt.winningRules) {
        Text(
            stringResource(
                R.string.boundary_rule,
                rule.surface,
                rule.parts.joinToString(" | "),
                stringResource(rule.scope.label()),
            )
        )
        TextButton(
            onClick = { submit(EffectiveOverrideAction.Reverse(prompt.identity, rule.id)) },
            enabled = enabled,
            modifier = Modifier.testTag("boundary_reverse_${rule.id.value}"),
        ) {
            Text(stringResource(R.string.boundary_reverse))
        }
    }
}

private data class BoundaryDraft(val scope: OverrideScope, val cuts: Set<Int>)

private fun boundaryDraftStateSaver(
    sessionKey: String,
    prompt: EffectiveOverridePrompt,
    text: ReaderBoundaryText,
): Saver<MutableState<BoundaryDraft>, Any> {
    val draftSaver = boundaryDraftSaver(sessionKey, prompt, text)
    // Reject the entire holder when its owner changes, so rememberSaveable runs
    // init. A nullable inner restore must never become MutableState(null).
    return Saver(
        save = { state -> with(draftSaver) { save(state.value) } },
        restore = { saved -> draftSaver.restore(saved)?.let { mutableStateOf(it) } },
    )
}

private fun boundaryDraftSaver(
    sessionKey: String,
    prompt: EffectiveOverridePrompt,
    text: ReaderBoundaryText,
): Saver<BoundaryDraft, Any> =
    listSaver(
        save = {
            listOf(
                sessionKey,
                prompt.identity.bincodeSerialize(),
                prompt.surface,
                it.scope.name,
                it.cuts.toIntArray(),
            )
        },
        restore = { values ->
            val identity = values.getOrNull(1) as? ByteArray
            val cuts = (values.getOrNull(4) as? IntArray)?.toSet()
            val scope = OverrideScope.entries.firstOrNull { it.name == values.getOrNull(3) }
            if (
                values.getOrNull(0) == sessionKey &&
                    identity != null &&
                    identity.contentEquals(prompt.identity.bincodeSerialize()) &&
                    values.getOrNull(2) == prompt.surface &&
                    cuts != null &&
                    cuts.all { it in text.splitOffsets } &&
                    scope != null
            )
                BoundaryDraft(scope, cuts)
            else null
        },
    )

private fun OverrideScope.label(): Int =
    when (this) {
        OverrideScope.ALLOCCURRENCES -> R.string.boundary_all
        OverrideScope.THISOCCURRENCE -> R.string.boundary_this
    }
