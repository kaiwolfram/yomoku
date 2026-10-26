package app.yomoku

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderWordSpansTest {
    private fun anchor(start: Long, end: Long) =
        TextAnchor(SourceId(1), ChapterId(2), TextRange(TextOffset(start), TextOffset(end)))

    private fun page(text: String, start: Long = 0): ReaderPage =
        ReaderPage(
            DocumentId(3),
            "Unicode",
            false,
            text,
            anchor(start, start + text.toByteArray(Charsets.UTF_8).size),
            ReadingPosition(SourceId(1), ChapterId(2), TextOffset(start)),
            1UL,
            2UL,
            emptyList(),
            emptyList(),
        )

    @Test
    fun unicodeRangesPreserveGapsCombiningMarksAndFullCrossingTokenIdentity() {
        val page = page("😀 𠮷か\u3099\u0000猫", 4)
        val key =
            ReaderTokenKey(page.documentId, page.anchor, ReaderAnalysis.Available(AnalysisId(4)))
        val first =
            ReaderToken(
                7UL,
                anchor(9, 19),
                TextRange(TextOffset(5), TextOffset(15)),
                "𠮷か\u3099",
                "ヨシガ",
                "𠮷か\u3099",
                ReaderVocabulary(VocabularyId(5), VocabularyStatus.NEW),
            )
        val second =
            ReaderToken(
                8UL,
                anchor(20, 23),
                TextRange(TextOffset(16), TextOffset(19)),
                "猫",
                null,
                null,
                null,
            )
        val snapshot =
            ReaderTokenSnapshot(page.documentId, page.anchor, key.analysis, listOf(first, second))
        val spans = readerDisplayTokens(page, key, snapshot)!!
        assertEquals(listOf(3 to 7, 8 to 9), spans.map { it.start to it.end })
        assertEquals("𠮷か\u3099", page.text.substring(spans.first().start, spans.first().end))
        val taps = mutableListOf<ULong>()
        val text = readerAnnotatedText(page.text, spans, false, Color.Black) { taps.add(it) }
        assertEquals(page.text, text.text)
        val links = text.getLinkAnnotations(0, text.length)
        assertEquals(2, links.size)
        assertEquals(Color(0xFFA9CCFF), links.first().item.styles!!.style!!.background)
        assertEquals(Color.Unspecified, links.last().item.styles!!.style!!.background)
        links.first().item.linkInteractionListener!!.onClick(links.first().item)
        assertEquals(listOf(7UL), taps)

        // The same full token can straddle either side of a page boundary.
        val full = "𠮷野家"
        for ((visible, start, end) in listOf(Triple("𠮷", 10L, 14L), Triple("野家", 14L, 20L))) {
            val clippedPage = page(visible, start)
            val clippedKey = key.copy(page = clippedPage.anchor)
            val crossing =
                first.copy(
                    anchor = anchor(10, 20),
                    surface = full,
                    displayRange = TextRange(TextOffset(0), TextOffset(end - start)),
                )
            val clipped = snapshot.copy(page = clippedPage.anchor, tokens = listOf(crossing))
            val span = readerDisplayTokens(clippedPage, clippedKey, clipped)!!.single()
            assertEquals(full, span.token.surface)
            assertEquals(visible, visible.substring(span.start, span.end))
        }
    }

    @Test
    fun invalidOrForeignRangesNeverCreateLinksAndMissingAnalysisStaysEmpty() {
        val page = page("𠮷猫")
        val key =
            ReaderTokenKey(page.documentId, page.anchor, ReaderAnalysis.Available(AnalysisId(4)))
        val token =
            ReaderToken(
                0UL,
                page.anchor,
                TextRange(TextOffset(0), TextOffset(7)),
                page.text,
                null,
                null,
                null,
            )
        val snapshot =
            ReaderTokenSnapshot(page.documentId, page.anchor, key.analysis, listOf(token))
        assertNotNull(readerDisplayTokens(page, key, snapshot))
        for (range in
            listOf(
                -1L to 7L,
                1L to 7L,
                0L to 5L,
                0L to 8L,
                4L to 4L,
                Long.MAX_VALUE to Long.MAX_VALUE,
            )) {
            assertNull(
                readerDisplayTokens(
                    page,
                    key,
                    snapshot.copy(
                        tokens =
                            listOf(
                                token.copy(
                                    displayRange =
                                        TextRange(TextOffset(range.first), TextOffset(range.second))
                                )
                            )
                    ),
                )
            )
        }
        assertNull(readerDisplayTokens(page, key, snapshot.copy(documentId = DocumentId(99))))
        assertNull(
            readerDisplayTokens(
                page,
                key,
                snapshot.copy(analysis = ReaderAnalysis.Available(AnalysisId(9))),
            )
        )
        assertNull(readerDisplayTokens(page, key, snapshot.copy(tokens = listOf(token, token))))
        assertNull(
            readerDisplayTokens(
                page,
                key,
                snapshot.copy(tokens = listOf(token.copy(surface = "違う"))),
            )
        )
        assertNull(readerDisplayTokens(page.copy(text = "\uD800"), key, snapshot))
        assertEquals(
            emptyList<ReaderDisplayToken>(),
            readerDisplayTokens(page, key, snapshot.copy(tokens = emptyList())),
        )
        val missingKey = key.copy(analysis = ReaderAnalysis.Missing)
        assertEquals(
            emptyList<ReaderDisplayToken>(),
            readerDisplayTokens(
                page,
                missingKey,
                snapshot.copy(analysis = ReaderAnalysis.Missing, tokens = emptyList()),
            ),
        )
        assertNull(
            readerDisplayTokens(page, missingKey, snapshot.copy(analysis = ReaderAnalysis.Missing))
        )
    }

    @Test
    fun sourceStylesPreserveUnicodeAndWordIdentityAroundWebsiteTapAreas() {
        val page = page("😀𠮷猫犬")
        val token =
            ReaderToken(
                7UL,
                page.anchor,
                TextRange(TextOffset(0), TextOffset(14)),
                page.text,
                null,
                null,
                ReaderVocabulary(VocabularyId(5), VocabularyStatus.NEW),
            )
        val key =
            ReaderTokenKey(page.documentId, page.anchor, ReaderAnalysis.Available(AnalysisId(4)))
        val tokens =
            readerDisplayTokens(
                page,
                key,
                ReaderTokenSnapshot(page.documentId, page.anchor, key.analysis, listOf(token)),
            )!!
        fun span(start: Long, end: Long, style: ReaderTextStyle) =
            ReaderTextSpan(TextRange(TextOffset(start), TextOffset(end)), style)
        val formatting =
            listOf(
                span(0, 14, ReaderTextStyle.Heading(2u)),
                span(4, 11, ReaderTextStyle.Emphasis),
                span(4, 8, ReaderTextStyle.Strong),
                span(8, 11, ReaderTextStyle.ExternalLink("https://example.org/猫")),
                // Nested source links must never overlap the chosen website target.
                span(4, 11, ReaderTextStyle.ExternalLink("https://example.org/𠮷猫")),
            )
        val taps = mutableListOf<ULong>()
        val urls = mutableListOf<String>()
        val text =
            readerAnnotatedText(
                page.text,
                tokens,
                false,
                Color.Black,
                formatting,
                Color.Blue,
                { urls.add(it) },
            ) {
                taps.add(it)
            }
        assertEquals(page.text, text.text)
        assertTrue(text.spanStyles.any { it.item.fontWeight == FontWeight.Bold && it.start == 0 })
        assertTrue(
            text.spanStyles.any {
                it.item.fontStyle == FontStyle.Italic && it.start == 2 && it.end == 5
            }
        )
        assertTrue(
            text.spanStyles.any {
                it.item.background == Color(0xFFA9CCFF) && it.start == 0 && it.end == 6
            }
        )
        val links = text.getLinkAnnotations(0, text.length)
        assertEquals(listOf(4 to 5, 2 to 4, 0 to 2, 5 to 6), links.map { it.start to it.end })
        links.forEach { it.item.linkInteractionListener!!.onClick(it.item) }
        assertEquals(listOf("https://example.org/猫", "https://example.org/𠮷猫"), urls)
        assertEquals(listOf(7UL, 7UL), taps)
        assertTrue(links.take(2).all { it.item is LinkAnnotation.Url })
        assertTrue(links.drop(2).all { it.item is LinkAnnotation.Clickable })
    }

    @Test
    fun malformedSourceFormattingNeverCreatesWebsiteLinks() {
        val text = "😀猫"
        val badStyles =
            listOf(
                ReaderTextStyle.Heading(0u),
                ReaderTextStyle.Heading(7u),
                ReaderTextStyle.ExternalLink("javascript:alert(1)"),
                ReaderTextStyle.ExternalLink("file:///tmp/book"),
                ReaderTextStyle.ExternalLink("#note"),
                ReaderTextStyle.ExternalLink("https:///path"),
                ReaderTextStyle.ExternalLink("https://"),
                ReaderTextStyle.ExternalLink("https://example.org/\n"),
            )
        val whole = TextRange(TextOffset(0), TextOffset(7))
        for (style in badStyles) assertNull(
            readerDisplayFormatting(text, listOf(ReaderTextSpan(whole, style)))
        )
        for ((start, end) in listOf(-1L to 7L, 1L to 7L, 0L to 5L, 0L to 8L, 4L to 4L, 7L to 0L)) {
            val invalid =
                listOf(
                    ReaderTextSpan(
                        TextRange(TextOffset(start), TextOffset(end)),
                        ReaderTextStyle.ExternalLink("https://example.org"),
                    )
                )
            assertNull(readerDisplayFormatting(text, invalid))
            val rendered = readerAnnotatedText(text, emptyList(), false, Color.Black, invalid) {}
            assertEquals(text, rendered.text)
            assertTrue(rendered.getLinkAnnotations(0, text.length).isEmpty())
        }
        assertNull(readerDisplayFormatting("\uD800", emptyList()))
    }

    @Test
    fun statusBackgroundsFollowBothProductPalettesWithKnownAndAbsentUnhighlighted() {
        val light = listOf(0xFFA9CCFF, 0xFFFFC5C0, 0xFFFFE29A, null, 0xFFBEC5CF)
        val dark = listOf(0xFF28558D, 0xFF783E44, 0xFF725A2D, null, 0xFF505B6B)
        for ((index, status) in VocabularyStatus.entries.withIndex()) {
            assertEquals(
                light[index]?.let { Color(it) } ?: Color.Unspecified,
                vocabularyBackground(status, false),
            )
            assertEquals(
                dark[index]?.let { Color(it) } ?: Color.Unspecified,
                vocabularyBackground(status, true),
            )
        }
        assertEquals(Color.Unspecified, vocabularyBackground(null, false))
        assertEquals(Color.Unspecified, vocabularyBackground(null, true))
    }
}
