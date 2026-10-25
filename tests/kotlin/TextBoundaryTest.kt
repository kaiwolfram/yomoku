package app.yomoku.tests

import app.yomoku.shared.ChapterText
import app.yomoku.shared.DocumentId
import app.yomoku.shared.ReaderDocument
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.TextAnchor
import app.yomoku.shared.TextOffset
import app.yomoku.shared.TextRange
import java.io.File

// Test-only exchange using production-generated payload codecs. The live JNI
// document path will be exercised when real document commands are implemented.
fun verifyTextBoundaries(directory: String) {
    val dir = File(directory)
    val document = DocumentId.bincodeDeserialize(File(dir, "document.bin").readBytes())
    check(document.value == Long.MAX_VALUE)
    File(dir, "document-return.bin").writeBytes(document.bincodeSerialize())
    File(dir, "invalid-id-zero.bin").writeBytes(DocumentId(0L).bincodeSerialize())
    File(dir, "invalid-id-negative.bin").writeBytes(DocumentId(-1L).bincodeSerialize())
    File(dir, "invalid-offset.bin").writeBytes(TextOffset(-1L).bincodeSerialize())

    for (index in 0..1) {
        val bytes = File(dir, "analysis-$index.bin").readBytes()
        val analysis = ReaderAnalysis.bincodeDeserialize(bytes)
        if (index == 0) {
            check((analysis as ReaderAnalysis.Available).value.value == Long.MAX_VALUE)
        } else {
            check(analysis is ReaderAnalysis.Missing)
        }
        check(analysis.bincodeSerialize().contentEquals(bytes))
        File(dir, "analysis-return-$index.bin").writeBytes(analysis.bincodeSerialize())
    }

    val orderedBytes = File(dir, "ordered.bin").readBytes()
    val ordered = ReaderDocument.bincodeDeserialize(orderedBytes)
    check(ordered.documentId.value == Long.MAX_VALUE && ordered.finishedAtLeastOnce)
    check(ordered.chapters.size == 7 && ordered.chapters.first().text.isEmpty())
    check(ordered.chapters.map { it.chapterId.value } == (0L..6L).map { Long.MAX_VALUE - it })
    check(ordered.readingPosition?.chapterId == ordered.chapters[2].chapterId)
    check(ordered.readingPosition.offset.value == "A𠮷".toByteArray(Charsets.UTF_8).size.toLong())
    check(ordered.bincodeSerialize().contentEquals(orderedBytes))
    File(dir, "ordered-return.bin").writeBytes(ordered.bincodeSerialize())

    val selections = listOf("", "日本語", "𠮷野家😀", "か\u3099", "👩‍👩‍👧‍👦", "第二行\n\n", "端")
    for ((index, selected) in selections.withIndex()) {
        val chapterBytes = File(dir, "chapter-$index.bin").readBytes()
        val chapter = ChapterText.bincodeDeserialize(chapterBytes)
        check(chapter.sourceId.value == 9_007_199_254_740_993L)
        check(chapter.chapterId.value == Long.MAX_VALUE - index)
        check(chapter.bincodeSerialize().contentEquals(chapterBytes))
        File(dir, "chapter-return-$index.bin").writeBytes(chapter.bincodeSerialize())

        val anchorBytes = File(dir, "anchor-$index.bin").readBytes()
        val anchor = TextAnchor.bincodeDeserialize(anchorBytes)
        check(anchor.sourceId == chapter.sourceId && anchor.chapterId == chapter.chapterId)
        check(anchor.bincodeSerialize().contentEquals(anchorBytes))
        File(dir, "anchor-return-$index.bin").writeBytes(anchor.bincodeSerialize())

        // String.indexOf/substring use JVM UTF-16 indices. Compute bytes from
        // those indices independently, including supplementary kanji/emoji.
        val utf16Start = chapter.text.indexOf(selected)
        check(utf16Start >= 0)
        val utf16End = utf16Start + selected.length
        val start = chapter.text.substring(0, utf16Start).toByteArray(Charsets.UTF_8).size.toLong()
        val end = chapter.text.substring(0, utf16End).toByteArray(Charsets.UTF_8).size.toLong()
        check(anchor.range.start.value == start && anchor.range.end.value == end)
        val utf8 = chapter.text.toByteArray(Charsets.UTF_8)
        check(utf8.copyOfRange(start.toInt(), end.toInt()).toString(Charsets.UTF_8) == selected)
        val tail =
            TextAnchor(
                chapter.sourceId,
                chapter.chapterId,
                TextRange(TextOffset(end), TextOffset(utf8.size.toLong())),
            )
        File(dir, "tail-$index.bin").writeBytes(tail.bincodeSerialize())
    }
}
