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
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ViewModel

/** Compact effective consumers; production activation is coordinated with the native model. */
@Composable
internal fun EffectiveReaderWords(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
    imageLoader: EpubImageLoader? = null,
) {
    // Compact revisions belong to one native session. A replacement cannot reuse a selection/draft.
    key(sessionKey) {
        EffectiveReaderWordContent(view, sessionKey, dispatch, onInteraction, imageLoader)
    }
}

@Composable
private fun EffectiveReaderWordContent(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
    imageLoader: EpubImageLoader?,
) {
    val page = view.reader ?: return
    val word = view.effectiveReaderWord ?: return
    val base = view.readerAnalysis?.let { ReaderTokenKey(page.documentId, page.anchor, it) }
    val snapshot = view.effectiveReaderTokens?.takeIf { it.identity.base == base }
    val idle = !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    val override = view.effectiveReaderOverride
    val canRead =
        idle &&
            !word.busy &&
            !word.customEntryPending &&
            view.effectiveReaderStatus?.pending != true &&
            override?.pending != true &&
            override?.busy != true
    // Reload belongs to the page retired by the edit, not to subsequent navigation.
    val reloadBase =
        remember(override?.reloadRequired) {
            base.takeIf { override?.reloadRequired == true }
        }
    var requested by remember(base) { mutableStateOf(false) }
    LaunchedEffect(base, snapshot, canRead, view.documentError) {
        if (snapshot != null) requested = true
        else if (
            base != null &&
                base != reloadBase &&
                !requested &&
                canRead &&
                view.documentError == null
        ) {
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
    val interaction by rememberUpdatedState(onInteraction)
    val dark = isSystemInDarkTheme()
    val foreground = MaterialTheme.colorScheme.onSurface
    val linkColor = MaterialTheme.colorScheme.primary
    val uriHandler = LocalUriHandler.current
    val text =
        remember(page, tokens, dark, foreground, linkColor, uriHandler) {
            readerAnnotatedSpans(
                page.text,
                tokens.orEmpty().map {
                    ReaderAnnotatedToken(
                        it.token.segmentIndex,
                        it.token.vocabulary?.status,
                        it.start,
                        it.end,
                    )
                },
                dark,
                foreground,
                page.formatting,
                linkColor,
                onExternalLink = { target ->
                    interaction()
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
                    interaction()
                    selected = tapped
                }
            }
        }
    ReaderImageContent(
        page,
        text,
        imageLoader,
        onInteraction,
        view.readerTypography,
        view.readerFurigana,
    )
    if (
        base != null &&
            (snapshot == null || tokens == null) &&
            (requested || snapshot != null || base == reloadBase || view.documentError != null) &&
            idle &&
            view.readerTime != null
    ) {
        TextButton(
            onClick = {
                onInteraction()
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
    EffectiveReaderBoundaries(
        view,
        sessionKey,
        canRead,
        { event ->
            if (event == Event.DismissReaderWord) selected = null
            dispatch(event)
        },
        onInteraction,
    )
    if (display != null) {
        val token = display.token
        ReaderWordPanel(
            ReaderWordHeader(token.surface, token.reading, token.dictionaryForm, token.vocabulary),
            canRead,
            onInteraction,
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
            val details =
                word.details?.takeIf {
                    it.selection == display.selection &&
                        it.token == token &&
                        word.selection == display.selection
                }
            if (details != null) {
                ReaderDictionary(details.encountered, details.candidates)
                if (details.candidates.words.isEmpty() && details.candidates.names.isEmpty()) {
                    EffectiveReaderCustomDictionary(details, word, canRead, dispatch, onInteraction)
                }
            } else {
                ReaderWordLookupState(word.error != null, canRead, onInteraction) {
                    dispatch(Event.LoadEffectiveReaderWord(display.selection))
                }
            }
            EffectiveReaderPendingActions(view, idle, dispatch, onInteraction)
        }
    } else {
        // Accepted commands survive dismissal and page invalidation. Keep Retry/Cancel reachable.
        EffectiveReaderPendingActions(view, idle, dispatch, onInteraction)
    }
}

@Composable
private fun EffectiveReaderPendingActions(
    view: ViewModel,
    idle: Boolean,
    dispatch: (Event) -> Unit,
    onInteraction: () -> Unit,
) {
    val word = view.effectiveReaderWord
    if (word?.customEntryPending == true && !word.busy) {
        EffectiveReaderRetry(
            "custom",
            R.string.word_custom_pending_failed,
            idle,
            Event.RetryEffectiveReaderCustomEntry,
            Event.CancelEffectiveReaderCustomEntry,
            dispatch,
            onInteraction,
        )
    }
    val status = view.effectiveReaderStatus
    if (status?.pending == true && !status.busy) {
        EffectiveReaderRetry(
            "status",
            R.string.word_status_pending_failed,
            idle,
            Event.RetryEffectiveReaderStatus,
            Event.CancelEffectiveReaderStatus,
            dispatch,
            onInteraction,
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
    onInteraction: () -> Unit,
) {
    Text(stringResource(message), Modifier.testTag("effective_${tag}_error"))
    Row {
        TextButton(
            onClick = {
                onInteraction()
                dispatch(retry)
            },
            enabled = enabled,
            modifier = Modifier.testTag("effective_${tag}_retry"),
        ) {
            Text(stringResource(R.string.retry))
        }
        TextButton(
            onClick = {
                onInteraction()
                dispatch(cancel)
            },
            enabled = enabled,
            modifier = Modifier.testTag("effective_${tag}_cancel"),
        ) {
            Text(stringResource(R.string.cancel))
        }
    }
}
