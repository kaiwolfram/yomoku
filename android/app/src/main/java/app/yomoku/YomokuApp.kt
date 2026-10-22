package app.yomoku

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
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel
import app.yomoku.theme.YomokuTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun YomokuApp(view: ViewModel, dispatch: (Event) -> Unit) {
    BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
        val useRail = maxWidth >= 600.dp
        Scaffold(
            topBar = {
                TopAppBar(
                    title = {
                        Text(
                            stringResource(
                                if (view.settingsOpen) R.string.settings_title
                                else view.destination.titleResource()
                            ),
                            modifier = Modifier.testTag("screen_title"),
                        )
                    },
                    navigationIcon = {
                        if (view.settingsOpen) {
                            TextButton(onClick = { dispatch(Event.CloseSettings) }) {
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
                if (!view.settingsOpen && !useRail) {
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
                if (!view.settingsOpen && useRail) {
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
                Text(
                    stringResource(
                        if (view.settingsOpen) R.string.settings_empty
                        else view.destination.emptyResource()
                    ),
                    modifier = Modifier.padding(16.dp).verticalScroll(rememberScrollState()),
                )
            }
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
    YomokuTheme { YomokuApp(ViewModel(Destination.LIBRARY, false)) {} }
}
