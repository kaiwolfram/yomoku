package app.yomoku

import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.width
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.*
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderRubyLayoutTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun byteRangesRejectMalformedProjectionAndClipWithoutChangingLiteralReadings() {
        val text = "𠮷猫e\u0301\u0000犬"
        fun ruby(start: Long, end: Long, reading: String = "よしねこ") =
            ReaderRuby(TextRange(TextOffset(start), TextOffset(end)), reading)
        val display = readerDisplayRuby(text, listOf(ruby(0, 7), ruby(11, 14, "イヌ")))!!
        assertEquals(ReaderDisplayRuby(0, 3, "よしねこ"), display.first())
        assertEquals(ReaderDisplayRuby(0, 1, "よしねこ"), display.first().clip(2, 3))
        assertNull(display.first().clip(3, text.length))
        for (invalid in
            listOf(
                listOf(ruby(1, 7)),
                listOf(ruby(0, 2)),
                listOf(ruby(0, 7), ruby(4, 7)),
                listOf(ruby(7, 4)),
                listOf(ruby(0, 7, "")),
                listOf(ruby(0, 7, "\uD800")),
            )) {
            assertNull(readerDisplayRuby(text, invalid))
        }
        assertNull(readerDisplayRuby("\uD800", emptyList()))
    }

    @Test
    fun wrappedHeadingReadingsPaintAboveCanonicalTextAtBothFontScaleAndThemeExtremes() {
        val dark = mutableStateOf(false)
        val large = mutableStateOf(false)
        lateinit var measurer: TextMeasurer
        lateinit var readingStyle: TextStyle
        val text = buildAnnotatedString {
            append("𠮷猫漢字東京日本語漢字猫犬漢字東京日本語\n猫")
            addStyle(SpanStyle(fontWeight = FontWeight.Bold, fontSize = 1.625.em), 0, 7)
        }
        val ruby =
            listOf(
                ReaderDisplayRuby(0, text.length - 2, "よしねこかんじとうきょうにほんご"),
                ReaderDisplayRuby(text.length - 1, text.length, "ネコ"),
            )
        compose.runOnUiThread {
            compose.activity.setContent {
                val original = LocalDensity.current
                androidx.compose.runtime.CompositionLocalProvider(
                    LocalDensity provides Density(original.density, if (large.value) 1.6f else 1f)
                ) {
                    MaterialTheme(
                        colorScheme = if (dark.value) darkColorScheme() else lightColorScheme()
                    ) {
                        measurer = rememberTextMeasurer()
                        val style =
                            readerTextStyle(
                                ReaderTypography(
                                    if (large.value) ReaderFontSize.SP24 else ReaderFontSize.SP14,
                                    ReaderLineSpacing.PERCENT125,
                                    true,
                                )
                            )
                        readingStyle =
                            style.copy(
                                fontSize = style.fontSize * 0.5f,
                                lineHeight = TextUnit.Unspecified,
                                lineHeightStyle = null,
                                platformStyle = PlatformTextStyle(includeFontPadding = false),
                            )
                        androidx.compose.runtime.CompositionLocalProvider(
                            LocalContentColor provides MaterialTheme.colorScheme.onSurface
                        ) {
                            ReaderRubyText(
                                text,
                                style,
                                ruby,
                                Modifier.width(180.dp)
                                    .background(if (dark.value) Color.Black else Color.White)
                                    .testTag("ruby"),
                            )
                        }
                    }
                }
            }
        }
        for (bigger in listOf(false, true)) {
            for (night in listOf(false, true)) {
                compose.runOnIdle {
                    large.value = bigger
                    dark.value = night
                }
                val node = compose.onNodeWithTag("ruby")
                node.assertTextEquals(text.text)
                val layouts = mutableListOf<TextLayoutResult>()
                node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action!!(
                    layouts
                )
                val layout = layouts.single()
                assertEquals(text, layout.layoutInput.text)
                assertTrue(layout.lineCount >= 3)
                assertFalse(layout.didOverflowWidth)
                val readings = ruby.associate {
                    it.reading to measurer.measure(it.reading, readingStyle, softWrap = false)
                }
                val placements = readerRubyPlacements(layout, ruby, readings, measurer)
                assertEquals(layout.lineCount, placements.size)
                val pixels = node.captureToImage().toPixelMap()
                for ((reading, bounds) in placements) {
                    assertTrue("top $bounds", bounds.top >= -1f)
                    assertTrue(bounds.left >= -1f)
                    assertTrue(bounds.right <= layout.size.width + 1f)
                    val line = placements.indexOfFirst {
                        it.reading == reading && it.bounds == bounds
                    }
                    assertTrue(
                        "reading intersects previous line",
                        bounds.top >= layout.getLineTop(line) - 1f,
                    )
                    assertTrue(bounds.bottom < layout.getLineBaseline(line))
                    val background = if (night) Color.Black else Color.White
                    var ink = 0
                    for (y in
                        bounds.top.toInt().coerceAtLeast(0) until
                            bounds.bottom.toInt().coerceAtMost(pixels.height)) {
                        for (x in
                            bounds.left.toInt().coerceAtLeast(0) until
                                bounds.right.toInt().coerceAtMost(pixels.width)) {
                            if (pixels[x, y] != background) ink++
                        }
                    }
                    assertTrue(
                        "reading has no visible pixels: ${reading.layoutInput.text}",
                        ink > 2,
                    )
                }
            }
        }
    }
}
