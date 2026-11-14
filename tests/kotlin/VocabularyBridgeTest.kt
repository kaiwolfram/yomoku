package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyVocabularyBridge() {
    verifyVocabularyWordBridge()
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
        NativeBridge(directory.resolve("bulk.sqlite3").path, listener, dictionary).use { bridge ->
            val id =
                checkNotNull(dispatch(bridge, Event.CreatePaste("猫 猫 犬"), 1).importSummary)
                    .documentId
            val calls = dictionary.calls.get()
            val now = System.currentTimeMillis()
            fun navigate(destination: Destination) {
                dispatch(bridge, Event.SelectDestination(destination))
            }
            fun loadTokens(): List<EffectivePageToken> {
                navigate(Destination.LIBRARY)
                val opened = dispatch(bridge, Event.OpenReader(id), 1)
                val page = checkNotNull(opened.reader)
                return checkNotNull(
                        dispatch(
                                bridge,
                                Event.LoadReaderTokens(
                                    ReaderTokenKey(
                                        id,
                                        page.anchor,
                                        checkNotNull(opened.readerAnalysis),
                                    )
                                ),
                                1,
                            )
                            .effectiveReaderTokens
                    )
                    .tokens
            }
            navigate(Destination.STATISTICS)
            dispatch(bridge, Event.LoadStatistics(now), 1)
            navigate(Destination.LIBRARY)
            dispatch(bridge, Event.LoadLibraryWordDistributions, 1)
            navigate(Destination.VOCABULARY)
            val initial = dispatch(bridge, Event.LoadVocabulary, 1)
            val cat = initial.vocabulary.rows.single { it.surface == "猫" }
            dispatch(bridge, Event.OpenVocabularyWord(cat.id), 1)
            val changed =
                dispatch(
                    bridge,
                    Event.SetVocabularyWordStatus(
                        VocabularyWordStatusCommand(cat.id, VocabularyStatus.LEARNING, now)
                    ),
                    1,
                )
            dispatch(bridge, Event.DismissVocabularyWord)
            check(
                changed.vocabulary.matchingCount == null &&
                    changed.statistics.data == null &&
                    changed.libraryWordDistributions == null
            )
            val rows = dispatch(bridge, Event.LoadVocabulary, 1).vocabulary.rows
            check(rows.single { it.id == cat.id }.status == VocabularyStatus.LEARNING)
            navigate(Destination.STATISTICS)
            val stats = checkNotNull(dispatch(bridge, Event.LoadStatistics(now), 1).statistics.data)
            check(stats.buckets.last().values.statuses.learning == 1UL)
            check(
                loadTokens()
                    .filter { it.surface == "猫" }
                    .all { it.vocabulary?.status == VocabularyStatus.LEARNING }
            )
            navigate(Destination.VOCABULARY)
            dispatch(bridge, Event.SetVocabularyQuery(query("猫")), 1)
            dispatch(bridge, Event.OpenVocabularyWord(cat.id), 1)
            dispatch(
                bridge,
                Event.SetVocabularyWordStatus(
                    VocabularyWordStatusCommand(cat.id, VocabularyStatus.IGNORED, now)
                ),
                1,
            )
            dispatch(bridge, Event.DismissVocabularyWord)
            navigate(Destination.STATISTICS)
            val statuses =
                checkNotNull(dispatch(bridge, Event.LoadStatistics(now), 1).statistics.data)
                    .buckets
                    .last()
                    .values
                    .statuses
            check(statuses.new == 1UL && statuses.ignored == 1UL)
            check(dictionary.calls.get() == calls)
        }
        check(effects.isEmpty())
    } finally {
        directory.deleteRecursively()
    }
}

private fun verifyVocabularyWordBridge() {
    val directory = Files.createTempDirectory("yomoku-vocabulary-word-").toFile()
    val callbacks = LinkedBlockingQueue<Thread>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                callbacks.add(Thread.currentThread())
            }

            override fun onError(error: FfiError) {
                throw error
            }
        }
    val provider = HostDictionary()
    val database = directory.resolve("db.sqlite3")
    fun dispatch(bridge: NativeBridge, event: Event, count: Int = 0): ViewModel {
        bridge.update(event.bincodeSerialize())
        repeat(count) {
            check(checkNotNull(callbacks.poll(60, TimeUnit.SECONDS)) != Thread.currentThread())
        }
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
    try {
        var savedId: VocabularyId? = null
        NativeBridge(database.path, listener, provider).use { bridge ->
            val book =
                checkNotNull(
                        dispatch(bridge, Event.CreatePaste("東京 東京 食べました 東京都"), 1).importSummary
                    )
                    .documentId
            dispatch(bridge, Event.RequestDocumentDeletion(book))
            dispatch(bridge, Event.ConfirmDocumentDeletion(book), 1)
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val rows = dispatch(bridge, Event.LoadVocabulary, 1).vocabulary.rows
            val tokyo = rows.single { it.surface == "東京" }
            val before = database.readBytes()
            provider.failWords = true
            val failed = dispatch(bridge, Event.OpenVocabularyWord(tokyo.id), 1).vocabulary.word
            check(failed?.error != null && failed.candidates == null)
            provider.failWords = false
            val loaded =
                checkNotNull(dispatch(bridge, Event.RetryVocabularyWord, 1).vocabulary.word)
            val candidates = checkNotNull(loaded.candidates)
            check(loaded.row == tokyo && loaded.row.encounteredCount == 0UL)
            check(candidates.words.isNotEmpty() && candidates.names.isNotEmpty())
            dispatch(bridge, Event.DismissVocabularyWord)
            val eaten = rows.single { it.surface == "食べました" }
            savedId = eaten.id
            val inflected =
                checkNotNull(
                    dispatch(bridge, Event.OpenVocabularyWord(eaten.id), 1)
                        .vocabulary
                        .word
                        ?.candidates
                )
            check(inflected.dictionaryForm == "食べる" && inflected.words.isNotEmpty())
            check(database.readBytes().contentEquals(before))
            dispatch(bridge, Event.DismissVocabularyWord)
            dispatch(
                bridge,
                Event.SetVocabularyQuery(
                    VocabularyQuery("食べました", listOf(VocabularyStatus.NEW), VocabularySort.NEWEST)
                ),
                1,
            )
            dispatch(bridge, Event.OpenVocabularyWord(eaten.id), 1)
            val command =
                VocabularyWordStatusCommand(
                    eaten.id,
                    VocabularyStatus.LEARNING,
                    System.currentTimeMillis(),
                )
            val changed = dispatch(bridge, Event.SetVocabularyWordStatus(command), 1)
            check(changed.vocabulary.wordStatus == null)
            check(changed.vocabulary.word?.row?.status == VocabularyStatus.LEARNING)
            val filtered = dispatch(bridge, Event.LoadVocabulary, 1)
            check(
                filtered.vocabulary.matchingCount == 0UL &&
                    filtered.vocabulary.word?.row?.id == eaten.id
            )
            check(filtered.vocabulary.word.candidates == inflected)
            dispatch(
                bridge,
                Event.SetVocabularyWordStatus(
                    command.copy(occurredAtUnixMs = command.occurredAtUnixMs + 1)
                ),
            )
            check(ViewModel.bincodeDeserialize(bridge.view()).vocabulary.word == null)
        }
        NativeBridge(database.path, listener, provider).use { bridge ->
            dispatch(bridge, Event.SelectDestination(Destination.VOCABULARY))
            val row =
                dispatch(bridge, Event.LoadVocabulary, 1).vocabulary.rows.single {
                    it.id == savedId
                }
            check(row.status == VocabularyStatus.LEARNING && row.encounteredCount == 0UL)
        }
        check(callbacks.isEmpty())
    } finally {
        directory.deleteRecursively()
    }
}
