package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyBulkDocumentDeletionBridge() {
    val directory = Files.createTempDirectory("yomoku-bulk-deletion-").toFile()
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
            fun dispatch(event: Event, callbacks: Int = 0): ViewModel {
                bridge.update(event.bincodeSerialize())
                repeat(callbacks) { checkNotNull(effects.poll(60, TimeUnit.SECONDS)) }
                return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
            }
            fun word(id: DocumentId): EffectiveWordDetails {
                val opened = dispatch(Event.OpenReader(id), 1)
                val key =
                    ReaderTokenKey(
                        id,
                        checkNotNull(opened.reader).anchor,
                        checkNotNull(opened.readerAnalysis),
                    )
                val tokens = checkNotNull(dispatch(Event.LoadReaderTokens(key), 1).effectiveReaderTokens)
                return checkNotNull(
                    dispatch(
                            Event.LoadEffectiveReaderWord(
                                EffectiveReaderSelection(
                                    tokens.identity,
                                    tokens.tokens.first { it.surface == "猫" }.segmentIndex,
                                )
                            ),
                            1,
                        )
                        .effectiveReaderWord?.details
                )
            }
            dispatch(Event.RefreshLibrary, 1)
            val text = "Bulk𠮷 か\u3099-1\n猫 猫"
            val first = checkNotNull(dispatch(Event.CreatePaste(text), 1).importSummary).documentId
            val second =
                checkNotNull(dispatch(Event.CreatePaste("Bulk𠮷 か\u3099-2\n猫"), 1).importSummary)
                    .documentId
            val survivor =
                checkNotNull(dispatch(Event.CreatePaste("Survivor\n猫"), 1).importSummary).documentId
            val page = checkNotNull(dispatch(Event.OpenReader(first), 1).reader)
            val snapshot = checkNotNull(dispatch(Event.LoadReaderTokens(ReaderTokenKey(first, page.anchor, checkNotNull(ViewModel.bincodeDeserialize(bridge.view()).readerAnalysis))), 1).effectiveReaderTokens)
            dispatch(Event.CompleteEffectiveReaderPage(snapshot.identity, page.position, 500), 2)
            val beforeWord = word(survivor)
            check(beforeWord.encountered == 2UL)
            dispatch(Event.CloseDocument)
            dispatch(Event.SearchLibraryTitles("Bulk𠮷"))
            val bytes = database.readBytes()
            val calls = dictionary.calls.get()
            dispatch(Event.ToggleLibraryDocumentSelection(second))
            val selected = dispatch(Event.SelectAllLibraryDocuments)
            check(selected.librarySelection == listOf(first, second))
            val prompt = dispatch(Event.RequestSelectedDocumentDeletion)
            val targets = checkNotNull(prompt.libraryBulkDeleteTargets)
            check(targets.map { it.documentId } == listOf(first, second))
            check(targets.first().finishedAtLeastOnce)
            dispatch(Event.CancelDocumentDeletion)
            check(database.readBytes().contentEquals(bytes))
            dispatch(Event.RequestSelectedDocumentDeletion)
            dispatch(Event.ConfirmSelectedDocumentDeletion(listOf(first)))
            check(database.readBytes().contentEquals(bytes))
            dispatch(Event.SearchLibraryTitles("Survivor"))
            val deleted = dispatch(Event.ConfirmSelectedDocumentDeletion(listOf(first, second)), 1)
            check(deleted.librarySelection == null && deleted.libraryBulkDeleteTargets == null)
            check(deleted.libraryDeleteError == null && deleted.libraryWordDistributions == null)
            check(
                deleted.libraryDocumentCount == 1UL &&
                    deleted.documents.single().documentId == survivor
            )
            check(dictionary.calls.get() == calls)
            check(
                word(survivor).let {
                    it.encountered == beforeWord.encountered &&
                        it.token.vocabulary == beforeWord.token.vocabulary
                }
            )
            dispatch(Event.CloseDocument)
            dispatch(Event.SearchLibraryTitles("Bulk𠮷"))
            val fresh = dispatch(Event.CreatePaste(text), 1)
            check(fresh.importSummary?.outcome == ImportOutcome.CREATED)
            check(
                fresh.documents.single().let {
                    it.documentId.value > survivor.value &&
                        it.progressPercent == 0UL &&
                        !it.finishedAtLeastOnce &&
                        it.lastReadSequence == null
                }
            )
        }
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            bridge.update(Event.RefreshLibrary.bincodeSerialize())
            checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            val view = ViewModel.bincodeDeserialize(bridge.view())
            check(
                view.libraryDocumentCount == 2UL &&
                    view.librarySelection == null &&
                    view.libraryBulkDeleteTargets == null
            )
        }
    } finally {
        directory.deleteRecursively()
    }
}
