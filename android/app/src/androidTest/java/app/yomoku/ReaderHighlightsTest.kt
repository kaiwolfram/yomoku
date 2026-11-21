package app.yomoku

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderHighlightsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun insetPaintPreservesUnicodeWrappedTextAndAdjacentTapTargetsInBothThemes() {
        val night = mutableStateOf(false)
        val taps = mutableListOf<ULong>()
        val text = "猫犬𠮷か\u3099東京日本語東京日本語𠮷猫\n。"
        val markEnd = text.lastIndexOf("猫")
        val dotStart = text.lastIndexOf("𠮷")
        val tokens =
            listOf(
                ReaderAnnotatedToken(0UL, VocabularyStatus.NEW, 0, 1),
                ReaderAnnotatedToken(1UL, VocabularyStatus.LEARNING, 1, 2),
                ReaderAnnotatedToken(
                    2UL,
                    VocabularyStatus.FAMILIAR,
                    2,
                    markEnd,
                ),
                ReaderAnnotatedToken(3UL, VocabularyStatus.KNOWN, markEnd, markEnd + 1),
            )
        var inset = 0f
        compose.setContent {
            inset = with(LocalDensity.current) { 1.dp.toPx() }
            val configuration =
                Configuration(LocalConfiguration.current).apply {
                    uiMode =
                        (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                            (if (night.value) Configuration.UI_MODE_NIGHT_YES
                            else Configuration.UI_MODE_NIGHT_NO)
                }
            CompositionLocalProvider(LocalConfiguration provides configuration) {
                MaterialTheme(
                    colorScheme = if (night.value) darkColorScheme() else lightColorScheme()
                ) {
                    val annotated =
                        readerAnnotatedSpans(
                            text,
                            tokens,
                            night.value,
                            if (night.value) Color.White else Color.Black,
                            emptyList(),
                            Color.Blue,
                            {},
                            marker = ReaderMarker(2 until markEnd, endsOnPage = true),
                            onToken = taps::add,
                        )
                    Box(
                        Modifier.testTag("text_with_margin")
                            .background(if (night.value) Color.Black else Color.White)
                            .padding(horizontal = 16.dp)
                    ) {
                        ReaderHighlightedText(
                            annotated,
                            readerTextStyle(null)
                                .copy(color = if (night.value) Color.White else Color.Black),
                            Modifier.width(140.dp).testTag("highlight_text"),
                        )
                    }
                }
            }
        }
        for (dark in listOf(false, true)) {
            compose.runOnUiThread {
                night.value = dark
            }
            val node = compose.onNodeWithTag("highlight_text")
            val layouts = mutableListOf<TextLayoutResult>()
            node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
            val layout = layouts.single()
            assertEquals(text, layout.layoutInput.text.text)
            val first = readerTextFragmentRects(layout, 0, 1, inset).single()
            val second = readerTextFragmentRects(layout, 1, 2, inset).single()
            assertEquals(inset * 2, second.left - first.right, 0.6f)
            val wrapped = readerTextFragmentRects(layout, 2, markEnd, inset)
            assertTrue(wrapped.size > 1)
            val pixels = node.captureToImage().toPixelMap()
            for ((index, status) in
                listOf(
                        VocabularyStatus.NEW,
                        VocabularyStatus.LEARNING,
                        VocabularyStatus.FAMILIAR,
                    )
                    .withIndex()) {
                val token = tokens[index]
                val color = vocabularyBackground(status, dark)
                assertTrue(
                    (0 until pixels.height).any { y ->
                        (0 until pixels.width).any { x -> pixels[x, y] == color }
                    }
                )
                for (rect in readerTextFragmentRects(layout, token.start, token.end, inset)) {
                    val x = (rect.left - inset / 2).toInt().coerceIn(0, pixels.width - 1)
                    assertTrue(
                        (rect.top.toInt().coerceAtLeast(0) until
                                rect.bottom.toInt().coerceAtMost(pixels.height))
                            .none { y -> pixels[x, y] == color }
                    )
                }
            }
            val foreground = if (dark) Color.White else Color.Black
            val expectedMarker =
                foreground
                    .copy(alpha = 0.7f)
                    .compositeOver(vocabularyBackground(VocabularyStatus.FAMILIAR, dark))
            for (rect in readerTextFragmentRects(layout, 2, markEnd)) {
                val y = (rect.bottom - inset / 2).toInt().coerceIn(0, pixels.height - 1)
                assertTrue(
                    "Marker must underline every wrapped fragment",
                    (rect.left.toInt() + inset.toInt() until rect.right.toInt() - inset.toInt())
                        .any { x ->
                            val pixel = pixels[x, y]
                            kotlin.math.abs(pixel.red - expectedMarker.red) < 0.01f &&
                                kotlin.math.abs(pixel.green - expectedMarker.green) < 0.01f &&
                                kotlin.math.abs(pixel.blue - expectedMarker.blue) < 0.01f
                        },
                )
            }
            val withMargin = compose.onNodeWithTag("text_with_margin")
            val marginPixels = withMargin.captureToImage().toPixelMap()
            val textOffset =
                node.fetchSemanticsNode().positionInRoot -
                    withMargin.fetchSemanticsNode().positionInRoot
            val background = if (dark) Color.Black else Color.White
            val expectedDot = foreground.copy(alpha = 0.7f).compositeOver(background)
            val finalMarkedLine = layout.getLineForOffset(dotStart)
            for (line in 0 until layout.lineCount) {
                val x = (textOffset.x + layout.getLineLeft(line) - 6 * inset).toInt()
                val y =
                    (textOffset.y + (layout.getLineTop(line) + layout.getLineBottom(line)) / 2)
                        .toInt()
                val actual = marginPixels[x, y]
                assertEquals(
                    "Margin dot on line $line in dark mode $dark",
                    if (line == finalMarkedLine) expectedDot.red else background.red,
                    actual.red,
                    0.01f,
                )
            }
            node.performTouchInput { click(layout.getBoundingBox(0).center) }
            assertEquals(0UL, taps.last())
            node.performTouchInput { click(layout.getBoundingBox(1).center) }
            assertEquals(1UL, taps.last())
            assertTrue(
                layout.layoutInput.text
                    .getStringAnnotations(READER_HIGHLIGHT, markEnd, markEnd + 1)
                    .isEmpty()
            )
            // The same annotations are retained and clipped by both image and block splitting.
            val fragment = layout.layoutInput.text.subSequence(4, dotStart)
            assertEquals(
                0,
                fragment.getStringAnnotations(READER_HIGHLIGHT, 0, fragment.length).single().start,
            )
            assertEquals(
                0,
                fragment.getStringAnnotations(READER_MARKER, 0, fragment.length).single().start,
            )
            assertEquals(
                fragment.length,
                fragment.getStringAnnotations(READER_MARKER, 0, fragment.length).single().end,
            )
            assertTrue(
                fragment.getStringAnnotations(READER_MARKER_DOT, 0, fragment.length).isEmpty()
            )
        }
    }
}
