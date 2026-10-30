package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Exercises the packaged JNI library and generated codecs on Android's runtime. */
@RunWith(AndroidJUnit4::class)
class CoreSessionTest {
    @Test
    fun commandsRenderRustStateAndSessionsOwnTheirHandles() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            CoreSession(
                    ":memory:",
                    sudachiDictionaryProvider(
                        InstrumentationRegistry.getInstrumentation().targetContext
                    ),
                )
                .use { core ->
                    CoreSession(
                            ":memory:",
                            sudachiDictionaryProvider(
                                InstrumentationRegistry.getInstrumentation().targetContext
                            ),
                        )
                        .use { independent ->
                            val renders = mutableListOf<CoreRenderMeasurement>()
                            core.onRenderMeasured = { renders.add(it) }
                            assertNotEquals(core.sessionKey, independent.sessionKey)
                            assertEquals(Destination.LIBRARY, core.view.destination)
                            for (destination in Destination.entries + Destination.LIBRARY) {
                                core.dispatch(Event.SelectDestination(destination))
                                assertEquals(destination, core.view.destination)
                                assertEquals(Destination.LIBRARY, independent.view.destination)
                            }
                            assertTrue(renders.isNotEmpty())
                            assertTrue(renders.all { !it.fromCallback && it.payloadBytes > 0 })
                            assertTrue(
                                renders.all {
                                    it.nativeViewNanos >= 0 &&
                                        it.decodeNanos >= 0 &&
                                        it.publishNanos >= 0
                                }
                            )
                            val measured = renders.size
                            core.onRenderMeasured = null
                            core.dispatch(Event.SelectDestination(Destination.NOSTR))
                            assertEquals(Destination.NOSTR, core.view.destination)
                            assertEquals(measured, renders.size)
                        }
                    core.dispatch(Event.SelectDestination(Destination.VOCABULARY))
                    assertEquals(Destination.VOCABULARY, core.view.destination)
                }

            repeat(10) {
                val core =
                    CoreSession(
                        ":memory:",
                        sudachiDictionaryProvider(
                            InstrumentationRegistry.getInstrumentation().targetContext
                        ),
                    )
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
