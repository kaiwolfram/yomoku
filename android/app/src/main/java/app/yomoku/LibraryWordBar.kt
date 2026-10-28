package app.yomoku

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.VocabularyStatus
import app.yomoku.shared.WordStatusCounts

@Composable
internal fun LibraryWordBar(counts: WordStatusCounts?, loading: Boolean, documentId: Long) {
    val dark = isSystemInDarkTheme()
    val values = counts?.let { listOf(it.new, it.learning, it.familiar, it.known, it.ignored) }
    val colors =
        VocabularyStatus.entries.map { status ->
            if (status == VocabularyStatus.KNOWN) Color(if (dark) 0xFF438A60 else 0xFF7CCB96)
            else vocabularyBackground(status, dark)
        }
    val total = values?.sumOf { it.toDouble() } ?: 0.0
    val description =
        when {
            loading -> stringResource(R.string.library_words_loading)
            counts == null -> stringResource(R.string.library_words_unavailable)
            total == 0.0 -> stringResource(R.string.library_words_empty)
            else ->
                stringResource(
                    R.string.library_words_distribution,
                    counts.new.toString(),
                    counts.learning.toString(),
                    counts.familiar.toString(),
                    counts.known.toString(),
                    counts.ignored.toString(),
                )
        }
    val track = androidx.compose.material3.MaterialTheme.colorScheme.surfaceVariant
    Canvas(
        Modifier.fillMaxWidth().height(6.dp).testTag("word_distribution_$documentId").semantics {
            contentDescription = description
        }
    ) {
        drawRect(track)
        if (values != null && total > 0.0) {
            var fraction = 0.0
            values.forEachIndexed { index, value ->
                val start = (fraction * size.width).toFloat()
                fraction += value.toDouble() / total
                val end = (fraction * size.width).toFloat().coerceAtMost(size.width)
                if (end > start)
                    drawRect(
                        colors[index],
                        Offset(start, 0f),
                        Size(end - start, size.height),
                    )
            }
        }
    }
}
