package app.yomoku

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ReaderWordSelection
import app.yomoku.shared.ViewModel
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderWordDetailsSessionTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun awaitIdle(core: CoreSession): ViewModel {
        val deadline = System.nanoTime() + 60_000_000_000L
        while (true) {
            lateinit var view: ViewModel
            instrumentation.runOnMainSync { view = core.view }
            if (!view.storageBusy) return view
            check(System.nanoTime() < deadline) { "Reader details worker did not finish" }
            Thread.sleep(10)
        }
    }

    private fun dispatch(core: CoreSession, event: Event): ViewModel {
        instrumentation.runOnMainSync { core.dispatch(event) }
        return awaitIdle(core)
    }

    @Test
    fun lazyVerifiedWordIndexRetriesOnWorkerAndReturnsSeparateMatchesAndDurableCount() {
        val publisher = sudachiDictionaryProvider(instrumentation.targetContext)
        val wordCalls = AtomicInteger()
        val fail = AtomicBoolean(true)
        val provider =
            object : DictionaryProvider {
                override fun openVerifiedDictionary(): Long = publisher.openVerifiedDictionary()

                override fun verifiedWordDictionaryPath(): String {
                    check(Looper.myLooper() != Looper.getMainLooper())
                    wordCalls.incrementAndGet()
                    return if (fail.get()) "" else publisher.verifiedWordDictionaryPath()
                }
            }
        lateinit var core: CoreSession
        instrumentation.runOnMainSync { core = CoreSession(":memory:", provider) }
        try {
            awaitIdle(core)
            assertEquals(0, wordCalls.get())
            val id = dispatch(core, Event.CreatePaste("東京 東京 食べた")).documents.single().documentId
            val opened = dispatch(core, Event.OpenReader(id))
            val page = opened.reader!!
            val key = ReaderTokenKey(id, page.anchor, opened.readerAnalysis!!)
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            assertEquals(0, wordCalls.get())
            val tokyo = tokens.tokens.first { it.surface == "東京" }
            val selection = ReaderWordSelection(key, tokyo.tokenIndex)
            val failed = dispatch(core, Event.LoadReaderWord(selection))
            assertNotNull(failed.readerWordError)
            assertNull(failed.documentError)
            assertNull(failed.readerWord)
            fail.set(false)
            val loaded = dispatch(core, Event.LoadReaderWord(selection))
            val details = loaded.readerWord!!
            assertEquals(selection, loaded.readerWordSelection)
            assertNull(loaded.readerWordError)
            assertEquals(tokyo, details.token)
            assertEquals(0UL, details.encountered)
            assertTrue(details.candidates.words.isNotEmpty())
            assertTrue(details.candidates.names.isNotEmpty())
            assertTrue(details.candidates.words.all { it.entry.readings.isNotEmpty() })
            assertTrue(details.candidates.words.all { it.entry.senses.isNotEmpty() })
            assertTrue(details.candidates.names.all { it.entry.names.isNotEmpty() })
            assertEquals(page, loaded.reader)
            assertEquals(opened.loadedDocument, loaded.loadedDocument)
            assertNull(loaded.readerTime)
            assertNull(loaded.completionNotice)
            assertEquals(2, wordCalls.get())
            val inflected = tokens.tokens.first { it.surface == "食べ" }
            val baseDetails =
                dispatch(core, Event.LoadReaderWord(ReaderWordSelection(key, inflected.tokenIndex)))
                    .readerWord!!
            assertEquals("食べる", baseDetails.candidates.dictionaryForm)
            assertTrue(baseDetails.candidates.words.isNotEmpty())
            assertEquals(2, wordCalls.get())
            dispatch(core, Event.CompleteReaderPage(id, page.anchor, page.position, 5678L))
            val reopened = dispatch(core, Event.OpenReader(id))
            assertNull(reopened.readerWord)
            assertNull(reopened.readerWordSelection)
            val rereadKey = ReaderTokenKey(id, reopened.reader!!.anchor, reopened.readerAnalysis!!)
            val rereadToken =
                dispatch(core, Event.LoadReaderTokens(rereadKey)).readerTokens!!.tokens.first {
                    it.surface == "東京"
                }
            val counted =
                dispatch(
                    core,
                    Event.LoadReaderWord(ReaderWordSelection(rereadKey, rereadToken.tokenIndex)),
                )
            assertEquals(2UL, counted.readerWord!!.encountered)
            dispatch(core, Event.OpenSettings)
            instrumentation.runOnMainSync {
                assertNull(core.view.readerWordSelection)
                assertFalse(core.view.storageBusy)
            }
        } finally {
            instrumentation.runOnMainSync { core.close() }
        }
    }

    @Test
    fun customEntryTransportPreservesUnicodeRetriesAndReopensWithoutChangingReaderOrBuiltins() {
        val context = instrumentation.targetContext
        val directory = File(context.noBackupFilesDir, "custom-word-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "reader.sqlite3")
        val provider = sudachiDictionaryProvider(context)
        lateinit var core: CoreSession
        instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
        try {
            awaitIdle(core)
            val id = dispatch(core, Event.CreatePaste("ヌヌヌヌヌヌ 東京")).documents.single().documentId
            val opened = dispatch(core, Event.OpenReader(id))
            val key = ReaderTokenKey(id, opened.reader!!.anchor, opened.readerAnalysis!!)
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).readerTokens!!
            val token = tokens.tokens.first { it.surface == "ヌヌヌヌヌヌ" }
            val selection = ReaderWordSelection(key, token.tokenIndex)
            val missing = dispatch(core, Event.LoadReaderWord(selection))
            assertTrue(missing.readerWord!!.candidates.words.isEmpty())
            assertTrue(missing.readerWord.candidates.names.isEmpty())
            val meaning = " literal meaning 📖\u0000 "
            val reading = " か\u3099𠮷\u0000 "
            val saved = dispatch(core, Event.SaveReaderCustomEntry(selection, meaning, reading))
            val entry = saved.readerWord!!.customEntry!!
            assertEquals(token.surface, entry.surface)
            assertEquals(meaning, entry.englishMeaning)
            assertEquals(reading, entry.reading)
            assertEquals(tokens, saved.readerTokens)
            assertEquals(opened.reader, saved.reader)
            assertEquals(0UL, saved.readerWord.encountered)
            val failed = dispatch(core, Event.SaveReaderCustomEntry(selection, " ", reading))
            assertNotNull(failed.readerCustomEntryError)
            assertEquals(entry, failed.readerWord!!.customEntry)
            assertNull(failed.readerWordError)
            assertNull(failed.documentError)
            val edited = dispatch(core, Event.SaveReaderCustomEntry(selection, "edited", reading))
            assertNull(edited.readerCustomEntryError)
            assertEquals("edited", edited.readerWord!!.customEntry!!.englishMeaning)
            val builtin =
                ReaderWordSelection(key, tokens.tokens.first { it.surface == "東京" }.tokenIndex)
            val builtins = dispatch(core, Event.LoadReaderWord(builtin)).readerWord!!
            assertTrue(builtins.candidates.words.isNotEmpty())
            assertTrue(builtins.candidates.names.isNotEmpty())
            assertNull(builtins.customEntry)
            assertEquals(
                builtins,
                dispatch(core, Event.SaveReaderCustomEntry(builtin, "override", reading)).readerWord,
            )
            assertEquals(
                builtins,
                dispatch(core, Event.DeleteReaderCustomEntry(builtin)).readerWord,
            )
            instrumentation.runOnMainSync { core.close() }
            instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
            awaitIdle(core)
            val reopened = dispatch(core, Event.OpenReader(id))
            assertEquals(opened.reader, reopened.reader)
            val restored = dispatch(core, Event.LoadReaderTokens(key))
            assertEquals(tokens, restored.readerTokens)
            val loaded = dispatch(core, Event.LoadReaderWord(selection)).readerWord!!
            assertEquals("edited", loaded.customEntry!!.englishMeaning)
            assertEquals(reading, loaded.customEntry.reading)
            val deleted = dispatch(core, Event.DeleteReaderCustomEntry(selection))
            assertNull(deleted.readerCustomEntryError)
            assertNull(deleted.readerWord!!.customEntry)
            assertNull(dispatch(core, Event.LoadReaderWord(selection)).readerWord!!.customEntry)
        } finally {
            instrumentation.runOnMainSync { core.close() }
            directory.deleteRecursively()
        }
    }
}
