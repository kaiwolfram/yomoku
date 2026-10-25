package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread

private class SrtListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()
    override fun onEffects(requests: ByteArray) { effects.add(Thread.currentThread() to requests) }
    override fun onError(error: FfiError) { error("SRT bridge error: $error") }

    fun dispatch(bridge: NativeBridge, event: Event): ViewModel {
        check(Event.bincodeDeserialize(event.bincodeSerialize()) == event)
        check(Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.single().effect is ShellEffect.Render)
        return completed(bridge)
    }

    fun completed(bridge: NativeBridge): ViewModel {
        val (thread, bytes) = checkNotNull(effects.poll(60, TimeUnit.SECONDS))
        check(thread != Thread.currentThread())
        check(Requests.bincodeDeserialize(bytes).value.single().effect is ShellEffect.Render)
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
}

internal fun verifySrtImportBridge() {
    val directory = Files.createTempDirectory("yomoku-srt-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = SrtListener()
        val dictionary = HostDictionary()
        fun event(bytes: ByteArray, encoding: TextEncoding? = null, filename: String = "本𠮷.srt") =
            Event.ImportSrt(SrtImport(bytes.map { it.toUByte() }, filename, encoding))
        val text = "\uFEFF1\r\n00:00:01,000 --> 00:00:02,000\r\n<i>猫</i>と猫が食べた。\r\n\r\n2\r\n00:00:02,000 --> 00:00:03,000\r\n｜𠮷《よし》😀か\u3099\u0000終"
        val bytes = text.toByteArray(Charsets.UTF_8)
        val canonical = "猫と猫が食べた。\n\n｜𠮷《よし》😀か\u3099\u0000終"
        val (id, analysis, position) = NativeBridge(database.path, listener, dictionary).use { bridge ->
            val cases = listOf(
                event(byteArrayOf(0xC2.toByte(), 0xA9.toByte())) to DocumentError.SRTENCODINGREQUIRED,
                event(byteArrayOf(0x82.toByte(), 0xA0.toByte()), TextEncoding.UTF8) to DocumentError.SRTINVALIDUTF8,
                event(byteArrayOf(-1), TextEncoding.SHIFTJIS) to DocumentError.SRTINVALIDSHIFTJIS,
                event(byteArrayOf(-1, -2, 0x41, 0)) to DocumentError.SRTUNSUPPORTEDENCODING,
                event(byteArrayOf(-1)) to DocumentError.SRTUNDECODABLE,
                event(byteArrayOf(0x20, 0x0D, 0x0A)) to DocumentError.EMPTYSRT,
                event("1\n00:00:01,000 --> 00:00:02,000\n<i> </i>".toByteArray(Charsets.UTF_8)) to DocumentError.EMPTYSRT,
            )
            for ((input, error) in cases) {
                val rejected = listener.dispatch(bridge, input)
                check(rejected.documentError == error)
                check(rejected.documents.isEmpty() && rejected.importSummary == null)
            }
            check(dictionary.calls.get() == 0)
            dictionary.fail = true
            check(listener.dispatch(bridge, event(bytes)).documentError == DocumentError.UNAVAILABLE)
            dictionary.fail = false
            val imported = listener.dispatch(bridge, event(bytes))
            check(imported.destination == Destination.LIBRARY && imported.contentRoute == ContentRoute.MAIN)
            check(imported.reader == null && imported.loadedDocument == null)
            check(imported.importSummary?.outcome == ImportOutcome.CREATED)
            check(imported.documents.single().title == "本𠮷.srt")
            val id = imported.documents.single().documentId
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.chapters.single().text == canonical)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            val analysis = checkNotNull(opened.readerAnalysis) as ReaderAnalysis.Available
            val page = checkNotNull(opened.reader)
            val key = ReaderTokenKey(id, page.anchor, analysis)
            val tokens = checkNotNull(listener.dispatch(bridge, Event.LoadReaderTokens(key)).readerTokens)
            val cats = tokens.tokens.filter { it.surface == "猫" }
            check(cats.size == 2 && cats.all { it.vocabulary?.status == VocabularyStatus.NEW })
            val vocabularyId = checkNotNull(cats.first().vocabulary).id
            listener.dispatch(bridge, Event.SetReaderStatus(ReaderStatusChange(key, vocabularyId, VocabularyStatus.FAMILIAR, 1000L)))
            val position = page.position.copy(offset = TextOffset("猫".toByteArray(Charsets.UTF_8).size.toLong()))
            listener.dispatch(bridge, Event.SaveReadingPosition(position))
            check(dictionary.calls.get() == 2)
            bridge.setForeground(false)
            // FIFO callback is a barrier after background cache release.
            listener.dispatch(bridge, Event.RefreshLibrary)
            dictionary.fail = true
            val duplicate = listener.dispatch(bridge, event(bytes, TextEncoding.SHIFTJIS, ""))
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            check(duplicate.importSummary.documentId == id)
            check(duplicate.reader == null && duplicate.loadedDocument == null)
            check(dictionary.calls.get() == 2)
            Triple(id, analysis, position)
        }
        val fresh = HostDictionary().also { it.fail = true }
        NativeBridge(database.path, listener, fresh).use { bridge ->
            val duplicate = listener.dispatch(bridge, event(bytes, TextEncoding.SHIFTJIS, "renamed.srt"))
            check(duplicate.documents.single().title == "本𠮷.srt")
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.readingPosition == position && loaded.chapters.single().text == canonical)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            check(opened.readerAnalysis == analysis)
            val page = checkNotNull(opened.reader)
            val tokens = checkNotNull(listener.dispatch(bridge, Event.LoadReaderTokens(ReaderTokenKey(id, page.anchor, analysis))).readerTokens)
            check(tokens.tokens.filter { it.surface == "猫" }.all { it.vocabulary?.status == VocabularyStatus.FAMILIAR })
            val crossFormat = listener.dispatch(bridge, Event.ImportTxt(TxtImport(bytes.map { it.toUByte() }, "different.txt", TextEncoding.SHIFTJIS)))
            check(crossFormat.importSummary?.documentId == id && crossFormat.importSummary.outcome == ImportOutcome.EXISTING)
            check(fresh.calls.get() == 0)
            fresh.fail = false
            val selected = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.UTF8, "ambiguous.srt"))
            val selectedId = checkNotNull(selected.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(selectedId)).loadedDocument).chapters.single().text == "©")
            val existing = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.SHIFTJIS))
            check(existing.importSummary?.documentId == selectedId && existing.importSummary.outcome == ImportOutcome.EXISTING)
            val legacy = listener.dispatch(bridge, event(byteArrayOf(0x82.toByte(), 0xA0.toByte()), null, "legacy.srt"))
            val legacyId = checkNotNull(legacy.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(legacyId)).loadedDocument).chapters.single().text == "あ")
            check(fresh.calls.get() == 1)
        }
        println("PASS: typed SRT bytes/encodings, atomic import summary, original Mode B, duplicate/reopen and lazy Full JNI")
        verifySrtCancellationBridge()
    } finally {
        directory.deleteRecursively()
    }
}

private fun verifySrtCancellationBridge() {
    val directory = Files.createTempDirectory("yomoku-srt-cancel-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = SrtListener()
        fun event(text: String) = Event.ImportSrt(SrtImport(text.toByteArray(Charsets.UTF_8).map { it.toUByte() }, "book.srt", TextEncoding.UTF8))
        val first = NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
            checkNotNull(listener.dispatch(bridge, event("猫")).importSummary).documentId
        }
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val delegate = HostDictionary()
        val provider = object : DictionaryProvider by delegate {
            override fun openVerifiedDictionary(): Long {
                entered.countDown()
                check(release.await(60, TimeUnit.SECONDS))
                return delegate.openVerifiedDictionary()
            }
        }
        NativeBridge(database.path, listener, provider).use { bridge ->
            bridge.cancelFileImport() // Idle calls cannot cancel the next accepted file.
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.single().documentId == first)
            bridge.update(event("犬 犬").bincodeSerialize())
            check(entered.await(60, TimeUnit.SECONDS))
            val returned = CountDownLatch(1)
            val failure = AtomicReference<Throwable?>()
            val cancel = thread(name = "srt-cancel-host") {
                try {
                    bridge.cancelFileImport()
                    bridge.cancelFileImport()
                } catch (error: Throwable) {
                    failure.set(error)
                } finally {
                    returned.countDown()
                }
            }
            try {
                check(returned.await(10, TimeUnit.SECONDS)) // Must bypass the blocked FIFO.
                check(failure.get() == null)
                check(ViewModel.bincodeDeserialize(bridge.view()).storageBusy)
                check(listener.effects.isEmpty())
            } finally {
                release.countDown()
                cancel.join(60_000)
            }
            val cancelled = listener.completed(bridge)
            check(cancelled.documentError == null && cancelled.importSummary == null)
            check(cancelled.documents.single().documentId == first)
            check(cancelled.reader == null && cancelled.contentRoute == ContentRoute.MAIN)
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.single().documentId == first)
            bridge.cancelFileImport()
            val retry = listener.dispatch(bridge, event("犬 犬"))
            check(retry.documents.size == 2 && retry.importSummary?.outcome == ImportOutcome.CREATED)
            val imported = checkNotNull(retry.importSummary).documentId
            val opened = listener.dispatch(bridge, Event.OpenReader(imported))
            val page = checkNotNull(opened.reader)
            val key = ReaderTokenKey(imported, page.anchor, checkNotNull(opened.readerAnalysis))
            val tokens = checkNotNull(listener.dispatch(bridge, Event.LoadReaderTokens(key)).readerTokens)
            check(tokens.tokens.filter { it.surface == "犬" }.let { it.size == 2 && it.all { token -> token.vocabulary?.status == VocabularyStatus.NEW } })
            check(delegate.calls.get() == 1)
            bridge.cancelFileImport() // No effect on a committed file or on reader work.
        }
        NativeBridge(database.path, listener, HostDictionary().also { it.fail = true }).use { bridge ->
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.size == 2)
            check(listener.dispatch(bridge, event("犬 犬")).importSummary?.outcome == ImportOutcome.EXISTING)
        }
        println("PASS: direct SRT cancellation bypasses blocked JNI worker, retains completed files, retries fresh and reopens")
    } finally {
        directory.deleteRecursively()
    }
}
