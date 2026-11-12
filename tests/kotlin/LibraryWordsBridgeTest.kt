package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyLibraryWordsBridge() {
    val directory = Files.createTempDirectory("yomoku-library-words-").toFile()
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
            fun dispatch(event: Event, asynchronous: Boolean = false): ViewModel {
                bridge.update(event.bincodeSerialize())
                if (asynchronous) checkNotNull(effects.poll(60, TimeUnit.SECONDS))
                return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
            }
            fun words(): List<LibraryWordDistribution> {
                val calls = dictionary.calls.get()
                val before = database.readBytes()
                val view = dispatch(Event.LoadLibraryWordDistributions, true)
                check(view.libraryWordError == null)
                check(database.readBytes().contentEquals(before))
                check(dictionary.calls.get() == calls)
                return checkNotNull(view.libraryWordDistributions)
            }
            fun open(id: DocumentId): EffectiveReaderPage {
                val view = dispatch(Event.OpenReader(id), true)
                val page = checkNotNull(view.reader)
                return checkNotNull(
                    dispatch(
                            Event.LoadReaderTokens(
                                ReaderTokenKey(id, page.anchor, checkNotNull(view.readerAnalysis))
                            ),
                            true,
                        )
                        .effectiveReaderTokens
                )
            }
            dispatch(Event.RefreshLibrary, true)
            val ids =
                listOf("a", "b").map { prefix ->
                    checkNotNull(
                            dispatch(
                                    Event.CreatePaste("$prefix\n猫 猫 犬 𠮷 が か\u3099"),
                                    true,
                                )
                                .importSummary
                        )
                        .documentId
                }
            val snapshot = open(ids[0])
            val surfaces = snapshot.tokens.distinctBy { it.surface }
            check(surfaces.size >= 5)

            surfaces.forEachIndexed { index, token ->
                dispatch(
                    Event.SetEffectiveReaderStatus(
                        EffectiveReaderSelection(snapshot.identity, token.segmentIndex),
                        checkNotNull(token.vocabulary).id,
                        VocabularyStatus.entries[index % 5],
                        100L + index,
                    ),
                    true,
                )
            }
            dispatch(Event.CloseDocument)
            val first = words()
            val expected =
                WordStatusCounts(
                    surfaces.indices.count { it % 5 == 0 }.toULong(),
                    surfaces.indices.count { it % 5 == 1 }.toULong(),
                    surfaces.indices.count { it % 5 == 2 }.toULong(),
                    surfaces.indices.count { it % 5 == 3 }.toULong(),
                    surfaces.indices.count { it % 5 == 4 }.toULong(),
                )
            check(first.map { it.counts } == listOf(expected, expected))
            dispatch(Event.SortLibrary(LibrarySort.TITLE))
            check(ViewModel.bincodeDeserialize(bridge.view()).libraryWordDistributions == first)
            open(ids[1])
            val page = checkNotNull(ViewModel.bincodeDeserialize(bridge.view()).reader)
            bridge.update(
                Event.CompleteEffectiveReaderPage(
                        checkNotNull(
                                ViewModel.bincodeDeserialize(bridge.view()).effectiveReaderTokens
                            )
                            .identity,
                        page.position,
                        500,
                    )
                    .bincodeSerialize()
            )
            repeat(2) {
                checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            }
            check(!ViewModel.bincodeDeserialize(bridge.view()).storageBusy)
            val completed = words()
            check(
                completed.all {
                    it.counts ==
                        WordStatusCounts(
                            0UL,
                            expected.learning,
                            expected.familiar,
                            expected.known + expected.new,
                            expected.ignored,
                        )
                }
            )
            val counts = WordStatusCounts(ULong.MAX_VALUE, 1UL, 2UL, 3UL, 4UL)
            check(WordStatusCounts.bincodeDeserialize(counts.bincodeSerialize()) == counts)
            check(effects.isEmpty())
        }
    } finally {
        directory.deleteRecursively()
    }
}
