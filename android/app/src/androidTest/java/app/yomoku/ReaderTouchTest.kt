package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.input.InputModeManager
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.LocalInputModeManager
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.ViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.VocabularyStatus
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderTouchTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun physicalTapsSelectTopRightWrappedAndAdjacentSurfacesExactlyOnce() {
        val source = "あいうえおかきくけはやくつるりたっぷり。"
        val surfaces = listOf("あいうえおかきくけ", "はやく", "つるり", "たっぷり")
        val tokens = surfaces.mapIndexed { index, surface ->
            val start = source.indexOf(surface)
            ReaderAnnotatedToken(
                index.toULong(),
                VocabularyStatus.NEW,
                start,
                start + surface.length,
            )
        }
        val taps = mutableListOf<String>()
        val size = mutableStateOf(16)
        val width = mutableStateOf(160)
        val spacing = mutableStateOf(1.5f)
        compose.setContent {
            MaterialTheme {
                val original = LocalViewConfiguration.current
                CompositionLocalProvider(
                    LocalViewConfiguration provides
                        object : ViewConfiguration by original {
                            override val minimumTouchTargetSize = DpSize.Zero
                        }
                ) {
                    ReaderHighlightedText(
                        readerAnnotatedSpans(
                            source,
                            tokens,
                            false,
                            Color.Black,
                            emptyList(),
                            Color.Blue,
                            {},
                            { taps.add(surfaces[it.toInt()]) },
                        ),
                        readerTextStyle(null)
                            .copy(
                                fontSize = size.value.sp,
                                lineHeight = (size.value * spacing.value).sp,
                            ),
                        Modifier.width(width.value.dp).testTag("touch_text"),
                    )
                }
            }
        }
        var sawWrappedWord = false
        for ((fontSize, columnWidth) in listOf(16 to 160, 24 to 240, 16 to 176, 24 to 216)) {
            compose.runOnIdle {
                size.value = fontSize
                width.value = columnWidth
                spacing.value = if (columnWidth == 216) 2f else 1.5f
            }
            val node = compose.onNodeWithTag("touch_text")
            val layouts = mutableListOf<TextLayoutResult>()
            node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
            val layout = layouts.single()
            sawWrappedWord =
                sawWrappedWord ||
                    layout.getLineForOffset(tokens[1].start) !=
                        layout.getLineForOffset(tokens[1].end - 1)
            for (token in tokens.drop(1)) {
                for (offset in token.start until token.end) {
                    val box = layout.getBoundingBox(offset)
                    for (point in
                        listOf(
                            box.center,
                            Offset(box.left + 0.1f, box.center.y),
                            Offset(box.right - 0.1f, box.center.y),
                            Offset(box.center.x, box.top + 0.1f),
                            Offset(box.center.x, box.bottom - 0.1f),
                        )) {
                        val before = taps.size
                        node.performTouchInput { click(point) }
                        compose.runOnIdle {
                            assertEquals(
                                "$fontSize sp / $columnWidth dp / offset $offset / $point",
                                listOf(source.substring(token.start, token.end)),
                                taps.drop(before),
                            )
                        }
                    }
                }
            }
        }
        assertTrue("fixture must exercise both fragments of a wrapped word", sawWrappedWord)
    }

    private fun layout(node: SemanticsNodeInteraction): TextLayoutResult {
        val results = mutableListOf<TextLayoutResult>()
        node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(results) }
        return results.single()
    }

    @OptIn(ExperimentalTestApi::class)
    @Test
    fun unicodeStyledLinksRejectBlankSpaceAndKeepAccessibilityAndKeyboardActivation() {
        val source = "𠮷か\u3099😀  外部 site\n猫。"
        val taps = mutableListOf<String>()
        lateinit var inputModeManager: InputModeManager
        val text =
            AnnotatedString.Builder(
                    readerAnnotatedSpans(
                        source,
                        listOf(
                            ReaderAnnotatedToken(0UL, VocabularyStatus.KNOWN, 0, 6),
                            ReaderAnnotatedToken(
                                1UL,
                                VocabularyStatus.NEW,
                                source.indexOf("猫"),
                                source.indexOf("猫") + 1,
                            ),
                        ),
                        false,
                        Color.Black,
                        emptyList(),
                        Color.Blue,
                        {},
                        { taps.add(if (it == 0UL) "𠮷か\u3099😀" else "猫") },
                    )
                )
                .apply {
                    addStyle(SpanStyle(fontSize = 28.sp, fontWeight = FontWeight.Bold), 0, 6)
                    addLink(
                        LinkAnnotation.Url(
                            "https://example.org",
                            linkInteractionListener = { taps.add("website") },
                        ),
                        8,
                        source.indexOf('\n'),
                    )
                }
                .toAnnotatedString()
        compose.setContent {
            inputModeManager = LocalInputModeManager.current
            MaterialTheme {
                ReaderHighlightedText(
                    text,
                    readerTextStyle(null),
                    Modifier.width(280.dp).height(180.dp).testTag("touch_text"),
                )
            }
        }
        val node = compose.onNodeWithTag("touch_text")
        val result = layout(node)
        for (offset in listOf(0, 2, 4, 8, source.indexOf("猫"))) {
            val before = taps.size
            node.performTouchInput { click(result.getBoundingBox(offset).center) }
            assertEquals(
                listOf(if (offset < 6) "𠮷か\u3099😀" else if (offset == 8) "website" else "猫"),
                taps.drop(before),
            )
        }
        val before = taps.size
        for (point in
            listOf(
                result.getBoundingBox(6).center,
                result.getBoundingBox(source.indexOf("site") - 1).center,
                Offset(result.size.width - 2f, result.getBoundingBox(source.indexOf("猫")).center.y),
                Offset(10f, result.size.height - 2f),
            )) {
            node.performTouchInput { click(point) }
        }
        assertEquals(before, taps.size)
        // Invoke the generated link node's accessibility action directly.
        val links = compose.onAllNodes(hasClickAction(), useUnmergedTree = true)
        assertEquals(3, links.fetchSemanticsNodes().size)
        links[0].performSemanticsAction(SemanticsActions.OnClick) { it() }
        assertEquals("𠮷か\u3099😀", taps.last())
        links[2].performSemanticsAction(SemanticsActions.OnClick) { it() }
        assertEquals("website", taps.last())
        compose.runOnIdle { inputModeManager.requestInputMode(InputMode.Keyboard) }
        links[1].performSemanticsAction(SemanticsActions.RequestFocus) { it() }
        links[1].assertIsFocused()
        links[1].performKeyInput { pressKey(Key.Enter) }
        compose.waitForIdle()
        assertEquals(listOf("𠮷か\u3099😀", "website", "猫"), taps.drop(before))
    }

    @Test
    fun scrollingMovementMultiplePointersAndLayoutReplacementCancelTaps() {
        val taps = mutableListOf<ULong>()
        val width = mutableStateOf(240)
        val size = mutableStateOf(16)
        val source = "猫犬".repeat(120)
        val text =
            mutableStateOf(
                readerAnnotatedSpans(
                    source,
                    listOf(ReaderAnnotatedToken(7UL, VocabularyStatus.NEW, 0, source.length)),
                    false,
                    Color.Black,
                    emptyList(),
                    Color.Blue,
                    {},
                    taps::add,
                )
            )
        lateinit var scroll: androidx.compose.foundation.ScrollState
        compose.setContent {
            MaterialTheme {
                scroll = rememberScrollState()
                Column(
                    Modifier.height(240.dp)
                        .width(width.value.dp)
                        .verticalScroll(scroll)
                        .testTag("scroll")
                ) {
                    ReaderHighlightedText(
                        text.value,
                        readerTextStyle(null)
                            .copy(fontSize = size.value.sp, lineHeight = (size.value * 1.5f).sp),
                        Modifier.testTag("touch_text"),
                    )
                }
            }
        }
        val node = compose.onNodeWithTag("touch_text")
        val point = layout(node).getBoundingBox(0).center
        node.performTouchInput {
            down(point)
            moveTo(point + Offset(100f, 0f))
            moveTo(point)
            up()
        }
        node.performTouchInput {
            down(0, point)
            down(1, point + Offset(40f, 0f))
            up(1)
            up(0)
        }
        node.performTouchInput { down(point) }
        compose.runOnIdle { width.value = 200 }
        node.performTouchInput { up() }
        node.performTouchInput { down(point) }
        compose.runOnIdle { size.value = 24 }
        node.performTouchInput { up() }
        node.performTouchInput { down(point) }
        compose.runOnIdle {
            text.value =
                AnnotatedString.Builder(text.value).apply { append("猫") }.toAnnotatedString()
        }
        node.performTouchInput { up() }
        node.performTouchInput {
            down(point)
            cancel()
        }
        compose.onNodeWithTag("scroll").performTouchInput { swipeUp(durationMillis = 500) }
        compose.runOnIdle {
            assertTrue("drag must still scroll the reader", scroll.value > 0)
            assertTrue(taps.isEmpty())
        }
        compose.runOnIdle { width.value = 240 }
        // A fresh gesture after cancellation still selects exactly once.
        compose.onNodeWithTag("scroll").performTouchInput { click(center) }
        compose.runOnIdle { assertEquals(listOf(7UL), taps) }
    }
}
