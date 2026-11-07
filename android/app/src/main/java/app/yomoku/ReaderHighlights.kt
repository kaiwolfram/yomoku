package app.yomoku

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.text.TextLayoutResult

// String annotations survive AnnotatedString.subSequence for image and visual-line blocks.
internal const val READER_HIGHLIGHT = "reader_highlight"

/** Separate visual runs per line, including bidi, without changing glyph or tap geometry. */
internal fun readerHighlightRects(
    layout: TextLayoutResult,
    start: Int,
    end: Int,
    inset: Float,
): List<Rect> = buildList {
    if (start >= end) return@buildList
    for (line in layout.getLineForOffset(start)..layout.getLineForOffset(end - 1)) {
        val left = maxOf(start, layout.getLineStart(line))
        val right = minOf(end, layout.getLineEnd(line, visibleEnd = true))
        val boxes = mutableListOf<Rect>()
        var offset = left
        while (offset < right) {
            val scalar = Character.codePointAt(layout.layoutInput.text.text, offset)
            if (!Character.isWhitespace(scalar) && scalar != 0xFFFC) {
                val box = layout.getBoundingBox(offset)
                if (box.width > 0f) boxes.add(box)
            }
            offset += Character.charCount(scalar)
        }
        var run: Rect? = null
        fun flush() {
            run?.let {
                if (it.width > 2 * inset)
                    add(Rect(it.left + inset, it.top, it.right - inset, it.bottom))
            }
        }
        for (box in boxes.sortedBy { it.left }) {
            val previous = run
            if (previous != null && box.left <= previous.right + 0.5f) {
                run =
                    Rect(
                        previous.left,
                        minOf(previous.top, box.top),
                        maxOf(previous.right, box.right),
                        maxOf(previous.bottom, box.bottom),
                    )
            } else {
                flush()
                run = box
            }
        }
        flush()
    }
}
