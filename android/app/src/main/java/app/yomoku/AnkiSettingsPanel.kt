package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import app.yomoku.shared.AnkiSettingsPhase
import app.yomoku.shared.AnkiSettingsView
import app.yomoku.shared.Event

@Composable
internal fun AnkiSettingsPanel(
    settings: AnkiSettingsView,
    discovery: AnkiDeckDiscoveryState,
    dispatch: (Event) -> Unit,
    checkProvider: () -> Unit,
    setupBusy: Boolean = false,
) {
    Column(Modifier.testTag("anki_settings"), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SettingsSectionHeading(stringResource(R.string.anki_settings_title))
        SettingsDescription(stringResource(R.string.anki_settings_description))
        Text(
            stringResource(
                when (settings.phase) {
                    AnkiSettingsPhase.UNLOADED -> R.string.anki_settings_unloaded
                    AnkiSettingsPhase.LOADING -> R.string.anki_settings_loading
                    AnkiSettingsPhase.READY -> R.string.anki_settings_ready
                    AnkiSettingsPhase.SAVING -> R.string.anki_settings_saving
                    AnkiSettingsPhase.FAILED -> R.string.anki_settings_failed
                }
            ),
            Modifier.testTag("anki_settings_status"),
        )
        settings.defaultDeckId?.let {
            Text(
                stringResource(R.string.anki_settings_deck_id, it),
                Modifier.testTag("anki_settings_deck"),
            )
        }
        settings.storageError?.let {
            Text(stringResource(it.messageResource()), color = MaterialTheme.colorScheme.error)
        }
        if (settings.phase != AnkiSettingsPhase.READY) {
            TextButton(
                onClick = {
                    dispatch(
                        if (settings.phase == AnkiSettingsPhase.UNLOADED) Event.LoadAnkiSettings
                        else Event.RetryAnkiSettings
                    )
                },
                modifier = Modifier.testTag("anki_settings_load"),
            ) {
                Text(
                    stringResource(
                        if (settings.phase == AnkiSettingsPhase.UNLOADED)
                            R.string.anki_settings_load
                        else R.string.anki_settings_reload
                    )
                )
            }
        }
        val ready = settings.phase == AnkiSettingsPhase.READY
        if (ready && settings.defaultDeckId == null)
            Text(stringResource(R.string.anki_settings_no_deck))
        if (settings.defaultDeckId != null) {
            TextButton(
                onClick = { dispatch(Event.SetAnkiDefaultDeck(null)) },
                enabled = ready,
                modifier = Modifier.testTag("anki_settings_clear"),
            ) {
                Text(stringResource(R.string.anki_settings_clear))
            }
        }
        TextButton(
            onClick = checkProvider,
            enabled = ready && !setupBusy && discovery != AnkiDeckDiscoveryState.Loading,
            modifier = Modifier.testTag("anki_decks_check"),
        ) {
            Text(stringResource(R.string.anki_settings_check))
        }
        when (discovery) {
            AnkiDeckDiscoveryState.Unchecked -> Unit
            AnkiDeckDiscoveryState.Loading -> Text(stringResource(R.string.anki_settings_checking))
            is AnkiDeckDiscoveryState.Failed -> {
                Text(
                    stringResource(discovery.error.messageResource()),
                    color = MaterialTheme.colorScheme.error,
                )
                if (discovery.error == AnkiReadError.PermissionDenied) {
                    Text(stringResource(R.string.anki_settings_permission_reason))
                }
            }
            is AnkiDeckDiscoveryState.Ready -> {
                val decks = discovery.decks.filter { !it.filtered }
                if (decks.isEmpty()) Text(stringResource(R.string.anki_settings_no_available_decks))
                if (
                    settings.defaultDeckId != null && decks.none { it.id == settings.defaultDeckId }
                ) {
                    Text(stringResource(R.string.anki_settings_missing_deck))
                }
                Column(Modifier.selectableGroup()) {
                    for (deck in decks) {
                        val selected = settings.defaultDeckId == deck.id
                        Row(
                            Modifier.fillMaxWidth()
                                .selectable(
                                    selected = selected,
                                    enabled = ready,
                                    role = Role.RadioButton,
                                    onClick = { dispatch(Event.SetAnkiDefaultDeck(deck.id)) },
                                )
                                .testTag("anki_deck_${deck.id}"),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            RadioButton(selected, onClick = null, enabled = ready)
                            Text(
                                stringResource(
                                    R.string.anki_settings_deck_option,
                                    deck.name,
                                    deck.id,
                                )
                            )
                        }
                    }
                }
            }
        }
    }
}

private fun AnkiReadError.messageResource(): Int =
    when (this) {
        AnkiReadError.ProviderUnavailable -> R.string.anki_error_unavailable
        AnkiReadError.UnsupportedProvider -> R.string.anki_error_unsupported
        AnkiReadError.PermissionDenied -> R.string.anki_error_permission
        AnkiReadError.CollectionUnavailable -> R.string.anki_error_collection
        AnkiReadError.DuplicateLookupFailed -> R.string.anki_error_duplicates
        AnkiReadError.InvalidResponse -> R.string.anki_error_response
        AnkiReadError.ProviderFailure -> R.string.anki_error_provider
    }

@Composable
internal fun AnkiModelSetupPanel(phase: app.yomoku.shared.AnkiSetupPhase) {
    Column(Modifier.testTag("anki_model_setup"), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            stringResource(
                when (phase) {
                    app.yomoku.shared.AnkiSetupPhase.Ready -> R.string.anki_model_ready
                    app.yomoku.shared.AnkiSetupPhase.Missing -> R.string.anki_model_missing
                    app.yomoku.shared.AnkiSetupPhase.Checking -> R.string.anki_export_checking
                    is app.yomoku.shared.AnkiSetupPhase.Failed -> phase.value.messageResource()
                    else -> R.string.anki_model_unchecked
                }
            )
        )
    }
}
