package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class StatisticsDatasetTest {
    @Test
    fun allTimeframesPreserveOpeningLifetimeValuesAndEveryLine() {
        StatisticsTimeframe.entries.forEach { timeframe ->
            val data = statisticsFixture(timeframe)
            val occurrences = StatisticsDataset(data, StatisticsChart.OCCURRENCES)
            assertEquals(
                listOf(85_000UL, 90_000UL, 100_000UL),
                occurrences.points.map { it.values.single() },
            )
            assertEquals(85_000UL, occurrences.minimum)
            assertEquals(100_000UL, occurrences.maximum)
            assertEquals(listOf(0f, 0.25f, 1f), occurrences.points.map { occurrences.x(it) })
            assertEquals(1f, occurrences.y(85_000UL))
            assertEquals(0f, occurrences.y(100_000UL))
            val statuses = StatisticsDataset(data, StatisticsChart.STATUSES)
            assertEquals(listOf(10UL, 20UL, 30UL, 40UL, 50UL), statuses.points.first().values)
            assertEquals(listOf(30UL, 0UL, 20UL, 60UL, 40UL), statuses.points.last().values)
            assertEquals(0UL, statuses.minimum)
            assertEquals(60UL, statuses.maximum)
            // Displaying charts never changes the transport's hidden completed time.
            assertEquals(999UL, data.buckets.last().values.reading.activeTimeMs)
        }
    }

    @Test
    fun unsignedExtremesRemainExactAndSmallDistancesDoNotCollapse() {
        val fixture = statisticsFixture()
        val data =
            fixture.copy(
                opening =
                    fixture.opening.copy(
                        reading = LifetimeReadingTotals(ULong.MAX_VALUE - 2UL, 0UL)
                    ),
                buckets =
                    fixture.buckets.mapIndexed { index, bucket ->
                        bucket.copy(
                            values =
                                bucket.values.copy(
                                    reading =
                                        LifetimeReadingTotals(
                                            ULong.MAX_VALUE - 1UL + index.toULong(),
                                            ULong.MAX_VALUE,
                                        )
                                )
                        )
                    },
            )
        val chart = StatisticsDataset(data, StatisticsChart.OCCURRENCES)
        assertEquals(ULong.MAX_VALUE, chart.maximum)
        assertEquals(listOf(1f, 0.5f, 0f), chart.points.map { chart.y(it.values.single()) })
    }

    @Test
    fun emptyAndConstantLinesHaveFiniteCoordinatesAndUtcDates() {
        val fixture = statisticsFixture()
        val zero =
            StatisticsValues(
                LifetimeReadingTotals(0UL, 0UL),
                WordStatusCounts(0UL, 0UL, 0UL, 0UL, 0UL),
            )
        val data =
            fixture.copy(opening = zero, buckets = fixture.buckets.map { it.copy(values = zero) })
        StatisticsChart.entries.forEach { chart ->
            val dataset = StatisticsDataset(data, chart)
            dataset.points.forEach { point ->
                assertTrue(dataset.x(point).isFinite())
                point.values.forEach { assertEquals(0.5f, dataset.y(it)) }
            }
        }
        assertEquals("1970-01-01", statisticsDate(0))
        assertEquals("1970-01-01", statisticsDate(86_399_999))
        assertEquals("1970-01-02", statisticsDate(86_400_000))
    }
}

internal fun statisticsFixture(
    timeframe: StatisticsTimeframe = StatisticsTimeframe.LASTYEAR
): StatisticsBuckets {
    val start = 1_700_000_000_000L
    val opening =
        StatisticsValues(
            LifetimeReadingTotals(85_000UL, 123UL),
            WordStatusCounts(10UL, 20UL, 30UL, 40UL, 50UL),
        )
    return StatisticsBuckets(
        timeframe,
        start,
        start + 4000,
        opening,
        listOf(
            StatisticsBucket(
                start,
                start + 1000,
                StatisticsValues(
                    LifetimeReadingTotals(90_000UL, 456UL),
                    WordStatusCounts(20UL, 10UL, 30UL, 50UL, 40UL),
                ),
            ),
            StatisticsBucket(
                start + 1000,
                start + 4000,
                StatisticsValues(
                    LifetimeReadingTotals(100_000UL, 999UL),
                    WordStatusCounts(30UL, 0UL, 20UL, 60UL, 40UL),
                ),
            ),
        ),
    )
}
