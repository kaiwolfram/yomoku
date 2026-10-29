package app.yomoku

import app.yomoku.shared.EffectivePageIdentity
import app.yomoku.shared.EffectivePageToken
import app.yomoku.shared.EffectiveReaderPage
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.EffectiveTokenEligibility
import app.yomoku.shared.OriginalBoundaryToken
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderPage
import app.yomoku.shared.ReaderToken
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ReaderTokenSnapshot
import app.yomoku.shared.TextAnchor
import app.yomoku.shared.TextRange

internal data class ReaderDisplayToken(val token: ReaderToken, val start: Int, val end: Int)

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
    val projection = ReaderTokenRanges.create(page) ?: return null
    var previousEnd = 0
    var previousIndex: ULong? = null
    return snapshot.tokens.map { token ->
        val range =
            projection.displayRange(token.anchor, token.displayRange, token.surface) ?: return null
        if (
            range.first < previousEnd ||
                (previousIndex != null && token.tokenIndex <= previousIndex)
        )
            return null
        previousEnd = range.last + 1
        previousIndex = token.tokenIndex
        ReaderDisplayToken(token, range.first, range.last + 1)
    }
}

internal data class EffectiveDisplayToken(
    val selection: EffectiveReaderSelection,
    val token: EffectivePageToken,
    val start: Int,
    val end: Int,
)

internal data class OriginalDisplayBoundary(
    val token: OriginalBoundaryToken,
    val start: Int,
    val end: Int,
)

/** Editing always uses original B, including whole tokens crossing a page edge. */
internal fun originalDisplayBoundaries(
    page: ReaderPage,
    identity: EffectivePageIdentity,
    snapshot: EffectiveReaderPage,
): List<OriginalDisplayBoundary>? {
    if (
        snapshot.identity != identity ||
            page.documentId != identity.base.documentId ||
            page.anchor != identity.base.page ||
            (identity.base.analysis is ReaderAnalysis.Missing &&
                snapshot.originalBoundaries.isNotEmpty())
    )
        return null
    val projection = ReaderTokenRanges.create(page) ?: return null
    var previousEnd = 0
    return snapshot.originalBoundaries.map { token ->
        val range =
            projection.displayRange(token.anchor, token.displayRange, token.surface) ?: return null
        if (range.first < previousEnd) return null
        previousEnd = range.last + 1
        OriginalDisplayBoundary(token, range.first, range.last + 1)
    }
}

/** Punctuation, whitespace and non-Japanese gaps cannot be merged into an edit. */
internal fun originalBoundaryAnchor(
    tokens: List<OriginalDisplayBoundary>,
    first: Int,
    last: Int,
): TextAnchor? {
    if (first !in tokens.indices || last !in tokens.indices) return null
    val selected = tokens.subList(minOf(first, last), maxOf(first, last) + 1)
    if (
        selected.zipWithNext().any { (left, right) ->
            left.token.anchor.sourceId != right.token.anchor.sourceId ||
                left.token.anchor.chapterId != right.token.anchor.chapterId ||
                left.token.anchor.range.end != right.token.anchor.range.start
        }
    )
        return null
    return selected
        .first()
        .token
        .anchor
        .copy(
            range =
                TextRange(
                    selected.first().token.anchor.range.start,
                    selected.last().token.anchor.range.end,
                )
        )
}

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
                (token.eligibility == EffectiveTokenEligibility.MANUALJAPANESESPAN &&
                    (token.reading != null || token.dictionaryForm != null)) ||
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

/** Geometry is shared with the live B renderer until its coordinated replacement. */
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
