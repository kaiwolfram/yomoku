package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyStatisticsBridge() {
    val directory = Files.createTempDirectory("yomoku-statistics-").toFile()
    val effects = LinkedBlockingQueue<Thread>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                check(
                    Requests.bincodeDeserialize(requests).value.single().effect
                        is ShellEffect.Render
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
    var retained: StatisticsBuckets? = null
    try {
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            dispatch(bridge, Event.SelectDestination(Destination.STATISTICS))
            check(
                ViewModel.bincodeDeserialize(bridge.view()).statistics.timeframe ==
                    StatisticsTimeframe.LASTYEAR
            )
            // Invalid dates remain statistics-only errors; Retry keeps the cutoff.
            val invalid = dispatch(bridge, Event.LoadStatistics(-1), 1)
            check(
                invalid.statistics.error == DocumentError.INCOMPATIBLEDATABASE &&
                    invalid.documentError == null
            )
            val retried = dispatch(bridge, Event.RetryStatistics, 1)
            check(
                retried.statistics.query?.asOfUnixMs == -1L &&
                    retried.statistics.error == DocumentError.INCOMPATIBLEDATABASE
            )
            val empty = dispatch(bridge, Event.LoadStatistics(0), 1)
            check(
                checkNotNull(empty.statistics.data)
                    .buckets
                    .single()
                    .values
                    .reading
                    .japaneseOccurrences == 0UL
            )
            dispatch(bridge, Event.SelectDestination(Destination.LIBRARY))
            val id =
                checkNotNull(dispatch(bridge, Event.CreatePaste("猫 猫 犬"), 1).importSummary)
                    .documentId
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
            check(tokens.tokens.map { it.surface } == listOf("猫", "猫", "犬"))
            val asOf = System.currentTimeMillis() + 1_000L
            dispatch(
                bridge,
                Event.CompleteEffectiveReaderPage(tokens.identity, page.position, asOf),
                2,
            )
            dispatch(bridge, Event.SelectDestination(Destination.STATISTICS))
            val calls = dictionary.calls.get()
            StatisticsTimeframe.entries.forEach { timeframe ->
                val result = dispatch(bridge, Event.SetStatisticsTimeframe(timeframe, asOf), 2)
                val data = checkNotNull(result.statistics.data)
                check(result.statistics.error == null && !result.statistics.loading)
                check(data.buckets.last().values.reading.japaneseOccurrences == 3UL)
                check(
                    data.buckets.last().values.statuses == WordStatusCounts(0UL, 0UL, 0UL, 2UL, 0UL)
                )
                check(data.endExclusiveUnixMs == asOf + 1)
                check(dictionary.calls.get() == calls)
                retained = data
            }
            dispatch(bridge, Event.OpenSettings)
            check(ViewModel.bincodeDeserialize(bridge.view()).statistics.data == retained)
            dispatch(bridge, Event.CloseSettings)
            dispatch(bridge, Event.SelectDestination(Destination.LIBRARY))
            dispatch(bridge, Event.RequestDocumentDeletion(id))
            dispatch(bridge, Event.ConfirmDocumentDeletion(id), 1)
        }
        val calls = dictionary.calls.get()
        NativeBridge(database.path, listener, dictionary).use { bridge ->
            val restored = dispatch(bridge, Event.LoadStatisticsTimeframe, 1)
            check(restored.statistics.timeframe == StatisticsTimeframe.ALLTIME)
            dispatch(bridge, Event.SelectDestination(Destination.STATISTICS))
            val prior = checkNotNull(retained)
            val result =
                dispatch(
                    bridge,
                    Event.SetStatisticsTimeframe(
                        StatisticsTimeframe.ALLTIME,
                        prior.endExclusiveUnixMs - 1,
                    ),
                    1,
                )
            check(result.statistics.data == prior)
            check(dictionary.calls.get() == calls)
        }
        check(effects.isEmpty())
    } finally {
        directory.deleteRecursively()
    }
}
