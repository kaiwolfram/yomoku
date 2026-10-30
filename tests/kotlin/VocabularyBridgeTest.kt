package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyVocabularyBridge() {
    val directory = Files.createTempDirectory("yomoku-vocabulary-").toFile()
    val effects = LinkedBlockingQueue<Thread>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                check(
                    Requests.bincodeDeserialize(requests).value.all {
                        it.effect is ShellEffect.Render
                    }
                )
                effects.add(Thread.currentThread())
            }

            override fun onError(error: FfiError) {
                throw error
            }
        }
    val dictionary = HostDictionary()
    val database = directory.resolve("db.sqlite3")
    fun dispatch(bridge: NativeBridge, event: Event, callbacks: Int = 0): ViewModel {
        bridge.update(event.bincodeSerialize())
        repeat(callbacks) {
            check(checkNotNull(effects.poll(60, TimeUnit.SECONDS)) != Thread.currentThread())
        }
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
    val allStatuses = VocabularyStatus.entries.toList()
    fun query(
        prefix: String = "",
        statuses: List<VocabularyStatus> = allStatuses,
        sort: VocabularySort = VocabularySort.NEWEST,
    ) = VocabularyQuery(prefix, statuses, sort)
    var retained: VocabularyState? = null
    try {
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val empty = dispatch(bridge, Event.LoadVocabulary, 1)
            check(empty.vocabulary.matchingCount == 0UL && empty.vocabulary.rows.isEmpty())
            dispatch(bridge, Event.LoadVocabulary)
            dispatch(bridge, Event.SelectDestination(Destination.LIBRARY))
            val id =
                checkNotNull(dispatch(bridge, Event.CreatePaste("猫 猫 犬"), 1).importSummary)
                    .documentId
            check(ViewModel.bincodeDeserialize(bridge.view()).vocabulary.matchingCount == null)
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val calls = dictionary.calls.get()
            val before = database.readBytes()
            VocabularySort.entries.forEach { sort ->
                val view = dispatch(bridge, Event.SetVocabularyQuery(query(sort = sort)), 1)
                check(view.vocabulary.matchingCount == 2UL && view.vocabulary.nextOffset == null)
                check(
                    view.vocabulary.rows.all {
                        it.status == VocabularyStatus.NEW &&
                            it.encounteredCount == 0UL &&
                            it.lastSeenUnixMs == null
                    }
                )
            }
            val filtered = dispatch(bridge, Event.SetVocabularyQuery(query("猫")), 1)
            check(filtered.vocabulary.rows.single().surface == "猫")
            check(filtered.vocabulary.matchingCount == 1UL)
            check(
                dispatch(bridge, Event.SetVocabularyQuery(query("ねこ")), 1)
                    .vocabulary
                    .matchingCount == 0UL
            )
            check(
                dispatch(bridge, Event.SetVocabularyQuery(query("猫", emptyList())), 1)
                    .vocabulary
                    .matchingCount == 0UL
            )
            check(
                dispatch(bridge, Event.SetVocabularyQuery(query("𠮷\u0000_%")), 1)
                    .vocabulary
                    .query
                    .prefix == "𠮷\u0000_%"
            )
            check(database.readBytes().contentEquals(before) && dictionary.calls.get() == calls)
            dispatch(bridge, Event.SelectDestination(Destination.LIBRARY))
            val opened = dispatch(bridge, Event.OpenReader(id), 1)
            val page = checkNotNull(opened.reader)
            val tokens =
                checkNotNull(
                    dispatch(
                            bridge,
                            Event.LoadReaderTokens(
                                ReaderTokenKey(id, page.anchor, checkNotNull(opened.readerAnalysis))
                            ),
                            1,
                        )
                        .effectiveReaderTokens
                )
            val now = System.currentTimeMillis()
            dispatch(
                bridge,
                Event.CompleteEffectiveReaderPage(tokens.identity, page.position, now),
                2,
            )
            check(ViewModel.bincodeDeserialize(bridge.view()).vocabulary.matchingCount == null)
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val read =
                dispatch(
                    bridge,
                    Event.SetVocabularyQuery(
                        query("猫", listOf(VocabularyStatus.KNOWN), VocabularySort.FREQUENCY)
                    ),
                    1,
                )
            val cat = read.vocabulary.rows.single()
            check(cat.encounteredCount == 2UL && cat.lastSeenUnixMs == now)
            retained = read.vocabulary
            dispatch(bridge, Event.OpenSettings)
            check(ViewModel.bincodeDeserialize(bridge.view()).vocabulary == retained)
            dispatch(bridge, Event.CloseSettings)
            dispatch(bridge, Event.LoadVocabulary)
            dispatch(bridge, Event.LoadMoreVocabulary)
            dispatch(bridge, Event.SelectDestination(Destination.LIBRARY))
            dispatch(bridge, Event.RequestDocumentDeletion(id))
            dispatch(bridge, Event.ConfirmDocumentDeletion(id), 1)
        }
        val calls = dictionary.calls.get()
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val prior = checkNotNull(retained)
            check(dispatch(bridge, Event.SetVocabularyQuery(prior.query), 1).vocabulary == prior)
            check(dictionary.calls.get() == calls)
        }
        check(effects.isEmpty())
    } finally {
        directory.deleteRecursively()
    }
}
