package app.yomoku

import android.content.ClipboardManager
import android.content.Context
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.ComposeTestRule
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*
import org.junit.Assert.assertEquals

internal fun copyLookup() =
    DictionaryLookup(
        DictionaryQuery("𠮷か\u3099", "different base form"),
        listOf(
            DictionaryPart(
                "𠮷",
                "different part base",
                TextRange(TextOffset(0), TextOffset(4)),
                true,
            ),
            DictionaryPart("か\u3099", null, TextRange(TextOffset(4), TextOffset(10)), false),
        ),
        revision = 1UL,
        loading = false,
    )

internal fun assertCopiedWord(compose: ComposeTestRule, context: Context, expected: String) {
    val button = compose.onNodeWithTag("word_copy")
    button
        .performScrollTo()
        .assertIsEnabled()
        .assertContentDescriptionEquals("Copy word")
        .assertWidthIsEqualTo(48.dp)
        .assertHeightIsEqualTo(48.dp)
    compose
        .onNodeWithTag("word_copy_icon", useUnmergedTree = true)
        .assertWidthIsEqualTo(12.dp)
        .assertHeightIsEqualTo(12.dp)
    button.performTouchInput { click(Offset(4f, centerY)) }
    compose.runOnIdle {
        val clip = context.getSystemService(ClipboardManager::class.java).primaryClip!!
        assertEquals(1, clip.itemCount)
        assertEquals(expected, clip.getItemAt(0).text.toString())
    }
}
