package app.yomoku

import app.yomoku.shared.EffectivePageIdentity
import app.yomoku.shared.EffectivePageToken
import app.yomoku.shared.EffectiveReaderPage
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderPage
import app.yomoku.shared.TextAnchor
import app.yomoku.shared.TextRange

/** Reject malformed UTF-16 and byte offsets inside a scalar, never round them. */
internal fun utf8Boundaries(text: String): Map<Long, Int>? {
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

internal data class EffectiveDisplayToken(
    val selection: EffectiveReaderSelection,
    val token: EffectivePageToken,
    val start: Int,
    val end: Int,
)

/** Keep the effective segment/revision on every tap, including crossing/manual words. */
internal fun effectiveDisplayTokens(
    page: ReaderPage,
    identity: EffectivePageIdentity,
    snapshot: EffectiveReaderPage,
): List<EffectiveDisplayToken>? {
    if (
        snapshot.identity != identity ||
            page.documentId != identity.base.documentId ||
            page.anchor != identity.base.page ||
            (identity.base.analysis is ReaderAnalysis.Missing && snapshot.tokens.isNotEmpty())
    )
        return null
    val projection = ReaderTokenRanges.create(page) ?: return null
    var previousEnd = 0
    var previousIndex: ULong? = null
    return snapshot.tokens.map { token ->
        val range =
            projection.displayRange(token.anchor, token.displayRange, token.surface) ?: return null
        if (
            range.first < previousEnd ||
                (previousIndex != null && token.segmentIndex <= previousIndex) ||
                listOfNotNull(token.reading, token.dictionaryForm).any {
                    it.isBlank() || utf8Boundaries(it) == null
                }
        )
            return null
        previousEnd = range.last + 1
        previousIndex = token.segmentIndex
        EffectiveDisplayToken(
            EffectiveReaderSelection(identity, token.segmentIndex),
            token,
            range.first,
            range.last + 1,
        )
    }
}

/** Validate canonical UTF-8 ranges before converting them to Compose UTF-16 ranges. */
private class ReaderTokenRanges
private constructor(
    private val page: ReaderPage,
    private val boundaries: Map<Long, Int>,
) {
    fun displayRange(anchor: TextAnchor, display: TextRange, surface: String): IntRange? {
        val start = boundaries[display.start.value] ?: return null
        val end = boundaries[display.end.value] ?: return null
        val pageStart = page.anchor.range.start.value
        val pageEnd = page.anchor.range.end.value
        if (
            start >= end ||
                anchor.sourceId != page.anchor.sourceId ||
                anchor.chapterId != page.anchor.chapterId ||
                anchor.range.start.value < 0 ||
                anchor.range.end.value <= anchor.range.start.value ||
                display.start.value != maxOf(anchor.range.start.value, pageStart) - pageStart ||
                display.end.value != minOf(anchor.range.end.value, pageEnd) - pageStart
        )
            return null
        val surfaceBoundaries = utf8Boundaries(surface) ?: return null
        val surfaceStart =
            surfaceBoundaries[maxOf(pageStart - anchor.range.start.value, 0)] ?: return null
        val surfaceEnd =
            surfaceBoundaries[minOf(pageEnd, anchor.range.end.value) - anchor.range.start.value]
                ?: return null
        if (
            surfaceBoundaries[anchor.range.end.value - anchor.range.start.value] !=
                surface.length ||
                surface.substring(surfaceStart, surfaceEnd) != page.text.substring(start, end)
        )
            return null
        return start until end
    }

    companion object {
        fun create(page: ReaderPage): ReaderTokenRanges? {
            val boundaries = utf8Boundaries(page.text) ?: return null
            val start = page.anchor.range.start.value
            val end = page.anchor.range.end.value
            if (start < 0 || end <= start || boundaries[end - start] != page.text.length)
                return null
            return ReaderTokenRanges(page, boundaries)
        }
    }
}
