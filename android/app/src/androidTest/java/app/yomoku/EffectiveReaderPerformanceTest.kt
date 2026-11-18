package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.*
import java.io.File
import java.security.MessageDigest
import java.util.Locale
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Opt-in measurements of the production JNI/FIFO/codecs, with no Activity or reader clock. */
@RunWith(AndroidJUnit4::class)
class EffectiveReaderPerformanceTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext

    private data class Sample(val view: ViewModel, val elapsedMs: Double, val mainMs: Double)

    private fun emit(line: String) {
        instrumentation.sendStatus(0, Bundle().apply { putString("stream", "$line\n") })
    }

    // Timestamp in the callback, so waking the instrumentation thread adds no polling interval.
    private fun command(core: CoreSession, event: Event? = null): Sample {
        val done = CompletableFuture<Pair<ViewModel, Long>>()
        var start = 0L
        var mainMs = 0.0
        instrumentation.runOnMainSync {
            start = SystemClock.elapsedRealtimeNanos()
            core.onViewChanged = {
                if (!core.view.storageBusy) {
                    done.complete(core.view to SystemClock.elapsedRealtimeNanos())
                }
            }
            if (event != null) core.dispatch(event)
            mainMs = (SystemClock.elapsedRealtimeNanos() - start) / 1e6
            if (!core.view.storageBusy) {
                done.complete(core.view to SystemClock.elapsedRealtimeNanos())
            }
        }
        val (view, end) =
            try {
                done.get(180, TimeUnit.SECONDS)
            } finally {
                instrumentation.runOnMainSync { core.onViewChanged = null }
            }
        assertNull(view.documentError)
        assertFalse(view.completionSavePending)
        return Sample(view, (end - start) / 1e6, mainMs)
    }

    private fun report(fixture: String, phase: String, metric: String, samples: List<Sample>) {
        fun values(select: (Sample) -> Double): String {
            val sorted = samples.map(select).sorted()
            val median = (sorted[(sorted.size - 1) / 2] + sorted[sorted.size / 2]) / 2
            return listOf(sorted.first(), median, sorted.last()).joinToString(",") {
                String.format(Locale.ROOT, "%.3f", it)
            }
        }
        emit(
            "MEASURE,$fixture,$phase,$metric,${samples.size},${values { it.elapsedMs }},${values { it.mainMs }}"
        )
        emit("SAMPLES,$fixture,$phase,$metric," + samples.joinToString(",") { "${it.elapsedMs}" })
    }

    private fun measure(
        fixture: String,
        phase: String,
        metric: String,
        iterations: Int,
        action: () -> Sample,
    ) {
        action() // One warmup, excluded from the distribution.
        report(fixture, phase, metric, List(iterations) { action() })
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

    private fun loadTokens(core: CoreSession): Sample {
        val before = core.view.effectiveReaderTokens?.identity
        val reader = checkNotNull(core.view.reader)
        val key = ReaderTokenKey(reader.documentId, reader.anchor, core.view.readerAnalysis!!)
        return command(core, Event.LoadReaderTokens(key)).also {
            val page = checkNotNull(it.view.effectiveReaderTokens)
            assertEquals(key, page.identity.base)
            assertNotEquals(before, page.identity) // A cached/no-op dispatch is not a sample.
            assertTrue(page.tokens.isNotEmpty())
        }
    }

    private fun phase(
        core: CoreSession,
        database: File,
        fixture: String,
        phase: String,
        iterations: Int,
        id: DocumentId,
    ) {
        measure(fixture, phase, "open_saved_position", iterations) {
            command(core, Event.OpenReader(id)).also {
                assertEquals(id, it.view.reader?.documentId)
            }
        }
        val before = digest(database)
        measure(fixture, phase, "effective_page", iterations) { loadTokens(core) }
        assertEquals(before, digest(database))
        val page = checkNotNull(core.view.effectiveReaderTokens)
        val selection = EffectiveReaderSelection(page.identity, page.tokens.first().segmentIndex)
        // The first word lookup persists missing A/B component analyses. Prepare
        // them before checking that steady-state reads leave the database intact.
        report(
            fixture,
            "setup",
            "first_word_prepare",
            listOf(command(core, Event.LoadEffectiveReaderWord(selection))),
        )
        val wordBefore = digest(database)
        val fullBeforeWord = dictionaryCalls.get()
        measure(fixture, phase, "word_lookup", iterations) {
            command(core, Event.LoadEffectiveReaderWord(selection)).also {
                assertNotNull(it.view.effectiveReaderWord?.details)
                assertNull(it.view.effectiveReaderWord?.error)
            }
        }
        assertEquals(fullBeforeWord, dictionaryCalls.get())
        if (core.view.reader!!.images.isNotEmpty()) {
            measure(fixture, phase, "page_image_read_decode", iterations) { loadImages(core) }
        }
        assertEquals(wordBefore, digest(database))
        measure(fixture, phase, "word_status", iterations) {
            val details = checkNotNull(core.view.effectiveReaderWord?.details)
            val entry = checkNotNull(details.token.vocabulary)
            val target =
                if (entry.status == VocabularyStatus.LEARNING) VocabularyStatus.FAMILIAR
                else VocabularyStatus.LEARNING
            command(core, Event.SetEffectiveReaderStatus(selection, entry.id, target, 1_000)).also {
                assertNull(it.view.effectiveReaderStatus?.error)
                assertEquals(
                    target,
                    it.view.effectiveReaderWord?.details?.token?.vocabulary?.status,
                )
                assertTrue(
                    it.view.effectiveReaderTokens!!
                        .tokens
                        .filter { token -> token.surface == details.token.surface }
                        .all { token -> token.vocabulary?.status == target }
                )
            }
        }
        command(core, Event.CloseDocument)
        val libraryBefore = digest(database)
        measure(fixture, phase, "effective_library", iterations) {
            command(core, Event.LoadLibraryWordDistributions).also {
                assertNotNull(it.view.libraryWordDistributions!!.single().counts)
            }
        }
        assertEquals(libraryBefore, digest(database))
        command(core, Event.OpenReader(id))
        loadTokens(core)
    }

    private fun loadImages(core: CoreSession): Sample {
        val reader = checkNotNull(core.view.reader)
        val assets = reader.images.map { it.asset }.distinct()
        val done = CompletableFuture<Long>()
        val images = mutableListOf<DecodedEpubImage>()
        val requests = mutableListOf<AutoCloseable>()
        var start = 0L
        var mainMs = 0.0
        instrumentation.runOnMainSync {
            start = SystemClock.elapsedRealtimeNanos()
            for (asset in assets) {
                requests +=
                    core.images.load(
                        EpubImageKey(reader.documentId.value, asset),
                        maxOf(1, EPUB_PAGE_PIXELS / assets.size),
                    ) { image ->
                        if (image == null)
                            done.completeExceptionally(AssertionError("Image decode failed"))
                        else {
                            images += image
                            if (images.size == assets.size)
                                done.complete(SystemClock.elapsedRealtimeNanos())
                        }
                    }
            }
            mainMs = (SystemClock.elapsedRealtimeNanos() - start) / 1e6
        }
        val end =
            try {
                done.get(60, TimeUnit.SECONDS)
            } finally {
                instrumentation.runOnMainSync {
                    requests.forEach { it.close() }
                    assertTrue(
                        images.sumOf { it.bitmap.allocationByteCount.toLong() } <=
                            EPUB_PAGE_PIXELS * 4L
                    )
                    images.forEach { it.bitmap.recycle() }
                }
            }
        return Sample(core.view, (end - start) / 1e6, mainMs)
    }

    private val dictionaryCalls = AtomicInteger()

    private fun fixture(name: String, bytes: ByteArray, iterations: Int) {
        val directory = File(context.noBackupFilesDir, "effective-measure-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "scratch.sqlite3")
        val actual = sudachiDictionaryProvider(context)
        val provider =
            object : DictionaryProvider by actual {
                override fun openVerifiedDictionary(): Long {
                    dictionaryCalls.incrementAndGet()
                    return actual.openVerifiedDictionary()
                }
            }
        lateinit var core: CoreSession
        instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
        try {
            command(core)
            val input = FileImportBytes(bytes)
            val event =
                when (name.substringAfterLast('.')) {
                    "epub" -> Event.ImportEpub(EpubImport(input, name))
                    "txt" -> Event.ImportTxt(TxtImport(input, name))
                    else -> error("Use a TXT or EPUB corpus")
                }
            val imported = command(core, event)
            report(name, "setup", "import", listOf(imported))
            val id = checkNotNull(imported.view.importSummary).documentId
            command(core, Event.OpenReader(id))
            val chapters =
                SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY)
                    .use { db ->
                        db.rawQuery(
                                "SELECT c.source_id, c.id, t.text FROM chapters c JOIN chapter_texts t ON t.chapter_id = c.id ORDER BY c.chapter_index",
                                null,
                            )
                            .use { rows ->
                                buildList {
                                    while (rows.moveToNext()) add(
                                        Triple(rows.getLong(0), rows.getLong(1), rows.getString(2))
                                    )
                                }
                            }
                    }
            val chapter = chapters.maxBy { it.third.codePointCount(0, it.third.length) }
            command(
                core,
                Event.SaveReadingPosition(
                    ReadingPosition(
                        SourceId(chapter.first),
                        ChapterId(chapter.second),
                        TextOffset(0),
                    )
                ),
            )
            emit(
                "FIXTURE,$name,bytes=${bytes.size},sha256=${MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }},chapters=${chapters.size},characters=${chapters.sumOf { it.third.codePointCount(0, it.third.length) }},largest_characters=${chapter.third.codePointCount(0, chapter.third.length)},database_bytes=${database.length()}"
            )
            val fullCalls = dictionaryCalls.get()
            phase(core, database, name, "mode_c", iterations, id)
            assertEquals(fullCalls, dictionaryCalls.get())
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READONLY).use { db
                ->
                for (table in listOf("reading_completions", "reading_occurrences")) {
                    db.rawQuery("SELECT count(*) FROM $table", null).use { rows ->
                        assertTrue(rows.moveToFirst())
                        assertEquals(0L, rows.getLong(0))
                    }
                }
            }
            val current = checkNotNull(core.view.reader)
            if (current.totalPages - current.pageNumber > iterations.toULong()) {
                measure(name, "mode_c", "warm_forward", iterations) {
                    val page = checkNotNull(core.view.reader)
                    val identity = checkNotNull(core.view.effectiveReaderTokens).identity
                    val sample =
                        command(
                            core,
                            Event.CompleteEffectiveReaderPage(identity, page.position, 2_000),
                        )
                    assertEquals(page.pageNumber + 1UL, sample.view.reader?.pageNumber)
                    loadTokens(core)
                    sample
                }
            }
        } finally {
            instrumentation.runOnMainSync { core.close() }
            val deadline = SystemClock.elapsedRealtime() + 30_000
            var closed = false
            while (!closed && SystemClock.elapsedRealtime() < deadline) {
                instrumentation.runOnMainSync { closed = core.isClosed }
                if (!closed) Thread.sleep(10)
            }
            assertTrue("Scratch native handle must close before removing its directory", closed)
            assertTrue(directory.deleteRecursively())
        }
    }

    @Test
    fun measureDisposableReader() {
        val args = InstrumentationRegistry.getArguments()
        assumeTrue("Opt in with -e yomokuMeasure true", args.getString("yomokuMeasure") == "true")
        val iterations = (args.getString("yomokuMeasureIterations") ?: "5").toInt()
        require(iterations in 1..100)
        emit(
            "DEVICE,${Build.FINGERPRINT},abi=${Build.SUPPORTED_ABIS.first()},sdk=${Build.VERSION.SDK_INT}"
        )
        emit(
            "COLUMNS,fixture,phase,metric,n,total_min_ms,total_median_ms,total_max_ms,dispatch_min_ms,dispatch_median_ms,dispatch_max_ms"
        )
        val names =
            args.getString("yomokuMeasureFiles").orEmpty().split(',').filter { it.isNotBlank() }
        for (name in names) {
            require(name == File(name).name && name !in listOf(".", ".."))
            val input = File(File(context.noBackupFilesDir, "effective-measure-inputs"), name)
            fixture(name, input.readBytes(), iterations)
        }
        val longText = "国家公務員と猫。東京の図書館で日本語の本を読みました。\n".repeat(2_000)
        fixture("synthetic-long.txt", longText.toByteArray(), iterations)
        fixture(
            "synthetic-chapters.epub",
            FileImportFixtures.epub(
                "status measurement",
                "<p>$longText</p>",
                secondBody = "<p>$longText</p>",
            ),
            iterations,
        )
        val images =
            (0 until 12).associate { index ->
                "images/$index.png" to
                    ("image/png" to
                        rasterImageBytes(
                            2048,
                            1536,
                            color = android.graphics.Color.rgb(index * 20, 80, 160),
                        ))
            }
        val body =
            (0 until 12).joinToString("") {
                "<p>国家公務員と猫。😀𠮷 mixed text <ruby>東京<rt>とうきょう</rt></ruby><img src='images/$it.png'/></p>"
            }
        fixture(
            "synthetic-images.epub",
            FileImportFixtures.epub("measurement", body, images, body),
            iterations,
        )
    }
}
