package app.yomoku

import android.graphics.Paint
import android.graphics.Path
import android.graphics.Typeface
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontSynthesis
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.font.createFontFamilyResolver
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.Event
import java.io.File
import java.security.MessageDigest
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderTypographyTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    @Test
    fun bundledBytesAndResolvedRegularBoldOutlinesMatchPinnedFont() {
        val context = compose.activity
        val bytes = context.resources.openRawResource(R.font.noto_serif_jp).use { it.readBytes() }
        assertEquals(
            "2fd527ba12b6a44ec30d796d633360da0aeba6c5d4af1304ce12bb4dc15a7dfc",
            MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") {
                "%02x".format(it)
            },
        )
        val resolver = createFontFamilyResolver(context)
        fun outline(typeface: Typeface): FloatArray {
            val paint =
                Paint().apply {
                    this.typeface = typeface
                    textSize = 48f
                }
            val text = "日本語かなカナLatin𠮷"
            val path = Path()
            paint.getTextPath(text, 0, text.length, 0f, 0f, path)
            assertFalse(path.isEmpty)
            return path.approximate(0.05f)
        }
        // AssetManager's public font builder cannot open a compressed res/font entry.
        // Use the already hash-verified bytes with the platform's file builder as oracle.
        val fontFile = File.createTempFile("reader-font-", ".ttf", context.cacheDir)
        try {
            fontFile.writeBytes(bytes)
            val outlines =
                listOf(FontWeight.Normal, FontWeight.Bold).map { weight ->
                    val resolved =
                        resolver
                            .resolve(readerFontFamily, weight, FontStyle.Normal, FontSynthesis.None)
                            .value as Typeface
                    val expected =
                        Typeface.Builder(fontFile)
                            .setWeight(weight.weight)
                            .setFontVariationSettings("'wght' ${weight.weight}")
                            .build()
                    // Resource loading may report the variable file's default weight (200).
                    // Compare actual outlines with the explicitly instantiated axis instead.
                    val upright = outline(resolved)
                    assertArrayEquals(outline(expected), upright, 0.0001f)
                    val italic =
                        resolver
                            .resolve(readerFontFamily, weight, FontStyle.Italic, FontSynthesis.All)
                            .value as Typeface
                    assertTrue(italic.isItalic)
                    assertFalse(upright.contentEquals(outline(italic)))
                    upright
                }
            assertFalse(outlines[0].contentEquals(outlines[1]))
            assertFalse(outlines[0].contentEquals(outline(Typeface.DEFAULT)))
        } finally {
            assertTrue(fontFile.delete())
        }
    }

    @Test
    fun pastedUnicodeReaderUsesBundledFontAcrossRecreationWithoutMovingPosition() {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "reader-font-${System.nanoTime()}"
        val text = "$title\n猫と犬。かなカナ Latin 😀𠮷。\n次の段落。"
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
        compose.waitUntil(90_000) { !core().view.storageBusy }
        val id = core().view.importSummary!!.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        fun checkReaderFont() {
            compose.waitUntil(30_000) {
                !core().view.storageBusy && core().view.readerTokens != null
            }
            val layouts = mutableListOf<TextLayoutResult>()
            compose
                .onNodeWithTag("reader_text")
                .fetchSemanticsNode()
                .config[SemanticsActions.GetTextLayoutResult]
                .action!!(layouts)
            assertEquals(text, layouts.single().layoutInput.text.text)
            assertEquals(readerFontFamily, layouts.single().layoutInput.style.fontFamily)
            assertFalse(layouts.single().didOverflowWidth)
        }
        checkReaderFont()
        val page = core().view.reader!!
        compose.activityRule.scenario.recreate()
        checkReaderFont()
        assertEquals(page, core().view.reader)
    }

    @Test
    fun completeFontLicenseIsAvailableOfflineAcrossRecreation() {
        val expected =
            loadOfflineNotices(
                compose.activity.assets,
                "noto-serif-jp",
                listOf("ATTRIBUTION.txt", "OFL.txt"),
            )
        assertTrue(expected.contains("(c) 2017-2024 Adobe"))
        assertTrue(expected.contains("Copyright 2012 Google Inc."))
        assertTrue(expected.contains("SIL OPEN FONT LICENSE Version 1.1"))
        assertTrue(expected.endsWith("OTHER DEALINGS IN THE FONT SOFTWARE.\n"))
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Reader font credits").performScrollTo().performClick()
        fun checkNotices() {
            compose.waitUntil(10_000) {
                compose.onAllNodes(hasText(expected)).fetchSemanticsNodes().size == 1
            }
            compose.onNodeWithTag("reader_font_notices_text").assertTextEquals(expected)
        }
        checkNotices()
        compose.activityRule.scenario.recreate()
        checkNotices()
        compose.onNodeWithText("Close").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
    }
}
