package app.yomoku

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiDeckDiscoveryTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    @Test
    fun readsAreExplicitOffMainCoalescedAndRetriedWithFreshResults() {
        val executor = Executors.newSingleThreadExecutor()
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val reads = AtomicInteger()
        lateinit var discovery: AnkiDeckDiscovery
        main {
            discovery =
                AnkiDeckDiscovery(executor) {
                    assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                    val count = reads.incrementAndGet()
                    entered.countDown()
                    assertTrue(release.await(30, TimeUnit.SECONDS))
                    when (count) {
                        1 -> AnkiReadResult.Failed(AnkiReadError.PermissionDenied)
                        2 -> throw SecurityException()
                        3 -> throw IllegalStateException()
                        else ->
                            AnkiReadResult.Value(
                                listOf(AnkiProviderDeck(Long.MAX_VALUE, "日本語", false))
                            )
                    }
                }
            assertEquals(AnkiDeckDiscoveryState.Unchecked, discovery.state)
            assertEquals(0, reads.get())
            discovery.check()
            discovery.check()
        }
        try {
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            assertEquals(1, reads.get())
            main { assertEquals(AnkiDeckDiscoveryState.Loading, discovery.state) }
            release.countDown()
            executor.submit {}.get(30, TimeUnit.SECONDS)
            instrumentation.waitForIdleSync()
            main {
                assertEquals(
                    AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied),
                    discovery.state,
                )
            }
            for (expected in
                listOf(
                    AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied),
                    AnkiDeckDiscoveryState.Failed(AnkiReadError.ProviderFailure),
                    AnkiDeckDiscoveryState.Ready(
                        listOf(AnkiProviderDeck(Long.MAX_VALUE, "日本語", false))
                    ),
                )) {
                main { discovery.check() }
                executor.submit {}.get(30, TimeUnit.SECONDS)
                instrumentation.waitForIdleSync()
                main { assertEquals(expected, discovery.state) }
            }
        } finally {
            release.countDown()
            main { discovery.close() }
            executor.shutdown()
            assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
        }
    }

    @Test
    fun closeSkipsQueuedReadAndDropsInFlightResult() {
        for (queued in listOf(true, false)) {
            val executor = Executors.newSingleThreadExecutor()
            val entered = CountDownLatch(1)
            val release = CountDownLatch(1)
            val reads = AtomicInteger()
            if (queued)
                executor.execute {
                    entered.countDown()
                    assertTrue(release.await(30, TimeUnit.SECONDS))
                }
            lateinit var discovery: AnkiDeckDiscovery
            main {
                discovery =
                    AnkiDeckDiscovery(executor) {
                        reads.incrementAndGet()
                        entered.countDown()
                        assertTrue(release.await(30, TimeUnit.SECONDS))
                        AnkiReadResult.Value(emptyList())
                    }
                discovery.check()
            }
            try {
                assertTrue(entered.await(30, TimeUnit.SECONDS))
                main { discovery.close() }
                release.countDown()
                executor.submit {}.get(30, TimeUnit.SECONDS)
                instrumentation.waitForIdleSync()
                main {
                    assertEquals(AnkiDeckDiscoveryState.Loading, discovery.state)
                    discovery.check()
                }
                assertEquals(if (queued) 0 else 1, reads.get())
            } finally {
                release.countDown()
                main { discovery.close() }
                executor.shutdown()
                assertTrue(executor.awaitTermination(30, TimeUnit.SECONDS))
            }
        }
    }
}
