package app.yomoku

/**
 * Literal partitions of a selected span. Offsets are UTF-16 scalar boundaries, never glyph guesses.
 */
internal class ReaderBoundaryText
private constructor(
    val surface: String,
    val offsets: List<Int>,
) {
    val splitOffsets: List<Int> = offsets.drop(1).dropLast(1)

    fun parts(cuts: Set<Int>): List<String>? {
        if (!splitOffsets.containsAll(cuts)) return null
        val edges = listOf(0) + cuts.sorted() + surface.length
        val parts = edges.zipWithNext { start, end -> surface.substring(start, end) }
        return parts.takeIf { values -> values.all { part -> part.any { !rustWhitespace(it) } } }
    }

    fun cuts(parts: List<String>): Set<Int>? {
        if (parts.isEmpty() || parts.joinToString("") != surface) return null
        var offset = 0
        val cuts =
            parts
                .dropLast(1)
                .map { part ->
                    offset += part.length
                    offset
                }
                .toSet()
        return cuts.takeIf { this.parts(it) == parts }
    }

    companion object {
        fun parse(surface: String): ReaderBoundaryText? {
            if (surface.isEmpty()) return null
            val offsets = mutableListOf(0)
            var index = 0
            while (index < surface.length) {
                val char = surface[index++]
                if (char.isHighSurrogate()) {
                    if (index == surface.length || !surface[index].isLowSurrogate()) return null
                    index++
                } else if (char.isLowSurrogate()) return null
                offsets.add(index)
            }
            return ReaderBoundaryText(surface, offsets)
        }
    }
}

// Match Rust str::trim/Surface::new (Unicode White_Space). Kotlin's isBlank also
// includes U+001C..U+001F and omits U+0085 on the JVM, changing valid literal parts.
private fun rustWhitespace(char: Char): Boolean =
    char in '\u0009'..'\u000D' ||
        char == '\u0020' ||
        char == '\u0085' ||
        char == '\u00A0' ||
        char == '\u1680' ||
        char in '\u2000'..'\u200A' ||
        char == '\u2028' ||
        char == '\u2029' ||
        char == '\u202F' ||
        char == '\u205F' ||
        char == '\u3000'
