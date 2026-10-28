package app.yomoku.tests

import app.yomoku.ReaderBoundaryText

fun verifyReaderBoundaryDraft() {
    val surface = "𠮷か\u3099\u0000猫\r\n犬"
    val text = checkNotNull(ReaderBoundaryText.parse(surface))
    check(text.offsets == listOf(0, 2, 3, 4, 5, 6, 7, 8, 9))
    check(text.parts(setOf(2, 4, 8)) == listOf("𠮷", "か\u3099", "\u0000猫\r\n", "犬"))
    check(text.cuts(listOf("𠮷", "か\u3099", "\u0000猫\r\n", "犬")) == setOf(2, 4, 8))
    check(text.parts(emptySet()) == listOf(surface))
    for (invalid in listOf(-1, 0, 1, surface.length, Int.MAX_VALUE)) {
        check(text.parts(setOf(invalid)) == null)
    }
    for (invalid in listOf("", "\uD800", "\uDC00", "猫\uD800犬", "\uD800\uD800")) {
        check(ReaderBoundaryText.parse(invalid) == null)
    }
    for (parts in
        listOf(
            emptyList(),
            listOf(""),
            listOf(surface, ""),
            listOf("", surface),
            listOf("猫"),
            listOf("\uD842", "\uDFB7か\u3099\u0000猫\r\n犬"),
        )) {
        check(text.cuts(parts) == null)
    }
    // Match Surface::new exactly, including JVM/Rust whitespace disagreements.
    for (whitespace in
        listOf(" ", "\t", "\r\n", "\u0085", "\u00A0", "\u2007", "\u202F", "\u3000")) {
        val spaces = checkNotNull(ReaderBoundaryText.parse("猫${whitespace}犬"))
        check(spaces.cuts(listOf("猫", whitespace, "犬")) == null)
        check(spaces.cuts(listOf("猫$whitespace", "犬")) != null)
        check(ReaderBoundaryText.parse(whitespace)!!.parts(emptySet()) == null)
    }
    for (literal in listOf("\u0000", "\u001C", "\u001F", "\u200B", "\uFEFF", "\u0301")) {
        check(ReaderBoundaryText.parse(literal)!!.parts(emptySet()) == listOf(literal))
    }
    // Every representable scalar partition must preserve exact literal text.
    for (sample in listOf(surface, "猫犬", "𠮷𩸽📖", "e\u0301\u0000|猫", "猫 犬", "猫\u0085犬")) {
        val value = checkNotNull(ReaderBoundaryText.parse(sample))
        for (mask in 0 until (1 shl value.splitOffsets.size)) {
            val cuts =
                value.splitOffsets.filterIndexed { index, _ -> mask and (1 shl index) != 0 }.toSet()
            val parts = value.parts(cuts) ?: continue
            check(parts.joinToString("") == sample)
            check(value.cuts(parts) == cuts)
            check(parts.all { ReaderBoundaryText.parse(it) != null })
        }
    }
}
