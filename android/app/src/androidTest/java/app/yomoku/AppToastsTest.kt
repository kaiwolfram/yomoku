package app.yomoku

import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AppToastsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun newFeedbackCancelsVisibleToastBeforeShowingAndDisposeCancelsOnce() {
        val calls = mutableListOf<String>()
        val toasts = AppToasts { message ->
            object : ToastMessage {
                override fun show() {
                    calls.add("show:$message")
                }

                override fun cancel() {
                    calls.add("cancel:$message")
                }
            }
        }
        toasts.show("3 new words marked as Known")
        toasts.show("Added to AnkiDroid.")
        toasts.cancel()
        toasts.cancel()
        assertEquals(
            listOf(
                "show:3 new words marked as Known",
                "cancel:3 new words marked as Known",
                "show:Added to AnkiDroid.",
                "cancel:Added to AnkiDroid.",
            ),
            calls,
        )
    }

    @Test
    fun ankiSuccessIsConsumedBeforeDisplayAndCannotReplayOnRecompositionOrRecreation() {
        val snapshot = mutableStateOf(scalarVocabularyView(emptyList()).copy(ankiAddedNotice = 7UL))
        val messages = mutableListOf<String>()
        val events = mutableListOf<Event>()
        fun render() {
            compose.runOnUiThread {
                compose.activity.setContent {
                    AnkiAddedToast(
                        snapshot.value,
                        { event ->
                            events.add(event)
                            snapshot.value = snapshot.value.copy(ankiAddedNotice = null)
                        },
                        { message ->
                            assertNull(snapshot.value.ankiAddedNotice)
                            messages.add(message)
                        },
                    )
                }
            }
        }
        render()
        compose.waitForIdle()
        assertEquals(listOf(Event.DismissAnkiAddedNotice(7UL)), events)
        assertEquals(listOf("Added to AnkiDroid."), messages)
        compose.runOnUiThread { snapshot.value = snapshot.value.copy(storageBusy = true) }
        compose.waitForIdle()
        compose.activityRule.scenario.recreate()
        render()
        compose.waitForIdle()
        assertEquals(1, messages.size)
        compose.runOnUiThread { snapshot.value = snapshot.value.copy(ankiAddedNotice = 8UL) }
        compose.waitForIdle()
        assertEquals(2, messages.size)
    }
}
