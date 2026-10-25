package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ReaderWordSelection
import app.yomoku.shared.ViewModel
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyReaderWordBridge() {
    val directory = Files.createTempDirectory("yomoku-word-jni-").toFile()
    val callbacks = LinkedBlockingQueue<Thread>()
    val listener = object : EffectsListener {
        override fun onEffects(requests: ByteArray) { callbacks.add(Thread.currentThread()) }
        override fun onError(error: FfiError) { error("Unexpected bridge failure: $error") }
    }
    fun dispatch(bridge: NativeBridge, event: Event, count: Int = 1): ViewModel {
        bridge.update(event.bincodeSerialize())
        repeat(count) { check(checkNotNull(callbacks.poll(60, TimeUnit.SECONDS)) != Thread.currentThread()) }
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
    val provider = HostDictionary()
    val database = directory.resolve("user.sqlite3")
    try {
        val id = NativeBridge(database.path, listener, provider).use { bridge ->
            dispatch(bridge, Event.RefreshLibrary)
            check(provider.wordCalls.get() == 0)
            val id = dispatch(bridge, Event.CreatePaste("東京 東京 食べた")).documents.single().documentId
            val opened = dispatch(bridge, Event.OpenReader(id))
            val page = checkNotNull(opened.reader)
            val key = ReaderTokenKey(id, page.anchor, checkNotNull(opened.readerAnalysis))
            val tokens = checkNotNull(dispatch(bridge, Event.LoadReaderTokens(key)).readerTokens)
            check(provider.wordCalls.get() == 0)
            val token = tokens.tokens.first { it.surface == "東京" }
            val selection = ReaderWordSelection(key, token.tokenIndex)
            val before = database.readBytes()
            provider.failWords = true
            val failed = dispatch(bridge, Event.LoadReaderWord(selection))
            check(failed.readerWord == null && failed.readerWordError != null)
            check(failed.documentError == null)
            provider.failWords = false
            val loaded = dispatch(bridge, Event.LoadReaderWord(selection))
            val word = checkNotNull(loaded.readerWord)
            check(loaded.readerWordSelection == selection && loaded.readerWordError == null)
            check(word.token == token && word.encountered == 0UL)
            check(word.candidates.words.isNotEmpty() && word.candidates.names.isNotEmpty())
            check(word.candidates.words.all { it.entry.senses.isNotEmpty() && it.entry.readings.isNotEmpty() })
            check(word.candidates.names.all { it.entry.names.isNotEmpty() })
            check(word.candidates.words.all { candidate -> candidate.forms.all { form ->
                form.matches.all { match -> match.senseIndices.all { it.toInt() < candidate.entry.senses.size } }
            } })
            check(ViewModel.bincodeDeserialize(loaded.bincodeSerialize()) == loaded)
            check(before.contentEquals(database.readBytes()))
            check(loaded.reader == page && loaded.readerTime == null && loaded.completionNotice == null)
            check(provider.wordCalls.get() == 2)
            check(provider.wordThreads.all { it != Thread.currentThread() })
            val inflected = tokens.tokens.first { it.surface == "食べ" }
            val inflectedDetails = checkNotNull(dispatch(bridge, Event.LoadReaderWord(
                ReaderWordSelection(key, inflected.tokenIndex))).readerWord)
            check(inflectedDetails.token.surface == "食べ")
            check(inflectedDetails.token.dictionaryForm == "食べる")
            check(inflectedDetails.candidates.dictionaryForm == "食べる")
            check(inflectedDetails.candidates.words.isNotEmpty())
            check(provider.wordCalls.get() == 2) // same worker keeps the immutable index open
            dispatch(bridge, Event.CompleteReaderPage(id, page.anchor, page.position, 5678L), 2)
            id
        }
        val fresh = HostDictionary()
        NativeBridge(database.path, listener, fresh).use { bridge ->
            val opened = dispatch(bridge, Event.OpenReader(id))
            check(opened.readerWord == null && opened.readerWordSelection == null)
            val page = checkNotNull(opened.reader)
            val key = ReaderTokenKey(id, page.anchor, checkNotNull(opened.readerAnalysis))
            val token = checkNotNull(dispatch(bridge, Event.LoadReaderTokens(key)).readerTokens).tokens.first { it.surface == "東京" }
            val before = database.readBytes()
            val details = checkNotNull(dispatch(bridge, Event.LoadReaderWord(ReaderWordSelection(key, token.tokenIndex))).readerWord)
            check(details.encountered == 2UL)
            check(fresh.calls.get() == 0 && fresh.wordCalls.get() == 1)
            check(before.contentEquals(database.readBytes()))
        }
    } finally { directory.deleteRecursively() }
    println("PASS: selected-word JNI word/name payloads, base lookup, count, retry, lazy workers and read-only reopen")
}
