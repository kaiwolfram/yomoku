package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertSame
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
                            val requests = mutableListOf<CoreRequestsMeasurement>()
                            core.onRenderMeasured = { renders.add(it) }
                            core.onRequestsMeasured = { requests.add(it) }
                            assertNotEquals(core.sessionKey, independent.sessionKey)
                            assertEquals(Destination.LIBRARY, core.view.destination)
                            for (destination in Destination.entries + Destination.LIBRARY) {
                                core.dispatch(Event.SelectDestination(destination))
                                assertEquals(destination, core.view.destination)
                                assertEquals(Destination.LIBRARY, independent.view.destination)
                            }
                            assertTrue(renders.isNotEmpty())
                            assertTrue(
                                renders.all {
                                    it.source == CoreRenderSource.DISPATCH && it.payloadBytes > 0
                                }
                            )
                            assertTrue(requests.isNotEmpty())
                            assertTrue(
                                requests.all {
                                    it.source == CoreRenderSource.DISPATCH &&
                                        it.eventEncodeNanos!! >= 0 &&
                                        it.nativeUpdateNanos!! >= 0 &&
                                        it.requestDecodeNanos >= 0 &&
                                        it.payloadBytes > 0 &&
                                        it.requestCount >= 0
                                }
                            )
                            assertTrue(
                                renders.all {
                                    it.nativeViewNanos >= 0 &&
                                        it.decodeNanos >= 0 &&
                                        it.publishNanos >= 0
                                }
                            )
                            val measured = renders.size
                            val measuredRequests = requests.size
                            core.onRenderMeasured = null
                            core.onRequestsMeasured = null
                            core.dispatch(Event.SelectDestination(Destination.NOSTR))
                            assertEquals(Destination.NOSTR, core.view.destination)
                            assertEquals(measured, renders.size)
                            assertEquals(measuredRequests, requests.size)
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

    @Test
    fun snapshotReplayRequiresObserverAndRetainsEqualStateInstance() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val loaded = CountDownLatch(1)
        lateinit var core: CoreSession
        instrumentation.runOnMainSync {
            core = CoreSession(":memory:", sudachiDictionaryProvider(instrumentation.targetContext))
            core.onViewChanged = { if (!core.view.storageBusy) loaded.countDown() }
            if (!core.view.storageBusy) loaded.countDown()
        }
        try {
            assertTrue(loaded.await(30, TimeUnit.SECONDS))
            assertThrows(IllegalStateException::class.java) { core.onRequestsMeasured = {} }
            assertThrows(IllegalStateException::class.java) { core.measureCurrentView() }
            instrumentation.runOnMainSync {
                assertThrows(IllegalStateException::class.java) { core.measureCurrentView() }
                val renders = mutableListOf<CoreRenderMeasurement>()
                val requests = mutableListOf<CoreRequestsMeasurement>()
                var changes = 0
                core.onViewChanged = { changes++ }
                core.onRenderMeasured = { renders.add(it) }
                core.onRequestsMeasured = { requests.add(it) }
                core.dispatch(Event.SelectDestination(Destination.NOSTR))
                val before = core.view
                val requestsBefore = requests.size
                val changesBefore = changes
                core.measureCurrentView()
                assertSame(before, core.view)
                assertEquals(requestsBefore, requests.size)
                assertEquals(changesBefore, changes)
                assertEquals(CoreRenderSource.REPLAY, renders.last().source)
                assertFalse(renders.last().published)
                core.onRenderMeasured = null
                assertThrows(IllegalStateException::class.java) { core.measureCurrentView() }
            }
        } finally {
            instrumentation.runOnMainSync { core.close() }
            assertTrue(core.isClosed)
        }
    }
}
