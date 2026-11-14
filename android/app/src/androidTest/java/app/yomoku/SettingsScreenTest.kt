package app.yomoku

import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SettingsScreenTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun groupedSettingsKeepAccessibleControlsAndFullWidthNoticesInBothThemes() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy && core.view.pageLength != null }
        val view =
            core.view.copy(
                settingsOpen = true,
                ankiSetup = AnkiSetupPhase.Ready,
                pageLength = PageLength.CHARACTERS250,
                readerTypography =
                    ReaderTypography(ReaderFontSize.SP16, ReaderLineSpacing.PERCENT150),
            )
        val dark = mutableStateOf(false)
        val events = mutableListOf<Event>()
        compose.runOnUiThread {
            compose.activity.setContent {
                MaterialTheme(
                    colorScheme = if (dark.value) darkColorScheme() else lightColorScheme()
                ) {
                    YomokuApp(view, "settings-layout", events::add)
                }
            }
        }
        compose.onNodeWithTag("reader_furigana").assertDoesNotExist()
        for (night in listOf(false, true)) {
            compose.runOnUiThread { dark.value = night }
            val reader = compose.onNodeWithText("Reader").performScrollTo()
            reader.assert(SemanticsMatcher.keyIsDefined(SemanticsProperties.Heading))
            val readerTop = reader.getUnclippedBoundsInRoot().top
            val ankiTop = compose.onNodeWithText("AnkiDroid").getUnclippedBoundsInRoot().top
            val aboutTop = compose.onNodeWithText("About").getUnclippedBoundsInRoot().top
            assertTrue(readerTop < ankiTop && ankiTop < aboutTop)
            listOf("reader_font_size", "reader_line_spacing").forEach { tag ->
                val label = compose.onNodeWithTag(tag).performScrollTo().getUnclippedBoundsInRoot()
                val decrease = compose.onNodeWithTag("${tag}_decrease").getUnclippedBoundsInRoot()
                val increase = compose.onNodeWithTag("${tag}_increase").getUnclippedBoundsInRoot()
                assertTrue(label.right <= decrease.left && decrease.right <= increase.left)
                assertTrue(label.top < increase.bottom && increase.top < label.bottom)
                assertTrue(
                    (decrease.right - decrease.left).value >= 48f &&
                        (decrease.bottom - decrease.top).value >= 48f
                )
                assertTrue(
                    (increase.right - increase.left).value >= 48f &&
                        (increase.bottom - increase.top).value >= 48f
                )
                compose.onNodeWithTag("${tag}_decrease").assertContentDescriptionEquals("Decrease")
                compose.onNodeWithTag("${tag}_increase").assertContentDescriptionEquals("Increase")
                compose.onNodeWithTag("${tag}_increase").performClick()
                assertTrue(events.last() is Event.SetReaderTypography)
            }
            listOf(250, 500, 750, 1000).forEach { size ->
                compose.onNodeWithTag("page_length_$size").performScrollTo().assertIsEnabled()
            }
            compose.onNodeWithTag("page_length_250").assertIsSelected()
            compose
                .onNodeWithText("AnkiDroid")
                .performScrollTo()
                .assert(SemanticsMatcher.keyIsDefined(SemanticsProperties.Heading))
            compose.onNodeWithTag("anki_settings").assertExists()
            compose.onNodeWithTag("anki_model_setup").assertDoesNotExist()
            compose
                .onNodeWithText("About")
                .performScrollTo()
                .assert(SemanticsMatcher.keyIsDefined(SemanticsProperties.Heading))
            val notices =
                listOf(
                    R.string.sudachi_notices_title,
                    R.string.word_dictionary_notices_title,
                    R.string.svg_notices_title,
                    R.string.reader_font_notices_title,
                )
            for (resource in notices) {
                val row =
                    compose
                        .onNodeWithText(compose.activity.getString(resource))
                        .performScrollTo()
                        .assertHasClickAction()
                val bounds = row.getUnclippedBoundsInRoot()
                assertTrue((bounds.bottom - bounds.top).value >= 48f)
                val width = bounds.right - bounds.left
                val pageBounds = compose.onNodeWithTag("page_length_250").getUnclippedBoundsInRoot()
                val pageWidth = pageBounds.right - pageBounds.left
                assertEquals(pageWidth.value, width.value, 0.5f)
                row.performClick()
                compose.onNodeWithText("Close").performClick()
            }
            val noticeBounds = notices.map {
                compose.onNodeWithText(compose.activity.getString(it)).getUnclippedBoundsInRoot()
            }
            noticeBounds.zipWithNext().forEach { (first, second) ->
                assertEquals(first.bottom.value, second.top.value, 0.5f)
            }
            assertTrue(
                compose.onAllNodesWithTag("bottom_navigation").fetchSemanticsNodes().isNotEmpty() ||
                    compose.onAllNodesWithTag("navigation_rail").fetchSemanticsNodes().isNotEmpty()
            )
        }
    }
}
