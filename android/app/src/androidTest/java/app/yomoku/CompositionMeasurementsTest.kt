package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CompositionMeasurementsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun observesChangedCompositionAndDetachesWithoutRetainingPasses() {
        val probe = CompositionMeasurements()
        var text by mutableStateOf("before")
        var observe by mutableStateOf(true)
        compose.setContent {
            if (observe) probe.Observe()
            Text(text)
        }
        compose.waitForIdle()
        compose.runOnUiThread {
            probe.start()
            text = "after"
        }
        compose.waitForIdle()
        compose.onNodeWithText("after").assertIsDisplayed()
        compose.runOnUiThread {
            val samples = probe.stop()
            assertTrue(samples.isNotEmpty())
            assertTrue(samples.all { it.endNanos >= it.startNanos && it.depth >= 0 })
            probe.start()
            text = "after"
        }
        compose.waitForIdle()
        compose.runOnUiThread { assertTrue(probe.stop().isEmpty()) }
        compose.runOnUiThread { observe = false }
        compose.waitForIdle()
        compose.runOnUiThread {
            probe.start()
            text = "detached"
        }
        compose.waitForIdle()
        compose.onNodeWithText("detached").assertIsDisplayed()
        compose.runOnUiThread { assertTrue(probe.stop().isEmpty()) }
    }
}
