package app.yomoku

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.LibrarySort
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ViewModel

@Composable
internal fun LibraryScreen(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    coverLoader: EpubCoverLoader? = null,
    libraryActionsEnabled: Boolean = true,
) {
    LaunchedEffect(
        libraryActionsEnabled,
        view.libraryWordDistributions,
        view.libraryWordError,
        view.storageBusy,
        view.completionSavePending,
    ) {
        if (
            libraryActionsEnabled &&
                view.libraryWordDistributions == null &&
                view.libraryWordError == null &&
                !view.storageBusy &&
                !view.completionSavePending
        ) {
            dispatch(Event.LoadLibraryWordDistributions)
        }
    }
    val distributions =
        remember(view.libraryWordDistributions) {
            view.libraryWordDistributions?.associateBy { it.documentId }
        }
    val canAct = libraryActionsEnabled && !view.storageBusy && !view.completionSavePending

    val actionsLabel = stringResource(R.string.library_document_actions)
    LibraryDeleteDialog(view, dispatch, enabled = libraryActionsEnabled)
    LibraryRenameDialog(view, dispatch, enabled = libraryActionsEnabled)
    Column(Modifier.fillMaxSize().padding(16.dp)) {
        StorageStatus(view)
        if (view.libraryWordError != null) {
            Text(stringResource(R.string.library_words_error))
            TextButton(
                onClick = { dispatch(Event.LoadLibraryWordDistributions) },
                enabled = libraryActionsEnabled && !view.storageBusy,
                modifier = Modifier.testTag("retry_library_words"),
            ) {
                Text(stringResource(R.string.retry))
            }
        }
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
        if (view.documents.size >= 2) {
            LibrarySortMenu(view.librarySort) { dispatch(Event.SortLibrary(it)) }
        }
        if (!view.storageBusy && view.documents.isEmpty() && view.documentError == null) {
            Box(
                modifier = Modifier.fillMaxWidth().weight(1f).testTag("library_empty_area"),
                contentAlignment = Alignment.Center,
            ) {
                Column(
                    modifier = Modifier.widthIn(max = 320.dp).testTag("library_empty_state"),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Icon(
                        painterResource(R.drawable.ic_library),
                        contentDescription = null,
                        modifier = Modifier.size(48.dp),
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Text(
                        stringResource(R.string.library_empty),
                        style = MaterialTheme.typography.titleMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.testTag("library_empty_message"),
                    )
                    Text(
                        stringResource(R.string.library_empty_hint),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                        modifier = Modifier.testTag("library_empty_hint"),
                    )
                }
            }
        } else {
            LazyColumn(modifier = Modifier.weight(1f).testTag("library_documents")) {
                items(view.documents, key = { it.documentId.value }) { document ->
                    var menuExpanded by
                        remember(view.librarySort, canAct) {
                            mutableStateOf(false)
                        }
                    Box {
                        Row(
                            modifier =
                                Modifier.fillMaxWidth()
                                    .combinedClickable(
                                        enabled = !view.storageBusy,
                                        onLongClickLabel = actionsLabel,
                                        onLongClick = { if (canAct) menuExpanded = true },
                                        onClick = {
                                            dispatch(Event.OpenReader(document.documentId))
                                        },
                                    )
                                    .padding(vertical = 8.dp)
                                    .testTag("document_${document.documentId.value}"),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            LibraryCover(document.documentId.value, coverLoader)
                            Column(Modifier.weight(1f)) {
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
                                            Modifier.testTag(
                                                "finished_${document.documentId.value}"
                                            )
                                        )
                                    }
                                }
                                Text(
                                    stringResource(
                                        R.string.reading_progress,
                                        document.progressPercent.toLong(),
                                    ),
                                    modifier =
                                        Modifier.testTag("progress_${document.documentId.value}"),
                                )
                                LibraryWordBar(
                                    distributions?.get(document.documentId)?.counts,
                                    loading =
                                        distributions == null && view.libraryWordError == null,
                                    documentId = document.documentId.value,
                                )
                            }
                        }
                        DropdownMenu(
                            expanded = menuExpanded && canAct,
                            onDismissRequest = { menuExpanded = false },
                        ) {
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.library_rename)) },
                                onClick = {
                                    menuExpanded = false
                                    dispatch(Event.RequestDocumentRename(document.documentId))
                                },
                                modifier =
                                    Modifier.testTag(
                                        "rename_document_${document.documentId.value}"
                                    ),
                            )
                            DropdownMenuItem(
                                text = { Text(stringResource(R.string.library_delete)) },
                                onClick = {
                                    menuExpanded = false
                                    dispatch(Event.RequestDocumentDeletion(document.documentId))
                                },
                                modifier =
                                    Modifier.testTag(
                                        "delete_document_${document.documentId.value}"
                                    ),
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LibrarySortMenu(sort: LibrarySort, onSort: (LibrarySort) -> Unit) {
    val labels =
        mapOf(
            LibrarySort.LASTREAD to R.string.library_sort_last_read,
            LibrarySort.TITLE to R.string.library_sort_title,
            LibrarySort.IMPORTDATE to R.string.library_sort_import_date,
        )
    val reverseSort =
        when (sort) {
            LibrarySort.LASTREAD -> LibrarySort.LASTREADASCENDING
            LibrarySort.LASTREADASCENDING -> LibrarySort.LASTREAD
            LibrarySort.TITLE -> LibrarySort.TITLEDESCENDING
            LibrarySort.TITLEDESCENDING -> LibrarySort.TITLE
            LibrarySort.IMPORTDATE -> LibrarySort.IMPORTDATEASCENDING
            LibrarySort.IMPORTDATEASCENDING -> LibrarySort.IMPORTDATE
        }
    val baseSort =
        when (sort) {
            LibrarySort.LASTREADASCENDING -> LibrarySort.LASTREAD
            LibrarySort.TITLEDESCENDING -> LibrarySort.TITLE
            LibrarySort.IMPORTDATEASCENDING -> LibrarySort.IMPORTDATE
            else -> sort
        }
    val ascending =
        sort in
            listOf(
                LibrarySort.TITLE,
                LibrarySort.LASTREADASCENDING,
                LibrarySort.IMPORTDATEASCENDING,
            )
    SortControls(
        labels = labels,
        baseSort = baseSort,
        ascending = ascending,
        tag = "library",
        onReverse = { onSort(reverseSort) },
        onSort = onSort,
    )
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
internal fun ReaderScreen(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    imageLoader: EpubImageLoader? = null,
    ankiBrowser: AnkiNoteBrowser? = null,
    onFinishRequested: (Offset) -> Unit = {},
) {
    val scroll =
        remember(sessionKey, view.reader?.documentId, view.reader?.anchor) { ScrollState(0) }
    val page = view.reader
    // Native state selects compact consumers. A missing/retired effective page never uses B.
    val snapshot = view.effectiveReaderTokens
    val identity = snapshot?.identity
    val effectivePageReady =
        remember(page, view.readerAnalysis, snapshot) {
            page != null &&
                snapshot != null &&
                view.readerAnalysis is ReaderAnalysis.Available &&
                identity?.base ==
                    ReaderTokenKey(page.documentId, page.anchor, view.readerAnalysis) &&
                effectiveDisplayTokens(page, snapshot.identity, snapshot) != null
        }
    val effectiveIdle =
        view.effectiveReaderWord?.let { !it.busy } != false &&
            view.effectiveReaderStatus?.let { !it.busy && !it.pending } != false
    val canNavigate =
        page != null &&
            !view.settingsOpen &&
            !view.storageBusy &&
            !view.completionSavePending &&
            effectiveIdle
    val canAdvance =
        canNavigate && page.position.offset != page.anchor.range.end && effectivePageReady
    val canGoBack = canNavigate && page.pageNumber > 1UL
    val navigationKey = Triple(sessionKey, page, identity)
    val currentKey by rememberUpdatedState(navigationKey)
    val currentCanAdvance by rememberUpdatedState(canAdvance)
    val currentCanGoBack by rememberUpdatedState(canGoBack)
    var finishCenter by remember { mutableStateOf<Offset?>(null) }
    val next = {
        if (canAdvance && currentCanAdvance && currentKey == navigationKey) {
            if (page.pageNumber == page.totalPages) finishCenter?.let(onFinishRequested)
            dispatch(
                Event.CompleteEffectiveReaderPage(
                    snapshot!!.identity,
                    page.position,
                    System.currentTimeMillis(),
                )
            )
        }
    }
    val previous = {
        if (canGoBack && currentCanGoBack && currentKey == navigationKey) {
            dispatch(Event.PreviousReaderPage(page.documentId, page.anchor, page.position))
        }
    }
    Column(
        modifier =
            Modifier.widthIn(max = 720.dp)
                .fillMaxSize()
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

            EffectiveReaderWords(
                view,
                sessionKey,
                dispatch,
                imageLoader,
                ankiBrowser,
            )

            Row(
                modifier = Modifier.fillMaxWidth().testTag("reader_navigation"),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                    TextButton(
                        onClick = previous,
                        enabled = canGoBack,
                        modifier = Modifier.testTag("previous_page"),
                    ) {
                        Text(stringResource(R.string.previous_page))
                    }
                }
                Text(
                    stringResource(
                        R.string.page_counter,
                        page.pageNumber.toLong(),
                        page.totalPages.toLong(),
                    ),
                    modifier = Modifier.testTag("page_counter"),
                )
                Box(Modifier.weight(1f), contentAlignment = Alignment.CenterEnd) {
                    TextButton(
                        onClick = next,
                        enabled = canAdvance,
                        modifier =
                            Modifier.testTag("next_page").onGloballyPositioned {
                                finishCenter =
                                    it.localToWindow(
                                        Offset(it.size.width / 2f, it.size.height / 2f)
                                    )
                            },
                    ) {
                        Text(
                            stringResource(
                                if (page.pageNumber == page.totalPages) R.string.finish_book
                                else R.string.next_page
                            )
                        )
                    }
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
    if (view.reader != null && view.documentError != null && view.completionSavePending) {
        TextButton(
            enabled = !view.storageBusy,
            onClick = { dispatch(Event.RetryReaderCompletion) },
        ) {
            Text(stringResource(R.string.retry))
        }
    }
}

@Composable
internal fun StorageStatus(view: ViewModel) {
    view.documentError?.let { error ->
        Text(
            stringResource(error.messageResource()),
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.testTag("document_error"),
        )
    }
}

internal fun DocumentError.messageResource(): Int =
    when (this) {
        DocumentError.EMPTYPASTE -> R.string.error_empty_paste
        DocumentError.UNAVAILABLE -> R.string.error_storage_unavailable
        DocumentError.INCOMPATIBLEDATABASE -> R.string.error_incompatible_database
        DocumentError.INVALIDDOCUMENT -> R.string.error_invalid_document
        DocumentError.NOTFOUND -> R.string.error_document_not_found
        DocumentError.INVALIDSETTINGS -> R.string.error_invalid_settings
        DocumentError.MISSINGANALYSIS -> R.string.error_missing_analysis
        DocumentError.TXTENCODINGREQUIRED -> R.string.error_txt_encoding_required
        DocumentError.TXTINVALIDUTF8 -> R.string.error_txt_invalid_utf8
        DocumentError.TXTINVALIDSHIFTJIS -> R.string.error_txt_invalid_shift_jis
        DocumentError.TXTUNSUPPORTEDENCODING -> R.string.error_txt_unsupported_encoding
        DocumentError.TXTUNDECODABLE -> R.string.error_txt_undecodable
        DocumentError.EMPTYTXT -> R.string.error_empty_txt
        DocumentError.SRTENCODINGREQUIRED -> R.string.error_srt_encoding_required
        DocumentError.SRTINVALIDUTF8 -> R.string.error_srt_invalid_utf8
        DocumentError.SRTINVALIDSHIFTJIS -> R.string.error_srt_invalid_shift_jis
        DocumentError.SRTUNSUPPORTEDENCODING -> R.string.error_srt_unsupported_encoding
        DocumentError.SRTUNDECODABLE -> R.string.error_srt_undecodable
        DocumentError.EMPTYSRT -> R.string.error_empty_srt
        DocumentError.INVALIDEPUB -> R.string.error_invalid_epub
    }
