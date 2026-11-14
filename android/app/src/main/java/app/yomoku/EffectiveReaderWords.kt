package app.yomoku

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.core.net.toUri
import app.yomoku.shared.AnkiSelection
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ViewModel

/** Production word consumers using compact selections owned by the native session. */
@Composable
internal fun EffectiveReaderWords(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    imageLoader: EpubImageLoader? = null,
    ankiBrowser: AnkiNoteBrowser? = null,
) {
    // Compact revisions belong to one native session. A replacement cannot reuse a selection/draft.
    key(sessionKey) {
        EffectiveReaderWordContent(
            view,
            sessionKey,
            dispatch,
            imageLoader,
            ankiBrowser,
        )
    }
}

@Composable
private fun EffectiveReaderWordContent(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    imageLoader: EpubImageLoader?,
    ankiBrowser: AnkiNoteBrowser?,
) {
    val page = view.reader ?: return
    val word = view.effectiveReaderWord ?: return
    val base = view.readerAnalysis?.let { ReaderTokenKey(page.documentId, page.anchor, it) }
    val snapshot = view.effectiveReaderTokens?.takeIf { it.identity.base == base }
    val idle = !view.storageBusy && !view.completionSavePending
    val canRead = idle && !word.busy && view.effectiveReaderStatus?.pending != true
    var requested by remember(base, page.position, view.pageLength) { mutableStateOf(false) }
    LaunchedEffect(base, page.position, view.pageLength, snapshot, canRead, view.documentError) {
        if (snapshot != null) requested = true
        else if (base != null && !requested && canRead && view.documentError == null) {
            requested = true
            dispatch(Event.LoadReaderTokens(base))
        }
    }
    val tokens =
        remember(page, snapshot) {
            snapshot?.let { effectiveDisplayTokens(page, it.identity, it) }
        }
    var selected by
        remember(base, snapshot?.identity, tokens == null) {
            mutableStateOf<EffectiveReaderSelection?>(null)
        }
    var wordRequested by remember(selected) { mutableStateOf(false) }
    val currentDispatch by rememberUpdatedState(dispatch)
    DisposableEffect(Unit) { onDispose { currentDispatch(Event.DismissReaderWord) } }
    LaunchedEffect(selected, canRead, word.selection) {
        val selection = selected
        if (selection != null) {
            if (wordRequested && word.selection != selection) selected = null
            else if (!wordRequested && canRead) {
                wordRequested = true
                dispatch(Event.LoadEffectiveReaderWord(selection))
            }
        }
    }
    val currentTokens by rememberUpdatedState(tokens)

    val dark = isSystemInDarkTheme()
    val foreground = MaterialTheme.colorScheme.onSurface
    val linkColor = MaterialTheme.colorScheme.primary
    val uriHandler = LocalUriHandler.current
    val text =
        remember(page, tokens, dark, foreground, linkColor, uriHandler) {
            readerAnnotatedText(
                page.text,
                tokens.orEmpty(),
                dark,
                foreground,
                page.formatting,
                linkColor,
                onExternalLink = { target ->
                    try {
                        uriHandler.openUri(target.toUri().normalizeScheme().toString())
                    } catch (_: IllegalArgumentException) {
                        /* No browser handles this source URL. */
                    }
                },
            ) { index ->
                val tapped = tokens?.singleOrNull { it.token.segmentIndex == index }?.selection
                // A retained accessibility/link callback cannot select the same index on a newer
                // page.
                if (tapped != null && currentTokens?.any { it.selection == tapped } == true) {
                    selected = tapped
                }
            }
        }
    ReaderImageContent(
        page,
        text,
        imageLoader,
        view.readerTypography,
    )
    if (
        base != null &&
            (snapshot == null || tokens == null) &&
            (requested || snapshot != null || view.documentError != null) &&
            idle
    ) {
        TextButton(
            onClick = {
                requested = true
                dispatch(Event.LoadReaderTokens(base))
            },
            enabled = canRead,
            modifier = Modifier.testTag("retry_word_lookup"),
        ) {
            Text(stringResource(R.string.retry_word_lookup))
        }
    }
    val display = tokens?.singleOrNull { it.selection == selected }
    // Keep observation ownership outside the sheet and its transient details subtree.
    // Status refreshes and switching sheet layouts must not restart duplicate queries.
    val details =
        word.details?.takeIf {
            it.selection == selected && word.selection == selected && display != null
        }
    ObserveReaderAnkiEntries(
        selected?.let { AnkiSelection.Effective(it) },
        view.ankiSetup,
        canRead && details != null,
        dispatch,
    )
    if (display != null) {
        val token = display.token
        ReaderWordPanel(
            ReaderWordHeader(token.surface, token.reading, token.dictionaryForm, token.vocabulary),
            canRead,
            onClose = {
                selected = null
                dispatch(Event.DismissReaderWord)
            },
            onStatus = { status ->
                token.vocabulary?.let {
                    dispatch(
                        Event.SetEffectiveReaderStatus(
                            display.selection,
                            it.id,
                            status,
                            System.currentTimeMillis(),
                        )
                    )
                }
            },
        ) {
            if (details != null) {
                val ankiSelection = AnkiSelection.Effective(details.selection)
                ReaderDictionary(
                    details.encountered,
                    details.candidates,
                    action = { entry ->
                        AnkiEntryAction(
                            view.ankiEntries.find {
                                it.selection == ankiSelection && it.key == entry
                            },
                            canRead,
                            dispatch,
                            ankiBrowser,
                        )
                    },
                    notice = { entry ->
                        AnkiEntryNotice(
                            view.ankiEntries.find {
                                it.selection == ankiSelection && it.key == entry
                            },
                            ankiBrowser,
                        )
                    },
                )
            } else {
                ReaderWordLookupState(word.error != null, canRead) {
                    dispatch(Event.LoadEffectiveReaderWord(display.selection))
                }
            }
            EffectiveReaderPendingActions(view, idle, dispatch)
        }
    } else {
        // Accepted commands survive dismissal and page invalidation. Keep Retry/Cancel reachable.
        EffectiveReaderPendingActions(view, idle, dispatch)
    }
}

@Composable
private fun EffectiveReaderPendingActions(
    view: ViewModel,
    idle: Boolean,
    dispatch: (Event) -> Unit,
) {
    val status = view.effectiveReaderStatus
    if (status?.pending == true && !status.busy) {
        EffectiveReaderRetry(
            "status",
            R.string.word_status_pending_failed,
            idle,
            Event.RetryEffectiveReaderStatus,
            Event.CancelEffectiveReaderStatus,
            dispatch,
        )
    }
}

@Composable
private fun EffectiveReaderRetry(
    tag: String,
    message: Int,
    enabled: Boolean,
    retry: Event,
    cancel: Event,
    dispatch: (Event) -> Unit,
) {
    Text(stringResource(message), Modifier.testTag("effective_${tag}_error"))
    Row {
        TextButton(
            onClick = {
                dispatch(retry)
            },
            enabled = enabled,
            modifier = Modifier.testTag("effective_${tag}_retry"),
        ) {
            Text(stringResource(R.string.retry))
        }
        TextButton(
            onClick = {
                dispatch(cancel)
            },
            enabled = enabled,
            modifier = Modifier.testTag("effective_${tag}_cancel"),
        ) {
            Text(stringResource(R.string.cancel))
        }
    }
}
