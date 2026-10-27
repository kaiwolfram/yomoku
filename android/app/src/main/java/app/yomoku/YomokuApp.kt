package app.yomoku

import android.widget.Toast
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel
import app.yomoku.theme.YomokuTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun YomokuApp(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    onReaderInteraction: () -> Unit = {},
    imageLoader: EpubImageLoader? = null,
    coverLoader: EpubCoverLoader? = null,
    onImportFiles: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    ReaderCompletionToast(view, dispatch) { message ->
        Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
    }
    var pasteText by rememberSaveable { mutableStateOf("") }
    LaunchedEffect(view.contentRoute) {
        if (view.contentRoute != ContentRoute.PASTE) pasteText = ""
    }
    val mainContent = view.contentRoute == ContentRoute.MAIN
    val showNavigation = !view.settingsOpen && mainContent
    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
        val useRail = maxWidth >= 600.dp
        Scaffold(
            topBar = {
                TopAppBar(
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
                            modifier = Modifier.testTag("screen_title"),
                        )
                    },
                    navigationIcon = {
                        if (view.settingsOpen || !mainContent) {
                            TextButton(
                                onClick = {
                                    dispatch(
                                        if (view.settingsOpen) Event.CloseSettings
                                        else Event.CloseDocument
                                    )
                                }
                            ) {
                                Text(stringResource(R.string.back))
                            }
                        }
                    },
                    actions = {
                        if (!view.settingsOpen) {
                            TextButton(onClick = { dispatch(Event.OpenSettings) }) {
                                Text(stringResource(R.string.settings_title))
                            }
                        }
                    },
                )
            },
            bottomBar = {
                if (showNavigation && !useRail) {
                    NavigationBar(modifier = Modifier.testTag("bottom_navigation")) {
                        for (destination in Destination.entries) {
                            NavigationBarItem(
                                selected = view.destination == destination,
                                onClick = { dispatch(Event.SelectDestination(destination)) },
                                icon = { DestinationIcon(destination) },
                                label = { Text(stringResource(destination.titleResource())) },
                            )
                        }
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
                        for (destination in Destination.entries) {
                            NavigationRailItem(
                                selected = view.destination == destination,
                                onClick = { dispatch(Event.SelectDestination(destination)) },
                                icon = { DestinationIcon(destination) },
                                label = { Text(stringResource(destination.titleResource())) },
                            )
                        }
                    }
                }
                Box(
                    modifier = Modifier.weight(1f).fillMaxHeight(),
                    contentAlignment = Alignment.TopCenter,
                ) {
                    when {
                        view.settingsOpen -> SettingsScreen(view, dispatch)
                        view.contentRoute == ContentRoute.PASTE ->
                            PasteScreen(view, pasteText, { pasteText = it }, dispatch)
                        view.contentRoute == ContentRoute.READER ->
                            ReaderScreen(view, dispatch, onReaderInteraction, imageLoader)
                        view.destination == Destination.LIBRARY ->
                            LibraryScreen(view, dispatch, onImportFiles, coverLoader)
                        else ->
                            Text(
                                stringResource(view.destination.emptyResource()),
                                Modifier.padding(16.dp),
                            )
                    }
                }
            }
        }
        BookCompletionCelebration(view, dispatch)
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
private fun DestinationIcon(destination: Destination) {
    Icon(painterResource(destination.iconResource()), contentDescription = null)
}

private fun Destination.titleResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.string.library_title
        Destination.NOSTR -> R.string.nostr_title
        Destination.STATISTICS -> R.string.statistics_title
        Destination.VOCABULARY -> R.string.vocabulary_title
    }

private fun Destination.emptyResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.string.library_empty
        Destination.NOSTR -> R.string.nostr_empty
        Destination.STATISTICS -> R.string.statistics_empty
        Destination.VOCABULARY -> R.string.vocabulary_empty
    }

private fun Destination.iconResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.drawable.ic_library
        Destination.NOSTR -> R.drawable.ic_nostr
        Destination.STATISTICS -> R.drawable.ic_statistics
        Destination.VOCABULARY -> R.drawable.ic_vocabulary
    }

@Preview
@Composable
private fun YomokuPreview() {
    YomokuTheme {
        YomokuApp(
            ViewModel(
                Destination.LIBRARY,
                false,
                emptyList(),
                null,
                false,
                null,
                ContentRoute.MAIN,
                null,
                null,
                null,
                null,
                readerTime = null,
                readingTimeSavePending = false,
                completionSavePending = false,
                completionNotice = null,
                bookCompletionNotice = null,
                readerTokens = null,
                readerFurigana = emptyList(),
            ),
            dispatch = {},
        )
    }
}
