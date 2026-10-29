package app.yomoku.tests

import app.yomoku.effectiveDisplayTokens
import app.yomoku.originalBoundaryAnchor
import app.yomoku.originalDisplayBoundaries
import app.yomoku.readerDisplayTokens
import app.yomoku.shared.*
import app.yomoku.utf8Boundaries

private fun anchor(start: Long, end: Long) =
    TextAnchor(SourceId(1), ChapterId(2), TextRange(TextOffset(start), TextOffset(end)))

private fun page(text: String = "😀 𠮷か\u3099\u0000猫", start: Long = 4) =
    ReaderPage(
        DocumentId(3),
        "Unicode",
        false,
        text,
        anchor(start, start + text.toByteArray(Charsets.UTF_8).size),
        ReadingPosition(SourceId(1), ChapterId(2), TextOffset(start)),
        1uL,
        2uL,
        emptyList(),
        emptyList(),
        emptyList(),
    )

private fun identity(page: ReaderPage) =
    EffectivePageIdentity(
        ULong.MAX_VALUE,
        ReaderTokenKey(page.documentId, page.anchor, ReaderAnalysis.Available(AnalysisId(4))),
    )

private fun snapshot(page: ReaderPage) =
    EffectiveReaderPage(
        identity(page),
        listOf(
            EffectivePageToken(
                7uL,
                anchor(9, 19),
                TextRange(TextOffset(5), TextOffset(15)),
                "𠮷か\u3099",
                EffectiveTokenEligibility.ANALYZERJAPANESE,
                "ヨシガ𠮷\u0000",
                "𠮷か\u3099",
                ReaderVocabulary(VocabularyId(5), VocabularyStatus.NEW),
            ),
            EffectivePageToken(
                ULong.MAX_VALUE,
                anchor(20, 23),
                TextRange(TextOffset(16), TextOffset(19)),
                "猫",
                EffectiveTokenEligibility.MANUALJAPANESESPAN,
                null,
                null,
                null,
            ),
        ),
        emptyList(),
    )

fun verifyEffectiveReaderDisplay() {
    verifyOriginalBoundarySelection()
    val page = page()
    val snapshot = snapshot(page)
    val identity = snapshot.identity
    val before = snapshot.bincodeSerialize()
    for (status in listOf(null) + VocabularyStatus.entries) {
        val tokens =
            snapshot.tokens.map {
                it.copy(vocabulary = status?.let { ReaderVocabulary(VocabularyId(5), it) })
            }
        val current = snapshot.copy(tokens = tokens)
        val spans = checkNotNull(effectiveDisplayTokens(page, identity, current))
        check(spans.map { it.start to it.end } == listOf(3 to 7, 8 to 9))
        check(spans.map { page.text.substring(it.start, it.end) } == listOf("𠮷か\u3099", "猫"))
        check(spans.map { it.token } == tokens)
        check(spans.map { it.selection.segmentIndex } == listOf(7uL, ULong.MAX_VALUE))
        check(spans.all { it.selection.page == identity })
        check(spans.last().token.reading == null && spans.last().token.dictionaryForm == null)
        // The shared geometry preserves the existing B renderer's ranges too.
        val legacy =
            ReaderTokenSnapshot(
                page.documentId,
                page.anchor,
                identity.base.analysis,
                tokens.map {
                    ReaderToken(
                        it.segmentIndex,
                        it.anchor,
                        it.displayRange,
                        it.surface,
                        it.reading,
                        it.dictionaryForm,
                        it.vocabulary,
                    )
                },
            )
        val old = checkNotNull(readerDisplayTokens(page, identity.base, legacy))
        check(old.map { it.start to it.end } == spans.map { it.start to it.end })
    }
    check(before.contentEquals(snapshot.bincodeSerialize()))

    // Crossing bases keep their whole surface, exact metadata and chapter segment.
    for ((visible, start, end) in listOf(Triple("𠮷", 10L, 14L), Triple("野家", 14L, 20L))) {
        val clipped = page(visible, start)
        val fresh = identity(clipped)
        val token =
            snapshot.tokens
                .first()
                .copy(
                    anchor = anchor(10, 20),
                    surface = "𠮷野家",
                    displayRange = TextRange(TextOffset(0), TextOffset(end - start)),
                )
        val span =
            checkNotNull(
                    effectiveDisplayTokens(
                        clipped,
                        fresh,
                        EffectiveReaderPage(fresh, listOf(token), emptyList()),
                    )
                )
                .single()
        check(visible.substring(span.start, span.end) == visible)
        check(span.token == token && span.selection == EffectiveReaderSelection(fresh, 7uL))
        check(
            effectiveDisplayTokens(
                clipped,
                identity,
                EffectiveReaderPage(fresh, listOf(token), emptyList()),
            ) == null
        )
    }

    val token = snapshot.tokens.first()
    val badTokens =
        listOf(
            token.copy(surface = "違う"),
            token.copy(surface = "\uD800"),
            token.copy(anchor = token.anchor.copy(sourceId = SourceId(99))),
            token.copy(anchor = token.anchor.copy(chapterId = ChapterId(99))),
            token.copy(anchor = anchor(-1, 19)),
            token.copy(anchor = anchor(19, 9)),
            token.copy(anchor = anchor(9, Long.MAX_VALUE)),
            token.copy(reading = "\uDC00"),
            token.copy(reading = " \t　"),
            token.copy(dictionaryForm = "\uD800"),
            token.copy(dictionaryForm = ""),
            token.copy(eligibility = EffectiveTokenEligibility.MANUALJAPANESESPAN),
            snapshot.tokens.last().copy(reading = "invented"),
            snapshot.tokens.last().copy(dictionaryForm = "invented"),
        ) +
            listOf(
                    -1L to 15L,
                    6L to 15L,
                    5L to 14L,
                    5L to 20L,
                    5L to 5L,
                    Long.MAX_VALUE to Long.MAX_VALUE,
                )
                .map { (start, end) ->
                    token.copy(displayRange = TextRange(TextOffset(start), TextOffset(end)))
                }
    for ((index, bad) in badTokens.withIndex()) {
        check(effectiveDisplayTokens(page, identity, snapshot.copy(tokens = listOf(bad))) == null) {
            "bad token $index"
        }
    }
    for (tokens in
        listOf(
            snapshot.tokens.reversed(),
            listOf(token, token),
            listOf(token, snapshot.tokens.last().copy(segmentIndex = token.segmentIndex)),
        )) {
        check(effectiveDisplayTokens(page, identity, snapshot.copy(tokens = tokens)) == null)
    }
    for (foreign in
        listOf(
            identity.copy(revision = identity.revision - 1uL),
            identity.copy(base = identity.base.copy(documentId = DocumentId(99))),
            identity.copy(
                base = identity.base.copy(analysis = ReaderAnalysis.Available(AnalysisId(99)))
            ),
            identity.copy(base = identity.base.copy(page = anchor(5, 23))),
        )) {
        check(effectiveDisplayTokens(page, identity, snapshot.copy(identity = foreign)) == null)
        check(effectiveDisplayTokens(page, foreign, snapshot) == null)
    }
    for (bad in
        listOf(
            page.copy(documentId = DocumentId(99)),
            page.copy(text = page.text.replace('猫', '犬')),
            page.copy(text = page.text + "extra"),
            page.copy(text = "\uD800"),
            page.copy(anchor = anchor(-1, 18)),
            page.copy(anchor = anchor(4, Long.MAX_VALUE)),
        )) check(effectiveDisplayTokens(bad, identity, snapshot) == null)

    val missing = identity.copy(base = identity.base.copy(analysis = ReaderAnalysis.Missing))
    check(
        effectiveDisplayTokens(
            page,
            missing,
            EffectiveReaderPage(missing, emptyList(), emptyList()),
        ) == emptyList<Any>()
    )
    check(effectiveDisplayTokens(page, missing, snapshot.copy(identity = missing)) == null)
    check(
        effectiveDisplayTokens(page, identity, snapshot.copy(tokens = emptyList())) ==
            emptyList<Any>()
    )
    // UTF-16 input is validated even for an otherwise empty/plain projection.
    check(
        effectiveDisplayTokens(
            page.copy(text = "\uD800"),
            missing,
            EffectiveReaderPage(missing, emptyList(), emptyList()),
        ) == null
    )
    for (text in listOf("\uD800", "\uDC00", "x\uD800x")) check(utf8Boundaries(text) == null)
    check(
        utf8Boundaries("😀𠮷か\u3099\u0000") ==
            mapOf(0L to 0, 4L to 2, 8L to 4, 11L to 5, 14L to 6, 15L to 7)
    )

    // Same written form can carry different contextual readings and segment IDs.
    val repeated = page("猫猫", 0)
    val repeatedIdentity = identity(repeated)
    val repeats =
        listOf(0L, 3L).mapIndexed { index, start ->
            token.copy(
                segmentIndex = index.toULong(),
                anchor = anchor(start, start + 3),
                surface = "猫",
                displayRange = TextRange(TextOffset(start), TextOffset(start + 3)),
                reading = "よみ$index",
            )
        }
    val spans =
        checkNotNull(
            effectiveDisplayTokens(
                repeated,
                repeatedIdentity,
                EffectiveReaderPage(repeatedIdentity, repeats, emptyList()),
            )
        )
    check(spans.map { it.token.reading } == listOf("よみ0", "よみ1"))
    check(spans[0].selection != spans[1].selection)
}

private fun verifyOriginalBoundarySelection() {
    val page = page("𠮷か\u3099\u0000猫。犬", 4)
    val id = identity(page)
    val originals =
        listOf(
            OriginalBoundaryToken(
                anchor(0, 14),
                TextRange(TextOffset(0), TextOffset(10)),
                "😀𠮷か\u3099",
            ),
            OriginalBoundaryToken(
                anchor(14, 18),
                TextRange(TextOffset(10), TextOffset(14)),
                "\u0000猫",
            ),
            OriginalBoundaryToken(anchor(21, 27), TextRange(TextOffset(17), TextOffset(20)), "犬猫"),
        )
    val snapshot = EffectiveReaderPage(id, emptyList(), originals)
    val tokens = checkNotNull(originalDisplayBoundaries(page, id, snapshot))
    check(tokens.map { it.start to it.end } == listOf(0 to 4, 4 to 6, 7 to 8))
    check(originalBoundaryAnchor(tokens, 0, 1) == anchor(0, 18))
    check(originalBoundaryAnchor(tokens, 1, 0) == anchor(0, 18))
    check(originalBoundaryAnchor(tokens, 2, 2) == anchor(21, 27))
    check(originalBoundaryAnchor(tokens, 0, 2) == null)
    check(originalBoundaryAnchor(tokens, -1, 0) == null)
    check(originalBoundaryAnchor(tokens, 0, 3) == null)
    // Effective splits and unresolved metadata never replace B edit boundaries.
    check(snapshot.tokens.isEmpty() && tokens.size == 3)
    for (bad in
        listOf(
            originals.first().copy(surface = "違う"),
            originals.first().copy(surface = "\uD800"),
            originals.first().copy(displayRange = TextRange(TextOffset(1), TextOffset(10))),
            originals.first().copy(anchor = anchor(0, 13)),
            originals.first().copy(anchor = originals.first().anchor.copy(sourceId = SourceId(99))),
        )) check(
        originalDisplayBoundaries(page, id, snapshot.copy(originalBoundaries = listOf(bad))) == null
    )
    check(
        originalDisplayBoundaries(
            page,
            id,
            snapshot.copy(originalBoundaries = originals.reversed()),
        ) == null
    )
    check(originalDisplayBoundaries(page, id.copy(revision = 1UL), snapshot) == null)
    val missing = id.copy(base = id.base.copy(analysis = ReaderAnalysis.Missing))
    check(originalDisplayBoundaries(page, missing, snapshot.copy(identity = missing)) == null)
    check(
        originalDisplayBoundaries(
                page,
                missing,
                EffectiveReaderPage(missing, emptyList(), emptyList()),
            )!!
            .isEmpty()
    )
}
