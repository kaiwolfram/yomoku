package app.yomoku

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import app.yomoku.shared.VocabularyStatus

@Composable
internal fun ReaderHighlightedText(
    text: AnnotatedString,
    style: TextStyle,
    modifier: Modifier,
) {
    var layout by remember(text, style) { mutableStateOf<TextLayoutResult?>(null) }
    val dark = isSystemInDarkTheme()
    Text(
        text,
        style = style,
        onTextLayout = {
            layout = it
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
