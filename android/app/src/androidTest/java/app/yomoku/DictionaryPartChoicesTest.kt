package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DictionaryPartChoicesTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun statusWritesKeepChipAndMorePartsPixelsWhileBlockingInputInBothThemes() {
        val pending = mutableStateOf(false)
        val dark = mutableStateOf(false)
        val lookup = mutableStateOf(copyLookup())
        val choices = mutableListOf<ULong?>()
        compose.setContent {
            MaterialTheme(colorScheme = if (dark.value) darkColorScheme() else lightColorScheme()) {
                Surface {
                    DictionaryPartChoices(lookup.value, !pending.value, pending.value) {
                        choices.add(it)
                    }
                }
            }
        }
        fun assertStable(tags: List<String>) {
            val before = tags.associateWith {
                compose.onNodeWithTag(it).captureToImage().asAndroidBitmap()
            }
            compose.runOnUiThread { pending.value = true }
            tags.forEach { tag ->
                val node = compose.onNodeWithTag(tag).assertIsNotEnabled()
                assertTrue(
                    "Appearance changed: $tag",
                    before.getValue(tag).sameAs(node.captureToImage().asAndroidBitmap()),
                )
                node.performTouchInput { click() }
            }
            assertTrue(choices.isEmpty())
            compose.runOnUiThread { pending.value = false }
            tags.forEach { compose.onNodeWithTag(it).assertIsEnabled() }
        }
        for (isDark in listOf(false, true)) {
            compose.runOnUiThread { dark.value = isDark }
            assertStable(listOf("dictionary_whole", "dictionary_part_0", "dictionary_more_parts"))
        }
        compose.onNodeWithTag("dictionary_more_parts").performClick()
        compose.onNodeWithTag("dictionary_part_1").assertExists()
        compose.runOnUiThread { lookup.value = lookup.value.copy(active = 1UL) }
        for (isDark in listOf(false, true)) {
            compose.runOnUiThread { dark.value = isDark }
            assertStable(listOf("dictionary_whole", "dictionary_part_0", "dictionary_part_1"))
            compose.onNodeWithTag("dictionary_part_1").assertIsSelected()
            compose.onNodeWithTag("dictionary_more_parts").assertDoesNotExist()
        }
        val settled = compose.onNodeWithTag("dictionary_part_1").captureToImage().asAndroidBitmap()
        compose.runOnUiThread { lookup.value = lookup.value.copy(loading = true) }
        compose.onNodeWithTag("dictionary_whole").assertIsNotEnabled()
        compose.onNodeWithTag("dictionary_part_1").assertIsNotEnabled()
        assertFalse(
            settled.sameAs(
                compose.onNodeWithTag("dictionary_part_1").captureToImage().asAndroidBitmap()
            )
        )
    }
}
