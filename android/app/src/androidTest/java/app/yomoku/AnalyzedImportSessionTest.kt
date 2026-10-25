package app.yomoku

import android.database.sqlite.SQLiteDatabase
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.TextOffset
import app.yomoku.shared.ViewModel
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnalyzedImportSessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun awaitIdle(core: CoreSession): ViewModel {
        val deadline = System.nanoTime() + 30_000_000_000L
        while (true) {
            lateinit var view: ViewModel
            instrumentation.runOnMainSync { view = core.view }
            if (!view.storageBusy) return view
            check(System.nanoTime() < deadline) { "Import callback timed out" }
            Thread.sleep(10)
        }
    }

    private fun dispatch(core: CoreSession, event: Event): ViewModel {
        instrumentation.runOnMainSync { core.dispatch(event) }
        return awaitIdle(core)
    }

    private fun awaitUnmapped(directory: File) {
        val deadline = System.nanoTime() + 10_000_000_000L
        val suffix = "${directory.parentFile!!.name}/${directory.name}/"
        while (File("/proc/self/maps").readText().contains(suffix)) {
            check(System.nanoTime() < deadline) { "Analyzer mapping retained after release" }
            Thread.sleep(10)
        }
    }

    @Test
    fun productionJniImportsAreAtomicDeduplicatedLazyAndRetryable() {
        val context = instrumentation.targetContext
        val directory = File(context.noBackupFilesDir, "analyzed-import-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val dictionaryDirectory = File(directory, "dictionary")
        val files =
            VerifiedDictionaryFiles(
                dictionaryDirectory,
                330_506_339L,
                "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1",
            ) {
                context.assets.open("sudachi/system_full.dic")
            }
        val publisher = sudachiDictionaryProvider(files)
        val calls = AtomicInteger()
        val workers = mutableSetOf<Int>()
        val fail = AtomicBoolean(true)
        val provider =
            object : DictionaryProvider {
                override fun verifiedWordDictionaryPath(): String = ""

                override fun openVerifiedDictionary(): Long {
                    check(Looper.myLooper() != Looper.getMainLooper())
                    synchronized(workers) { workers.add(android.os.Process.myTid()) }
                    calls.incrementAndGet()
                    return if (fail.get()) 0L else publisher.openVerifiedDictionary()
                }
            }
        val database = File(directory, "reader.sqlite3")
        var active: CoreSession? = null
        try {
            lateinit var core: CoreSession
            instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
            active = core
            assertTrue(awaitIdle(core).documents.isEmpty())
            assertEquals(0, calls.get())
            assertFalse(dictionaryDirectory.exists())
            val text = "猫と犬。猫。English 123 📖\r\n"
            val failed = dispatch(core, Event.CreatePaste(text))
            assertEquals(DocumentError.UNAVAILABLE, failed.documentError)
            assertTrue(failed.documents.isEmpty())
            assertNull(failed.importSummary)
            fail.set(false)
            SQLiteDatabase.openDatabase(database.path, null, SQLiteDatabase.OPEN_READWRITE).use { db
                ->
                fun count(table: String): Long =
                    db.rawQuery("SELECT count(*) FROM $table", null).use {
                        assertTrue(it.moveToFirst())
                        it.getLong(0)
                    }
                val tables =
                    listOf(
                        "sources",
                        "chapters",
                        "documents",
                        "chapter_texts",
                        "chapter_analyses",
                        "analyzed_tokens",
                        "vocabulary",
                        "vocabulary_status_events",
                        "paste_content_hashes",
                        "reading_completions",
                    )
                for (table in tables) assertEquals(table, 0L, count(table))
                db.execSQL(
                    "CREATE TRIGGER fail_import BEFORE INSERT ON analyzed_tokens BEGIN SELECT RAISE(ABORT, 'test rollback'); END"
                )
                assertEquals(
                    DocumentError.UNAVAILABLE,
                    dispatch(core, Event.CreatePaste(text)).documentError,
                )
                for (table in tables) assertEquals(table, 0L, count(table))
                assertEquals(2, calls.get())
                db.execSQL("DROP TRIGGER fail_import")
                val created = dispatch(core, Event.CreatePaste(text))
                assertNull(created.documentError)
                assertEquals(ImportOutcome.CREATED, created.importSummary!!.outcome)
                assertEquals(ContentRoute.MAIN, created.contentRoute)
                assertNull(created.reader)
                assertEquals(2, calls.get()) // SQL rollback reused healthy analyzer
                assertEquals(1L, count("chapter_analyses"))
                assertEquals(3L, count("vocabulary")) // exact 猫, と, 犬; repeated 猫 once
                assertEquals(3L, count("vocabulary_status_events"))
                assertEquals(0L, count("reading_completions"))
                db.rawQuery("SELECT split_mode FROM chapter_analyses", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals("b", it.getString(0))
                }
                val id = created.documents.single().documentId
                val reader = dispatch(core, Event.OpenReader(id)).reader!!
                val position = reader.position.copy(offset = TextOffset(3))
                dispatch(core, Event.SaveReadingPosition(position))
                dispatch(core, Event.CloseDocument)
                // Test-only manual status fixture; duplicate must preserve state and history.
                db.execSQL("UPDATE vocabulary SET status = 'learning' WHERE surface = '猫'")
                db.execSQL(
                    "INSERT INTO vocabulary_status_events(semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms) SELECT 1, id, 'new', 'learning', 'manual', 1 FROM vocabulary WHERE surface = '猫'"
                )
                val tokenCount = count("analyzed_tokens")
                val duplicate = dispatch(core, Event.CreatePaste(text))
                assertEquals(ImportOutcome.EXISTING, duplicate.importSummary!!.outcome)
                assertEquals(id, duplicate.documents.single().documentId)
                assertTrue(duplicate.documents.single().progressPercent > 0UL)
                assertNull(duplicate.reader)
                assertEquals(2, calls.get())
                assertEquals(tokenCount, count("analyzed_tokens"))
                assertEquals(4L, count("vocabulary_status_events"))
                assertEquals(1L, count("documents"))
                db.rawQuery("SELECT status FROM vocabulary WHERE surface = '猫'", null).use {
                    assertTrue(it.moveToFirst())
                    assertEquals("learning", it.getString(0))
                }
                instrumentation.runOnMainSync { core.setForeground(false) }
                dispatch(core, Event.RefreshLibrary) // FIFO barrier after release
                awaitUnmapped(dictionaryDirectory)
                // Hidden work is accepted atomically, then releases its own cache.
                assertEquals(
                    ImportOutcome.EXISTING,
                    dispatch(core, Event.CreatePaste(text)).importSummary!!.outcome,
                )
                assertEquals(3, calls.get())
                awaitUnmapped(dictionaryDirectory)
                instrumentation.runOnMainSync { core.setForeground(true) }
                dispatch(core, Event.RefreshLibrary)
                assertEquals(3, calls.get())
                assertEquals(position, dispatch(core, Event.OpenReader(id)).reader!!.position)
                dispatch(core, Event.CloseDocument)
                assertEquals(
                    ImportOutcome.EXISTING,
                    dispatch(core, Event.CreatePaste(text)).importSummary!!.outcome,
                )
                assertEquals(4, calls.get())
                assertEquals(1, synchronized(workers) { workers.size })
                // Accepted work drains after close; Android UI never joins the worker.
                db.beginTransaction()
                try {
                    instrumentation.runOnMainSync {
                        core.dispatch(Event.CreatePaste("別の猫。"))
                        core.close()
                    }
                    active = null
                } finally {
                    db.endTransaction()
                }
                awaitUnmapped(dictionaryDirectory)
                assertEquals(2L, count("documents"))
                assertEquals(2L, count("chapter_analyses"))
                assertEquals(0L, count("reading_completions"))
            }
            instrumentation.runOnMainSync { core.close() }
            active = null
            awaitUnmapped(dictionaryDirectory)
        } finally {
            instrumentation.runOnMainSync { active?.close() }
            awaitUnmapped(dictionaryDirectory)
            assertTrue(directory.deleteRecursively())
        }
    }
}
