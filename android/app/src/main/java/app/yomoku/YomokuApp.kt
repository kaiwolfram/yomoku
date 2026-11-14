package app.yomoku

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiSettingsPhase
import app.yomoku.shared.AnkiSettingsView
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import app.yomoku.shared.LibrarySort
import app.yomoku.shared.StatisticsState
import app.yomoku.shared.StatisticsTimeframe
import app.yomoku.shared.ViewModel
import app.yomoku.shared.VocabularyQuery
import app.yomoku.shared.VocabularySort
import app.yomoku.shared.VocabularyState
import app.yomoku.shared.VocabularyStatus
import app.yomoku.theme.YomokuTheme

private val navigationDestinations =
    listOf(Destination.LIBRARY, Destination.STATISTICS, Destination.VOCABULARY)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun YomokuApp(
    view: ViewModel,
    sessionKey: String,
    dispatch: (Event) -> Unit,
    imageLoader: EpubImageLoader? = null,
    coverLoader: EpubCoverLoader? = null,
    libraryActionsEnabled: Boolean = true,
    importActive: Boolean = false,
    ankiDecks: AnkiDeckDiscoveryState = AnkiDeckDiscoveryState.Unchecked,
    checkAnkiProvider: () -> Unit = {},
    ankiBrowser: AnkiNoteBrowser? = null,
    onImportBook: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val toasts = androidx.compose.runtime.remember(context) { AppToasts(context) }
    androidx.compose.runtime.DisposableEffect(toasts) { onDispose { toasts.cancel() } }
    ReaderCompletionToast(view, dispatch, toasts::show)
    AnkiAddedToast(view, dispatch, toasts::show)
    var pasteText by rememberSaveable { mutableStateOf("") }
    val vocabularyScroll =
        rememberSaveable(sessionKey, view.vocabulary.query, saver = LazyListState.Saver) {
            LazyListState()
        }
    LaunchedEffect(view.contentRoute) {
        if (view.contentRoute != ContentRoute.PASTE) pasteText = ""
    }
    val loadingVisible = rememberAppLoadingVisible(appIsLoading(view, ankiDecks, importActive))
    val mainContent = view.contentRoute == ContentRoute.MAIN
    val showNavigation = mainContent
    var completionOrigin by remember(sessionKey) { mutableStateOf<Offset?>(null) }
    var overlayPosition by remember { mutableStateOf(Offset.Zero) }
    BoxWithConstraints(
        modifier =
            Modifier.fillMaxSize().onGloballyPositioned {
                overlayPosition = it.positionInWindow()
            }
    ) {
        val useRail = maxWidth >= 600.dp
        Scaffold(
            topBar = {
                TopAppBar(
                    modifier = Modifier.testTag("app_toolbar"),
                    title = {
                        Text(
                            when {
                                view.settingsOpen -> stringResource(R.string.settings_title)
                                view.contentRoute == ContentRoute.PASTE ->
                                    stringResource(R.string.paste_title)
                                view.contentRoute == ContentRoute.READER ->
                                    view.reader?.title ?: stringResource(R.string.reader_title)
                                else -> stringResource(view.destination.titleResource())
                            },
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.testTag("screen_title"),
                        )
                    },
                    actions = {
                        if (
                            mainContent &&
                                !view.settingsOpen &&
                                view.destination == Destination.LIBRARY
                        ) {
                            Box(Modifier.size(48.dp)) {
                                if (loadingVisible) AppLoadingIndicator()
                                else
                                    LibraryAddMenu(
                                        view,
                                        dispatch,
                                        onImportBook,
                                        libraryActionsEnabled,
                                    )
                            }
                        } else if (loadingVisible) {
                            AppLoadingIndicator()
                        }
                        if (!view.settingsOpen && view.contentRoute == ContentRoute.READER) {
                            key(sessionKey, view.reader?.documentId) {
                                ReaderTypographyMenu(view, dispatch)
                            }
                        }
                    },
                    navigationIcon = {
                        if (!view.settingsOpen && !mainContent) {
                            IconButton(onClick = { dispatch(Event.CloseDocument) }) {
                                Icon(
                                    painterResource(R.drawable.ic_arrow_back),
                                    contentDescription = stringResource(R.string.back),
                                )
                            }
                        }
                    },
                )
            },
            bottomBar = {
                if (showNavigation && !useRail) {
                    NavigationBar(modifier = Modifier.testTag("bottom_navigation")) {
                        for (destination in navigationDestinations) {
                            NavigationBarItem(
                                selected = !view.settingsOpen && view.destination == destination,
                                onClick = { dispatch(Event.SelectDestination(destination)) },
                                icon = { DestinationIcon(destination) },
                                label = { Text(stringResource(destination.titleResource())) },
                            )
                        }
                        NavigationBarItem(
                            selected = view.settingsOpen,
                            onClick = { dispatch(Event.OpenSettings) },
                            icon = { SettingsIcon() },
                            label = { Text(stringResource(R.string.settings_title)) },
                            modifier = Modifier.testTag("navigation_settings"),
                        )
                    }
                }
            },
        ) { insets ->
            Row(modifier = Modifier.fillMaxSize().padding(insets).consumeWindowInsets(insets)) {
                if (showNavigation && useRail) {
                    NavigationRail(
                        modifier =
                            Modifier.testTag("navigation_rail")
                                .fillMaxHeight()
                                .verticalScroll(rememberScrollState())
                    ) {
                        for (destination in navigationDestinations) {
                            NavigationRailItem(
                                selected = !view.settingsOpen && view.destination == destination,
                                onClick = { dispatch(Event.SelectDestination(destination)) },
                                icon = { DestinationIcon(destination) },
                                label = { Text(stringResource(destination.titleResource())) },
                            )
                        }
                        NavigationRailItem(
                            selected = view.settingsOpen,
                            onClick = { dispatch(Event.OpenSettings) },
                            icon = { SettingsIcon() },
                            label = { Text(stringResource(R.string.settings_title)) },
                            modifier = Modifier.testTag("navigation_settings"),
                        )
                    }
                }
                Box(
                    modifier = Modifier.weight(1f).fillMaxHeight(),
                    contentAlignment = Alignment.TopCenter,
                ) {
                    when {
                        view.settingsOpen ->
                            SettingsScreen(
                                view,
                                dispatch,
                                ankiDecks,
                                checkAnkiProvider,
                                ankiBrowser,
                            )
                        view.contentRoute == ContentRoute.PASTE ->
                            PasteScreen(view, pasteText, { pasteText = it }, dispatch)
                        view.contentRoute == ContentRoute.READER ->
                            ReaderScreen(
                                view,
                                sessionKey,
                                dispatch,
                                imageLoader,
                                ankiBrowser,
                                onFinishRequested = { completionOrigin = it - overlayPosition },
                            )
                        view.destination == Destination.LIBRARY ->
                            LibraryScreen(
                                view,
                                dispatch,
                                coverLoader,
                                libraryActionsEnabled,
                            )
                        view.destination == Destination.VOCABULARY ->
                            VocabularyScreen(view, sessionKey, vocabularyScroll, dispatch)
                        view.destination == Destination.STATISTICS ->
                            StatisticsScreen(
                                view,
                                sessionKey,
                                dispatch,
                            )
                        else ->
                            Text(
                                stringResource(view.destination.emptyResource()),
                                Modifier.padding(16.dp),
                            )
                    }
                }
            }
        }
        BookCompletionCelebration(view, completionOrigin, { completionOrigin = null }, dispatch)
    }
}

@Composable
internal fun ReaderCompletionToast(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    show: (String) -> Unit,
) {
    val notice = view.completionNotice
    val message = stringResource(R.string.words_marked_known, notice?.newlyKnown.toString())
    LaunchedEffect(notice?.commandId) {
        if (notice != null) {
            // Consume synchronously so recreation cannot replay a displayed notice.
            dispatch(Event.DismissCompletionNotice)
            if (notice.newlyKnown > 0UL) show(message)
        }
    }
}

@Composable
private fun SettingsIcon() {
    Icon(painterResource(R.drawable.ic_settings), contentDescription = null)
}

@Composable
private fun DestinationIcon(destination: Destination) {
    Icon(painterResource(destination.iconResource()), contentDescription = null)
}

private fun Destination.titleResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.string.library_title
        Destination.STATISTICS -> R.string.statistics_title
        Destination.VOCABULARY -> R.string.vocabulary_title
    }

private fun Destination.emptyResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.string.library_empty
        Destination.STATISTICS -> R.string.statistics_empty
        Destination.VOCABULARY -> R.string.vocabulary_empty
    }

private fun Destination.iconResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.drawable.ic_library
        Destination.STATISTICS -> R.drawable.ic_statistics
        Destination.VOCABULARY -> R.drawable.ic_vocabulary
    }

@Preview
@Composable
private fun YomokuPreview() {
    YomokuTheme {
        YomokuApp(
            ViewModel(
                ankiAddedNotice = null,
                ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
                ankiSetup = app.yomoku.shared.AnkiSetupPhase.Unchecked,
                ankiEntries = emptyList(),
                vocabulary =
                    VocabularyState(
                        VocabularyQuery(
                            "",
                            VocabularyStatus.entries.toList(),
                            VocabularySort.NEWEST,
                        ),
                        rows = emptyList(),
                        loading = false,
                    ),
                statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
                destination = Destination.LIBRARY,
                settingsOpen = false,
                documents = emptyList(),
                librarySort = LibrarySort.LASTREAD,
                libraryWordDistributions = null,
                libraryWordError = null,
                libraryDeleteTarget = null,
                libraryDeleteError = null,
                libraryRename = null,
                storageBusy = false,
                documentError = null,
                contentRoute = ContentRoute.MAIN,
                reader = null,
                readerAnalysis = null,
                pageLength = null,
                readerTypography = null,
                completionSavePending = false,
                completionNotice = null,
                bookCompletionNotice = null,
                effectiveReaderTokens = null,
            ),
            sessionKey = "preview",
            dispatch = {},
        )
    }
}

@Composable
internal fun AnkiAddedToast(view: ViewModel, dispatch: (Event) -> Unit, show: (String) -> Unit) {
    val notice = view.ankiAddedNotice
    val message = stringResource(R.string.anki_added_toast)
    LaunchedEffect(notice) {
        if (notice != null) {
            dispatch(Event.DismissAnkiAddedNotice(notice))
            show(message)
        }
    }
}
