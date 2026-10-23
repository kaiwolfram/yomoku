package app.yomoku

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.security.MessageDigest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SudachiAssetsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun installedFullDictionaryAndCompleteNoticesMatchPinnedRelease() {
        val assets = compose.activity.assets
        val expected =
            mapOf(
                "system_full.dic" to
                    "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1",
                "LEGAL" to "725a8776b38e058b185e905594bc9a2437dbf3787df022fffeefedb9a84e4665",
                "LICENSE-2.0.txt" to
                    "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",
            )
        for ((name, checksum) in expected) {
            val digest = MessageDigest.getInstance("SHA-256")
            var size = 0L
            assets.open("sudachi/$name").use { input ->
                val buffer = ByteArray(64 * 1024)
                while (true) {
                    val count = input.read(buffer)
                    if (count == -1) break
                    size += count
                    digest.update(buffer, 0, count)
                }
            }
            assertEquals(name, checksum, digest.digest().joinToString("") { "%02x".format(it) })
            if (name == "system_full.dic") assertEquals(330_506_339L, size)
        }
    }

    @Test
    fun settingsShowsCompleteOfflineNoticesAcrossRecreation() {
        val assets = compose.activity.assets
        val expected =
            listOf("LEGAL", "LICENSE-2.0.txt").joinToString("\n\n") { name ->
                "$name\n\n" + assets.open("sudachi/$name").bufferedReader().use { it.readText() }
            }
        compose.onNodeWithText("Settings").performClick()
        compose.onNodeWithText("Sudachi licenses and notices").performScrollTo().performClick()
        compose.waitUntil(10_000) {
            compose
                .onAllNodes(androidx.compose.ui.test.hasText(expected))
                .fetchSemanticsNodes()
                .size == 1
        }
        compose.onNodeWithTag("sudachi_notices_text").assertTextEquals(expected)
        compose.activityRule.scenario.recreate()
        compose.waitUntil(10_000) {
            compose
                .onAllNodes(androidx.compose.ui.test.hasText(expected))
                .fetchSemanticsNodes()
                .size == 1
        }
        compose.onNodeWithTag("sudachi_notices_text").assertTextEquals(expected)
        compose.onNodeWithText("Close").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
    }
}
