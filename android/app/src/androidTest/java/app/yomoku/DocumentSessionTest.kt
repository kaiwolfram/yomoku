package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.TextOffset
import app.yomoku.shared.ViewModel
import java.io.File
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
        val deadline = System.nanoTime() + 10_000_000_000L
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
    fun privateDatabasePersistsAcrossSessionsAndFailuresCanRetry() {
        val directory =
            File(instrumentation.targetContext.filesDir, "storage-test-${System.nanoTime()}")
        assertTrue(directory.mkdir())
        val database = File(directory, "yomoku.sqlite3")
        var active: CoreSession? = null
        try {
            lateinit var core: CoreSession
            instrumentation.runOnMainSync { core = CoreSession(database.absolutePath) }
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
            assertEquals(text, document!!.chapter.text)
            val opened = dispatch(core, Event.OpenReader(document.documentId)).reader!!
            val position = opened.position.copy(offset = TextOffset(5))
            val saved = dispatch(core, Event.SaveReadingPosition(position))
            assertNull(saved.documentError)
            assertEquals(position, saved.reader!!.position)
            document = saved.loadedDocument!!
            val summaries = saved.documents
            assertEquals(
                DocumentError.EMPTYPASTE,
                dispatch(core, Event.CreatePaste(" \n")).documentError,
            )
            assertEquals(1, dispatch(core, Event.RefreshLibrary).documents.size)
            instrumentation.runOnMainSync { core.close() }
            active = null
            instrumentation.runOnMainSync { core = CoreSession(database.absolutePath) }
            active = core
            assertEquals(summaries, awaitIdle(core).documents)
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
                core = CoreSession(File(missing, "db.sqlite3").absolutePath)
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
