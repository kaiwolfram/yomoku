package app.yomoku

import android.content.Context
import android.content.ContextWrapper
import android.os.Debug
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.util.Log
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.io.FileInputStream
import java.util.UUID
import java.util.concurrent.Callable
import java.util.concurrent.CompletableFuture
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Installed Compose + Rust session + real Full mapping in the same process. */
@RunWith(AndroidJUnit4::class)
class SudachiAppProbeTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun verifiedDescriptorWorkerReuseLifecycleAndRelease() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(context.noBackupFilesDir, "app-probe-${UUID.randomUUID()}")
        val privateContext =
            object : ContextWrapper(context) {
                override fun getApplicationContext(): Context = this

                override fun getNoBackupFilesDir(): File = directory
            }
        val worker = ProbeWorker(sudachiDictionaryFiles(privateContext))
        val factory =
            object : ViewModelProvider.Factory {
                override fun <T : ViewModel> create(modelClass: Class<T>): T =
                    modelClass.cast(ProbeOwner(worker))!!
            }
        lateinit var owner: ProbeOwner
        try {
            compose.onNodeWithTag("screen_title").assertTextEquals("Library")
            compose.activityRule.scenario.onActivity {
                owner = ViewModelProvider(it, factory)[ProbeOwner::class.java]
            }
            assertFalse(directory.exists()) // Constructing/retaining a worker is lazy.
            memory("compose_baseline")

            // A failed native load must leave this worker available for a valid retry.
            assertTrue(directory.mkdirs())
            val invalid = File(directory, "invalid.dic")
            invalid.writeText("not the pinned dictionary")
            assertEquals(
                -4L,
                worker.call {
                    FileInputStream(invalid).use { input ->
                        ParcelFileDescriptor.dup(input.fd).use { native.nativeOpen(it.fd) }
                    }
                },
            )
            assertTrue(invalid.delete()) // Rejected mapping is already gone.

            val started = SystemClock.elapsedRealtimeNanos()
            val generation = worker.open()
            log("extract_verify_map_ms=${elapsed(started)} generation=$generation")
            memory("mapped")
            assertTrue(generation > 0)
            assertMapped(directory, true)
            // The Kotlin stream and duplicate FD closed before any of these calls.
            for ((fixture, tokens) in listOf(0 to 3L, 1 to 2L, 2 to 1L)) {
                assertEquals(tokens, worker.validate(fixture))
            }
            assertTrue(worker.validate(3) > 0) // Mixed/OOV/normalization/NUL/Unicode.
            assertEquals(-1L, worker.validate(99))
            assertEquals(-3L, SudachiAppProbe().nativeValidate(generation, 1))
            assertEquals(-3L, SudachiAppProbe().nativeClose(generation))
            worker.call {
                files.open().use { input ->
                    ParcelFileDescriptor.dup(input.fd).use {
                        assertEquals(-2L, native.nativeOpen(it.fd))
                    }
                }
            }
            measureWorkload(worker, 4, 330L, 20)
            measureWorkload(worker, 5, 16002L, 3)
            memory("analyzed")

            compose.activityRule.scenario.recreate()
            compose.activityRule.scenario.onActivity {
                assertSame(owner, ViewModelProvider(it, factory)[ProbeOwner::class.java])
            }
            compose.onNodeWithTag("screen_title").assertTextEquals("Library")
            assertEquals(generation, worker.open())
            assertEquals(2L, worker.validate(1))
            memory("recreated")

            // Actual Activity stop/resume; this does not simulate kernel memory pressure.
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            memory("background")
            assertEquals(generation, worker.open())
            assertEquals(330L, worker.validate(4))
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            assertEquals(generation, worker.open())
            assertEquals(2L, worker.validate(1))
            memory("resumed")
            assertEquals(1, worker.call { initializations })

            // Finishing clears the retained owner and queues native drop on its worker.
            compose.activityRule.scenario.close()
            assertEquals(0L, worker.closed.get(30, TimeUnit.SECONDS))
            assertTrue(worker.executor.awaitTermination(30, TimeUnit.SECONDS))
            assertMapped(directory, false)
            memory("closed")

            // Fresh worker re-verifies cached publication and initializes a new generation.
            val reopened = ProbeWorker(sudachiDictionaryFiles(privateContext))
            try {
                val reopenStarted = SystemClock.elapsedRealtimeNanos()
                val next = reopened.open()
                assertTrue(next > generation)
                log("verified_reopen_map_ms=${elapsed(reopenStarted)} generation=$next")
                assertEquals(-3L, reopened.call { native.nativeValidate(generation, 1) })
                assertEquals(2L, reopened.validate(1))
            } finally {
                reopened.close()
                assertEquals(0L, reopened.closed.get(30, TimeUnit.SECONDS))
                assertTrue(reopened.executor.awaitTermination(30, TimeUnit.SECONDS))
                assertMapped(directory, false)
            }
            log("validation=PASS")
        } finally {
            // All mappings must drop before removing test-owned dictionary files.
            worker.close()
            worker.closed.get(30, TimeUnit.SECONDS)
            assertTrue(worker.executor.awaitTermination(30, TimeUnit.SECONDS))
            assertTrue(!directory.exists() || directory.deleteRecursively())
        }
    }

    private fun measureWorkload(worker: ProbeWorker, fixture: Int, tokens: Long, runs: Int) {
        val timings =
            (1..runs)
                .map {
                    val started = SystemClock.elapsedRealtimeNanos()
                    assertEquals(tokens, worker.validate(fixture))
                    (SystemClock.elapsedRealtimeNanos() - started) / 1_000_000.0
                }
                .sorted()
        log("fixture=$fixture runs=$runs tokens=$tokens median_ms=${timings[runs / 2]}")
    }

    private fun memory(stage: String) {
        val memory = Debug.MemoryInfo()
        Debug.getMemoryInfo(memory)
        log("stage=$stage pss_kib=${memory.totalPss} private_dirty_kib=${memory.totalPrivateDirty}")
        for (line in File("/proc/self/status").readLines()) {
            if (
                listOf("VmRSS:", "VmHWM:", "RssAnon:", "RssFile:", "VmSwap:").any(line::startsWith)
            ) {
                log("stage=$stage $line")
            }
        }
    }

    private fun assertMapped(directory: File, expected: Boolean) {
        assertEquals(
            expected,
            File("/proc/self/maps").useLines { lines ->
                // /proc may name /data/data while Context uses /data/user/0.
                // The unique test-directory suffix identifies the same inode.
                lines.any { it.contains("/${directory.name}/sudachi/full-") }
            },
        )
    }

    private fun elapsed(start: Long) = (SystemClock.elapsedRealtimeNanos() - start) / 1_000_000.0

    private fun log(message: String) = Log.i("SudachiAppProbe", message)
}

/** Test-only lifecycle owner: the production Rust EffectRouter remains unchanged. */
private class ProbeOwner(val worker: ProbeWorker) : ViewModel() {
    override fun onCleared() = worker.close()
}

private class ProbeWorker(val files: VerifiedDictionaryFiles) : AutoCloseable {
    val executor = Executors.newSingleThreadExecutor()
    val closed = CompletableFuture<Long>()
    val native by lazy { SudachiAppProbe() }
    var initializations = 0
    private var generation = 0L

    fun <T> call(operation: ProbeWorker.() -> T): T =
        executor.submit(Callable { operation() }).get(60, TimeUnit.SECONDS)

    fun open(): Long = call {
        if (generation == 0L) {
            files.open().use { input ->
                ParcelFileDescriptor.dup(input.fd).use {
                    generation = native.nativeOpen(it.fd)
                    check(generation > 0) { "Native initialization failed: $generation" }
                }
            }
            initializations++
        }
        generation
    }

    fun validate(fixture: Int): Long = call { native.nativeValidate(generation, fixture) }

    @Synchronized
    override fun close() {
        if (executor.isShutdown) return
        executor.execute {
            try {
                val result = if (generation > 0) native.nativeClose(generation) else 0L
                closed.complete(result)
            } catch (failure: Throwable) {
                closed.completeExceptionally(failure)
            }
        }
        executor.shutdown()
    }
}
