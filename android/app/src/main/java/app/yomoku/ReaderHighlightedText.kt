package app.yomoku

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
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
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.takeOrElse
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.ResolvedTextDirection
import androidx.compose.ui.unit.dp
import app.yomoku.shared.VocabularyStatus

@Composable
internal fun ReaderHighlightedText(
    text: AnnotatedString,
    style: TextStyle,
    modifier: Modifier,
) {
    var layout by remember(text, style) { mutableStateOf<TextLayoutResult?>(null) }
    val targets =
        remember(text, layout) {
            layout
                ?.let { result ->
                    text.getLinkAnnotations(0, text.length).map { range ->
                        range.item to readerTextFragmentRects(result, range.start, range.end)
                    }
                }
                .orEmpty()
        }
    val dark = isSystemInDarkTheme()
    val markerColor = style.color.takeOrElse { LocalContentColor.current }.copy(alpha = 0.7f)
    Text(
        text,
        style = style,
        onTextLayout = {
            layout = it
        },
        modifier =
            modifier
                .pointerInput(text, style, layout) {
                    fun targetAt(position: Offset) =
                        targets
                            .firstOrNull { (_, rects) -> rects.any { it.contains(position) } }
                            ?.first
                    awaitEachGesture {
                        // Intercept before generated link nodes, whose rounded/clipped bounds can
                        // miss glyph edges or cover another fragment. Keep their semantics intact.
                        val down = awaitFirstDown(pass = PointerEventPass.Initial)
                        val target = targetAt(down.position)
                        down.consume()
                        if (target == null) return@awaitEachGesture
                        var cancelled = false
                        while (true) {
                            val event = awaitPointerEvent(PointerEventPass.Initial)
                            val change = event.changes.singleOrNull()
                            if (
                                change == null ||
                                    change.id != down.id ||
                                    change.isConsumed ||
                                    (change.position - down.position).getDistance() >
                                        viewConfiguration.touchSlop
                            )
                                cancelled = true
                            if (change?.changedToUpIgnoreConsumed() == true) {
                                if (!cancelled && targetAt(change.position) === target) {
                                    change.consume()
                                    target.linkInteractionListener?.onClick(target)
                                }
                                break
                            }
                            // Leave movement unconsumed so the reader's vertical scroll can win.
                            if (
                                awaitPointerEvent(PointerEventPass.Final).changes.any {
                                    it.isConsumed
                                }
                            )
                                cancelled = true
                            if (event.changes.none { it.pressed }) break
                        }
                    }
                }
                .drawWithCache {
                    val highlights =
                        layout
                            ?.let { result ->
                                text
                                    .getStringAnnotations(READER_HIGHLIGHT, 0, text.length)
                                    .flatMap { range ->
                                        val color =
                                            vocabularyBackground(
                                                VocabularyStatus.valueOf(range.item),
                                                dark,
                                            )
                                        readerTextFragmentRects(
                                                result,
                                                range.start,
                                                range.end,
                                                1.dp.toPx(),
                                            )
                                            .map { it to color }
                                    }
                            }
                            .orEmpty()
                    val markers =
                        layout
                            ?.let { result ->
                                text.getStringAnnotations(READER_MARKER, 0, text.length).flatMap {
                                    range ->
                                    readerTextFragmentRects(result, range.start, range.end)
                                }
                            }
                            .orEmpty()
                    val thickness = 1.dp.toPx()
                    val dotRadius = 2.dp.toPx()
                    val dotDistance = 6.dp.toPx()
                    val dot = layout?.let { result ->
                        text
                            .getStringAnnotations(READER_MARKER_DOT, 0, text.length)
                            .singleOrNull()
                            ?.let { range ->
                                val line = result.getLineForOffset(range.start)
                                val direction =
                                    result.getParagraphDirection(result.getLineStart(line))
                                val x =
                                    if (direction == ResolvedTextDirection.Rtl)
                                        result.getLineRight(line) + dotDistance
                                    else result.getLineLeft(line) - dotDistance
                                Offset(
                                    x,
                                    (result.getLineTop(line) + result.getLineBottom(line)) / 2f,
                                )
                            }
                    }
                    onDrawWithContent {
                        for ((rect, color) in highlights) drawRect(
                            color,
                            Offset(rect.left, rect.top),
                            rect.size,
                        )
                        drawContent()
                        for (rect in markers) drawRect(
                            markerColor,
                            Offset(rect.left, rect.bottom - thickness),
                            Size(rect.width, thickness),
                        )
                        if (dot != null) drawCircle(markerColor, dotRadius, dot)
                    }
                },
    )
}
