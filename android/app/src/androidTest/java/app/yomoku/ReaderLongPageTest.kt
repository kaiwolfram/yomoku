package app.yomoku

import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderLongPageTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun visualBlocksKeepCanonicalUnicodeWrappingAndCrossingLinkIdentity() {
        verifyLayout("章\n" + "猫𠮷e\u0301😀 犬 ".repeat(240) + "\n終。", split = true)
    }

    @Test
    fun mixedDirectionParagraphsKeepTheirCompleteLayoutAndEmbeddingContext() {
        verifyLayout(
            "章\n" + "猫𠮷e\u0301😀 犬 ".repeat(120) + "\nאבג " + "犬𠮷😀 abc ".repeat(160) + "\n終。",
            split = false,
        )
    }

    private fun verifyLayout(source: String, split: Boolean) {
        val tapped = mutableListOf<ULong>()
        val text =
            readerAnnotatedSpans(
                source,
                // A single selectable surface crosses every rendering boundary.
                listOf(ReaderAnnotatedToken(7UL, VocabularyStatus.NEW, 2, source.length - 3)),
                false,
                Color.Black,
                emptyList(),
                Color.Blue,
                {},
            ) {
                tapped.add(it)
            }
        val anchor =
            TextAnchor(
                SourceId(1),
                ChapterId(2),
                TextRange(TextOffset(0), TextOffset(source.toByteArray().size.toLong())),
            )
        val page =
            ReaderPage(
                DocumentId(3),
                "Long",
                false,
                source,
                anchor,
                ReadingPosition(anchor.sourceId, anchor.chapterId, TextOffset(0)),
                1UL,
                1UL,
                emptyList(),
                emptyList(),
                emptyList(),
            )
        lateinit var measurer: TextMeasurer
        compose.runOnUiThread {
            compose.activity.setContent {
                YomokuTheme {
                    measurer = rememberTextMeasurer()
                    Column(Modifier.width(340.dp).verticalScroll(rememberScrollState())) {
                        ReaderImageContent(page, text, null, {})
                    }
                }
            }
        }
        compose.waitForIdle()
        val blocks =
            compose
                .onAllNodes(
                    SemanticsMatcher("reader text blocks") {
                        it.config
                            .getOrElse(SemanticsProperties.TestTag) { "" }
                            .let { tag ->
                                tag == "reader_text" || tag.startsWith("reader_text_block_")
                            }
                    }
                )
                .fetchSemanticsNodes()
                .map { node ->
                    val layouts = mutableListOf<TextLayoutResult>()
                    node.config[SemanticsActions.GetTextLayoutResult].action!!(layouts)
                    layouts.single()
                }
        if (split) assertTrue(blocks.size > 3) else assertEquals(1, blocks.size)
        assertEquals(source, blocks.joinToString("") { it.layoutInput.text.text })
        compose.runOnUiThread {
            val original =
                measurer.measure(
                    text,
                    blocks.first().layoutInput.style,
                    constraints = blocks.first().layoutInput.constraints,
                )
            var offset = 0
            var line = 0
            var top = 0f
            for (block in blocks) {
                for (local in 0 until block.lineCount) {
                    assertEquals(original.getLineStart(line), offset + block.getLineStart(local))
                    assertEquals(original.getLineEnd(line), offset + block.getLineEnd(local))
                    // A block's final line can include trailing-space advance that a wrapped
                    // line omits. Compare painted glyphs rather than that invisible advance.
                    val glyphs =
                        (block.getLineStart(local) until block.getLineEnd(local)).filter {
                            !block.layoutInput.text[it].isWhitespace() &&
                                !Character.isLowSurrogate(block.layoutInput.text[it])
                        }
                    for (glyph in listOfNotNull(glyphs.firstOrNull(), glyphs.lastOrNull())) {
                        val expected = original.getBoundingBox(offset + glyph)
                        val actual = block.getBoundingBox(glyph)
                        assertEquals(
                            "line $line glyph $glyph left",
                            expected.left,
                            actual.left,
                            0.1f,
                        )
                        assertEquals(
                            "line $line glyph $glyph right",
                            expected.right,
                            actual.right,
                            0.1f,
                        )
                    }
                    // Independent layout heights may round by one pixel per block.
                    assertEquals(
                        original.getLineBaseline(line),
                        top + block.getLineBaseline(local),
                        blocks.size.toFloat(),
                    )
                    line++
                }
                val annotated = block.layoutInput.text
                val link = annotated.getLinkAnnotations(0, annotated.length).single().item
                assertEquals(TextLinkStyles(), link.styles)
                link.linkInteractionListener!!.onClick(link)
                offset += annotated.length
                top += block.size.height
            }
            assertEquals(original.lineCount, line)
        }
        assertEquals(List(blocks.size) { 7UL }, tapped)
    }
}
