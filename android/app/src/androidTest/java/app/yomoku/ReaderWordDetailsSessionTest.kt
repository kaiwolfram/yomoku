package app.yomoku

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ViewModel
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
            assertEquals(opened.reader, loaded.reader)
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
}
