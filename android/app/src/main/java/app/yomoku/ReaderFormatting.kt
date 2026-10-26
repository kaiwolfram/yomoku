package app.yomoku

import app.yomoku.shared.ReaderTextSpan
import app.yomoku.shared.ReaderTextStyle

internal data class ReaderDisplayFormatting(
    val style: ReaderTextStyle,
    val start: Int,
    val end: Int,
)

/** A malformed range never creates a partial style or website click target. */
internal fun readerDisplayFormatting(
    text: String,
    spans: List<ReaderTextSpan>,
): List<ReaderDisplayFormatting>? {
    val boundaries = utf8Boundaries(text) ?: return null
    return spans.map { span ->
        val start = boundaries[span.range.start.value] ?: return null
        val end = boundaries[span.range.end.value] ?: return null
        if (start >= end) return null
        when (val style = span.style) {
            is ReaderTextStyle.Heading -> if (style.value.toInt() !in 1..6) return null
            is ReaderTextStyle.ExternalLink -> {
                val scheme = style.value.substringBefore("://", "")
                val rest = style.value.substringAfter("://", "")
                if (
                    !(scheme.equals("http", true) || scheme.equals("https", true)) ||
                        rest.isEmpty() ||
                        rest.startsWith('/') ||
                        style.value.any { it.isWhitespace() || it.isISOControl() }
                )
                    return null
            }
            else -> Unit
        }
        ReaderDisplayFormatting(span.style, start, end)
    }
}

/** Half-open text ranges represented as nonempty Kotlin IntRanges. */
internal fun readerUncoveredRanges(start: Int, end: Int, covered: List<IntRange>): List<IntRange> {
    var remaining = listOf(start until end)
    for (cover in covered) {
        remaining = remaining.flatMap { range ->
            if (cover.last < range.first || cover.first > range.last) listOf(range)
            else
                buildList {
                    if (range.first < cover.first) add(range.first until cover.first)
                    if (range.last > cover.last) add((cover.last + 1)..range.last)
                }
        }
    }
    return remaining
}
