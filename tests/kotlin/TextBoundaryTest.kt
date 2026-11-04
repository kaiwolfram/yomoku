package app.yomoku.tests

import app.yomoku.shared.ChapterText
import app.yomoku.shared.DocumentError
import app.yomoku.shared.DocumentId
import app.yomoku.shared.EffectiveReaderPage
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.EffectiveReaderWordState
import app.yomoku.shared.EffectiveTokenEligibility
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderDocument
import app.yomoku.shared.ReaderPage
import app.yomoku.shared.TextAnchor
import app.yomoku.shared.TextOffset
import app.yomoku.shared.TextRange
import app.yomoku.shared.ViewModel
import app.yomoku.shared.VocabularyStatus
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

    val illustratedBytes = File(dir, "illustrated.bin").readBytes()
    val illustrated = ReaderDocument.bincodeDeserialize(illustratedBytes)
    val chapterImages = illustrated.images.single()
    check(chapterImages.chapterId == illustrated.chapters.single().chapterId)
    val image = chapterImages.images.single()
    check(image.asset.path == "OPS/images/挿絵😀.png" && image.asset.mediaType == "image/png")
    check(image.alt == "説明𠮷か\u3099\u0000")
    check(image.range.start.value == "😀𠮷か\u3099\u0000".toByteArray(Charsets.UTF_8).size.toLong())
    check(image.range.end.value - image.range.start.value == 3L)
    check(illustrated.bincodeSerialize().contentEquals(illustratedBytes))
    File(dir, "illustrated-return.bin").writeBytes(illustrated.bincodeSerialize())
    val pageBytes = File(dir, "illustrated-page.bin").readBytes()
    val illustratedPage = ReaderPage.bincodeDeserialize(pageBytes)
    check(illustratedPage.images == chapterImages.images)
    val ruby = illustrated.sourceRuby.single()
    check(ruby.chapterId == illustrated.chapters.single().chapterId)
    check(ruby.ruby.single().reading == "よし𠮷か\u3099\u0000")
    check(ruby.ruby.single().range.start.value == 4L)
    check(
        ruby.ruby.single().range.end.value ==
            "😀𠮷か\u3099".toByteArray(Charsets.UTF_8).size.toLong()
    )
    check(illustratedPage.sourceRuby == ruby.ruby)
    check(illustratedPage.text == illustrated.chapters.single().text)
    check(illustratedPage.bincodeSerialize().contentEquals(pageBytes))
    File(dir, "illustrated-page-return.bin").writeBytes(illustratedPage.bincodeSerialize())

    val effectiveBytes = File(dir, "effective-page.bin").readBytes()
    val effective = EffectiveReaderPage.bincodeDeserialize(effectiveBytes)
    check(effective.identity.revision == ULong.MAX_VALUE)
    check(effective.identity.base.page == illustratedPage.anchor)
    check(effective.tokens.map { it.segmentIndex } == listOf(0uL, 1uL))
    check(effective.tokens.all { it.surface == illustratedPage.text })
    check(effective.tokens[0].eligibility == EffectiveTokenEligibility.ANALYZERJAPANESE)
    check(effective.tokens[0].reading == "読み𠮷か\u3099\u0000")
    check(effective.tokens[0].dictionaryForm == "原形𠮷か\u3099\u0000")
    check(effective.tokens[0].vocabulary?.status == VocabularyStatus.KNOWN)
    check(effective.tokens[1].eligibility == EffectiveTokenEligibility.ANALYZERJAPANESE)
    check(effective.tokens[1].reading == null && effective.tokens[1].dictionaryForm == null)
    check(effective.tokens[1].vocabulary == null)
    check(effective.identity.bincodeSerialize().size == 60)
    check(effective.bincodeSerialize().contentEquals(effectiveBytes))
    File(dir, "effective-page-return.bin").writeBytes(effective.bincodeSerialize())

    val completionBytes = File(dir, "effective-completion-event.bin").readBytes()
    val completion = Event.bincodeDeserialize(completionBytes) as Event.CompleteEffectiveReaderPage
    check(completion.identity == effective.identity)
    check(completion.position == illustratedPage.position)
    check(completion.completedAtUnixMs == Long.MAX_VALUE)
    check(completion.bincodeSerialize().contentEquals(completionBytes))
    File(dir, "effective-completion-event-return.bin").writeBytes(completion.bincodeSerialize())
    val viewBytes = File(dir, "effective-app-view.bin").readBytes()
    val view = ViewModel.bincodeDeserialize(viewBytes)
    check(view.effectiveReaderTokens == effective && view.readerTokens == null)
    check(view.completionSavePending && !view.storageBusy)
    check(view.bincodeSerialize().contentEquals(viewBytes))
    File(dir, "effective-app-view-return.bin").writeBytes(view.bincodeSerialize())
    val effectiveSelectionBytes = File(dir, "effective-selection.bin").readBytes()
    val effectiveSelection = EffectiveReaderSelection.bincodeDeserialize(effectiveSelectionBytes)
    check(
        effectiveSelection.page == effective.identity &&
            effectiveSelection.segmentIndex == ULong.MAX_VALUE
    )
    check(effectiveSelectionBytes.size == 68)
    check(effectiveSelection.bincodeSerialize().contentEquals(effectiveSelectionBytes))
    File(dir, "effective-selection-return.bin").writeBytes(effectiveSelection.bincodeSerialize())

    val effectiveWordBytes = File(dir, "effective-word.bin").readBytes()
    val effectiveWord = EffectiveReaderWordState.bincodeDeserialize(effectiveWordBytes)
    check(effectiveWord.selection == effectiveSelection)
    val details = checkNotNull(effectiveWord.details)
    check(details.selection == effectiveSelection)
    check(details.token == effective.tokens[1])
    check(details.encountered == ULong.MAX_VALUE)
    check(details.candidates.dictionaryForm == null)
    check(details.candidates.words.isEmpty() && details.candidates.names.isEmpty())
    val customEntry = checkNotNull(details.customEntry)
    check(customEntry.englishMeaning == " literal\u0000meaning 📖 ")
    check(customEntry.reading == " よみか\u3099\u0000 ")
    check(effectiveWord.customEntryPending && !effectiveWord.busy)
    check(effectiveWord.error == null && effectiveWord.customEntryError != null)
    check(effectiveWord.bincodeSerialize().contentEquals(effectiveWordBytes))
    File(dir, "effective-word-return.bin").writeBytes(effectiveWord.bincodeSerialize())
    check(view.effectiveReaderWord == effectiveWord)
    val effectiveStatus = checkNotNull(view.effectiveReaderStatus)
    check(effectiveStatus.pending && !effectiveStatus.busy)
    check(effectiveStatus.error == DocumentError.UNAVAILABLE)
    for (index in 0..7) {
        val bytes = File(dir, "effective-word-event-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(bytes)
        when (index) {
            0 -> check((event as Event.LoadEffectiveReaderWord).value == effectiveSelection)
            1 -> {
                event as Event.SaveEffectiveReaderCustomEntry
                check(event.selection == effectiveSelection)
                check(event.englishMeaning == customEntry.englishMeaning)
                check(event.reading == customEntry.reading)
            }
            2 ->
                check((event as Event.DeleteEffectiveReaderCustomEntry).value == effectiveSelection)
            3 -> check(event is Event.RetryEffectiveReaderCustomEntry)
            4 -> check(event is Event.CancelEffectiveReaderCustomEntry)
            5 -> {
                event as Event.SetEffectiveReaderStatus
                check(event.selection == effectiveSelection)
                check(event.vocabularyId.value == Long.MAX_VALUE)
                check(event.status == VocabularyStatus.IGNORED)
                check(event.occurredAtUnixMs == Long.MAX_VALUE)
            }
            6 -> check(event is Event.RetryEffectiveReaderStatus)
            7 -> check(event is Event.CancelEffectiveReaderStatus)
        }
        check(event.bincodeSerialize().contentEquals(bytes))
        File(dir, "effective-word-event-$index-return.bin").writeBytes(event.bincodeSerialize())
    }

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
