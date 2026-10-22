package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith

/** Exercises the packaged JNI library and generated codecs on Android's runtime. */
@RunWith(AndroidJUnit4::class)
class CoreSessionTest {
    @Test
    fun commandsRenderRustStateAndSessionsOwnTheirHandles() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            CoreSession().use { core ->
                CoreSession().use { independent ->
                    assertEquals(Destination.LIBRARY, core.view.destination)
                    for (destination in Destination.entries + Destination.LIBRARY) {
                        core.dispatch(Event.SelectDestination(destination))
                        assertEquals(destination, core.view.destination)
                        assertEquals(Destination.LIBRARY, independent.view.destination)
                    }
                }
                core.dispatch(Event.SelectDestination(Destination.VOCABULARY))
                assertEquals(Destination.VOCABULARY, core.view.destination)
            }

            repeat(10) {
                val core = CoreSession()
                assertEquals(Destination.LIBRARY, core.view.destination)
                core.close()
                core.close()
                assertThrows(IllegalStateException::class.java) {
                    core.dispatch(Event.SelectDestination(Destination.NOSTR))
                }
            }
        }
    }
}
