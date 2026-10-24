package app.yomoku

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryScreen(view: ViewModel, dispatch: (Event) -> Unit) {
    Column(Modifier.fillMaxSize().padding(16.dp)) {
        Button(onClick = { dispatch(Event.OpenPaste) }, enabled = !view.storageBusy) {
            Text(stringResource(R.string.paste_title))
        }
        StorageStatus(view)
        view.importSummary?.let { summary ->
            Column(modifier = Modifier.fillMaxWidth().testTag("import_summary")) {
                Text(
                    stringResource(
                        when (summary.outcome) {
                            ImportOutcome.CREATED -> R.string.import_created
                            ImportOutcome.EXISTING -> R.string.import_existing
                        },
                        summary.title,
                    ),
                    modifier = Modifier.testTag("import_summary_message"),
                )
                TextButton(onClick = { dispatch(Event.DismissImportSummary) }) {
                    Text(stringResource(R.string.close))
                }
            }
        }
        if (view.documentError != null) {
            TextButton(onClick = { dispatch(Event.RefreshLibrary) }, enabled = !view.storageBusy) {
                Text(stringResource(R.string.retry))
            }
        }
        if (!view.storageBusy && view.documents.isEmpty() && view.documentError == null) {
            Text(stringResource(R.string.library_empty))
        }
        LazyColumn(modifier = Modifier.weight(1f).testTag("library_documents")) {
            items(view.documents, key = { it.documentId.value }) { document ->
                Column(
                    modifier =
                        Modifier.fillMaxWidth()
                            .clickable(enabled = !view.storageBusy) {
                                dispatch(Event.OpenReader(document.documentId))
                            }
                            .padding(vertical = 20.dp)
                            .testTag("document_${document.documentId.value}")
                ) {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            document.title,
                            style = MaterialTheme.typography.titleMedium,
                            modifier = Modifier.weight(1f),
                        )
                        if (document.finishedAtLeastOnce) {
                            FinishedCheckmark(
                                Modifier.testTag("finished_${document.documentId.value}")
                            )
                        }
                    }
                    Text(
                        stringResource(
                            R.string.reading_progress,
                            document.progressPercent.toLong(),
                        ),
                        modifier = Modifier.testTag("progress_${document.documentId.value}"),
                    )
                }
            }
        }
    }
}

@Composable
internal fun PasteScreen(
    view: ViewModel,
    text: String,
    onTextChanged: (String) -> Unit,
    dispatch: (Event) -> Unit,
) {
    Column(
        modifier = Modifier.widthIn(max = 720.dp).fillMaxSize().imePadding().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StorageStatus(view)
        OutlinedTextField(
            value = text,
            onValueChange = onTextChanged,
            label = { Text(stringResource(R.string.paste_label)) },
            enabled = !view.storageBusy,
            modifier = Modifier.fillMaxWidth().weight(1f).testTag("paste_text"),
        )
        Button(
            onClick = { dispatch(Event.CreatePaste(text)) },
            enabled = !view.storageBusy && text.isNotBlank(),
        ) {
            Text(stringResource(R.string.save_to_library))
        }
    }
}

@Composable
internal fun ReaderScreen(view: ViewModel, dispatch: (Event) -> Unit) {
    val scroll = remember(view.reader?.documentId, view.reader?.anchor) { ScrollState(0) }
    val page = view.reader
    val canNavigate =
        page != null &&
            !view.storageBusy &&
            !view.readingTimeSavePending &&
            !view.completionSavePending
    val canAdvance = canNavigate && page.position.offset != page.anchor.range.end
    val canGoBack = canNavigate && page.pageNumber > 1UL
    val next = {
        if (canAdvance) {
            dispatch(
                Event.CompleteReaderPage(
                    page.documentId,
                    page.anchor,
                    page.position,
                    System.currentTimeMillis(),
                )
            )
        }
    }
    val previous = {
        if (canGoBack) {
            dispatch(Event.PreviousReaderPage(page.documentId, page.anchor, page.position))
        }
    }
    val swipeThreshold = with(LocalDensity.current) { 48.dp.toPx() }
    Column(
        modifier =
            Modifier.widthIn(max = 720.dp)
                .fillMaxSize()
                .pointerInput(page, canNavigate) {
                    var horizontal = 0f
                    detectHorizontalDragGestures(
                        onDragStart = { horizontal = 0f },
                        onDragCancel = { horizontal = 0f },
                        onDragEnd = {
                            if (horizontal <= -swipeThreshold) next()
                            else if (horizontal >= swipeThreshold) previous()
                        },
                        onHorizontalDrag = { change, amount ->
                            if (canNavigate) {
                                change.consume()
                                horizontal += amount
                            }
                        },
                    )
                }
                .verticalScroll(scroll)
                .testTag("reader_scroll")
                .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(24.dp),
    ) {
        StorageStatus(view)
        ReaderSaveRetry(view, dispatch)
        page?.let { page ->
            if (page.finishedAtLeastOnce) {
                FinishedCheckmark(Modifier.testTag("reader_finished"))
            }
            Text(
                page.text,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.testTag("reader_text"),
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextButton(
                    onClick = previous,
                    enabled = canGoBack,
                    modifier = Modifier.testTag("previous_page"),
                ) {
                    Text(stringResource(R.string.previous_page))
                }
                Text(
                    stringResource(
                        R.string.page_counter,
                        page.pageNumber.toLong(),
                        page.totalPages.toLong(),
                    ),
                    modifier = Modifier.testTag("page_counter"),
                )
                TextButton(
                    onClick = next,
                    enabled = canAdvance,
                    modifier = Modifier.testTag("next_page"),
                ) {
                    Text(stringResource(R.string.next_page))
                }
            }
        }
    }
}

@Composable
private fun FinishedCheckmark(modifier: Modifier = Modifier) {
    Icon(
        painterResource(R.drawable.ic_finished),
        contentDescription = stringResource(R.string.finished_at_least_once),
        modifier = modifier,
        tint = MaterialTheme.colorScheme.primary,
    )
}

@Composable
internal fun ReaderSaveRetry(view: ViewModel, dispatch: (Event) -> Unit) {
    if (
        view.reader != null &&
            view.documentError != null &&
            (view.readingTimeSavePending || view.completionSavePending || view.readerTime == null)
    ) {
        TextButton(
            enabled = !view.storageBusy,
            onClick = {
                dispatch(
                    when {
                        view.readingTimeSavePending -> Event.RetryReadingTimeCheckpoint
                        view.completionSavePending -> Event.RetryReaderCompletion
                        else -> Event.RestoreReadingTime
                    }
                )
            },
        ) {
            Text(stringResource(R.string.retry))
        }
    }
}

@Composable
internal fun StorageStatus(view: ViewModel) {
    if (view.storageBusy)
        LinearProgressIndicator(modifier = Modifier.fillMaxWidth().testTag("storage_busy"))
    view.documentError?.let { error ->
        Text(
            stringResource(
                when (error) {
                    DocumentError.EMPTYPASTE -> R.string.error_empty_paste
                    DocumentError.UNAVAILABLE -> R.string.error_storage_unavailable
                    DocumentError.INCOMPATIBLEDATABASE -> R.string.error_incompatible_database
                    DocumentError.INVALIDDOCUMENT -> R.string.error_invalid_document
                    DocumentError.NOTFOUND -> R.string.error_document_not_found
                    DocumentError.INVALIDSETTINGS -> R.string.error_invalid_settings
                    DocumentError.MISSINGANALYSIS -> R.string.error_missing_analysis
                }
            ),
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.testTag("document_error"),
        )
    }
}
