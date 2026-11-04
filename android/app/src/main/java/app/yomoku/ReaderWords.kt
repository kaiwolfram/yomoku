package app.yomoku

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import app.yomoku.shared.ReaderTextSpan
import app.yomoku.shared.ReaderTextStyle
import app.yomoku.shared.ReaderVocabulary
import app.yomoku.shared.VocabularyStatus

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
    tokens: List<EffectiveDisplayToken>,
    dark: Boolean,
    foreground: Color,
    formatting: List<ReaderTextSpan> = emptyList(),
    linkColor: Color = foreground,
    onExternalLink: (String) -> Unit = {},
    onToken: (ULong) -> Unit,
): AnnotatedString =
    readerAnnotatedSpans(
        text,
        tokens.map {
            ReaderAnnotatedToken(
                it.token.segmentIndex,
                it.token.vocabulary?.status,
                it.start,
                it.end,
            )
        },
        dark,
        foreground,
        formatting,
        linkColor,
        onExternalLink,
        onToken,
    )

internal data class ReaderAnnotatedToken(
    val index: ULong,
    val status: VocabularyStatus?,
    val start: Int,
    val end: Int,
)

/** Shared formatting/geometry only; each reader owns its selection identity. */
internal fun readerAnnotatedSpans(
    text: String,
    tokens: List<ReaderAnnotatedToken>,
    dark: Boolean,
    foreground: Color,
    formatting: List<ReaderTextSpan>,
    linkColor: Color,
    onExternalLink: (String) -> Unit,
    onToken: (ULong) -> Unit,
): AnnotatedString = buildAnnotatedString {
    append(text)
    val spans = readerDisplayFormatting(text, formatting).orEmpty()
    val external = mutableListOf<IntRange>()
    for ((style, start, end) in spans) {
        when (style) {
            is ReaderTextStyle.Heading ->
                addStyle(
                    SpanStyle(
                        fontWeight = FontWeight.Bold,
                        fontSize = (1.75f - style.value.toInt() * 0.125f).em,
                    ),
                    start,
                    end,
                )
            ReaderTextStyle.Emphasis ->
                addStyle(SpanStyle(fontStyle = FontStyle.Italic), start, end)
            ReaderTextStyle.Strong -> addStyle(SpanStyle(fontWeight = FontWeight.Bold), start, end)
            is ReaderTextStyle.ExternalLink -> {
                // Malformed nested source links still produce disjoint click targets.
                for (range in readerUncoveredRanges(start, end, external)) {
                    addLink(
                        LinkAnnotation.Url(
                            style.value,
                            TextLinkStyles(
                                SpanStyle(
                                    color = linkColor,
                                    textDecoration = TextDecoration.Underline,
                                )
                            ),
                            linkInteractionListener = { onExternalLink(style.value) },
                        ),
                        range.first,
                        range.last + 1,
                    )
                    external.add(range)
                }
            }
        }
    }
    for ((index, status, start, end) in tokens) {
        addStyle(
            SpanStyle(background = vocabularyBackground(status, dark)),
            start,
            end,
        )
        // Source website links own their tap area; the remaining token fragments
        // retain their owning selection identity and vocabulary highlight.
        for (range in readerUncoveredRanges(start, end, external)) {
            // These styles never depend on hover/focus/press. Keep them outside the link:
            // Compose otherwise remaps the full annotation list for every word on each
            // interaction, making long logical pages quadratic in their token count.
            addStyle(
                SpanStyle(color = foreground, textDecoration = TextDecoration.None),
                range.first,
                range.last + 1,
            )
            addLink(
                LinkAnnotation.Clickable(
                    tag = "reader_token_${index}",
                    // Empty explicitly: null lets Material install its dynamic website style.
                    styles = TextLinkStyles(),
                    linkInteractionListener = { onToken(index) },
                ),
                range.first,
                range.last + 1,
            )
        }
    }
}

@Composable
@OptIn(ExperimentalMaterial3Api::class)
internal fun ReaderWordPanel(
    token: ReaderWordHeader,
    enabled: Boolean,
    onInteraction: () -> Unit,
    onClose: () -> Unit,
    onStatus: (VocabularyStatus) -> Unit,
    details: @Composable () -> Unit,
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
            Row(
                Modifier.fillMaxWidth()
                    .clip(RoundedCornerShape(50))
                    .selectableGroup()
                    .testTag("word_status_pill")
            ) {
                for (status in VocabularyStatus.entries) {
                    val selected = token.vocabulary?.status == status
                    val dark = isSystemInDarkTheme()
                    val tint =
                        if (status == VocabularyStatus.KNOWN) {
                            Color(if (dark) 0xFF316747 else 0xFFBCE8C9)
                        } else vocabularyBackground(status, dark)
                    Box(
                        contentAlignment = Alignment.Center,
                        modifier =
                            Modifier.weight(1f)
                                .heightIn(min = 48.dp)
                                .background(if (selected) tint else tint.copy(alpha = 0.35f))
                                .then(
                                    if (selected)
                                        Modifier.border(
                                            2.dp,
                                            MaterialTheme.colorScheme.onSurface.copy(alpha = 0.65f),
                                            RoundedCornerShape(50),
                                        )
                                    else Modifier
                                )
                                .selectable(
                                    selected = selected,
                                    enabled = enabled && token.vocabulary != null,
                                    role = Role.RadioButton,
                                ) {
                                    onInteraction()
                                    onStatus(status)
                                }
                                .testTag("word_status_$status")
                                .padding(horizontal = 2.dp, vertical = 12.dp),
                    ) {
                        Text(
                            stringResource(status.labelResource()),
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                            textAlign = TextAlign.Center,
                            color = MaterialTheme.colorScheme.onSurface,
                        )
                    }
                }
            }
            TextButton(onClick = close) { Text(stringResource(R.string.close)) }
            details()
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

internal data class ReaderWordHeader(
    val surface: String,
    val reading: String?,
    val dictionaryForm: String?,
    val vocabulary: ReaderVocabulary?,
)

@Composable
internal fun ReaderWordLookupState(
    failed: Boolean,
    enabled: Boolean,
    onInteraction: () -> Unit,
    onRetry: () -> Unit,
) {
    if (failed) {
        Text(stringResource(R.string.word_lookup_failed), Modifier.testTag("word_lookup_error"))
        TextButton(
            onClick = {
                onInteraction()
                onRetry()
            },
            enabled = enabled,
            modifier = Modifier.testTag("retry_word_details"),
        ) {
            Text(stringResource(R.string.retry_word_lookup))
        }
    } else {
        Text(stringResource(R.string.word_lookup_loading))
    }
}

internal fun VocabularyStatus.labelResource(): Int =
    when (this) {
        VocabularyStatus.NEW -> R.string.status_new
        VocabularyStatus.LEARNING -> R.string.status_learning
        VocabularyStatus.FAMILIAR -> R.string.status_familiar
        VocabularyStatus.KNOWN -> R.string.status_known
        VocabularyStatus.IGNORED -> R.string.status_ignored
    }
