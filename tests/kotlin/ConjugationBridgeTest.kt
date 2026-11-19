package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyConjugationBridge() {
    val directory = Files.createTempDirectory("yomoku-conjugations-").toFile()
    val callbacks = LinkedBlockingQueue<ByteArray>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                callbacks.add(requests)
            }

            override fun onError(error: FfiError) {
                throw error
            }
        }
    val database = directory.resolve("reader.sqlite3")
    fun current(bridge: NativeBridge) = ViewModel.bincodeDeserialize(bridge.view())
    fun dispatch(bridge: NativeBridge, event: Event, count: Int = 1): ViewModel {
        bridge.update(event.bincodeSerialize())
        repeat(count) { checkNotNull(callbacks.poll(60, TimeUnit.SECONDS)) }
        return current(bridge).also { check(!it.storageBusy && it.documentError == null) }
    }
    fun tokens(bridge: NativeBridge): EffectiveReaderPage {
        val view = current(bridge)
        val page = checkNotNull(view.reader)
        return checkNotNull(
            dispatch(
                    bridge,
                    Event.LoadReaderTokens(
                        ReaderTokenKey(
                            page.documentId,
                            page.anchor,
                            checkNotNull(view.readerAnalysis),
                        )
                    ),
                )
                .effectiveReaderTokens
        )
    }
    fun word(
        bridge: NativeBridge,
        page: EffectiveReaderPage,
        surface: String = "食べました",
    ): EffectiveWordDetails {
        val token = page.tokens.first { it.surface == surface }
        return checkNotNull(
            dispatch(
                    bridge,
                    Event.LoadEffectiveReaderWord(
                        EffectiveReaderSelection(page.identity, token.segmentIndex)
                    ),
                )
                .effectiveReaderWord
                ?.details
        )
    }
    try {
        lateinit var id: DocumentId
        NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
            // Repeated joined forms occur on separate logical pages.
            val text = "x".repeat(497) + ". 食べました。\n食べました。"
            id = checkNotNull(dispatch(bridge, Event.CreatePaste(text)).importSummary).documentId
            dispatch(bridge, Event.OpenReader(id))
            bridge.update(Event.OpenSettings.bincodeSerialize())
            dispatch(bridge, Event.SetPageLength(PageLength.CHARACTERS500))
            bridge.update(Event.CloseSettings.bincodeSerialize())
            val first = tokens(bridge)
            check(first.tokens.map { it.surface } == listOf("食べました"))
            val token = first.tokens.single()
            check(token.dictionaryForm == "食べる")
            val entry = checkNotNull(token.vocabulary)
            dispatch(
                bridge,
                Event.SetEffectiveReaderStatus(
                    EffectiveReaderSelection(first.identity, token.segmentIndex),
                    entry.id,
                    VocabularyStatus.LEARNING,
                    100L,
                ),
            )
            val updated = tokens(bridge)
            val details = word(bridge, updated)
            check(details.encountered == 1UL)
            check(details.candidates.surface == "食べました")
            check(details.candidates.dictionaryForm == "食べる")
            check(details.candidates.words.flatMap { it.forms }.any { it.form == "食べる" })
            check(details.lookup.parts.any { it.surface == "食べ" && it.dictionaryForm == "食べる" })
            check(
                details.candidates.words
                    .flatMap { it.forms }
                    .all { it.form in listOf("食べました", "食べる") }
            )
            val position = checkNotNull(current(bridge).reader).position
            dispatch(bridge, Event.CompleteEffectiveReaderPage(updated.identity, position, 200L), 2)
            val second = tokens(bridge)
            check(
                second.tokens.all {
                    it.surface == "食べました" &&
                        it.vocabulary?.id == entry.id &&
                        it.vocabulary.status == VocabularyStatus.LEARNING
                }
            )
            check(word(bridge, second).encountered == 2UL)
            dispatch(
                bridge,
                Event.CompleteEffectiveReaderPage(
                    second.identity,
                    checkNotNull(current(bridge).reader).position,
                    300L,
                ),
                2,
            )
            val library = dispatch(bridge, Event.LoadLibraryWordDistributions)
            check(
                library.libraryWordDistributions?.single()?.counts ==
                    WordStatusCounts(0UL, 1UL, 0UL, 0UL, 0UL)
            )
            val duplicate = dispatch(bridge, Event.CreatePaste(text))
            check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
            check(duplicate.importSummary.documentId == id)
        }
        NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
            dispatch(bridge, Event.OpenReader(id))
            val page = tokens(bridge)
            check(page.tokens.single().vocabulary?.status == VocabularyStatus.LEARNING)
            check(word(bridge, page).encountered == 3UL)

            val text = "すごっ。端っこ。猫っぽかった。走って帰る。その日。動き出した。"
            val imported = checkNotNull(dispatch(bridge, Event.CreatePaste(text)).importSummary)
            dispatch(bridge, Event.OpenReader(imported.documentId))
            val merged = tokens(bridge)
            check(
                merged.tokens.map { it.surface } ==
                    listOf("すごっ", "端っこ", "猫っぽかった", "走って", "帰る", "その日", "動き出した")
            )
            val details = word(bridge, merged, "動き出した")
            check(details.candidates.surface == "動き出した")
            check(details.candidates.dictionaryForm == "動き出す")
            check(details.candidates.words.flatMap { it.forms }.any { it.form == "動き出す" })
        }
        println(
            "PASS: joined grammar and compounds through JNI, base/components, counts, status and reopen"
        )
    } finally {
        directory.deleteRecursively()
    }
}
