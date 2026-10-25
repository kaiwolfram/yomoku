package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

private class TxtListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()
    override fun onEffects(requests: ByteArray) { effects.add(Thread.currentThread() to requests) }
    override fun onError(error: FfiError) { error("TXT bridge error: $error") }

    fun dispatch(bridge: NativeBridge, event: Event): ViewModel {
        check(Event.bincodeDeserialize(event.bincodeSerialize()) == event)
        check(Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.single().effect is ShellEffect.Render)
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
            check(loaded.chapter.text == canonical)
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
            val duplicate = listener.dispatch(bridge, event(bytes, TextEncoding.SHIFTJIS, "renamed.txt"))
            check(duplicate.documents.single().title == "本𠮷.txt")
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            val loaded = checkNotNull(listener.dispatch(bridge, Event.LoadPaste(id)).loadedDocument)
            check(loaded.readingPosition == position && loaded.chapter.text == canonical)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            check(opened.readerAnalysis == analysis)
            val page = checkNotNull(opened.reader)
            val tokens = checkNotNull(listener.dispatch(bridge, Event.LoadReaderTokens(ReaderTokenKey(id, page.anchor, analysis))).readerTokens)
            check(tokens.tokens.filter { it.surface == "猫" }.all { it.vocabulary?.status == VocabularyStatus.FAMILIAR })
            check(fresh.calls.get() == 0)
            fresh.fail = false
            val selected = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.UTF8, "ambiguous.txt"))
            val selectedId = checkNotNull(selected.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(selectedId)).loadedDocument).chapter.text == "©")
            val existing = listener.dispatch(bridge, event(byteArrayOf(0xC2.toByte(), 0xA9.toByte()), TextEncoding.SHIFTJIS))
            check(existing.importSummary?.documentId == selectedId && existing.importSummary.outcome == ImportOutcome.EXISTING)
            val legacy = listener.dispatch(bridge, event(byteArrayOf(0x82.toByte(), 0xA0.toByte()), null, "legacy.txt"))
            val legacyId = checkNotNull(legacy.importSummary).documentId
            check(checkNotNull(listener.dispatch(bridge, Event.LoadPaste(legacyId)).loadedDocument).chapter.text == "あ")
            check(fresh.calls.get() == 1)
        }
        println("PASS: typed TXT bytes/encodings, atomic import summary, original Mode B, duplicate/reopen and lazy Full JNI")
    } finally {
        directory.deleteRecursively()
    }
}
