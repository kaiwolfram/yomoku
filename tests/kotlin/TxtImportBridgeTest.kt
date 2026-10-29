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

private class TxtListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()
    override fun onEffects(requests: ByteArray) { effects.add(Thread.currentThread() to requests) }
    override fun onError(error: FfiError) { error("TXT bridge error: $error") }

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

internal fun verifyTxtImportBridge() {
    val directory = Files.createTempDirectory("yomoku-txt-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = TxtListener()
        val dictionary = HostDictionary()
        fun event(bytes: ByteArray, encoding: TextEncoding? = null, filename: String = "本𠮷.txt") =
            Event.ImportTxt(TxtImport(bytes.map { it.toUByte() }, filename, encoding))
        val text = "\uFEFF｜猫《ねこ》と猫が食べた。\r\n𠮷😀か\u3099\u0000終"
        val bytes = text.toByteArray(Charsets.UTF_8)
        val canonical = "猫と猫が食べた。\r\n𠮷😀か\u3099\u0000終"
        val (id, analysis, position) = NativeBridge(database.path, listener, dictionary).use { bridge ->
            val cases = listOf(
                event(byteArrayOf(0xC2.toByte(), 0xA9.toByte())) to DocumentError.TXTENCODINGREQUIRED,
                event(byteArrayOf(0x82.toByte(), 0xA0.toByte()), TextEncoding.UTF8) to DocumentError.TXTINVALIDUTF8,
                event(byteArrayOf(-1), TextEncoding.SHIFTJIS) to DocumentError.TXTINVALIDSHIFTJIS,
                event(byteArrayOf(-1, -2, 0x41, 0)) to DocumentError.TXTUNSUPPORTEDENCODING,
                event(byteArrayOf(-1)) to DocumentError.TXTUNDECODABLE,
                event(byteArrayOf(0x20, 0x0D, 0x0A)) to DocumentError.EMPTYTXT,
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
            check(imported.documents.single().title == "本𠮷.txt")
            val id = imported.documents.single().documentId
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.chapters.single().text == canonical)
            val ruby = loaded.sourceRuby.single()
            check(ruby.chapterId == loaded.chapters.single().chapterId)
            check(ruby.ruby.single().reading == "ねこ")
            check(ruby.ruby.single().range == TextRange(TextOffset(0L), TextOffset(3L)))
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            val analysis = checkNotNull(opened.readerAnalysis) as ReaderAnalysis.Available
            val page = checkNotNull(opened.reader)
            check(page.sourceRuby == ruby.ruby)
            check(opened.readerFurigana.isEmpty()) // default off, source metadata stays available
            bridge.update(Event.OpenSettings.bincodeSerialize())
            val enabled = listener.dispatch(bridge, Event.SetReaderTypography(checkNotNull(opened.readerTypography).copy(furigana = true)))
            check(enabled.readerFurigana == page.sourceRuby) // before any token lookup
            check(enabled.reader == page && enabled.loadedDocument == loaded)
            bridge.update(Event.CloseSettings.bincodeSerialize())
            val key = ReaderTokenKey(id, page.anchor, analysis)
            val tokenView = listener.dispatch(bridge, Event.LoadReaderTokens(key))
            val tokens = checkNotNull(tokenView.effectiveReaderTokens)
            val cats = tokens.tokens.filter { it.surface == "猫" }
            check(cats.size == 2 && cats.all { it.vocabulary?.status == VocabularyStatus.NEW })
            check(tokenView.readerFurigana.first() == page.sourceRuby.single())
            check(tokenView.readerFurigana.first().reading != cats.first().reading) // literal source hiragana wins over analyzer katakana
            check(tokenView.readerFurigana.single { it.range == cats[1].displayRange }.reading == cats[1].reading)
            check(tokenView.readerFurigana.size == tokens.tokens.count { it.reading != null })
            val vocabularyId = checkNotNull(cats.first().vocabulary).id
            val familiar = listener.dispatch(bridge, Event.SetEffectiveReaderStatus(EffectiveReaderSelection(tokens.identity, cats.first().segmentIndex), vocabularyId, VocabularyStatus.FAMILIAR, 1000L))
            check(familiar.readerFurigana == tokenView.readerFurigana)
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
            val duplicate = listener.dispatch(bridge, event(bytes, TextEncoding.SHIFTJIS, "renamed.txt"))
            check(duplicate.documents.single().title == "本𠮷.txt")
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.readingPosition == position && loaded.chapters.single().text == canonical)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            check(opened.readerAnalysis == analysis)
            val page = checkNotNull(opened.reader)
            check(page.sourceRuby.single().reading == "ねこ")
            check(page.sourceRuby == loaded.sourceRuby.single().ruby)
            check(opened.readerTypography?.furigana == true)
            check(opened.readerFurigana == page.sourceRuby) // no Full and no token snapshot yet
            val tokenView = listener.dispatch(bridge, Event.LoadReaderTokens(ReaderTokenKey(id, page.anchor, analysis)))
            val tokens = checkNotNull(tokenView.effectiveReaderTokens)
            check(tokens.tokens.filter { it.surface == "猫" }.all { it.vocabulary?.status == VocabularyStatus.FAMILIAR })
            check(tokenView.readerFurigana.first() == page.sourceRuby.single())
            val secondCat = tokens.tokens.filter { it.surface == "猫" }[1]
            check(tokenView.readerFurigana.single { it.range == secondCat.displayRange }.reading == secondCat.reading)
            check(fresh.calls.get() == 0)
            fresh.fail = false
            val selected = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.UTF8, "ambiguous.txt"))
            val selectedId = checkNotNull(selected.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(selectedId)).loadedDocument).chapters.single().text == "©")
            val existing = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.SHIFTJIS))
            check(existing.importSummary?.documentId == selectedId && existing.importSummary.outcome == ImportOutcome.EXISTING)
            val legacy = listener.dispatch(bridge, event(byteArrayOf(0x82.toByte(), 0xA0.toByte()), null, "legacy.txt"))
            val legacyId = checkNotNull(legacy.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(legacyId)).loadedDocument).chapters.single().text == "あ")
            check(fresh.calls.get() == 1)
        }
        println("PASS: typed TXT bytes/encodings, atomic import summary, original Mode B, duplicate/reopen and lazy Full JNI")
        verifyTxtCancellationBridge()
    } finally {
        directory.deleteRecursively()
    }
}

private fun verifyTxtCancellationBridge() {
    val directory = Files.createTempDirectory("yomoku-txt-cancel-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = TxtListener()
        fun event(text: String) = Event.ImportTxt(TxtImport(text.toByteArray(Charsets.UTF_8).map { it.toUByte() }, "book.txt", TextEncoding.UTF8))
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
            bridge.cancelTxtImport() // Idle calls cannot cancel the next accepted file.
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.single().documentId == first)
            bridge.update(event("犬 犬").bincodeSerialize())
            check(entered.await(60, TimeUnit.SECONDS))
            val returned = CountDownLatch(1)
            val failure = AtomicReference<Throwable?>()
            val cancel = thread(name = "txt-cancel-host") {
                try {
                    bridge.cancelTxtImport()
                    bridge.cancelTxtImport()
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
            bridge.cancelTxtImport()
            val retry = listener.dispatch(bridge, event("犬 犬"))
            check(retry.documents.size == 2 && retry.importSummary?.outcome == ImportOutcome.CREATED)
            val imported = checkNotNull(retry.importSummary).documentId
            val opened = listener.dispatch(bridge, Event.OpenReader(imported))
            val page = checkNotNull(opened.reader)
            val key = ReaderTokenKey(imported, page.anchor, checkNotNull(opened.readerAnalysis))
            val tokens = checkNotNull(listener.dispatch(bridge, Event.LoadReaderTokens(key)).effectiveReaderTokens)
            check(tokens.tokens.filter { it.surface == "犬" }.let { it.size == 2 && it.all { token -> token.vocabulary?.status == VocabularyStatus.NEW } })
            check(delegate.calls.get() == 1)
            bridge.cancelTxtImport() // No effect on a committed file or on reader work.
        }
        NativeBridge(database.path, listener, HostDictionary().also { it.fail = true }).use { bridge ->
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents.size == 2)
            check(listener.dispatch(bridge, event("犬 犬")).importSummary?.outcome == ImportOutcome.EXISTING)
        }
        println("PASS: direct TXT cancellation bypasses blocked JNI worker, retains completed files, retries fresh and reopens")
    } finally {
        directory.deleteRecursively()
    }
}
