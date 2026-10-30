package app.yomoku.tests

import app.yomoku.shared.*
import java.io.File

internal fun verifyStatisticsCodecs(dir: String) {
    StatisticsTimeframe.entries.forEachIndexed { index, timeframe ->
        val bytes = File(dir, "statistics-view-$index.bin").readBytes()
        val view = ViewModel.bincodeDeserialize(bytes)
        val state = view.statistics
        check(state.timeframe == timeframe && state.query?.timeframe == timeframe)
        check(state.query.asOfUnixMs == 1_800_000_000_123L)
        val data = checkNotNull(state.data)
        check(data.timeframe == timeframe && data.endExclusiveUnixMs == 1_800_000_000_124L)
        check(data.opening == data.buckets.single().values)
        check(data.opening.reading.japaneseOccurrences == ULong.MAX_VALUE)
        check(data.opening.reading.activeTimeMs == 9_007_199_254_740_993UL)
        check(
            data.opening.statuses == WordStatusCounts(1UL, 2UL, 3UL, 9_007_199_254_740_993UL, 5UL)
        )
        check(data.opening.knownDictionaryForms == ULong.MAX_VALUE - 1UL)
        check(view.bincodeSerialize().contentEquals(bytes))
        File(dir, "statistics-view-$index-return.bin").writeBytes(view.bincodeSerialize())
        val eventBytes = File(dir, "statistics-event-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(eventBytes)
        check(event == Event.SetStatisticsTimeframe(timeframe, 1_800_000_000_123L))
        check(event.bincodeSerialize().contentEquals(eventBytes))
        File(dir, "statistics-event-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
    repeat(3) { index ->
        val bytes = File(dir, "statistics-state-$index.bin").readBytes()
        val view = ViewModel.bincodeDeserialize(bytes)
        check(view.statistics.data == null && view.statistics.query == null)
        check(view.statistics.timeframe == StatisticsTimeframe.LASTYEAR)
        check(view.statistics.loading == (index == 1))
        check(view.statistics.error == if (index == 2) DocumentError.INVALIDDOCUMENT else null)
        check(view.bincodeSerialize().contentEquals(bytes))
        File(dir, "statistics-state-$index-return.bin").writeBytes(view.bincodeSerialize())
    }
}
