package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyLibrarySortBridge() {
    val directory = Files.createTempDirectory("yomoku-library-sort-").toFile()
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
    val texts = listOf("beta\n本文", "ALPHA𠮷か\u3099\u0000\n本文", "alpha\n本文")
    try {
        val database = directory.resolve("db.sqlite3")
        lateinit var selected: DocumentId
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            fun dispatch(event: Event, asynchronous: Boolean = false): ViewModel {
                val immediate = Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize()))
                check(immediate.value.single().effect is ShellEffect.Render)
                if (asynchronous) checkNotNull(effects.poll(60, TimeUnit.SECONDS))
                return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
            }
            dispatch(Event.RefreshLibrary, true)
            val imported = texts.map { text ->
                val view = dispatch(Event.CreatePaste(text), true)
                view.documents.single { it.documentId == view.importSummary!!.documentId }
            }
            val calls = dictionary.calls.get()
            val before = database.readBytes()
            check(ViewModel.bincodeDeserialize(bridge.view()).librarySort == LibrarySort.LASTREAD)
            check(ViewModel.bincodeDeserialize(bridge.view()).documents == imported.reversed())
            check(
                dispatch(Event.SortLibrary(LibrarySort.TITLE)).documents ==
                    listOf(imported[2], imported[1], imported[0])
            )
            check(
                dispatch(Event.SearchLibraryTitles("ALPHA")).documents ==
                    listOf(imported[2], imported[1])
            )
            check(
                dispatch(Event.SortLibrary(LibrarySort.IMPORTDATE)).documents ==
                    listOf(imported[2], imported[1])
            )
            dispatch(Event.SearchLibraryTitles(""))
            dispatch(Event.OpenSettings)
            check(dispatch(Event.CloseSettings).librarySort == LibrarySort.IMPORTDATE)
            check(dispatch(Event.RefreshLibrary, true).documents == imported.reversed())
            check(database.readBytes().contentEquals(before))
            selected = imported[0].documentId
            val opened = dispatch(Event.OpenReader(checkNotNull(selected)), true)
            check(opened.reader?.documentId == selected)
            dispatch(Event.CloseDocument)
            val recent = dispatch(Event.SortLibrary(LibrarySort.LASTREAD))
            check(
                recent.documents.map { it.documentId } ==
                    listOf(selected, imported[2].documentId, imported[1].documentId)
            )
            val sequence = checkNotNull(recent.documents[0].lastReadSequence)
            check(sequence > 0)
            val duplicate = dispatch(Event.CreatePaste(texts[0]), true)
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            check(duplicate.documents == recent.documents)
            check(dispatch(Event.RefreshLibrary, true).documents == recent.documents)
            check(dictionary.calls.get() == calls && effects.isEmpty())
            val after = database.readBytes()
            dispatch(Event.SortLibrary(LibrarySort.TITLE))
            dispatch(Event.SortLibrary(LibrarySort.LASTREAD))
            check(database.readBytes().contentEquals(after))
        }
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            check(ViewModel.bincodeDeserialize(bridge.view()).librarySort == LibrarySort.LASTREAD)
            bridge.update(Event.RefreshLibrary.bincodeSerialize())
            checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            val reopened = ViewModel.bincodeDeserialize(bridge.view())
            check(reopened.documents[0].documentId == selected)
            check(reopened.documents[0].lastReadSequence == 1L)
            check(reopened.documents.drop(1).all { it.lastReadSequence == null })
        }
    } finally {
        directory.deleteRecursively()
    }
}
