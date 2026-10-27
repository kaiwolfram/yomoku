package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.PageLength
import app.yomoku.shared.ReaderRuby
import app.yomoku.shared.ReaderStatusChange
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.TextEncoding
import app.yomoku.shared.TextOffset
import app.yomoku.shared.TextRange
import app.yomoku.shared.TxtImport
import app.yomoku.shared.ViewModel
import app.yomoku.shared.VocabularyStatus
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DocumentSessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun awaitIdle(core: CoreSession): ViewModel {
        val deadline = System.nanoTime() + 30_000_000_000L
        while (true) {
            lateinit var view: ViewModel
            instrumentation.runOnMainSync { view = core.view }
            if (!view.storageBusy) return view
            check(System.nanoTime() < deadline) {
                "Storage callback did not reach Android main thread"
            }
            Thread.sleep(10)
        }
    }

    private fun dispatch(core: CoreSession, event: Event): ViewModel {
        instrumentation.runOnMainSync { core.dispatch(event) }
        return awaitIdle(core)
    }

    @Test
    fun txtBytesEncodingRetryAtomicSourceAndDuplicateReopenUseProductionJni() {
        val context = instrumentation.targetContext
        val directory = File(context.noBackupFilesDir, "txt-bridge-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "reader.sqlite3")
        val calls = AtomicInteger()
        val actual = sudachiDictionaryProvider(context)
        val provider =
            object : DictionaryProvider {
                override fun verifiedWordDictionaryPath(): String = ""

                override fun openVerifiedDictionary(): Long {
                    calls.incrementAndGet()
                    return actual.openVerifiedDictionary()
                }
            }
        fun event(bytes: ByteArray, encoding: TextEncoding? = null, filename: String = "本𠮷.txt") =
            Event.ImportTxt(TxtImport(bytes.map { it.toUByte() }, filename, encoding))
        lateinit var core: CoreSession
        instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
        try {
            awaitIdle(core)
            val ambiguous = byteArrayOf(0xC2.toByte(), 0xA9.toByte())
            assertEquals(
                DocumentError.TXTENCODINGREQUIRED,
                dispatch(core, event(ambiguous)).documentError,
            )
            assertEquals(
                DocumentError.TXTINVALIDUTF8,
                dispatch(core, event(byteArrayOf(0x82.toByte()), TextEncoding.UTF8)).documentError,
            )
            assertEquals(0, calls.get())
            val bytes = "\uFEFF｜猫《ねこ》と猫。\r\n𠮷😀\u0000終".toByteArray(Charsets.UTF_8)
            val created = dispatch(core, event(bytes))
            assertNull(created.documentError)
            assertEquals(ImportOutcome.CREATED, created.importSummary!!.outcome)
            assertEquals("本𠮷.txt", created.documents.single().title)
            assertNull(created.reader)
            val id = created.documents.single().documentId
            val loaded = dispatch(core, Event.LoadPaste(id)).loadedDocument!!
            assertEquals("猫と猫。\r\n𠮷😀\u0000終", loaded.chapters.single().text)
            val ruby = loaded.sourceRuby.single()
            assertEquals(loaded.chapters.single().chapterId, ruby.chapterId)
            assertEquals(
                ReaderRuby(TextRange(TextOffset(0), TextOffset(3)), "ねこ"),
                ruby.ruby.single(),
            )
            val opened = dispatch(core, Event.OpenReader(id))
            val page = opened.reader!!
            assertEquals(ruby.ruby, page.sourceRuby)
            val key = ReaderTokenKey(id, page.anchor, opened.readerAnalysis!!)
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            val cats = tokens.tokens.filter { it.surface == "猫" }
            assertEquals(2, cats.size)
            dispatch(
                core,
                Event.SetReaderStatus(
                    ReaderStatusChange(
                        key,
                        cats.first().vocabulary!!.id,
                        VocabularyStatus.FAMILIAR,
                        1234,
                    )
                ),
            )
            val position = page.position.copy(offset = TextOffset(3))
            dispatch(core, Event.SaveReadingPosition(position))
            dispatch(core, Event.CloseDocument)
            instrumentation.runOnMainSync { core.setForeground(false) }
            dispatch(core, Event.RefreshLibrary)
            val duplicate = dispatch(core, event(bytes, TextEncoding.SHIFTJIS, ""))
            assertEquals(ImportOutcome.EXISTING, duplicate.importSummary!!.outcome)
            assertEquals(id, duplicate.importSummary.documentId)
            assertEquals(1, calls.get())
            android.database.sqlite.SQLiteDatabase.openDatabase(
                    database.path,
                    null,
                    android.database.sqlite.SQLiteDatabase.OPEN_READONLY,
                )
                .use { db ->
                    db.rawQuery("SELECT original_bytes, text_encoding FROM imported_files", null)
                        .use {
                            assertTrue(it.moveToFirst())
                            assertTrue(bytes.contentEquals(it.getBlob(0)))
                            assertEquals("utf8", it.getString(1))
                            assertFalse(it.moveToNext())
                        }
                    db.rawQuery("SELECT start_byte, end_byte, reading FROM source_ruby", null).use {
                        assertTrue(it.moveToFirst())
                        assertEquals(0, it.getInt(0))
                        assertEquals(3, it.getInt(1))
                        assertEquals("ねこ", it.getString(2))
                    }
                    db.rawQuery("SELECT count(*) FROM reading_completions", null).use {
                        assertTrue(it.moveToFirst())
                        assertEquals(0, it.getInt(0))
                    }
                }
            instrumentation.runOnMainSync { core.close() }
            instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
            awaitIdle(core)
            assertEquals(
                ImportOutcome.EXISTING,
                dispatch(core, event(bytes)).importSummary!!.outcome,
            )
            val reopened = dispatch(core, Event.OpenReader(id)).reader!!
            assertEquals(position, reopened.position)
            assertEquals(ruby.ruby, reopened.sourceRuby)
            val restored = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            assertTrue(
                restored.tokens
                    .filter { it.surface == "猫" }
                    .all { it.vocabulary!!.status == VocabularyStatus.FAMILIAR }
            )
            assertEquals(1, calls.get())
            dispatch(core, Event.CloseDocument)
            val selected = dispatch(core, event(ambiguous, TextEncoding.SHIFTJIS))
            val legacyId = selected.importSummary!!.documentId
            assertEquals(
                "ﾂｩ",
                dispatch(core, Event.LoadPaste(legacyId)).loadedDocument!!.chapters.single().text,
            )
        } finally {
            instrumentation.runOnMainSync { core.close() }
            assertTrue(directory.deleteRecursively())
        }
    }

    @Test
    fun manualStatusesShareExactSurfacesAcrossDocumentsWithoutMovingOrCompleting() {
        lateinit var core: CoreSession
        instrumentation.runOnMainSync {
            core = CoreSession(":memory:", sudachiDictionaryProvider(instrumentation.targetContext))
        }
        try {
            awaitIdle(core)
            val first = dispatch(core, Event.CreatePaste("猫 猫 犬")).documents.single().documentId
            val opened = dispatch(core, Event.OpenReader(first))
            val page = opened.reader!!
            val key = ReaderTokenKey(first, page.anchor, opened.readerAnalysis!!)
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            val cats = tokens.tokens.filter { it.surface == "猫" }
            assertEquals(2, cats.size)
            val id = cats.first().vocabulary!!.id
            assertTrue(tokens.tokens.all { it.vocabulary!!.status == VocabularyStatus.NEW })
            for (status in VocabularyStatus.entries) {
                val changed =
                    dispatch(
                        core,
                        Event.SetReaderStatus(ReaderStatusChange(key, id, status, 1234L)),
                    )
                assertNull(changed.documentError)
                assertEquals(page, changed.reader)
                assertEquals(opened.loadedDocument, changed.loadedDocument)
                assertNull(changed.readerTime)
                assertNull(changed.completionNotice)
                assertNull(changed.bookCompletionNotice)
                val expected =
                    tokens.tokens.map { token ->
                        if (token.surface == "猫") {
                            token.copy(vocabulary = token.vocabulary!!.copy(status = status))
                        } else token
                    }
                assertEquals(expected, changed.readerTokens!!.tokens)
                assertEquals(
                    changed.readerTokens,
                    dispatch(core, Event.LoadReaderTokens(key)).readerTokens,
                )
            }
            val second =
                dispatch(core, Event.CreatePaste("犬 猫"))
                    .documents
                    .single { it.documentId != first }
                    .documentId
            val secondOpened = dispatch(core, Event.OpenReader(second))
            val secondKey =
                ReaderTokenKey(second, secondOpened.reader!!.anchor, secondOpened.readerAnalysis!!)
            val secondTokens = dispatch(core, Event.LoadReaderTokens(secondKey)).readerTokens!!
            assertEquals(id, secondTokens.tokens.single { it.surface == "猫" }.vocabulary!!.id)
            assertEquals(
                VocabularyStatus.IGNORED,
                secondTokens.tokens.single { it.surface == "猫" }.vocabulary!!.status,
            )
            dispatch(
                core,
                Event.SetReaderStatus(
                    ReaderStatusChange(secondKey, id, VocabularyStatus.LEARNING, 2345L)
                ),
            )
            dispatch(core, Event.OpenReader(first))
            val shared = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            assertTrue(
                shared.tokens
                    .filter { it.surface == "猫" }
                    .all { it.vocabulary!!.status == VocabularyStatus.LEARNING }
            )
            assertEquals(
                VocabularyStatus.NEW,
                shared.tokens.single { it.surface == "犬" }.vocabulary!!.status,
            )
        } finally {
            instrumentation.runOnMainSync { core.close() }
        }
    }

    @Test
    fun privateDatabasePersistsAcrossSessionsAndFailuresCanRetry() {
        val directory =
            File(instrumentation.targetContext.filesDir, "storage-test-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "yomoku.sqlite3")
        var active: CoreSession? = null
        try {
            lateinit var core: CoreSession
            instrumentation.runOnMainSync {
                core =
                    CoreSession(
                        database.absolutePath,
                        sudachiDictionaryProvider(instrumentation.targetContext),
                    )
            }
            active = core
            assertTrue(awaitIdle(core).documents.isEmpty())
            assertTrue(database.isFile)
            val text = "  題𠮷📖  \r\nか\u3099👩‍👩‍👧‍👦\u0000終\n"
            val created = dispatch(core, Event.CreatePaste(text))
            assertNull(created.documentError)
            assertEquals("  題𠮷📖  ", created.documents.single().title)
            assertNull(created.loadedDocument)
            var document =
                dispatch(core, Event.LoadPaste(created.documents.single().documentId))
                    .loadedDocument
            assertNotNull(document)
            assertEquals(text, document!!.chapters.single().text)
            val opened = dispatch(core, Event.OpenReader(document.documentId)).reader!!
            val position = opened.position.copy(offset = TextOffset(5))
            val saved = dispatch(core, Event.SaveReadingPosition(position))
            assertNull(saved.documentError)
            assertEquals(position, saved.reader!!.position)
            document = saved.loadedDocument!!
            dispatch(core, Event.OpenSettings)
            assertEquals(
                PageLength.CHARACTERS750,
                dispatch(core, Event.SetPageLength(PageLength.CHARACTERS750)).pageLength,
            )
            dispatch(core, Event.CloseSettings)
            val summaries = saved.documents
            assertEquals(
                DocumentError.EMPTYPASTE,
                dispatch(core, Event.CreatePaste(" \n")).documentError,
            )
            assertEquals(1, dispatch(core, Event.RefreshLibrary).documents.size)
            instrumentation.runOnMainSync { core.close() }
            active = null
            instrumentation.runOnMainSync {
                core =
                    CoreSession(
                        database.absolutePath,
                        sudachiDictionaryProvider(instrumentation.targetContext),
                    )
            }
            active = core
            val initial = awaitIdle(core)
            assertEquals(summaries, initial.documents)
            assertEquals(PageLength.CHARACTERS750, initial.pageLength)
            assertEquals(
                document,
                dispatch(core, Event.LoadPaste(document.documentId)).loadedDocument,
            )
            assertEquals(
                position,
                dispatch(core, Event.OpenReader(document.documentId)).reader!!.position,
            )
            instrumentation.runOnMainSync { core.close() }
            active = null

            val missing = File(directory, "missing")
            instrumentation.runOnMainSync {
                core =
                    CoreSession(
                        File(missing, "db.sqlite3").absolutePath,
                        sudachiDictionaryProvider(instrumentation.targetContext),
                    )
            }
            active = core
            assertEquals(DocumentError.UNAVAILABLE, awaitIdle(core).documentError)
            assertFalse(missing.exists())
            assertTrue(missing.mkdir())
            assertNull(dispatch(core, Event.RefreshLibrary).documentError)
        } finally {
            instrumentation.runOnMainSync { active?.close() }
            assertTrue(directory.deleteRecursively())
        }
    }
}
