package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.TextLayoutResult
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderFuriganaTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitSaved() {
        compose.waitUntil(60_000) {
            val view = core().view
            !view.storageBusy && !view.completionSavePending
        }
        compose.waitForIdle()
    }

    private fun awaitReader() {
        compose.waitUntil(90_000) {
            core().view.effectiveReaderTokens != null && !core().view.storageBusy
        }
        awaitSaved()
    }

    private fun settings() {
        compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
        awaitSaved()
    }

    private fun closeSettings() {
        compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
        awaitSaved()
    }

    private fun restore(typography: ReaderTypography, length: PageLength) {
        settings()
        compose.runOnUiThread { core().dispatch(Event.SetReaderTypography(typography)) }
        awaitSaved()
        compose.runOnUiThread { core().dispatch(Event.SetPageLength(length)) }
        awaitSaved()
        closeSettings()
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

    private fun tap(tag: String, word: String) {
        val node = compose.onNodeWithTag(tag).performScrollTo()
        val layout = layout(tag)
        node.performTouchInput {
            click(layout.getBoundingBox(layout.layoutInput.text.text.indexOf(word)).center)
        }
        compose.waitForIdle()
    }

    @Test
    fun visibleSwitchRetriesCommitsAndKeepsEpubWordLinkImageAndPositionIdentities() {
        awaitSaved()
        val original = core().view.readerTypography!!
        val length = core().view.pageLength!!
        try {
            restore(original.copy(furigana = false), length)
            val title = "furigana-${System.nanoTime()}"
            val bytes =
                FileImportFixtures.epub(
                    title,
                    """<h1><ruby>章<rt>しょう</rt></ruby></h1>
                <p><strong><ruby>𠮷猫<rt>よしねこ</rt></ruby></strong> 猫。
                <a href="https://example.org/ruby"><ruby>外部<rt>がいぶ</rt></ruby></a>
                <img src="image.svg" alt="挿絵"/><ruby>犬<rt>いぬ</rt></ruby>。</p>""",
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
            val tokens = core().view.effectiveReaderTokens
            assertTrue(core().view.readerFurigana.isEmpty())
            val plainHeight = layout("reader_text_0").size.height
            settings()
            val switch = compose.onNodeWithTag("reader_furigana").performScrollTo().assertIsOff()
            SQLiteDatabase.openDatabase(
                    File(compose.activity.filesDir, "yomoku.sqlite3").path,
                    null,
                    SQLiteDatabase.OPEN_READWRITE,
                )
                .use { db ->
                    try {
                        db.execSQL(
                            "CREATE TRIGGER fail_ruby_ui BEFORE UPDATE ON reader_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END"
                        )
                        switch.performClick()
                        compose.waitUntil(30_000) {
                            !core().view.storageBusy && core().view.documentError != null
                        }
                        switch.assertIsOff()
                        assertFalse(core().view.readerTypography!!.furigana)
                    } finally {
                        db.execSQL("DROP TRIGGER IF EXISTS fail_ruby_ui")
                    }
                }
            switch.performClick()
            awaitSaved()
            switch.assertIsOn()
            compose.activityRule.scenario.recreate()
            compose.onNodeWithTag("reader_furigana").performScrollTo().assertIsOn()
            closeSettings()
            awaitReader()
            assertEquals(page, core().view.reader)
            assertEquals(tokens, core().view.effectiveReaderTokens)
            val readings = core().view.readerFurigana
            assertTrue(readings.containsAll(page.sourceRuby))
            assertTrue(readings.any { it.reading == "ネコ" })
            assertTrue(layout("reader_text_0").size.height > plainHeight)
            assertEquals(
                page.text.substringBefore('\uFFFC'),
                layout("reader_text_0").layoutInput.text.text,
            )
            tap("reader_text_0", "猫。")
            compose.onNodeWithTag("word_surface").assertTextEquals("猫")
            val selection = core().view.effectiveReaderWord?.selection!!
            for (status in VocabularyStatus.entries) {
                compose.onNodeWithTag("word_status_$status").performClick()
                awaitSaved()
                assertEquals(selection, core().view.effectiveReaderWord?.selection)
                assertEquals(readings, core().view.readerFurigana)
            }
            compose.onNodeWithText("Close").performClick()
            val instrumentation = InstrumentationRegistry.getInstrumentation()
            val monitor =
                instrumentation.addMonitor(
                    IntentFilter(Intent.ACTION_VIEW).apply {
                        addCategory(Intent.CATEGORY_BROWSABLE)
                        addDataScheme("https")
                    },
                    ActivityResult(Activity.RESULT_OK, null),
                    true,
                )
            try {
                tap("reader_text_0", "外部")
                assertEquals(1, monitor.hits)
                assertNull(core().view.effectiveReaderWord?.selection)
            } finally {
                instrumentation.removeMonitor(monitor)
            }
            compose.waitUntil(30_000) {
                compose.onAllNodesWithTag("reader_image_0").fetchSemanticsNodes().size == 1
            }
            compose.onNodeWithTag("reader_image_0").performScrollTo().performClick()
            compose.onNodeWithText("Close").performClick()
            val after = page.text.indexOf('\uFFFC') + 1
            tap("reader_text_$after", "犬")
            compose.onNodeWithTag("word_surface").assertTextEquals("犬")
            compose.onNodeWithText("Close").performClick()
            compose.activityRule.scenario.recreate()
            awaitReader()
            assertEquals(page, core().view.reader)
            assertEquals(readings, core().view.readerFurigana)
            settings()
            compose.onNodeWithTag("reader_furigana").performScrollTo().assertIsOn().performClick()
            awaitSaved()
            closeSettings()
            awaitReader()
            assertTrue(core().view.readerFurigana.isEmpty())
            assertEquals(plainHeight, layout("reader_text_0").size.height)
            assertEquals(page, core().view.reader)
        } finally {
            restore(original, length)
        }
    }

    @Test
    fun txtCrossingSourceBaseKeepsCompleteReadingThroughLogicalReflowAndMovement() {
        awaitSaved()
        val original = core().view.readerTypography!!
        val length = core().view.pageLength!!
        try {
            restore(original.copy(furigana = true), PageLength.CHARACTERS250)
            val title = "ruby-crossing-${System.nanoTime()}.txt"
            val reading = "ねこたち"
            val source = "｜${"猫。".repeat(300)}《$reading》\n犬。"
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.ImportTxt(
                            TxtImport(
                                source.toByteArray().map { it.toUByte() },
                                title,
                                TextEncoding.UTF8,
                            )
                        )
                    )
            }
            compose.waitUntil(90_000) { !core().view.storageBusy }
            val id = core().view.importSummary!!.documentId
            compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
            awaitReader()
            val first = core().view.reader!!
            assertEquals("猫。".repeat(125), first.text)
            assertEquals(reading, core().view.readerFurigana.single().reading)
            assertEquals(first.text, layout("reader_text").layoutInput.text.text)
            compose.onNodeWithTag("next_page").performScrollTo().performClick()
            compose.waitUntil(30_000) { core().view.reader?.pageNumber == 2UL }
            awaitReader()
            val second = core().view.reader!!
            assertEquals(reading, core().view.readerFurigana.first().reading)
            settings()
            compose.runOnUiThread {
                core().dispatch(Event.SetPageLength(PageLength.CHARACTERS1500))
            }
            awaitSaved()
            closeSettings()
            awaitReader()
            assertEquals(second.position, core().view.reader!!.position)
            assertEquals(
                source.substringAfter('｜').substringBefore('《') + "\n犬。",
                core().view.reader!!.text,
            )
            assertEquals(reading, core().view.readerFurigana.first().reading)
            assertFalse(layout("reader_text").didOverflowWidth)
        } finally {
            restore(original, length)
        }
    }
}
