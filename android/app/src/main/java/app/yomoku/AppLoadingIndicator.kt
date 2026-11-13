package app.yomoku

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiExportPhase
import app.yomoku.shared.AnkiSettingsPhase
import app.yomoku.shared.AnkiSetupPhase
import app.yomoku.shared.ViewModel
import kotlinx.coroutines.delay

internal fun appIsLoading(
    view: ViewModel,
    discovery: AnkiDeckDiscoveryState,
    importActive: Boolean,
): Boolean =
    importActive ||
        view.storageBusy ||
        view.vocabulary.loading ||
        view.statistics.loading ||
        view.ankiSettings.phase == AnkiSettingsPhase.LOADING ||
        view.ankiSettings.phase == AnkiSettingsPhase.SAVING ||
        discovery == AnkiDeckDiscoveryState.Loading ||
        view.ankiSetup == AnkiSetupPhase.Checking ||
        when (view.ankiExport?.phase) {
            AnkiExportPhase.Checking,
            AnkiExportPhase.Ready,
            AnkiExportPhase.Invoking -> true
            else -> false
        }

/** One delay per continuous interval of aggregate work; completion removes it in this frame. */
@Composable
internal fun AppLoadingIndicator(loading: Boolean) {
    var visible by remember(loading) { mutableStateOf(false) }
    LaunchedEffect(loading) {
        if (loading) {
            delay(300)
            visible = true
        }
    }
    if (loading && visible) {
        val description = stringResource(R.string.app_loading)
        Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
            CircularProgressIndicator(
                modifier =
                    Modifier.size(24.dp).testTag("app_loading").semantics {
                        contentDescription = description
                    },
                strokeWidth = 2.dp,
            )
        }
    }
}
