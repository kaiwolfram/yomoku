package app.yomoku.tests

import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.file.Files
import java.util.concurrent.CountDownLatch
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

private class EpubListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()

    override fun onEffects(requests: ByteArray) {
        effects.add(Thread.currentThread() to requests)
    }

    override fun onError(error: FfiError) {
        error("EPUB bridge error: $error")
    }

    fun dispatch(bridge: NativeBridge, event: Event, callbackCount: Int = 1): ViewModel {
        check(Event.bincodeDeserialize(event.bincodeSerialize()) == event)
        check(
            Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize()))
                .value
                .single()
                .effect is ShellEffect.Render
        )
        return completed(bridge, callbackCount)
    }

    fun completed(bridge: NativeBridge, callbackCount: Int = 1): ViewModel {
        repeat(callbackCount) {
            val (thread, bytes) = checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            check(thread != Thread.currentThread())
            check(Requests.bincodeDeserialize(bytes).value.single().effect is ShellEffect.Render)
        }
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
}

private fun epubBytes(second: String): ByteArray {
    val fixture = File("crates/yomoku-import/tests/fixtures/epub")
    val output = ByteArrayOutputStream()
    ZipOutputStream(output).use { zip ->
        val files =
            linkedMapOf(
                "mimetype" to "application/epub+zip".toByteArray(),
                "META-INF/container.xml" to fixture.resolve("container.xml").readBytes(),
                "OPS/book.opf" to fixture.resolve("book.opf").readBytes(),
                "OPS/text/z-first.xhtml" to fixture.resolve("first.xhtml").readBytes(),
                "OPS/text/a-second.xhtml" to
                    "<html xmlns='http://www.w3.org/1999/xhtml'><body><p>$second</p></body></html>"
                        .toByteArray(Charsets.UTF_8),
                "OPS/images/cover.png" to "fixture cover bytes".toByteArray(),
                "OPS/images/挿絵.png" to "fixture image bytes".toByteArray(),
            )
        for ((name, bytes) in files) {
            zip.putNextEntry(ZipEntry(name).also { it.time = 0 })
            zip.write(bytes)
            zip.closeEntry()
        }
    }
    return output.toByteArray()
}

private fun epubEvent(bytes: ByteArray, filename: String = "本𠮷.epub") =
    Event.ImportEpub(EpubImport(bytes.map { it.toUByte() }, filename))

internal fun verifyEpubImportBridge() {
    val directory = Files.createTempDirectory("yomoku-epub-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = EpubListener()
        val dictionary = HostDictionary()
        val text = "犬 犬 猫 𠮷😀か\u3099 English"
        val bytes = epubBytes(text)
        val (id, savedPosition, originalAnalysis) =
            NativeBridge(database.path, listener, dictionary).use { bridge ->
                for (invalid in
                    listOf(
                        byteArrayOf(),
                        byteArrayOf(0x82.toByte(), 0xA0.toByte()),
                        "PK truncated".toByteArray(),
                        epubBytes("<unclosed>"),
                    )) {
                    val rejected = listener.dispatch(bridge, epubEvent(invalid))
                    check(rejected.documentError == DocumentError.INVALIDEPUB)
                    check(rejected.documents.isEmpty() && rejected.importSummary == null)
                }
                check(dictionary.calls.get() == 0)
                dictionary.fail = true
                check(
                    listener.dispatch(bridge, epubEvent(bytes)).documentError ==
                        DocumentError.UNAVAILABLE
                )
                dictionary.fail = false
                val imported = listener.dispatch(bridge, epubEvent(bytes))
                check(imported.documentError == null)
                check(
                    imported.contentRoute == ContentRoute.MAIN &&
                        imported.destination == Destination.LIBRARY
                )
                check(imported.reader == null)
                val summary = checkNotNull(imported.importSummary)
                check(summary.outcome == ImportOutcome.CREATED && summary.title == "日本語の本 & EPUB")
                val id = summary.documentId
                verifyEpubAssetReads(bridge, id)
                val loaded = checkNotNull(listener.dispatch(bridge, Event.OpenReader(id)).reader)
                check(
                    loaded.text ==
                        File("crates/yomoku-import/tests/fixtures/epub/first-expected.txt")
                            .readText()
                            .trimEnd('\n')
                )
                val opened = listener.dispatch(bridge, Event.OpenReader(id))
                check(checkNotNull(opened.reader).totalPages == 2uL)
                val formatted = checkNotNull(opened.reader)
                check(formatted.formatting.size == 4)
                val image = formatted.images.single()
                check(
                    image.asset.path == "OPS/images/挿絵.png" && image.asset.mediaType == "image/png"
                )
                check(image.alt == "挿絵")
                val imageText = formatted.text.toByteArray(Charsets.UTF_8)
                check(
                    imageText
                        .copyOfRange(image.range.start.value.toInt(), image.range.end.value.toInt())
                        .toString(Charsets.UTF_8) == "\uFFFC"
                )
                for ((style, expected) in
                    listOf(
                        ReaderTextStyle.Heading(1u) to "第一章 出会い",
                        ReaderTextStyle.Emphasis to "出会い",
                        ReaderTextStyle.Strong to "𠮷😀",
                        ReaderTextStyle.ExternalLink("https://example.org/book?q=1&x=2") to "外部",
                    )) {
                    val span = formatted.formatting.single { it.style == style }
                    val utf8 = formatted.text.toByteArray(Charsets.UTF_8)
                    check(
                        utf8
                            .copyOfRange(
                                span.range.start.value.toInt(),
                                span.range.end.value.toInt(),
                            )
                            .toString(Charsets.UTF_8) == expected
                    )
                }
                val firstTokens =
                    checkNotNull(
                        listener
                            .dispatch(
                                bridge,
                                Event.LoadReaderTokens(
                                    ReaderTokenKey(
                                        id,
                                        formatted.anchor,
                                        checkNotNull(opened.readerAnalysis),
                                    )
                                ),
                            )
                            .effectiveReaderTokens
                    )
                val moved =
                    listener.dispatch(
                        bridge,
                        Event.CompleteEffectiveReaderPage(
                            firstTokens.identity,
                            formatted.position,
                            1234L,
                        ),
                        callbackCount = 2,
                    )
                val second = checkNotNull(moved.reader)
                check(second.text == text)
                val position = second.position
                check(checkNotNull(moved.reader).pageNumber == 2uL)
                check(checkNotNull(moved.reader).formatting.isEmpty())
                check(checkNotNull(moved.reader).images.isEmpty())
                val analysis = checkNotNull(moved.readerAnalysis) as ReaderAnalysis.Available
                check(analysis != opened.readerAnalysis)
                val key = ReaderTokenKey(id, checkNotNull(moved.reader).anchor, analysis)
                val tokens =
                    checkNotNull(
                        listener.dispatch(bridge, Event.LoadReaderTokens(key)).effectiveReaderTokens
                    )
                val dogs = tokens.tokens.filter { it.surface == "犬" }
                check(dogs.size == 2 && dogs.all { it.vocabulary?.status == VocabularyStatus.NEW })
                check(dogs.all { it.anchor.chapterId == second.anchor.chapterId })
                listener.dispatch(
                    bridge,
                    Event.SetEffectiveReaderStatus(
                        EffectiveReaderSelection(tokens.identity, dogs[0].segmentIndex),
                        checkNotNull(dogs[0].vocabulary).id,
                        VocabularyStatus.LEARNING,
                        1000L,
                    ),
                )
                val savedPosition =
                    position.copy(
                        offset = TextOffset("犬 ".toByteArray(Charsets.UTF_8).size.toLong())
                    )
                val saved = listener.dispatch(bridge, Event.SaveReadingPosition(savedPosition))
                check(saved.documents.single().progressPercent in 1uL..99uL)
                bridge.setForeground(false)
                listener.dispatch(bridge, Event.RefreshLibrary)
                dictionary.fail = true
                val duplicate = listener.dispatch(bridge, epubEvent(bytes, ""))
                check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
                check(duplicate.documents == saved.documents)
                check(duplicate.reader == null)
                check(dictionary.calls.get() == 2)
                Triple(id, savedPosition, analysis)
            }
        val fresh = HostDictionary().also { it.fail = true }
        NativeBridge(database.path, listener, fresh).use { bridge ->
            verifyEpubAssetReads(bridge, id)
            val duplicate = listener.dispatch(bridge, epubEvent(bytes, ""))
            check(
                duplicate.importSummary?.documentId == id &&
                    duplicate.importSummary.outcome == ImportOutcome.EXISTING
            )
            val loaded = checkNotNull(listener.dispatch(bridge, Event.OpenReader(id)).reader)
            check(loaded.totalPages == 2UL && loaded.position == savedPosition)
            val opened = listener.dispatch(bridge, Event.OpenReader(id))
            check(
                opened.readerAnalysis == originalAnalysis &&
                    checkNotNull(opened.reader).pageNumber == 2uL
            )
            val key = ReaderTokenKey(id, checkNotNull(opened.reader).anchor, originalAnalysis)
            val tokens =
                checkNotNull(
                    listener.dispatch(bridge, Event.LoadReaderTokens(key)).effectiveReaderTokens
                )
            check(
                tokens.tokens
                    .filter { it.surface == "犬" }
                    .all { it.vocabulary?.status == VocabularyStatus.LEARNING }
            )
            for (event in
                listOf(
                    Event.ImportTxt(
                        TxtImport(bytes.map { it.toUByte() }, "", TextEncoding.SHIFTJIS)
                    ),
                    Event.ImportSrt(
                        SrtImport(bytes.map { it.toUByte() }, "", TextEncoding.SHIFTJIS)
                    ),
                )) {
                val crossFormat = listener.dispatch(bridge, event)
                check(
                    crossFormat.importSummary?.documentId == id &&
                        crossFormat.importSummary.outcome == ImportOutcome.EXISTING
                )
            }
            check(fresh.calls.get() == 0)
            // First interpretation also wins for non-container bytes submitted as EPUB.
            fresh.fail = false
            val plain = "English first interpretation".toByteArray()
            val imported =
                listener.dispatch(
                    bridge,
                    Event.ImportTxt(TxtImport(plain.map { it.toUByte() }, "first.txt", null)),
                )
            val cross = listener.dispatch(bridge, epubEvent(plain, ""))
            check(
                cross.importSummary?.documentId == imported.importSummary?.documentId &&
                    cross.importSummary?.outcome == ImportOutcome.EXISTING
            )
        }
        verifyEpubAssetAfterClose(database, id)
        verifyEpubCancellationBridge()
        println(
            "PASS: typed EPUB whole-book import, real Full per-chapter Mode C, container errors, duplicates/status/progress/reopen and direct Cancel JNI"
        )
    } finally {
        directory.deleteRecursively()
    }
}

private fun verifyEpubAssetAfterClose(database: File, id: DocumentId) {
    val entered = CountDownLatch(1)
    val release = CountDownLatch(1)
    val provider =
        object : DictionaryProvider {
            override fun verifiedWordDictionaryPath(): String = error("no word lookup")

            override fun openVerifiedDictionary(): Long {
                entered.countDown()
                check(release.await(60, TimeUnit.SECONDS))
                return 0L
            }
        }
    val bridge = NativeBridge(database.path, EpubListener(), provider)
    val response = AssetResponse()
    try {
        bridge.update(Event.CreatePaste("new queued paste".repeat(2)).bincodeSerialize())
        check(entered.await(60, TimeUnit.SECONDS))
        bridge.loadEpubImageAsset(id.value, "OPS/images/挿絵.png", "image/png", response)
        bridge.close()
    } finally {
        release.countDown()
        bridge.close()
    }
    response.awaitBytes("fixture image bytes".toByteArray())
}

private fun verifyEpubCancellationBridge() {
    val directory = Files.createTempDirectory("yomoku-epub-cancel-jni-").toFile()
    try {
        val listener = EpubListener()
        val database = directory.resolve("db.sqlite3")
        val prior =
            NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
                listener
                    .dispatch(
                        bridge,
                        Event.ImportTxt(
                            TxtImport("prior".toByteArray().map { it.toUByte() }, "prior.txt", null)
                        ),
                    )
                    .documents
            }
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        val delegate = HostDictionary()
        val provider =
            object : DictionaryProvider by delegate {
                override fun openVerifiedDictionary(): Long {
                    entered.countDown()
                    check(release.await(60, TimeUnit.SECONDS))
                    return delegate.openVerifiedDictionary()
                }
            }
        val bytes = epubBytes("犬 犬")
        NativeBridge(database.path, listener, provider).use { bridge ->
            bridge.cancelFileImport()
            listener.dispatch(bridge, Event.RefreshLibrary)
            bridge.update(epubEvent(bytes).bincodeSerialize())
            try {
                check(entered.await(60, TimeUnit.SECONDS))
                bridge.cancelFileImport() // The dictionary provider blocks the FIFO.
                check(ViewModel.bincodeDeserialize(bridge.view()).storageBusy)
                check(listener.effects.isEmpty())
            } finally {
                release.countDown()
            }
            val cancelled = listener.completed(bridge)
            check(cancelled.documentError == null && cancelled.importSummary == null)
            check(cancelled.documents == prior)
            check(listener.dispatch(bridge, Event.RefreshLibrary).documents == prior)
            bridge.cancelFileImport()
            val retried = listener.dispatch(bridge, epubEvent(bytes))
            check(retried.documentError == null && retried.documents.size == 2)
            check(retried.importSummary?.outcome == ImportOutcome.CREATED)
            val loaded =
                listener.dispatch(
                    bridge,
                    Event.OpenReader(checkNotNull(retried.importSummary).documentId),
                )
            check(checkNotNull(loaded.reader).totalPages == 2UL)
            check(delegate.calls.get() == 1)
        }
    } finally {
        directory.deleteRecursively()
    }
}
