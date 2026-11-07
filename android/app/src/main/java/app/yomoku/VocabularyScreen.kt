package app.yomoku

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.filter

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun VocabularyScreen(
    view: ViewModel,
    sessionKey: String,
    scroll: LazyListState,
    dispatch: (Event) -> Unit,
) {
    val state = view.vocabulary
    val query = state.query
    val frozen = state.selectingAll || state.confirmation != null || state.bulkError != null
    val enabled = (!view.storageBusy || state.loading) && !frozen
    val canSelect = !view.storageBusy && !frozen
    val selection = state.selection
    BackHandler(enabled = selection != null || frozen) {
        if (!view.storageBusy)
            dispatch(
                if (state.confirmation != null) Event.CancelVocabularyBulkAction
                else Event.CancelVocabularySelection
            )
    }
    VocabularyBulkDialog(view, dispatch)
    // The core owns attempted/error state. Resuming visibility or unrelated work
    // can start an unattempted read, but cannot retry a failed page.
    LaunchedEffect(sessionKey, view.storageBusy, state.matchingCount, frozen) {
        if (!view.storageBusy && !frozen && state.matchingCount == null && state.error == null)
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
        frozen,
    ) {
        if (
            lastId != null &&
                state.nextOffset != null &&
                !state.loading &&
                state.error == null &&
                !view.storageBusy &&
                !frozen
        ) {
            snapshotFlow { scroll.layoutInfo.visibleItemsInfo.any { it.key == lastId } }
                .distinctUntilChanged()
                .filter { it }
                .collect { dispatch(Event.LoadMoreVocabulary) }
        }
    }
    Column(Modifier.fillMaxSize()) {
        if (selection != null) VocabularySelectionActions(view, dispatch, canSelect)
        LazyColumn(
            modifier = Modifier.weight(1f).testTag("vocabulary_scroll"),
            state = scroll,
            contentPadding = PaddingValues(16.dp),
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
                                Text(stringResource(R.string.library_clear_search))
                            }
                    },
                )
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
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
                if (state.matchingCount != null)
                    Text(
                        stringResource(R.string.vocabulary_matches, state.matchingCount.toString()),
                        modifier = Modifier.testTag("vocabulary_count"),
                    )
            }
            items(state.rows, key = { it.id.value }) { row ->
                val isSelected = selection?.containsSortedVocabularyId(row.id) == true
                val selectLabel = stringResource(R.string.vocabulary_select_word)
                Row(
                    Modifier.fillMaxWidth()
                        .combinedClickable(
                            enabled = canSelect,
                            onLongClickLabel = selectLabel,
                            onLongClick = { dispatch(Event.ToggleVocabularySelection(row.id)) },
                            onClick = { dispatch(Event.ToggleVocabularySelection(row.id)) },
                        )
                        .semantics { if (selection != null) selected = isSelected }
                        .testTag("vocabulary_row_${row.id.value}"),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (selection != null) Checkbox(checked = isSelected, onCheckedChange = null)
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
            if (state.matchingCount == 0UL)
                item { Text(stringResource(R.string.vocabulary_no_matches)) }
            if (state.loading || (state.matchingCount == null && state.error == null))
                item {
                    LinearProgressIndicator(Modifier.fillMaxWidth().testTag("vocabulary_loading"))
                    Text(stringResource(R.string.vocabulary_loading))
                }
            if (state.error != null)
                item(key = "error") {
                    Text(
                        stringResource(R.string.vocabulary_error),
                        modifier = Modifier.testTag("vocabulary_error"),
                    )
                    Button(
                        onClick = { dispatch(Event.RetryVocabulary) },
                        enabled = !view.storageBusy && !frozen,
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
    var expanded by remember { mutableStateOf(false) }
    val labels =
        mapOf(
            VocabularySort.NEWEST to R.string.vocabulary_sort_newest,
            VocabularySort.FREQUENCY to R.string.vocabulary_sort_frequency,
            VocabularySort.LASTSEEN to R.string.vocabulary_sort_last_seen,
            VocabularySort.ALPHABETICAL to R.string.vocabulary_sort_alphabetical,
        )
    Box {
        TextButton(
            onClick = { expanded = true },
            enabled = enabled,
            modifier = Modifier.testTag("vocabulary_sort"),
        ) {
            Text(stringResource(R.string.library_sort, stringResource(labels.getValue(sort))))
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            labels.forEach { (option, label) ->
                DropdownMenuItem(
                    text = { Text(stringResource(label)) },
                    onClick = {
                        expanded = false
                        onSort(option)
                    },
                    enabled = enabled,
                    modifier = Modifier.testTag("vocabulary_sort_${option.name}"),
                )
            }
        }
    }
}
