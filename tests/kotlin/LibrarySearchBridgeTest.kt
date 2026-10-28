package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.Event
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyLibrarySearchBridge() {
    val directory = Files.createTempDirectory("yomoku-library-search-").toFile()
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
    try {
        val database = directory.resolve("db.sqlite3")
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            fun dispatch(event: Event, asynchronous: Boolean): ViewModel {
                val immediate = Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize()))
                check(immediate.value.single().effect is ShellEffect.Render)
                if (asynchronous) checkNotNull(effects.poll(60, TimeUnit.SECONDS))
                return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
            }
            dispatch(Event.RefreshLibrary, true)
            dispatch(Event.CreatePaste("Title RUST\n本文だけの検索語。"), true)
            dispatch(Event.CreatePaste("題𠮷か\u3099\u0000%_\n別の本文。"), true)
            val original = ViewModel.bincodeDeserialize(bridge.view()).documents
            check(original.size == 2)
            val before = database.readBytes()
            val calls = dictionary.calls.get()
            for ((query, expected) in
                listOf(
                    "rust" to listOf(original[1]),
                    "𠮷か\u3099\u0000%_" to listOf(original[0]),
                    "本文だけの検索語" to emptyList(),
                    "" to original,
                )) {
                val view = dispatch(Event.SearchLibraryTitles(query), false)
                check(view.libraryTitleQuery == query && view.libraryDocumentCount == 2UL)
                check(view.documents == expected)
                check(!view.readingTimeSavePending && !view.completionSavePending)
                check(view.documentError == null)
            }
            dispatch(Event.SearchLibraryTitles("RUST"), false)
            val refreshed = dispatch(Event.RefreshLibrary, true)
            check(
                refreshed.libraryTitleQuery == "RUST" && refreshed.documents == listOf(original[1])
            )
            check(dictionary.calls.get() == calls && effects.isEmpty())
            check(database.readBytes().contentEquals(before))
        }
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            val initial = ViewModel.bincodeDeserialize(bridge.view())
            check(initial.libraryTitleQuery.isEmpty())
            bridge.update(Event.RefreshLibrary.bincodeSerialize())
            checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            val reopened = ViewModel.bincodeDeserialize(bridge.view())
            check(reopened.documents.size == 2 && reopened.libraryDocumentCount == 2UL)
        }
    } finally {
        directory.deleteRecursively()
    }
}
