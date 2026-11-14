package app.yomoku

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filter

@Composable
internal fun VocabularyScreen(
    view: ViewModel,
    sessionKey: String,
    scroll: LazyListState,
    dispatch: (Event) -> Unit,
) {
    val state = view.vocabulary
    val query = state.query
    // Presentation may retain a successful sort, but pagination always uses core state below.
    var presentation by
        remember(sessionKey, query.prefix, query.statuses) {
            mutableStateOf(VocabularyPresentation(query.sort))
        }
    val nextPresentation = presentation.update(state)
    SideEffect { presentation = nextPresentation }
    val displayed = nextPresentation.display(state)
    val enabled = (!view.storageBusy || state.loading) && state.wordStatus == null
    val canOpenWord = !view.storageBusy && !state.loading && state.wordStatus == null
    VocabularyWordSheet(view, dispatch)
    // The core owns attempted/error state. Resuming visibility or unrelated work
    // can start an unattempted read, but cannot retry a failed page.
    LaunchedEffect(sessionKey, view.storageBusy, state.matchingCount) {
        if (!view.storageBusy && state.matchingCount == null && state.error == null)
            dispatch(Event.LoadVocabulary)
    }
    val lastId = state.rows.lastOrNull()?.id?.value
    LaunchedEffect(
        sessionKey,
        query,
        lastId,
        state.nextOffset,
        state.loading,
        state.error,
        view.storageBusy,
    ) {
        if (
            lastId != null &&
                state.nextOffset != null &&
                !state.loading &&
                state.error == null &&
                !view.storageBusy
        ) {
            snapshotFlow { scroll.layoutInfo.visibleItemsInfo.any { it.key == lastId } }
                .distinctUntilChanged()
                .filter { it }
                .collect { dispatch(Event.LoadMoreVocabulary) }
        }
    }
    Column(Modifier.fillMaxSize()) {
        if (state.word == null) VocabularyWordStatusRetry(view, dispatch)
        LazyColumn(
            modifier = Modifier.weight(1f).testTag("vocabulary_scroll"),
            state = scroll,
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            item(key = "controls") {
                OutlinedTextField(
                    value = query.prefix,
                    onValueChange = { dispatch(Event.SetVocabularyQuery(query.copy(prefix = it))) },
                    label = { Text(stringResource(R.string.vocabulary_search)) },
                    singleLine = true,
                    enabled = enabled,
                    modifier = Modifier.fillMaxWidth().testTag("vocabulary_search"),
                    trailingIcon = {
                        if (query.prefix.isNotEmpty())
                            TextButton(
                                onClick = {
                                    dispatch(Event.SetVocabularyQuery(query.copy(prefix = "")))
                                },
                                enabled = enabled,
                                modifier = Modifier.testTag("vocabulary_clear_search"),
                            ) {
                                Text(stringResource(R.string.clear_search))
                            }
                    },
                )
                Row(
                    modifier =
                        Modifier.fillMaxWidth()
                            .horizontalScroll(rememberScrollState())
                            .testTag("vocabulary_filters"),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    VocabularyStatus.entries.forEach { status ->
                        FilterChip(
                            selected = status in query.statuses,
                            onClick = {
                                val statuses =
                                    VocabularyStatus.entries.filter {
                                        if (it == status) it !in query.statuses
                                        else it in query.statuses
                                    }
                                dispatch(Event.SetVocabularyQuery(query.copy(statuses = statuses)))
                            },
                            enabled = enabled,
                            label = { Text(stringResource(status.labelResource())) },
                            modifier = Modifier.testTag("vocabulary_status_${status.name}"),
                        )
                    }
                }
                VocabularySortMenu(query.sort, enabled) {
                    dispatch(Event.SetVocabularyQuery(query.copy(sort = it)))
                }
                if (displayed.matchingCount != null)
                    Text(
                        stringResource(
                            R.string.vocabulary_matches,
                            displayed.matchingCount.toString(),
                        ),
                        modifier = Modifier.testTag("vocabulary_count"),
                    )
            }
            items(displayed.rows, key = { it.id.value }) { row ->
                Row(
                    Modifier.fillMaxWidth()
                        .clickable(
                            enabled = canOpenWord,
                            onClick = { dispatch(Event.OpenVocabularyWord(row.id)) },
                        )
                        .testTag("vocabulary_row_${row.id.value}"),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(row.surface, style = MaterialTheme.typography.titleMedium)
                        Text(stringResource(row.status.labelResource()))
                        Text(
                            stringResource(
                                R.string.word_encountered,
                                row.encounteredCount.toString(),
                            ),
                            style = MaterialTheme.typography.bodySmall,
                        )
                        HorizontalDivider(Modifier.padding(top = 8.dp))
                    }
                }
            }
            if (displayed.matchingCount == 0UL)
                item { Text(stringResource(R.string.vocabulary_no_matches)) }
            if (state.error != null)
                item(key = "error") {
                    Text(
                        stringResource(R.string.vocabulary_error),
                        modifier = Modifier.testTag("vocabulary_error"),
                    )
                    Button(
                        onClick = { dispatch(Event.RetryVocabulary) },
                        enabled = !view.storageBusy,
                        modifier = Modifier.testTag("vocabulary_retry"),
                    ) {
                        Text(stringResource(R.string.retry))
                    }
                }
        }
    }
}

@Composable
private fun VocabularySortMenu(
    sort: VocabularySort,
    enabled: Boolean,
    onSort: (VocabularySort) -> Unit,
) {
    val labels =
        mapOf(
            VocabularySort.NEWEST to R.string.vocabulary_sort_newest,
            VocabularySort.FREQUENCY to R.string.vocabulary_sort_frequency,
            VocabularySort.LASTSEEN to R.string.vocabulary_sort_last_seen,
            VocabularySort.ALPHABETICAL to R.string.vocabulary_sort_alphabetical,
        )
    val base =
        when (sort) {
            VocabularySort.OLDEST -> VocabularySort.NEWEST
            VocabularySort.FREQUENCYASCENDING -> VocabularySort.FREQUENCY
            VocabularySort.LASTSEENASCENDING -> VocabularySort.LASTSEEN
            VocabularySort.ALPHABETICALDESCENDING -> VocabularySort.ALPHABETICAL
            else -> sort
        }
    val reverse =
        when (sort) {
            VocabularySort.NEWEST -> VocabularySort.OLDEST
            VocabularySort.OLDEST -> VocabularySort.NEWEST
            VocabularySort.FREQUENCY -> VocabularySort.FREQUENCYASCENDING
            VocabularySort.FREQUENCYASCENDING -> VocabularySort.FREQUENCY
            VocabularySort.LASTSEEN -> VocabularySort.LASTSEENASCENDING
            VocabularySort.LASTSEENASCENDING -> VocabularySort.LASTSEEN
            VocabularySort.ALPHABETICAL -> VocabularySort.ALPHABETICALDESCENDING
            VocabularySort.ALPHABETICALDESCENDING -> VocabularySort.ALPHABETICAL
        }
    SortControls(
        labels = labels,
        baseSort = base,
        ascending =
            sort in
                listOf(
                    VocabularySort.OLDEST,
                    VocabularySort.FREQUENCYASCENDING,
                    VocabularySort.LASTSEENASCENDING,
                    VocabularySort.ALPHABETICAL,
                ),
        tag = "vocabulary",
        enabled = enabled,
        onReverse = { onSort(reverse) },
        onSort = onSort,
    )
}

/** Retain only a successful presentation during sort changes within the same filter/session. */
private data class VocabularyPresentation(
    val sort: VocabularySort,
    val successful: VocabularyState? = null,
    val sorting: Boolean = false,
) {
    fun update(state: VocabularyState): VocabularyPresentation =
        VocabularyPresentation(
            sort = state.query.sort,
            successful =
                when {
                    state.error != null -> null
                    !state.loading -> state.takeIf { it.matchingCount != null }
                    else -> successful
                },
            sorting = (sorting || sort != state.query.sort) && state.loading && state.error == null,
        )

    fun display(state: VocabularyState): VocabularyState =
        if (sorting && state.matchingCount == null) successful ?: state else state
}
