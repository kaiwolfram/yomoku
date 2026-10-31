package app.yomoku

import app.yomoku.shared.StatisticsBuckets
import app.yomoku.shared.StatisticsValues
import java.time.Instant
import java.time.ZoneOffset

internal enum class StatisticsChart(val title: Int, val lines: List<Int>) {
    OCCURRENCES(R.string.statistics_occurrences, listOf(R.string.statistics_occurrences)),
    STATUSES(
        R.string.statistics_statuses,
        listOf(
            R.string.status_new,
            R.string.status_learning,
            R.string.status_familiar,
            R.string.status_known,
            R.string.status_ignored,
        ),
    );

    fun values(value: StatisticsValues): List<ULong> =
        when (this) {
            OCCURRENCES -> listOf(value.reading.japaneseOccurrences)
            STATUSES ->
                value.statuses.let {
                    listOf(it.new, it.learning, it.familiar, it.known, it.ignored)
                }
        }
}

internal data class StatisticsPoint(val unixMs: Long, val values: List<ULong>)

/** Preserve exact unsigned lifetime values; convert only relative drawing distances. */
internal class StatisticsDataset(data: StatisticsBuckets, chart: StatisticsChart) {
    val points =
        listOf(StatisticsPoint(data.startUnixMs, chart.values(data.opening))) +
            data.buckets.map { StatisticsPoint(it.endExclusiveUnixMs, chart.values(it.values)) }
    val minimum = points.minOf { it.values.min() }
    val maximum = points.maxOf { it.values.max() }

    fun x(point: StatisticsPoint): Float {
        val span = points.last().unixMs - points.first().unixMs
        return if (span == 0L) 0f
        else ((point.unixMs - points.first().unixMs).toDouble() / span.toDouble()).toFloat()
    }

    fun y(value: ULong): Float =
        if (maximum == minimum) 0.5f
        else (1.0 - (value - minimum).toDouble() / (maximum - minimum).toDouble()).toFloat()
}

internal fun statisticsDate(unixMs: Long): String =
    Instant.ofEpochMilli(unixMs).atOffset(ZoneOffset.UTC).toLocalDate().toString()
