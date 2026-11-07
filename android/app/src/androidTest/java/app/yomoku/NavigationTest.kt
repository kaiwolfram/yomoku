package app.yomoku

import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotSelected
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NavigationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun tabsAndSettingsRenderRustStateAndReturnToSelectedTab() {
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
        val wide = compose.activity.resources.configuration.screenWidthDp >= 600
        val navigationTag = if (wide) "navigation_rail" else "bottom_navigation"
        compose
            .onNodeWithTag(if (wide) "navigation_rail" else "bottom_navigation")
            .assertIsDisplayed()
        compose
            .onNodeWithTag(if (wide) "bottom_navigation" else "navigation_rail")
            .assertDoesNotExist()
        compose.onNodeWithText("Feed").assertDoesNotExist()
        compose.onNodeWithText("Nostr").assertDoesNotExist()
        compose.onNodeWithText("Coming Soon! 🌱").assertDoesNotExist()
        compose
            .onAllNodes(hasClickAction() and hasAnyAncestor(hasTestTag(navigationTag)))
            .assertCountEquals(4)
        for (title in listOf("Statistics", "Vocabulary", "Library")) {
            compose.onNode(hasText(title) and hasClickAction()).performClick().assertIsSelected()
            compose.onNodeWithTag("screen_title").assertTextEquals(title)
            compose.onAllNodes(hasText("Settings") and hasClickAction()).assertCountEquals(1)
            compose.onNodeWithTag("navigation_settings").performClick().assertIsSelected()
            compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
            compose.onNodeWithTag(navigationTag).assertIsDisplayed()
            compose.onNodeWithText("Back").assertDoesNotExist()
            compose.onNode(hasText(title) and hasClickAction()).assertIsNotSelected()
            compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
            compose.onNodeWithTag("screen_title").assertTextEquals(title)
        }
        compose.onNodeWithText("Settings").performClick()
        compose.waitForIdle()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
        for (title in listOf("Statistics", "Vocabulary", "Library")) {
            compose.onNodeWithTag("navigation_settings").performClick()
            compose.onNode(hasText(title) and hasClickAction()).performClick().assertIsSelected()
            compose.onNodeWithTag("screen_title").assertTextEquals(title)
            compose.onNodeWithTag("navigation_settings").assertIsNotSelected()
        }
    }

    @Test
    fun settingsExposesOnlyFourPageLengthsAndRetainsLegacyValue() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy && core.view.pageLength != null }
        val original = core.view.pageLength!!
        try {
            compose.onNodeWithTag("navigation_settings").performClick()
            compose.runOnUiThread {
                core.dispatch(Event.SetPageLength(app.yomoku.shared.PageLength.CHARACTERS1500))
            }
            compose.waitUntil(30_000) { !core.view.storageBusy }
            compose.onNodeWithText("Back").assertDoesNotExist()
            for (size in listOf(250, 500, 750, 1000)) {
                compose.onNodeWithTag("page_length_$size").performScrollTo().assertExists()
            }
            compose.onNodeWithTag("page_length_1500").assertDoesNotExist()
            assertEquals(app.yomoku.shared.PageLength.CHARACTERS1500, core.view.pageLength)
            compose.activityRule.scenario.recreate()
            assertEquals(app.yomoku.shared.PageLength.CHARACTERS1500, core.view.pageLength)
            compose.onNode(hasText("Library") and hasClickAction()).performClick()
            compose.onNodeWithTag("screen_title").assertTextEquals("Library")
        } finally {
            compose.runOnUiThread {
                core.dispatch(Event.OpenSettings)
                core.dispatch(Event.SetPageLength(original))
            }
            compose.waitUntil(30_000) { !core.view.storageBusy }
            compose.runOnUiThread { core.dispatch(Event.CloseSettings) }
        }
    }

    @Test
    fun recreationRetainsNativeSessionAndFinishingClosesIt() {
        compose.onNode(hasText("Vocabulary") and hasClickAction()).performClick()
        compose.onNodeWithText("Settings").performClick()
        lateinit var original: CoreSession
        compose.activityRule.scenario.onActivity {
            original = ViewModelProvider(it)[CoreSessionOwner::class.java].session
        }
        val sessionKey = original.sessionKey
        compose.activityRule.scenario.recreate()
        compose.activityRule.scenario.onActivity {
            val retained = ViewModelProvider(it)[CoreSessionOwner::class.java].session
            assertSame(original, retained)
            assertEquals(sessionKey, retained.sessionKey)
        }
        compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
        compose.onNodeWithTag("navigation_settings").assertIsSelected()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("screen_title").assertTextEquals("Vocabulary")
        compose.activityRule.scenario.close()
        compose.runOnUiThread {
            assertThrows(IllegalStateException::class.java) {
                original.dispatch(Event.SelectDestination(Destination.LIBRARY))
            }
        }
    }
}
