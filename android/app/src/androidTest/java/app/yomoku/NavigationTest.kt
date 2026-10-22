package app.yomoku

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
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
        compose
            .onNodeWithTag(if (wide) "navigation_rail" else "bottom_navigation")
            .assertIsDisplayed()
        compose
            .onNodeWithTag(if (wide) "bottom_navigation" else "navigation_rail")
            .assertDoesNotExist()
        for (title in listOf("Nostr", "Statistics", "Vocabulary", "Library")) {
            compose.onNode(hasText(title) and hasClickAction()).performClick().assertIsSelected()
            compose.onNodeWithTag("screen_title").assertTextEquals(title)
            compose.onNodeWithText("Settings").performClick()
            compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
            compose.onNodeWithTag("bottom_navigation").assertDoesNotExist()
            compose.onNodeWithTag("navigation_rail").assertDoesNotExist()
            compose.onNodeWithText("Back").performClick()
            compose.onNodeWithTag("screen_title").assertTextEquals(title)
        }
        compose.onNodeWithText("Settings").performClick()
        compose.waitForIdle()
        compose.runOnUiThread { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithTag("screen_title").assertTextEquals("Library")
    }

    @Test
    fun recreationRetainsNativeSessionAndFinishingClosesIt() {
        compose.onNode(hasText("Vocabulary") and hasClickAction()).performClick()
        compose.onNodeWithText("Settings").performClick()
        lateinit var original: CoreSession
        compose.activityRule.scenario.onActivity {
            original = ViewModelProvider(it)[CoreSessionOwner::class.java].session
        }
        compose.activityRule.scenario.recreate()
        compose.activityRule.scenario.onActivity {
            assertSame(original, ViewModelProvider(it)[CoreSessionOwner::class.java].session)
        }
        compose.onNodeWithTag("screen_title").assertTextEquals("Settings")
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithTag("screen_title").assertTextEquals("Vocabulary")
        compose.activityRule.scenario.close()
        compose.runOnUiThread {
            assertThrows(IllegalStateException::class.java) {
                original.dispatch(Event.SelectDestination(Destination.LIBRARY))
            }
        }
    }
}
