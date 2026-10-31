package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Build
import android.os.Bundle
import android.os.Debug
import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.view.FrameMetrics
import android.view.Window
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Opt-in production JNI/SQLite/Compose measurements; never opens the user's database. */
@RunWith(AndroidJUnit4::class)
class VocabularyPerformanceTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var core: CoreSession
    private var measureRender = false

    private fun emit(line: String) {
        instrumentation.sendStatus(0, Bundle().apply { putString("stream", "$line\n") })
    }

    private fun idle() {
        compose.waitForIdle()
        compose.waitUntil(90_000) { !core.view.storageBusy }
        compose.waitForIdle()
        assertNull(core.view.documentError)
        assertNull(core.view.vocabulary.error)
        assertNull(core.view.vocabulary.bulkError)
    }

    private fun dispatch(event: Event): Double {
        var duration = 0L
        compose.runOnUiThread {
            val start = SystemClock.elapsedRealtimeNanos()
            core.dispatch(event)
            duration = SystemClock.elapsedRealtimeNanos() - start
        }
        idle()
        return duration / 1e6
    }

    private fun digest(file: File): String {
        val hash = MessageDigest.getInstance("SHA-256")
        file.inputStream().buffered().use { input ->
            val buffer = ByteArray(64 * 1024)
            while (true) {
                val size = input.read(buffer)
                if (size < 0) break
                hash.update(buffer, 0, size)
            }
        }
        return hash.digest().joinToString("") { "%02x".format(it) }
    }

    private fun seed(database: File, entries: Int) {
        SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use { db ->
            db.setForeignKeyConstraintsEnabled(true)
            db.beginTransaction()
            try {
                db.compileStatement(
                        "INSERT INTO vocabulary(id,surface,status,registered_at_unix_ms) VALUES (?,?,'new',1)"
                    )
                    .use { word ->
                        db.compileStatement(
                                "INSERT INTO vocabulary_status_events(semantics_version,vocabulary_id,old_status,new_status,cause,occurred_at_unix_ms) VALUES(1,?,NULL,'new','registration',1)"
                            )
                            .use { event ->
                                repeat(entries) { index ->
                                    val id = index + 1L
                                    word.bindLong(1, id)
                                    word.bindString(2, "測定-" + id.toString().padStart(6, '0'))
                                    word.executeInsert()
                                    event.bindLong(1, id)
                                    event.executeInsert()
                                }
                            }
                    }
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
            }
        }
    }

    private data class Memory(val javaBytes: Long, val nativeBytes: Long, val rssKib: Long)

    private fun memory(): Memory {
        val runtime = Runtime.getRuntime()
        val rss =
            File("/proc/self/status").useLines { lines ->
                lines.first { it.startsWith("VmRSS:") }.split(Regex("\\s+"))[1].toLong()
            }
        return Memory(
            runtime.totalMemory() - runtime.freeMemory(),
            Debug.getNativeHeapAllocatedSize(),
            rss,
        )
    }

    private inner class MemorySampler : AutoCloseable {
        val samples = mutableListOf(memory())
        private val failure = AtomicReference<Throwable>()
        @Volatile private var running = true
        private val thread = Thread {
            try {
                while (running) {
                    Thread.sleep(25)
                    samples.add(memory())
                }
            } catch (_: InterruptedException) {
                // Explicit stop. The final sample is taken after join.
            } catch (error: Throwable) {
                failure.set(error)
            }
        }
            .apply { start() }

        override fun close() {
            running = false
            thread.interrupt()
            thread.join(5_000)
            check(!thread.isAlive)
            failure.get()?.let { throw it }
            samples.add(memory())
        }
    }

    private data class Frame(val vsync: Long, val duration: Long, val droppedReports: Int)

    private inner class Frames : AutoCloseable {
        private val thread = HandlerThread("vocabulary-frame-metrics").apply { start() }
        private val handler = Handler(thread.looper)
        private val frames = mutableListOf<Frame>()
        private val listener = Window.OnFrameMetricsAvailableListener { _, metrics, dropped ->
            frames.add(
                Frame(
                    metrics.getMetric(FrameMetrics.INTENDED_VSYNC_TIMESTAMP),
                    metrics.getMetric(FrameMetrics.TOTAL_DURATION),
                    dropped,
                )
            )
        }

        init {
            compose.runOnUiThread {
                compose.activity.window.addOnFrameMetricsAvailableListener(listener, handler)
            }
        }

        fun between(start: Long, end: Long): List<Frame> {
            val ready = CountDownLatch(1)
            var result = emptyList<Frame>()
            handler.post {
                result = frames.filter { it.vsync in start..end && it.duration >= 0 }
                ready.countDown()
            }
            check(ready.await(5, TimeUnit.SECONDS))
            return result
        }

        override fun close() {
            compose.runOnUiThread {
                compose.activity.window.removeOnFrameMetricsAvailableListener(listener)
            }
            thread.quitSafely()
            thread.join(5_000)
            check(!thread.isAlive)
        }
    }

    private fun measure(
        frames: Frames,
        entries: Int,
        iteration: Int,
        metric: String,
        action: () -> Double,
    ) {
        val renders = mutableListOf<CoreRenderMeasurement>()
        val requests = mutableListOf<CoreRequestsMeasurement>()
        if (measureRender)
            compose.runOnUiThread {
                core.onRenderMeasured = { renders.add(it) }
                core.onRequestsMeasured = { requests.add(it) }
            }
        val sampler = MemorySampler()
        // FrameMetrics uses the System.nanoTime timebase, unlike elapsedRealtime after suspend.
        val start = System.nanoTime()
        val dispatchMs: Double
        val end: Long
        try {
            dispatchMs = action()
            idle()
            end = System.nanoTime()
        } finally {
            sampler.close()
            if (measureRender)
                compose.runOnUiThread {
                    core.onRenderMeasured = null
                    core.onRequestsMeasured = null
                }
        }
        val measured = frames.between(start, end)
        if (iteration < 0) return // Full scenario warmup, excluded from output.
        val durations = measured.map { it.duration / 1e6 }.sorted()
        val median =
            if (durations.isEmpty()) 0.0
            else (durations[(durations.size - 1) / 2] + durations[durations.size / 2]) / 2
        val memory = sampler.samples
        emit(
            "MEASURE,$entries,$iteration,$metric,${core.view.vocabulary.rows.size},${(end - start) / 1e6},$dispatchMs,${durations.size},$median,${durations.lastOrNull() ?: 0.0},${durations.count { it > 1000.0 / 60 }},${measured.sumOf { it.droppedReports }},${memory.maxOf { it.javaBytes }},${memory.maxOf { it.nativeBytes }},${memory.maxOf { it.rssKib }},${memory.size}"
        )
        emit("FRAMES,$entries,$iteration,$metric," + durations.joinToString(","))
        if (measureRender) {
            if (metric != "scroll_loaded_rows") assertTrue(renders.isNotEmpty())
            if (metric == "select_all") assertTrue(renders.any { it.selectedIds == entries })
            if (metric == "confirmation") assertTrue(renders.any { it.confirmationIds == entries })
            if (metric == "equal_confirmation") {
                assertTrue(requests.isEmpty())
                assertEquals(1, renders.size)
                assertEquals(CoreRenderSource.REPLAY, renders.single().source)
                assertFalse(renders.single().published)
                assertEquals(entries, renders.single().selectedIds)
                assertEquals(entries, renders.single().confirmationIds)
            }
            renders.forEachIndexed { index, render ->
                assertTrue(render.nativeViewNanos >= 0)
                assertTrue(render.decodeNanos >= 0)
                assertTrue(render.publishNanos >= 0)
                assertTrue(render.payloadBytes > 0)
                emit(
                    "RENDER,$entries,$iteration,$metric,$index,${render.source.name.lowercase()},${render.nativeViewNanos / 1e6},${render.decodeNanos / 1e6},${render.publishNanos / 1e6},${render.payloadBytes},${render.loadedRows},${render.selectedIds},${render.confirmationIds},${render.published}"
                )
            }
            requests.forEachIndexed { index, request ->
                assertTrue(request.requestDecodeNanos >= 0)
                assertTrue(request.payloadBytes > 0)
                assertTrue(request.requestCount >= 0)
                when (request.source) {
                    CoreRenderSource.DISPATCH -> {
                        assertTrue(request.eventEncodeNanos!! >= 0)
                        assertTrue(request.nativeUpdateNanos!! >= 0)
                    }
                    CoreRenderSource.CALLBACK -> {
                        assertNull(request.eventEncodeNanos)
                        assertNull(request.nativeUpdateNanos)
                    }
                    CoreRenderSource.REPLAY -> error("Snapshot replay has no requests")
                }
                emit(
                    "REQUEST,$entries,$iteration,$metric,$index,${request.source.name.lowercase()},${request.eventEncodeNanos?.let { it / 1e6 } ?: ""},${request.nativeUpdateNanos?.let { it / 1e6 } ?: ""},${request.requestDecodeNanos / 1e6},${request.payloadBytes},${request.requestCount}"
                )
            }
        }
    }

    private fun scrollTo(id: Long) {
        compose.onNodeWithTag("vocabulary_scroll").performScrollToKey(id)
        idle()
    }

    @Test
    fun measureDisposableVocabulary() {
        val args = InstrumentationRegistry.getArguments()
        assumeTrue("Opt in with -e yomokuMeasure true", args.getString("yomokuMeasure") == "true")
        measureRender = args.getString("yomokuMeasureRender") == "true"
        val entries = (args.getString("yomokuVocabularyEntries") ?: "10000").toInt()
        val loaded = (args.getString("yomokuVocabularyLoaded") ?: "1000").toInt()
        val iterations = (args.getString("yomokuMeasureIterations") ?: "5").toInt()
        require(entries in 200..100_000)
        require(loaded in 200..entries && loaded % 100 == 0)
        require(iterations in 1..100)
        val directory =
            File(compose.activity.noBackupFilesDir, "vocabulary-measure-${System.nanoTime()}")
        check(directory.mkdir())
        val database = File(directory, "scratch.sqlite3")
        val calls = AtomicInteger()
        val provider =
            object : DictionaryProvider by sudachiDictionaryProvider(compose.activity) {
                override fun openVerifiedDictionary(): Long {
                    calls.incrementAndGet()
                    error("Vocabulary reads must never load the analyzer dictionary")
                }
            }
        var created = false
        try {
            compose.runOnUiThread { core = CoreSession(database.path, provider) }
            created = true
            idle()
            compose.runOnUiThread { core.close() }
            assertTrue(core.isClosed)
            seed(database, entries)
            val before = digest(database)
            compose.runOnUiThread { core = CoreSession(database.path, provider) }
            idle()
            compose.setContent {
                YomokuTheme { YomokuApp(core.view, core.sessionKey, core::dispatch) }
            }
            dispatch(Event.SelectDestination(Destination.VOCABULARY))
            compose.waitUntil(90_000) { core.view.vocabulary.matchingCount != null }
            idle()
            emit(
                "DEVICE,${Build.FINGERPRINT},abi=${Build.SUPPORTED_ABIS.first()},sdk=${Build.VERSION.SDK_INT},heap_max_bytes=${Runtime.getRuntime().maxMemory()},database_bytes=${database.length()}"
            )
            emit(
                "COLUMNS,entries,iteration,metric,loaded_rows,idle_ms,dispatch_ms,frames,frame_median_ms,frame_max_ms,frames_over_16_667_ms,dropped_reports,java_peak_bytes,native_peak_bytes,rss_peak_kib,memory_samples"
            )
            if (measureRender) {
                emit(
                    "RENDER_COLUMNS,entries,iteration,metric,index,source,native_view_ms,decode_ms,publish_ms,payload_bytes,loaded_rows,selected_ids,confirmation_ids,published"
                )
                emit(
                    "REQUEST_COLUMNS,entries,iteration,metric,index,source,event_encode_ms,native_update_ms,request_decode_ms,payload_bytes,request_count"
                )
            }
            Frames().use { frames ->
                for (iteration in -1 until iterations) {
                    // Change the literal prefix each time; both match the complete fixture.
                    measure(frames, entries, iteration, "first_page") {
                        dispatch(
                            Event.SetVocabularyQuery(
                                core.view.vocabulary.query.copy(
                                    prefix = if (iteration % 2 == 0) "測定" else "測定-"
                                )
                            )
                        )
                    }
                    assertEquals(entries.toULong(), core.view.vocabulary.matchingCount)
                    assertEquals(100, core.view.vocabulary.rows.size)
                    measure(frames, entries, iteration, "append_100_to_200") {
                        scrollTo(core.view.vocabulary.rows.last().id.value)
                        0.0 // Scroll input has no single native dispatch interval.
                    }
                    assertEquals(200, core.view.vocabulary.rows.size)
                    while (core.view.vocabulary.rows.size < loaded) scrollTo(
                        core.view.vocabulary.rows.last().id.value
                    )
                    assertEquals(loaded, core.view.vocabulary.rows.size)
                    assertEquals(
                        (entries downTo entries - loaded + 1).map { VocabularyId(it.toLong()) },
                        core.view.vocabulary.rows.map { it.id },
                    )
                    assertTrue(
                        core.view.vocabulary.rows.all {
                            it.status == VocabularyStatus.NEW && it.encounteredCount == 0UL
                        }
                    )
                    // Stop short of the last row so this scroll cannot append another page.
                    measure(frames, entries, iteration, "scroll_loaded_rows") {
                        scrollTo(core.view.vocabulary.rows.first().id.value)
                        scrollTo(core.view.vocabulary.rows[loaded - 20].id.value)
                        0.0
                    }
                    assertEquals(loaded, core.view.vocabulary.rows.size)
                    compose.onNodeWithTag("vocabulary_scroll").performScrollToIndex(0)
                    idle()
                    measure(frames, entries, iteration, "select_all") {
                        dispatch(Event.SelectAllVocabulary)
                    }
                    assertEquals(
                        (1L..entries.toLong()).map(::VocabularyId),
                        core.view.vocabulary.selection,
                    )
                    compose
                        .onNodeWithTag("vocabulary_selected_count")
                        .assertTextEquals("Selected words: $entries")
                    measure(frames, entries, iteration, "confirmation") {
                        dispatch(Event.RequestVocabularyBulkAction(VocabularyBulkAction.Delete))
                    }
                    compose.onNodeWithTag("vocabulary_bulk_dialog").assertIsDisplayed()
                    assertEquals(
                        core.view.vocabulary.selection,
                        core.view.vocabulary.confirmation!!.ids,
                    )
                    if (iteration >= 0)
                        emit(
                            "PAYLOAD,$entries,$iteration,${core.view.vocabulary.rows.size},${core.view.bincodeSerialize().size}"
                        )
                    if (measureRender) {
                        val frozenView = core.view
                        measure(frames, entries, iteration, "equal_confirmation") {
                            var duration = 0L
                            compose.runOnUiThread {
                                val start = SystemClock.elapsedRealtimeNanos()
                                core.measureCurrentView()
                                duration = SystemClock.elapsedRealtimeNanos() - start
                            }
                            duration / 1e6
                        }
                        assertSame(frozenView, core.view)
                        compose.onNodeWithTag("vocabulary_bulk_dialog").assertIsDisplayed()
                    }
                    dispatch(Event.CancelVocabularyBulkAction)
                    dispatch(Event.CancelVocabularySelection)
                }
            }
            assertEquals(0, calls.get())
            assertEquals(before, digest(database))
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { db
                ->
                db.rawQuery("PRAGMA integrity_check", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals("ok", it.getString(0))
                }
                db.rawQuery("PRAGMA foreign_key_check", null).use { assertFalse(it.moveToFirst()) }
                for (table in
                    listOf("reading_completions", "reading_occurrences", "pending_reading_time")) {
                    db.rawQuery("SELECT count(*) FROM $table", null).use {
                        assertTrue(it.moveToFirst())
                        assertEquals(0L, it.getLong(0))
                    }
                }
            }
            emit(
                "GUARDS,$entries,unchanged_sha256=$before,analyzer_calls=${calls.get()},integrity=ok"
            )
        } finally {
            if (created) {
                compose.runOnUiThread { core.close() }
                assertTrue("Close scratch native handle before deleting files", core.isClosed)
            }
            assertTrue(directory.deleteRecursively())
            assertFalse(directory.exists())
        }
    }
}
