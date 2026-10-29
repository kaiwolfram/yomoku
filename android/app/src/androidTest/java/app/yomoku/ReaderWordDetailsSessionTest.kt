package app.yomoku

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
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
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).effectiveReaderTokens!!
            assertEquals(0, wordCalls.get())
            val tokyo = tokens.tokens.first { it.surface == "東京" }
            val selection = EffectiveReaderSelection(tokens.identity, tokyo.segmentIndex)
            val failed = dispatch(core, Event.LoadEffectiveReaderWord(selection))
            assertNotNull(failed.effectiveReaderWord?.error)
            assertNull(failed.documentError)
            assertNull(failed.effectiveReaderWord?.details)
            fail.set(false)
            val loaded = dispatch(core, Event.LoadEffectiveReaderWord(selection))
            val details = loaded.effectiveReaderWord!!.details!!
            assertEquals(selection, loaded.effectiveReaderWord.selection)
            assertNull(loaded.effectiveReaderWord.error)
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
                dispatch(
                        core,
                        Event.LoadEffectiveReaderWord(
                            EffectiveReaderSelection(tokens.identity, inflected.segmentIndex)
                        ),
                    )
                    .effectiveReaderWord!!
                    .details!!
            assertEquals("食べる", baseDetails.candidates.dictionaryForm)
            assertTrue(baseDetails.candidates.words.isNotEmpty())
            assertEquals(2, wordCalls.get())
            dispatch(core, Event.CompleteEffectiveReaderPage(tokens.identity, page.position, 5678L))
            val reopened = dispatch(core, Event.OpenReader(id))
            assertNull(reopened.effectiveReaderWord?.details)
            assertNull(reopened.effectiveReaderWord?.selection)
            val rereadKey = ReaderTokenKey(id, reopened.reader!!.anchor, reopened.readerAnalysis!!)
            val rereadToken =
                dispatch(core, Event.LoadReaderTokens(rereadKey))
                    .effectiveReaderTokens!!
                    .tokens
                    .first {
                        it.surface == "東京"
                    }
            val counted =
                dispatch(
                    core,
                    Event.LoadEffectiveReaderWord(
                        EffectiveReaderSelection(
                            core.view.effectiveReaderTokens!!.identity,
                            rereadToken.segmentIndex,
                        )
                    ),
                )
            assertEquals(2UL, counted.effectiveReaderWord!!.details!!.encountered)
            dispatch(core, Event.OpenSettings)
            instrumentation.runOnMainSync {
                assertNull(core.view.effectiveReaderWord?.selection)
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
            val tokens = dispatch(core, Event.LoadReaderTokens(key)).effectiveReaderTokens!!
            val token = tokens.tokens.first { it.surface == "ヌヌヌヌヌヌ" }
            val selection = EffectiveReaderSelection(tokens.identity, token.segmentIndex)
            val missing = dispatch(core, Event.LoadEffectiveReaderWord(selection))
            assertTrue(missing.effectiveReaderWord!!.details!!.candidates.words.isEmpty())
            assertTrue(missing.effectiveReaderWord.details.candidates.names.isEmpty())
            val meaning = " literal meaning 📖\u0000 "
            val reading = " か\u3099𠮷\u0000 "
            val saved =
                dispatch(core, Event.SaveEffectiveReaderCustomEntry(selection, meaning, reading))
            val entry = saved.effectiveReaderWord!!.details!!.customEntry!!
            assertEquals(token.surface, entry.surface)
            assertEquals(meaning, entry.englishMeaning)
            assertEquals(reading, entry.reading)
            assertEquals(tokens, saved.effectiveReaderTokens)
            assertEquals(opened.reader, saved.reader)
            assertEquals(0UL, saved.effectiveReaderWord.details.encountered)
            val failed =
                dispatch(core, Event.SaveEffectiveReaderCustomEntry(selection, " ", reading))
            assertNull(failed.effectiveReaderWord?.customEntryError)
            assertEquals(entry, failed.effectiveReaderWord!!.details!!.customEntry)
            assertNull(failed.effectiveReaderWord.error)
            assertNull(failed.documentError)
            val edited =
                dispatch(core, Event.SaveEffectiveReaderCustomEntry(selection, "edited", reading))
            assertNull(edited.effectiveReaderWord?.customEntryError)
            assertEquals(
                "edited",
                edited.effectiveReaderWord!!.details!!.customEntry!!.englishMeaning,
            )
            val builtin =
                EffectiveReaderSelection(
                    tokens.identity,
                    tokens.tokens.first { it.surface == "東京" }.segmentIndex,
                )
            val builtins =
                dispatch(core, Event.LoadEffectiveReaderWord(builtin))
                    .effectiveReaderWord!!
                    .details!!
            assertTrue(builtins.candidates.words.isNotEmpty())
            assertTrue(builtins.candidates.names.isNotEmpty())
            assertNull(builtins.customEntry)
            assertEquals(
                builtins,
                dispatch(core, Event.SaveEffectiveReaderCustomEntry(builtin, "override", reading))
                    .effectiveReaderWord
                    ?.details,
            )
            assertEquals(
                builtins,
                dispatch(core, Event.DeleteEffectiveReaderCustomEntry(builtin))
                    .effectiveReaderWord
                    ?.details,
            )
            instrumentation.runOnMainSync { core.close() }
            instrumentation.runOnMainSync { core = CoreSession(database.path, provider) }
            awaitIdle(core)
            val reopened = dispatch(core, Event.OpenReader(id))
            assertEquals(opened.reader, reopened.reader)
            val restored = dispatch(core, Event.LoadReaderTokens(key))
            assertEquals(tokens, restored.effectiveReaderTokens)
            val reopenedSelection =
                EffectiveReaderSelection(
                    restored.effectiveReaderTokens!!.identity,
                    token.segmentIndex,
                )
            val loaded =
                dispatch(core, Event.LoadEffectiveReaderWord(reopenedSelection))
                    .effectiveReaderWord!!
                    .details!!
            assertEquals("edited", loaded.customEntry!!.englishMeaning)
            assertEquals(reading, loaded.customEntry.reading)
            val deleted = dispatch(core, Event.DeleteEffectiveReaderCustomEntry(reopenedSelection))
            assertNull(deleted.effectiveReaderWord?.customEntryError)
            assertNull(deleted.effectiveReaderWord!!.details!!.customEntry)
            assertNull(
                dispatch(core, Event.LoadEffectiveReaderWord(reopenedSelection))
                    .effectiveReaderWord!!
                    .details!!
                    .customEntry
            )
        } finally {
            instrumentation.runOnMainSync { core.close() }
            directory.deleteRecursively()
        }
    }
}
