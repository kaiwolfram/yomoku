package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyDocumentRenameBridge() {
    val directory = Files.createTempDirectory("yomoku-document-rename-").toFile()
    val effects = LinkedBlockingQueue<ByteArray>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                effects.add(requests)
            }

            override fun onError(error: FfiError) {
                throw error
            }
        }
    val dictionary = HostDictionary()
    val database = directory.resolve("db.sqlite3")
    val title = "  改題𠮷 か\u3099\u0000'\n  "
    try {
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            fun view() = ViewModel.bincodeDeserialize(bridge.view())
            fun dispatch(event: Event, callbacks: Int = 0): ViewModel {
                bridge.update(event.bincodeSerialize())
                repeat(callbacks) { checkNotNull(effects.poll(60, TimeUnit.SECONDS)) }
                return view().also { check(!it.storageBusy) }
            }
            dispatch(Event.RefreshLibrary, 1)
            val text = "原題\n猫 猫 犬"
            val id = checkNotNull(dispatch(Event.CreatePaste(text), 1).importSummary).documentId
            dispatch(Event.OpenReader(id), 1)
            val page = checkNotNull(view().reader)
            dispatch(Event.CompleteReaderPage(id, page.anchor, page.position, 500), 2)
            dispatch(Event.CreatePaste(text), 1)
            val before = view().documents.single()
            dispatch(Event.LoadLibraryWordDistributions, 1)
            val counts = view().libraryWordDistributions
            val calls = dictionary.calls.get()
            val bytes = database.readBytes()
            dispatch(Event.RequestDocumentRename(id))
            check(view().libraryRename?.title == "原題")
            dispatch(Event.EditDocumentTitle(id, title))
            check(view().libraryRename?.title == title)
            dispatch(Event.CancelDocumentRename)
            check(view().libraryRename == null)
            check(database.readBytes().contentEquals(bytes))
            dispatch(Event.RequestDocumentRename(id))
            dispatch(Event.EditDocumentTitle(id, title))
            val saved = dispatch(Event.SaveDocumentTitle(id), 1)
            check(saved.libraryRename == null)
            check(saved.documents.single() == before.copy(title = title))
            check(saved.libraryWordDistributions == counts)
            check(saved.importSummary?.title == title)
            check(dictionary.calls.get() == calls)
            val duplicate = dispatch(Event.CreatePaste(text), 1)
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            check(duplicate.documents.single() == before.copy(title = title))
            check(dictionary.calls.get() == calls)
            dispatch(Event.OpenReader(id), 1)
            check(view().reader?.title == title)
            check(view().reader?.text == text)
        }
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            bridge.update(Event.RefreshLibrary.bincodeSerialize())
            checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            val view = ViewModel.bincodeDeserialize(bridge.view())
            check(view.documents.single().title == title)
            check(view.libraryRename == null)
        }
    } finally {
        directory.deleteRecursively()
    }
}
