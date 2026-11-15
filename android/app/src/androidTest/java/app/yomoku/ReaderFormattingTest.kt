package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderFormattingTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitReader() {
        compose.waitUntil(60_000) {
            val view = core().view
            !view.storageBusy && !view.completionSavePending && view.effectiveReaderTokens != null
        }
        compose.waitForIdle()
    }

    private fun annotated() =
        compose
            .onNodeWithTag("reader_text")
            .fetchSemanticsNode()
            .config[SemanticsProperties.Text]
            .single()

    private fun tap(text: String) {
        val node = compose.onNodeWithTag("reader_text").performScrollTo()
        val layouts = mutableListOf<androidx.compose.ui.text.TextLayoutResult>()
        node.fetchSemanticsNode().config[SemanticsActions.GetTextLayoutResult].action!!(layouts)
        assertEquals(readerFontFamily, layouts.single().layoutInput.style.fontFamily)
        val box = layouts.single().getBoundingBox(annotated().text.indexOf(text))
        node.performTouchInput { click(box.center) }
        compose.waitForIdle()
    }

    @Test
    fun epubStylesLinksWordTapsAndRecreationKeepCanonicalPosition() {
        compose.waitUntil(30_000) { !core().view.storageBusy }
        val title = "formatting-${System.nanoTime()}"
        val bytes =
            FileImportFixtures.epub(
                title,
                """<h1>章 <em>見出し</em></h1><p><strong>𠮷😀</strong> 猫。</p>
            <p><a href="https://example.org/book?q=1&amp;x=2">外部 site</a>
            <a href="two.xhtml#note">note</a> <a href="javascript:alert(1)">script</a></p>""",
            )
        compose.runOnUiThread {
            core().dispatch(Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, "$title.epub")))
        }
        compose.waitUntil(90_000) { !core().view.storageBusy }
        val id = core().view.importSummary!!.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        awaitReader()
        val first = core().view.reader!!
        assertEquals(4, first.formatting.size)
        val text = annotated()
        assertEquals(first.text, text.text)
        assertTrue(text.spanStyles.any { it.item.fontWeight == FontWeight.Bold && it.start == 0 })
        assertTrue(text.spanStyles.any { it.item.fontStyle == FontStyle.Italic })
        val website =
            text.getLinkAnnotations(0, text.length).filter { it.item is LinkAnnotation.Url }
        assertEquals(1, website.size)
        assertEquals("外部 site", text.text.substring(website.single().start, website.single().end))
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
            tap("site")
            assertEquals(1, monitor.hits)
            assertEquals(first.anchor, core().view.reader!!.anchor)
            assertNull(core().view.effectiveReaderWord?.selection)
            tap("note")
            tap("script")
            assertEquals(1, monitor.hits)
            assertEquals(first.position, core().view.reader!!.position)
        } finally {
            instrumentation.removeMonitor(monitor)
        }
        compose.activityRule.scenario.recreate()
        awaitReader()
        assertEquals(first.formatting, core().view.reader!!.formatting)
        assertEquals(first.position, core().view.reader!!.position)
        tap("猫")
        compose.onNodeWithTag("reader_word_panel").assertIsDisplayed()
        compose.onNodeWithTag("word_surface").assertTextEquals("猫")
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        awaitReader()
        compose.onNodeWithTag("next_page").performScrollTo().assertIsEnabled().performClick()
        compose.waitUntil(30_000) { core().view.reader?.pageNumber == 2UL }
        awaitReader()
        assertTrue(core().view.reader!!.formatting.isEmpty())
        compose.onNodeWithTag("previous_page").performScrollTo().performClick()
        compose.waitUntil(30_000) { core().view.reader?.pageNumber == 1UL }
        awaitReader()
        assertEquals(first.formatting, core().view.reader!!.formatting)
        assertEquals(first.anchor, core().view.reader!!.anchor)
    }
}
