package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.DocumentError
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.DocumentId
import app.yomoku.shared.Event
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel
import app.yomoku.shared.TextOffset
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

private class DocumentListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()
    val errors = LinkedBlockingQueue<FfiError>()

    override fun onEffects(requests: ByteArray) {
        effects.add(Thread.currentThread() to requests)
    }

    override fun onError(error: FfiError) {
        errors.add(error)
    }

    fun dispatch(bridge: NativeBridge, event: Event): ViewModel {
        val immediate = Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize()))
        check(immediate.value.single().effect is ShellEffect.Render)
        val (thread, bytes) = checkNotNull(effects.poll(10, TimeUnit.SECONDS))
        check(thread != Thread.currentThread())
        check(Requests.bincodeDeserialize(bytes).value.single().effect is ShellEffect.Render)
        check(errors.isEmpty())
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
}

internal fun verifyDocumentBridge() {
    val directory = Files.createTempDirectory("yomoku-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = DocumentListener()
        val text = "  題𠮷📖  \r\nか\u3099👩‍👩‍👧‍👦\u0000終\n"
        val document = NativeBridge(database.path, listener).use { bridge ->
            check(!database.exists())
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.isEmpty())
            val created = listener.dispatch(bridge, Event.CreatePaste(text))
            check(created.documents.single().title == "  題𠮷📖  ")
            check(created.loadedDocument == null)
            val id = created.documents.single().documentId
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.chapter.text == text)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            check(opened.contentRoute == ContentRoute.READER)
            val page = checkNotNull(opened.reader)
            check(page.documentId == id && page.text == text)
            check(page.anchor.sourceId == loaded.chapter.sourceId)
            check(page.anchor.range.end.value == text.toByteArray(Charsets.UTF_8).size.toLong())
            check(page.position.offset.value == 0L)
            check(page.pageNumber == 1UL && page.totalPages == 1UL)
            var saved = loaded
            for (byte in listOf(text.toByteArray(Charsets.UTF_8).size.toLong(), 5L, 0L, 5L)) {
                val position = page.position.copy(offset = TextOffset(byte))
                val updated = listener.dispatch(bridge, Event.SaveReadingPosition(position))
                check(updated.documentError == null)
                check(updated.reader?.position == position)
                saved = checkNotNull(updated.loadedDocument)
                check(saved.readingPosition == position)
                check(updated.documents.single().progressPercent == (byte * 100 / text.toByteArray(Charsets.UTF_8).size).toULong())
            }
            val invalid = page.position.copy(offset = TextOffset(3L))
            val rejected = listener.dispatch(bridge, Event.SaveReadingPosition(invalid))
            check(rejected.documentError == DocumentError.INVALIDDOCUMENT)
            check(rejected.loadedDocument == saved)
            bridge.update(Event.CloseDocument.bincodeSerialize())
            check(ViewModel.bincodeDeserialize(bridge.view()).contentRoute == ContentRoute.MAIN)
            check(listener.dispatch(bridge, Event.CreatePaste(" \n")).documentError == DocumentError.EMPTYPASTE)
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.size == 1)
            check(listener.dispatch(bridge, Event.LoadPaste(DocumentId(Long.MAX_VALUE))).documentError == DocumentError.NOTFOUND)
            saved
        }
        val reopened = DocumentListener()
        NativeBridge(database.path, reopened).use { bridge ->
            check(reopened.dispatch(bridge, Event.RefreshLibrary).documents.single().documentId == document.documentId)
            check(reopened.dispatch(bridge, Event.LoadPaste(document.documentId)).loadedDocument == document)
            check(reopened.dispatch(bridge, Event.OpenReader(document.documentId)).reader?.text == text)
            check(ViewModel.bincodeDeserialize(bridge.view()).reader?.position == document.readingPosition)
        }
        val failure = DocumentListener()
        NativeBridge(directory.resolve("missing/db.sqlite3").path, failure).use { bridge ->
            check(failure.dispatch(bridge, Event.RefreshLibrary).documentError == DocumentError.UNAVAILABLE)
            check(directory.resolve("missing").mkdir())
            check(failure.dispatch(bridge, Event.RefreshLibrary).documentError == null)
        }
        // Closing during an operation must safely release the JNI callback after
        // the worker completes; the registry assertions in main wait for this.
        repeat(20) {
            NativeBridge(":memory:", DocumentListener()).use { bridge ->
                bridge.update(Event.RefreshLibrary.bincodeSerialize())
            }
        }
        println("PASS: async JNI storage callbacks, Unicode paste/list/load/reopen, typed failures and close during I/O")
    } finally {
        check(directory.deleteRecursively())
    }
}
