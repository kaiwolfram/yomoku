package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyDocumentDeletionBridge() {
    val directory = Files.createTempDirectory("yomoku-document-deletion-").toFile()
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
            fun view() = ViewModel.bincodeDeserialize(bridge.view())
            fun dispatch(event: Event, callbacks: Int = 0): ViewModel {
                bridge.update(event.bincodeSerialize())
                repeat(callbacks) { checkNotNull(effects.poll(60, TimeUnit.SECONDS)) }
                return view().also { check(!it.storageBusy) }
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
                val cat = tokens.tokens.first { it.surface == "猫" }
                return checkNotNull(
                    dispatch(Event.LoadEffectiveReaderWord(EffectiveReaderSelection(tokens.identity, cat.segmentIndex)), 1)
                        .effectiveReaderWord?.details
                )
            }
            dispatch(Event.RefreshLibrary, 1)
            val text = "Deletion-1\n猫 猫"
            val first = checkNotNull(dispatch(Event.CreatePaste(text), 1).importSummary).documentId
            val second =
                checkNotNull(dispatch(Event.CreatePaste("Deletion-2\n猫"), 1).importSummary)
                    .documentId
            dispatch(Event.OpenReader(first), 1)
            val page = checkNotNull(view().reader)
            val snapshot = checkNotNull(dispatch(Event.LoadReaderTokens(ReaderTokenKey(first, page.anchor, checkNotNull(view().readerAnalysis))), 1).effectiveReaderTokens)
            dispatch(Event.CompleteEffectiveReaderPage(snapshot.identity, page.position, 500), 2)
            val beforeWord = word(second)
            check(beforeWord.encountered == 2UL)
            check(beforeWord.token.vocabulary?.status == VocabularyStatus.KNOWN)
            dispatch(Event.CloseDocument)
            val beforeBytes = database.readBytes()
            val calls = dictionary.calls.get()
            dispatch(Event.ConfirmDocumentDeletion(first))
            check(database.readBytes().contentEquals(beforeBytes))
            val prompt =
                checkNotNull(dispatch(Event.RequestDocumentDeletion(first)).libraryDeleteTarget)
            check(prompt.documentId == first && prompt.finishedAtLeastOnce)
            dispatch(Event.CancelDocumentDeletion)
            check(view().libraryDeleteTarget == null)
            check(database.readBytes().contentEquals(beforeBytes))
            dispatch(Event.RequestDocumentDeletion(first))
            val deleted = dispatch(Event.ConfirmDocumentDeletion(first), 1)
            check(deleted.libraryDeleteTarget == null && deleted.libraryDeleteError == null)
            check(deleted.documents.single().documentId == second)
            check(deleted.libraryWordDistributions == null)
            check(dictionary.calls.get() == calls)
            check(
                word(second).let {
                    it.encountered == beforeWord.encountered &&
                        it.token.vocabulary == beforeWord.token.vocabulary
                }
            )
            dispatch(Event.CloseDocument)
            val reimported = dispatch(Event.CreatePaste(text), 1)
            val newId = checkNotNull(reimported.importSummary).documentId
            check(newId.value > first.value)
            check(reimported.importSummary.outcome == ImportOutcome.CREATED)
            check(
                reimported.documents
                    .single { it.documentId == newId }
                    .let {
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
            check(view.libraryDocumentCount == 2UL && view.libraryDeleteTarget == null)
            check(view.documents.none { it.finishedAtLeastOnce })
        }
    } finally {
        directory.deleteRecursively()
    }
}
