package app.yomoku

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.PlatformTextStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import app.yomoku.shared.ReaderRuby
import app.yomoku.shared.VocabularyStatus

internal data class ReaderDisplayRuby(val start: Int, val end: Int, val reading: String) {
    fun clip(start: Int, end: Int): ReaderDisplayRuby? {
        val left = maxOf(this.start, start)
        val right = minOf(this.end, end)
        return if (left < right) copy(start = left - start, end = right - start) else null
    }
}

/** Reject the complete projection on malformed/overlapping UTF-8 ranges. */
internal fun readerDisplayRuby(text: String, ruby: List<ReaderRuby>): List<ReaderDisplayRuby>? {
    val boundaries = utf8Boundaries(text) ?: return null
    var previousEnd = 0
    return ruby.map {
        val start = boundaries[it.range.start.value] ?: return null
        val end = boundaries[it.range.end.value] ?: return null
        if (
            start < previousEnd ||
                start >= end ||
                it.reading.isEmpty() ||
                utf8Boundaries(it.reading) == null
        )
            return null
        previousEnd = end
        ReaderDisplayRuby(start, end, it.reading)
    }
}

internal data class ReaderRubyPlacement(val reading: TextLayoutResult, val bounds: Rect)

/**
 * Readings are paint only: the canonical annotated Text owns selection, accessibility, word/link
 * gestures and every offset. A visual line/image split repeats the complete reading, just like a
 * logical page split; compounds have no safe partial mapping.
 */
internal fun readerRubyPlacements(
    layout: TextLayoutResult,
    ruby: List<ReaderDisplayRuby>,
    readings: Map<String, TextLayoutResult>,
    measurer: TextMeasurer,
): List<ReaderRubyPlacement> = buildList {
    val text = layout.layoutInput.text
    val naturalStyle =
        layout.layoutInput.style.copy(
            lineHeight = TextUnit.Unspecified,
            lineHeightStyle = null,
        )
    val lineAscents = mutableMapOf<Int, Float>()
    for (annotation in ruby) {
        val reading = readings.getValue(annotation.reading)
        for (line in
            layout.getLineForOffset(annotation.start)..layout.getLineForOffset(
                    annotation.end - 1
                )) {
            val start = maxOf(annotation.start, layout.getLineStart(line))
            val end = minOf(annotation.end, layout.getLineEnd(line, visibleEnd = true))
            if (start >= end) continue
            // Union scalar boxes also supports supplementary, combining and bidi text.
            var left = Float.POSITIVE_INFINITY
            var right = Float.NEGATIVE_INFINITY
            var offset = start
            while (offset < end) {
                if (text[offset] != '\uFFFC' && !text[offset].isWhitespace()) {
                    val box = layout.getBoundingBox(offset)
                    left = minOf(left, box.left)
                    right = maxOf(right, box.right)
                }
                offset += Character.charCount(Character.codePointAt(text.text, offset))
            }
            if (right <= left) continue
            val ascent =
                lineAscents.getOrPut(line) {
                    measurer
                        .measure(
                            text.subSequence(
                                layout.getLineStart(line),
                                layout.getLineEnd(line, visibleEnd = true),
                            ),
                            naturalStyle,
                            softWrap = false,
                        )
                        .firstBaseline
                }
            val width = minOf(right - left, reading.size.width.toFloat())
            val top = layout.getLineBaseline(line) - ascent - reading.size.height
            add(
                ReaderRubyPlacement(
                    reading,
                    Rect(
                        (left + right - width) / 2f,
                        top,
                        (left + right + width) / 2f,
                        top + reading.size.height,
                    ),
                )
            )
        }
    }
}

@Composable
internal fun ReaderRubyText(
    text: AnnotatedString,
    style: TextStyle,
    ruby: List<ReaderDisplayRuby>,
    modifier: Modifier = Modifier,
) {
    if (ruby.isEmpty()) {
        ReaderHighlightedText(text, style, modifier)
        return
    }
    val measurer = rememberTextMeasurer()
    val color = LocalContentColor.current
    val naturalStyle =
        style.copy(
            lineHeight = TextUnit.Unspecified,
            lineHeightStyle = null,
            platformStyle = PlatformTextStyle(includeFontPadding = false),
        )
    val readingStyle = naturalStyle.copy(fontSize = style.fontSize * 0.5f, color = color)
    val readings =
        remember(ruby, readingStyle, measurer) {
            ruby
                .map { it.reading }
                .distinct()
                .associateWith {
                    measurer.measure(it, readingStyle, softWrap = false)
                }
        }
    // Include the largest styled line (e.g. EPUB heading), font scaling and literal
    // reading height. Bottom alignment reserves the added space above every base.
    val lineHeight =
        remember(text, naturalStyle, readings, measurer) {
            val natural = measurer.measure(text, naturalStyle, softWrap = false)
            val height =
                (0 until natural.lineCount).maxOf {
                    natural.getLineBottom(it) - natural.getLineTop(it)
                }
            height
        }
    val density = androidx.compose.ui.platform.LocalDensity.current
    val rubyStyle =
        style.copy(
            lineHeight =
                with(density) {
                    (maxOf(style.lineHeight.toPx(), lineHeight) +
                            readings.values.maxOf { it.size.height })
                        .toSp()
                },
            platformStyle = PlatformTextStyle(includeFontPadding = false),
            lineHeightStyle =
                LineHeightStyle(LineHeightStyle.Alignment.Bottom, LineHeightStyle.Trim.None),
        )
    var layout by remember(text, rubyStyle) { mutableStateOf<TextLayoutResult?>(null) }
    ReaderHighlightedText(
        text,
        style = rubyStyle,
        onTextLayout = { layout = it },
        modifier =
            modifier.drawWithCache {
                val placements =
                    layout?.let { readerRubyPlacements(it, ruby, readings, measurer) }.orEmpty()
                onDrawWithContent {
                    drawContent()
                    for ((reading, bounds) in placements) {
                        val origin = Offset(bounds.left, bounds.top)
                        scale(bounds.width / reading.size.width.coerceAtLeast(1), 1f, origin) {
                            drawText(reading, topLeft = origin)
                        }
                    }
                }
            },
    )
}

@Composable
private fun ReaderHighlightedText(
    text: AnnotatedString,
    style: TextStyle,
    modifier: Modifier,
    onTextLayout: (TextLayoutResult) -> Unit = {},
) {
    var layout by remember(text, style) { mutableStateOf<TextLayoutResult?>(null) }
    val dark = isSystemInDarkTheme()
    Text(
        text,
        style = style,
        onTextLayout = {
            layout = it
            onTextLayout(it)
        },
        modifier =
            modifier.drawWithCache {
                val highlights =
                    layout
                        ?.let { result ->
                            text.getStringAnnotations(READER_HIGHLIGHT, 0, text.length).flatMap {
                                range ->
                                val color =
                                    vocabularyBackground(VocabularyStatus.valueOf(range.item), dark)
                                readerHighlightRects(result, range.start, range.end, 1.dp.toPx())
                                    .map { it to color }
                            }
                        }
                        .orEmpty()
                onDrawWithContent {
                    for ((rect, color) in highlights) drawRect(
                        color,
                        Offset(rect.left, rect.top),
                        rect.size,
                    )
                    drawContent()
                }
            },
    )
}
