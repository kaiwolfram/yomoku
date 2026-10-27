package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderTypographySettingsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitSaved() {
        compose.waitUntil(30_000) {
            !core().view.storageBusy &&
                !core().view.readingTimeSavePending &&
                !core().view.completionSavePending
        }
        compose.waitForIdle()
    }

    private fun awaitReader() {
        compose.waitUntil(60_000) {
            !core().view.storageBusy &&
                core().view.readerTime != null &&
                core().view.readerTokens != null
        }
        compose.waitForIdle()
    }

    private fun setTypography(value: ReaderTypography) {
        compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
        awaitSaved()
        compose.runOnUiThread { core().dispatch(Event.SetReaderTypography(value)) }
        awaitSaved()
        assertEquals(value, core().view.readerTypography)
    }

    private fun closeSettings() {
        compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
        awaitSaved()
    }

    private fun layout(tag: String): TextLayoutResult {
        val result = mutableListOf<TextLayoutResult>()
        compose
            .onNodeWithTag(tag)
            .fetchSemanticsNode()
            .config[SemanticsActions.GetTextLayoutResult]
            .action!!(result)
        return result.single()
    }

    @Test
    fun controlsCommitRetryPreserveExactOffsetAndPersistAcrossRecreationAndDocuments() {
        awaitSaved()
        val original = core().view.readerTypography!!
        try {
            setTypography(ReaderTypography(ReaderFontSize.SP16, ReaderLineSpacing.PERCENT150, true))
            closeSettings()
            val text = "typography-${System.nanoTime()}\n" + "𠮷猫と犬。かな Latin 😀。".repeat(150)
            compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
            compose.waitUntil(90_000) { !core().view.storageBusy }
            val id = core().view.importSummary!!.documentId
            compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
            awaitReader()
            val position =
                core()
                    .view
                    .reader!!
                    .position
                    .copy(offset = TextOffset(text.indexOf('\n').toLong() + 1L + 4L))
            compose.runOnUiThread { core().dispatch(Event.SaveReadingPosition(position)) }
            awaitReader()
            val page = core().view.reader!!
            val oldLines = layout("reader_text").lineCount
            val tokens = core().view.readerTokens
            compose.onNodeWithText("Settings").performClick()
            awaitSaved()
            val database = File(compose.activity.filesDir, "yomoku.sqlite3")
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use { db
                ->
                try {
                    db.execSQL(
                        "CREATE TRIGGER fail_typography_ui BEFORE UPDATE ON reader_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END"
                    )
                    compose
                        .onNodeWithTag("reader_font_size_increase")
                        .performScrollTo()
                        .performClick()
                    compose.waitUntil(30_000) {
                        !core().view.storageBusy && core().view.documentError != null
                    }
                    assertEquals(ReaderFontSize.SP16, core().view.readerTypography!!.fontSize)
                    compose.onNodeWithTag("reader_font_size").assertTextEquals("Font size: 16")
                } finally {
                    db.execSQL("DROP TRIGGER IF EXISTS fail_typography_ui")
                }
            }
            compose.onNodeWithTag("reader_font_size_increase").performClick()
            awaitSaved()
            compose.onNodeWithTag("reader_font_size_increase").performClick()
            awaitSaved()
            compose.onNodeWithTag("reader_line_spacing_increase").performScrollTo().performClick()
            awaitSaved()
            val expected = ReaderTypography(ReaderFontSize.SP20, ReaderLineSpacing.PERCENT175, true)
            assertEquals(expected, core().view.readerTypography)
            assertNull(core().view.documentError)
            compose.activityRule.scenario.recreate()
            compose.onNodeWithTag("reader_font_size").assertTextEquals("Font size: 20")
            compose.onNodeWithTag("reader_line_spacing").assertTextEquals("Line spacing: 175%")
            closeSettings()
            awaitReader()
            assertEquals(page, core().view.reader)
            assertEquals(tokens, core().view.readerTokens)
            val layout = layout("reader_text")
            assertEquals(20.sp, layout.layoutInput.style.fontSize)
            assertEquals(35.sp, layout.layoutInput.style.lineHeight)
            assertEquals(page.text, layout.layoutInput.text.text)
            assertTrue(layout.lineCount > oldLines)
            assertFalse(layout.didOverflowWidth)
            compose.activityRule.scenario.recreate()
            awaitReader()
            assertEquals(page, core().view.reader)
            assertEquals(20.sp, layout("reader_text").layoutInput.style.fontSize)
            // A new document gets the same global setting without changing this book's offset.
            compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
            awaitSaved()
            compose.runOnUiThread {
                core().dispatch(Event.CreatePaste("global-${System.nanoTime()}\n猫。"))
            }
            compose.waitUntil(90_000) { !core().view.storageBusy }
            val second = core().view.importSummary!!.documentId
            compose.runOnUiThread { core().dispatch(Event.OpenReader(second)) }
            awaitReader()
            assertEquals(20.sp, layout("reader_text").layoutInput.style.fontSize)
            compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
            awaitSaved()
            compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
            awaitReader()
            assertEquals(position, core().view.reader!!.position)
        } finally {
            setTypography(original)
            closeSettings()
        }
    }

    @Test
    fun extremeSizesKeepImageRatioCanonicalFragmentsAndWordTapIdentity() {
        awaitSaved()
        val original = core().view.readerTypography!!
        try {
            val title = "typography-image-${System.nanoTime()}"
            val bytes =
                FileImportFixtures.epub(
                    title,
                    "<p><strong>𠮷猫</strong>前<img src='image.svg' alt='挿絵'/>後 猫と犬。</p>",
                    images =
                        mapOf(
                            "image.svg" to
                                ("image/svg+xml" to
                                    "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='100'><rect width='400' height='100' fill='blue'/></svg>"
                                        .toByteArray())
                        ),
                )
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub"))
                    )
            }
            compose.waitUntil(90_000) { !core().view.storageBusy }
            val id = core().view.importSummary!!.documentId
            compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
            awaitReader()
            val page = core().view.reader!!
            for (value in
                listOf(
                    ReaderTypography(ReaderFontSize.SP24, ReaderLineSpacing.PERCENT200, false),
                    ReaderTypography(ReaderFontSize.SP14, ReaderLineSpacing.PERCENT125, false),
                )) {
                setTypography(value)
                compose
                    .onNodeWithTag(
                        "reader_font_size_" +
                            if (value.fontSize == ReaderFontSize.SP24) "increase" else "decrease"
                    )
                    .assertIsNotEnabled()
                compose
                    .onNodeWithTag(
                        "reader_line_spacing_" +
                            if (value.lineSpacing == ReaderLineSpacing.PERCENT200) "increase"
                            else "decrease"
                    )
                    .assertIsNotEnabled()
                closeSettings()
                awaitReader()
                compose.waitUntil(30_000) {
                    compose.onAllNodesWithTag("reader_image_0").fetchSemanticsNodes().size == 1
                }
                assertEquals(page, core().view.reader)
                val fragments =
                    compose
                        .onAllNodes(
                            SemanticsMatcher("text fragments") {
                                it.config
                                    .getOrElse(SemanticsProperties.TestTag) { "" }
                                    .startsWith("reader_text_")
                            }
                        )
                        .fetchSemanticsNodes()
                assertEquals(
                    page.text.replace("\uFFFC", ""),
                    fragments.joinToString("") {
                        it.config[SemanticsProperties.Text].single().text
                    },
                )
                for (node in fragments) {
                    val tag = node.config[SemanticsProperties.TestTag]
                    val style = layout(tag).layoutInput.style
                    assertEquals(value.fontSize.spValue.sp, style.fontSize)
                    assertEquals(
                        (value.fontSize.spValue * value.lineSpacing.percent / 100f).sp,
                        style.lineHeight,
                    )
                }
                val image = compose.onNodeWithTag("reader_image_0").performScrollTo()
                val bounds = image.getUnclippedBoundsInRoot()
                assertEquals(
                    4f,
                    (bounds.right - bounds.left).value / (bounds.bottom - bounds.top).value,
                    0.03f,
                )
                val tag = fragments.last().config[SemanticsProperties.TestTag]
                val node = compose.onNodeWithTag(tag).performScrollTo()
                val layout = layout(tag)
                val index = layout.layoutInput.text.text.indexOf("猫")
                node.performTouchInput { click(layout.getBoundingBox(index).center) }
                compose.waitUntil(30_000) {
                    core().view.readerWordSelection != null && !core().view.storageBusy
                }
                compose.onNodeWithTag("reader_word_panel").assertExists()
                assertEquals(
                    "猫",
                    core()
                        .view
                        .readerTokens!!
                        .tokens
                        .single { it.tokenIndex == core().view.readerWordSelection!!.tokenIndex }
                        .surface,
                )
                compose.onNodeWithText("Close").performClick()
                assertEquals(page, core().view.reader)
            }
        } finally {
            setTypography(original)
            closeSettings()
        }
    }
}
