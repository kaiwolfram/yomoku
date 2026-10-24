package app.yomoku

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderPage
import app.yomoku.shared.ReaderStatusChange
import app.yomoku.shared.ReaderToken
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ReaderTokenSnapshot
import app.yomoku.shared.ViewModel
import app.yomoku.shared.VocabularyStatus

internal data class ReaderDisplayToken(val token: ReaderToken, val start: Int, val end: Int)

/** Reject malformed UTF-16 and byte offsets inside a scalar, never round them. */
private fun utf8Boundaries(text: String): Map<Long, Int>? {
    val boundaries = mutableMapOf(0L to 0)
    var utf16 = 0
    var bytes = 0L
    while (utf16 < text.length) {
        val char = text[utf16]
        val width =
            when {
                char.isHighSurrogate() -> {
                    if (utf16 + 1 >= text.length || !text[utf16 + 1].isLowSurrogate()) return null
                    2
                }
                char.isLowSurrogate() -> return null
                else -> 1
            }
        bytes +=
            when {
                width == 2 -> 4
                char.code < 0x80 -> 1
                char.code < 0x800 -> 2
                else -> 3
            }
        utf16 += width
        boundaries[bytes] = utf16
    }
    return boundaries
}

/** Full identity stays on the token; only the clipped display range becomes UTF-16. */
internal fun readerDisplayTokens(
    page: ReaderPage,
    key: ReaderTokenKey,
    snapshot: ReaderTokenSnapshot,
): List<ReaderDisplayToken>? {
    if (
        page.documentId != key.documentId ||
            page.anchor != key.page ||
            snapshot.documentId != key.documentId ||
            snapshot.page != key.page ||
            snapshot.analysis != key.analysis ||
            (key.analysis is app.yomoku.shared.ReaderAnalysis.Missing &&
                snapshot.tokens.isNotEmpty())
    )
        return null
    val boundaries = utf8Boundaries(page.text) ?: return null
    val pageStart = page.anchor.range.start.value
    val pageEnd = page.anchor.range.end.value
    if (
        pageStart < 0 || pageEnd <= pageStart || boundaries[pageEnd - pageStart] != page.text.length
    ) {
        return null
    }
    var previousEnd = 0
    var previousIndex: ULong? = null
    return snapshot.tokens.map { token ->
        val start = boundaries[token.displayRange.start.value] ?: return null
        val end = boundaries[token.displayRange.end.value] ?: return null
        val anchor = token.anchor
        if (
            start < previousEnd ||
                start >= end ||
                (previousIndex != null && token.tokenIndex <= previousIndex) ||
                anchor.sourceId != page.anchor.sourceId ||
                anchor.chapterId != page.anchor.chapterId ||
                anchor.range.start.value < 0 ||
                anchor.range.end.value <= anchor.range.start.value ||
                token.displayRange.start.value !=
                    maxOf(anchor.range.start.value, pageStart) - pageStart ||
                token.displayRange.end.value != minOf(anchor.range.end.value, pageEnd) - pageStart
        )
            return null
        val surfaceBoundaries = utf8Boundaries(token.surface) ?: return null
        val surfaceStart =
            surfaceBoundaries[maxOf(pageStart - anchor.range.start.value, 0)] ?: return null
        val surfaceEnd =
            surfaceBoundaries[minOf(pageEnd, anchor.range.end.value) - anchor.range.start.value]
                ?: return null
        if (
            surfaceBoundaries[anchor.range.end.value - anchor.range.start.value] !=
                token.surface.length ||
                token.surface.substring(surfaceStart, surfaceEnd) != page.text.substring(start, end)
        )
            return null
        previousEnd = end
        previousIndex = token.tokenIndex
        ReaderDisplayToken(token, start, end)
    }
}

internal fun vocabularyBackground(status: VocabularyStatus?, dark: Boolean): Color =
    when (status) {
        VocabularyStatus.NEW -> Color(if (dark) 0xFF28558D else 0xFFA9CCFF)
        VocabularyStatus.LEARNING -> Color(if (dark) 0xFF783E44 else 0xFFFFC5C0)
        VocabularyStatus.FAMILIAR -> Color(if (dark) 0xFF725A2D else 0xFFFFE29A)
        VocabularyStatus.IGNORED -> Color(if (dark) 0xFF505B6B else 0xFFBEC5CF)
        VocabularyStatus.KNOWN,
        null -> Color.Unspecified
    }

internal fun readerAnnotatedText(
    text: String,
    tokens: List<ReaderDisplayToken>,
    dark: Boolean,
    foreground: Color,
    onToken: (ULong) -> Unit,
): AnnotatedString = buildAnnotatedString {
    append(text)
    for ((token, start, end) in tokens) {
        addLink(
            LinkAnnotation.Clickable(
                tag = "reader_token_${token.tokenIndex}",
                styles =
                    TextLinkStyles(
                        style =
                            SpanStyle(
                                color = foreground,
                                background = vocabularyBackground(token.vocabulary?.status, dark),
                                textDecoration = TextDecoration.None,
                            )
                    ),
                linkInteractionListener = { onToken(token.tokenIndex) },
            ),
            start,
            end,
        )
    }
}

@Composable
internal fun ReaderWords(view: ViewModel, dispatch: (Event) -> Unit, onInteraction: () -> Unit) {
    val page = view.reader ?: return
    val key = view.readerAnalysis?.let { ReaderTokenKey(page.documentId, page.anchor, it) }
    val snapshot = view.readerTokens
    val canWrite = !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
    var requested by remember(key) { mutableStateOf(false) }
    LaunchedEffect(key, snapshot, canWrite, view.documentError) {
        if (snapshot != null) requested = false
        else if (key != null && !requested && canWrite && view.documentError == null) {
            requested = true
            dispatch(Event.LoadReaderTokens(key))
        }
    }
    val tokens =
        remember(page, key, snapshot) {
            if (key != null && snapshot != null) readerDisplayTokens(page, key, snapshot) else null
        }
    var selectedIndex by remember(key, tokens == null) { mutableStateOf<ULong?>(null) }
    val currentInteraction by rememberUpdatedState(onInteraction)
    val dark = isSystemInDarkTheme()
    val foreground = MaterialTheme.colorScheme.onSurface
    val text =
        remember(key, page.text, tokens, dark, foreground) {
            readerAnnotatedText(page.text, tokens.orEmpty(), dark, foreground) { index ->
                currentInteraction()
                selectedIndex = index
            }
        }
    Text(
        text,
        style = MaterialTheme.typography.bodyLarge,
        modifier = Modifier.testTag("reader_text"),
    )
    if (
        key != null &&
            snapshot == null &&
            (requested || view.documentError != null) &&
            canWrite &&
            view.readerTime != null
    ) {
        TextButton(
            onClick = {
                requested = true
                dispatch(Event.LoadReaderTokens(key))
            },
            modifier = Modifier.testTag("retry_word_lookup"),
        ) {
            Text(stringResource(R.string.retry_word_lookup))
        }
    }
    val selected = tokens?.singleOrNull { it.token.tokenIndex == selectedIndex }?.token
    if (selected != null && key != null) {
        ReaderWordPanel(
            selected,
            view,
            canWrite,
            onInteraction,
            onClose = { selectedIndex = null },
        ) { status ->
            selected.vocabulary?.let {
                dispatch(
                    Event.SetReaderStatus(
                        ReaderStatusChange(key, it.id, status, System.currentTimeMillis())
                    )
                )
            }
        }
    }
}

@Composable
@OptIn(ExperimentalMaterial3Api::class)
private fun ReaderWordPanel(
    token: ReaderToken,
    view: ViewModel,
    enabled: Boolean,
    onInteraction: () -> Unit,
    onClose: () -> Unit,
    onStatus: (VocabularyStatus) -> Unit,
) {
    val close = {
        onInteraction()
        onClose()
    }
    // Sheets/popups have their own window: Activity does not see their input.
    val interaction =
        Modifier.pointerInput(onInteraction) {
                awaitPointerEventScope {
                    while (true) {
                        awaitPointerEvent(PointerEventPass.Initial)
                        onInteraction()
                    }
                }
            }
            .onPreviewKeyEvent {
                onInteraction()
                false
            }
    val windowSize = LocalWindowInfo.current.containerSize
    val windowHeight = with(LocalDensity.current) { windowSize.height.toDp() }
    val wide = with(LocalDensity.current) { windowSize.width.toDp() >= 600.dp }
    val content: @Composable () -> Unit = {
        Column(
            Modifier.heightIn(max = windowHeight * 0.75f)
                .fillMaxWidth()
                .then(interaction)
                .testTag("reader_word_panel")
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                token.surface,
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier.testTag("word_surface"),
            )
            token.reading?.let { Text(stringResource(R.string.word_reading, it)) }
            token.dictionaryForm?.let {
                Text(stringResource(R.string.word_dictionary_form, it))
            }
            Row(Modifier.fillMaxWidth().selectableGroup()) {
                for (status in VocabularyStatus.entries) {
                    val selected = token.vocabulary?.status == status
                    Text(
                        stringResource(status.labelResource()),
                        style = MaterialTheme.typography.labelSmall,
                        textAlign = TextAlign.Center,
                        modifier =
                            Modifier.weight(1f)
                                .selectable(
                                    selected = selected,
                                    enabled = enabled && token.vocabulary != null,
                                    role = Role.RadioButton,
                                ) {
                                    onInteraction()
                                    onStatus(status)
                                }
                                .heightIn(min = 48.dp)
                                .padding(vertical = 16.dp)
                                .testTag("word_status_$status"),
                        color =
                            if (selected) MaterialTheme.colorScheme.primary
                            else MaterialTheme.colorScheme.onSurface,
                    )
                }
            }
            StorageStatus(view)
            TextButton(onClick = close) { Text(stringResource(R.string.close)) }
        }
    }
    if (wide) {
        Popup(
            alignment = Alignment.TopEnd,
            onDismissRequest = close,
            properties = PopupProperties(focusable = true),
        ) {
            Surface(
                shape = MaterialTheme.shapes.large,
                tonalElevation = 6.dp,
                shadowElevation = 6.dp,
                modifier =
                    Modifier.widthIn(max = 360.dp)
                        .fillMaxWidth()
                        .heightIn(max = windowHeight * 0.85f)
                        .padding(16.dp)
                        .then(interaction)
                        .testTag("word_side_panel"),
            ) {
                content()
            }
        }
    } else {
        ModalBottomSheet(
            onDismissRequest = close,
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
            modifier = interaction.testTag("word_bottom_sheet"),
        ) {
            content()
        }
    }
}

private fun VocabularyStatus.labelResource(): Int =
    when (this) {
        VocabularyStatus.NEW -> R.string.status_new
        VocabularyStatus.LEARNING -> R.string.status_learning
        VocabularyStatus.FAMILIAR -> R.string.status_familiar
        VocabularyStatus.KNOWN -> R.string.status_known
        VocabularyStatus.IGNORED -> R.string.status_ignored
    }
