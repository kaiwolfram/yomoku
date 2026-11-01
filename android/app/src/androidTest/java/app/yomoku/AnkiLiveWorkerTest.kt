package app.yomoku

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.AnkiLiveOperation
import app.yomoku.shared.AnkiLiveResult
import app.yomoku.shared.AnkiReadModelLayout
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiLiveWorkerTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val layout =
        AnkiReadModelLayout(
            "Frozen 𠮷",
            listOf("Surface", "Reading", "Meanings", "Sentence"),
            "css",
            "Word",
            "{{Surface}}",
            "{{Sentence}}",
        )
    private val operation = AnkiLiveOperation.CreateModel("𠮷猫", Long.MAX_VALUE, layout)

    @Test
    fun invokesOnceOffMainAndDeliversOnlyThroughMainQueue() {
        val executor = Executors.newSingleThreadExecutor()
        val calls = LinkedBlockingQueue<AnkiLiveOperation>()
        val posted = LinkedBlockingQueue<() -> Unit>()
        val delivered = mutableListOf<Pair<UInt, AnkiLiveResult>>()
        val worker =
            AnkiLiveWorker(
                executor,
                {
                    assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                    calls.add(it)
                    AnkiLiveResult.ModelCreated(Long.MAX_VALUE)
                },
                { posted.add(it) },
            ) { id, result ->
                assertEquals(Looper.getMainLooper(), Looper.myLooper())
                delivered.add(id to result)
            }
        try {
            instrumentation.runOnMainSync {
                worker.dispatch(7u, operation)
                worker.dispatch(7u, operation)
            }
            assertEquals(operation, calls.poll(30, TimeUnit.SECONDS))
            val callback = checkNotNull(posted.poll(30, TimeUnit.SECONDS))
            assertTrue(delivered.isEmpty())
            instrumentation.runOnMainSync {
                callback()
                worker.dispatch(7u, operation)
            }
            executor.submit {}.get(30, TimeUnit.SECONDS)
            assertTrue(calls.isEmpty())
            assertTrue(posted.isEmpty())
            assertEquals(listOf(7u to AnkiLiveResult.ModelCreated(Long.MAX_VALUE)), delivered)
        } finally {
            instrumentation.runOnMainSync { worker.close() }
            executor.shutdownNow()
        }
    }

    @Test
    fun closeCancelsQueuedWorkAndRejectsFurtherDispatch() {
        val executor = Executors.newSingleThreadExecutor()
        val release = LinkedBlockingQueue<Unit>()
        executor.execute { release.poll(30, TimeUnit.SECONDS) }
        val calls = LinkedBlockingQueue<AnkiLiveOperation>()
        val posted = LinkedBlockingQueue<() -> Unit>()
        val worker =
            AnkiLiveWorker(
                executor,
                {
                    calls.add(it)
                    AnkiLiveResult.NoSubmission
                },
                { posted.add(it) },
            ) { _, _ ->
                error("closed callback")
            }
        try {
            instrumentation.runOnMainSync {
                worker.dispatch(1u, operation)
                worker.close()
                worker.dispatch(2u, operation)
            }
            release.add(Unit)
            executor.submit {}.get(30, TimeUnit.SECONDS)
            assertTrue(calls.isEmpty())
            assertTrue(posted.isEmpty())
        } finally {
            release.offer(Unit)
            executor.shutdownNow()
        }
    }

    @Test
    fun inFlightResultsCannotReachClosedOrNewOwners() {
        val executor = Executors.newSingleThreadExecutor()
        val started = LinkedBlockingQueue<Unit>()
        val release = LinkedBlockingQueue<Unit>()
        val posted = LinkedBlockingQueue<() -> Unit>()
        val worker =
            AnkiLiveWorker(
                executor,
                {
                    started.add(Unit)
                    checkNotNull(release.poll(30, TimeUnit.SECONDS))
                    AnkiLiveResult.ModelUncertain(Long.MAX_VALUE)
                },
                { posted.add(it) },
            ) { _, _ ->
                error("old callback")
            }
        try {
            instrumentation.runOnMainSync { worker.dispatch(1u, operation) }
            checkNotNull(started.poll(30, TimeUnit.SECONDS))
            instrumentation.runOnMainSync { worker.close() }
            release.add(Unit)
            val callback = checkNotNull(posted.poll(30, TimeUnit.SECONDS))
            instrumentation.runOnMainSync { callback() }
            // A new session owns its own ledger and callback; no work is copied to it.
            val delivered = LinkedBlockingQueue<AnkiLiveResult>()
            val fresh =
                AnkiLiveWorker(
                    executor,
                    { AnkiLiveResult.NoSubmission },
                    { posted.add(it) },
                ) { _, result ->
                    delivered.add(result)
                }
            instrumentation.runOnMainSync { fresh.dispatch(1u, operation) }
            val freshCallback = checkNotNull(posted.poll(30, TimeUnit.SECONDS))
            instrumentation.runOnMainSync {
                freshCallback()
                fresh.close()
            }
            assertEquals(AnkiLiveResult.NoSubmission, delivered.poll())
            assertTrue(posted.isEmpty())
        } finally {
            release.offer(Unit)
            executor.shutdownNow()
        }
    }

    @Test
    fun unexpectedExceptionsAreUncertainForEachMutationKind() {
        val executor = Executors.newSingleThreadExecutor()
        val posted = LinkedBlockingQueue<() -> Unit>()
        val delivered = LinkedBlockingQueue<AnkiLiveResult>()
        val worker =
            AnkiLiveWorker(
                executor,
                { throw SecurityException("unknown submission point") },
                { posted.add(it) },
            ) { _, result ->
                delivered.add(result)
            }
        val note =
            AnkiLiveOperation.InsertNote(
                "𠮷猫",
                listOf("𠮷猫", "", "", "sentence"),
                Long.MAX_VALUE,
                9007199254740993L,
                layout,
            )
        try {
            instrumentation.runOnMainSync {
                worker.dispatch(1u, operation)
                worker.dispatch(2u, note)
            }
            repeat(2) {
                val callback = checkNotNull(posted.poll(30, TimeUnit.SECONDS))
                instrumentation.runOnMainSync { callback() }
            }
            assertEquals(AnkiLiveResult.ModelUncertain(null), delivered.poll())
            assertEquals(AnkiLiveResult.NoteUncertain(null), delivered.poll())
        } finally {
            instrumentation.runOnMainSync { worker.close() }
            executor.shutdownNow()
        }
    }
}
