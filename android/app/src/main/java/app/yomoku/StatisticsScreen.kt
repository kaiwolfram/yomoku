package app.yomoku

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*

@Composable
internal fun StatisticsScreen(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
) {
    val state = view.statistics
    // Reuse a retained cutoff across Settings/recreation. Capture now only for
    // an invalidated/first query, never on busy/error render callbacks.
    val asOf =
        remember(sessionKey, state.query) { state.query?.asOfUnixMs ?: System.currentTimeMillis() }
    LaunchedEffect(sessionKey, view.storageBusy, state.query) {
        if (!view.storageBusy) dispatch(Event.LoadStatistics(asOf))
    }
    val data = state.data
    val scroll = rememberLazyListState()
    LazyColumn(
        Modifier.fillMaxSize().testTag("statistics_scroll"),
        state = scroll,
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        item {
            val timeframeScroll =
                rememberLazyListState(
                    initialFirstVisibleItemIndex =
                        StatisticsTimeframe.entries.indexOf(state.timeframe)
                )
            LazyRow(
                state = timeframeScroll,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth().testTag("statistics_timeframes"),
            ) {
                items(StatisticsTimeframe.entries, key = { it.name }) { timeframe ->
                    FilterChip(
                        selected = state.timeframe == timeframe,
                        onClick = {
                            dispatch(
                                Event.SetStatisticsTimeframe(timeframe, System.currentTimeMillis())
                            )
                        },
                        enabled = !view.storageBusy || state.loading,
                        label = { Text(stringResource(timeframe.label())) },
                        modifier = Modifier.testTag("statistics_timeframe_${timeframe.name}"),
                    )
                }
            }
            Text(
                stringResource(state.timeframe.grouping()),
                style = MaterialTheme.typography.bodySmall,
            )
        }
        if (state.loading || (data == null && state.error == null))
            item {
                LinearProgressIndicator(Modifier.fillMaxWidth().testTag("statistics_loading"))
                Text(stringResource(R.string.statistics_loading))
            }
        if (state.error != null)
            item {
                Text(
                    stringResource(R.string.statistics_error),
                    modifier = Modifier.testTag("statistics_error"),
                )
                Button(onClick = { dispatch(Event.RetryStatistics) }, enabled = !view.storageBusy) {
                    Text(stringResource(R.string.retry))
                }
            }
        if (data != null) {
            item {
                Text(
                    stringResource(
                        R.string.statistics_range,
                        statisticsDate(data.startUnixMs),
                        statisticsDate(data.endExclusiveUnixMs - 1),
                    )
                )
            }
            items(StatisticsChart.entries, key = { it.name }) { chart ->
                val dataset = remember(data, chart) { StatisticsDataset(data, chart) }
                OutlinedCard(modifier = Modifier.testTag("statistics_card_${chart.name}")) {
                    Column(
                        Modifier.padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        StatisticsPlot(dataset, chart)
                    }
                }
            }
        }
    }
}

@Composable
private fun StatisticsPlot(dataset: StatisticsDataset, chart: StatisticsChart) {
    val dark = isSystemInDarkTheme()
    val colors =
        when (chart) {
            StatisticsChart.OCCURRENCES -> listOf(MaterialTheme.colorScheme.primary)
            StatisticsChart.STATUSES ->
                VocabularyStatus.entries.map { status ->
                    if (status == VocabularyStatus.KNOWN)
                        Color(if (dark) 0xFF438A60 else 0xFF7CCB96)
                    else vocabularyBackground(status, dark)
                }
        }
    val title = stringResource(chart.title)
    val labels = chart.lines.map { stringResource(it) }
    val description =
        title +
            ": " +
            labels
                .mapIndexed { index, label ->
                    "$label ${dataset.points.first().values[index]} → ${dataset.points.last().values[index]}"
                }
                .joinToString(", ")
    val axis = MaterialTheme.colorScheme.outline
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(dataset.maximum.toString(), style = MaterialTheme.typography.labelSmall)
        Canvas(
            Modifier.fillMaxWidth()
                .height(160.dp)
                .testTag("statistics_chart_${chart.name}")
                .semantics { contentDescription = description }
        ) {
            val inset = 4.dp.toPx()
            val width = (size.width - 2 * inset).coerceAtLeast(0f)
            val height = (size.height - 2 * inset).coerceAtLeast(0f)
            drawLine(axis, Offset(inset, inset), Offset(inset, inset + height))
            drawLine(axis, Offset(inset, inset + height), Offset(inset + width, inset + height))
            labels.indices.forEach { line ->
                val path = Path()
                var previousY = inset + dataset.y(dataset.points.first().values[line]) * height
                path.moveTo(inset, previousY)
                dataset.points.drop(1).forEach { point ->
                    val x = inset + dataset.x(point) * width
                    val y = inset + dataset.y(point.values[line]) * height
                    path.lineTo(x, previousY)
                    path.lineTo(x, y)
                    previousY = y
                }
                drawPath(
                    path,
                    colors[line],
                    style = Stroke(width = 3.dp.toPx()),
                )
            }
        }
        Text(dataset.minimum.toString(), style = MaterialTheme.typography.labelSmall)
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(
                statisticsDate(dataset.points.first().unixMs),
                style = MaterialTheme.typography.labelSmall,
            )
            Text(
                statisticsDate(dataset.points.last().unixMs - 1),
                style = MaterialTheme.typography.labelSmall,
            )
        }
        labels.forEachIndexed { index, label ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Canvas(Modifier.size(24.dp, 16.dp)) {
                    drawLine(
                        colors[index],
                        Offset(0f, size.height / 2),
                        Offset(size.width, size.height / 2),
                        strokeWidth = 3.dp.toPx(),
                    )
                }
                Text(
                    "$label: ${dataset.points.last().values[index]}",
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
    }
}

private fun StatisticsTimeframe.label(): Int =
    when (this) {
        StatisticsTimeframe.LASTWEEK -> R.string.statistics_week
        StatisticsTimeframe.LASTMONTH -> R.string.statistics_month
        StatisticsTimeframe.LASTSIXMONTHS -> R.string.statistics_six_months
        StatisticsTimeframe.LASTYEAR -> R.string.statistics_year
        StatisticsTimeframe.ALLTIME -> R.string.statistics_all_time
    }

private fun StatisticsTimeframe.grouping(): Int =
    when (this) {
        StatisticsTimeframe.LASTWEEK,
        StatisticsTimeframe.LASTMONTH -> R.string.statistics_daily
        StatisticsTimeframe.LASTSIXMONTHS,
        StatisticsTimeframe.LASTYEAR -> R.string.statistics_weekly
        StatisticsTimeframe.ALLTIME -> R.string.statistics_monthly
    }
