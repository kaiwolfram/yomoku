package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import app.yomoku.shared.ViewModel

@Composable
internal fun SettingsScreen(view: ViewModel, dispatch: (Event) -> Unit) {
    Column(
        modifier =
            Modifier.widthIn(max = 720.dp)
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StorageStatus(view)
        Text(
            stringResource(R.string.page_length_title),
            style = MaterialTheme.typography.titleMedium,
        )
        Text(stringResource(R.string.page_length_description))
        Column(Modifier.selectableGroup()) {
            for (length in PageLength.entries) {
                val characters =
                    when (length) {
                        PageLength.CHARACTERS250 -> 250
                        PageLength.CHARACTERS500 -> 500
                        PageLength.CHARACTERS750 -> 750
                        PageLength.CHARACTERS1000 -> 1000
                        PageLength.CHARACTERS1500 -> 1500
                    }
                val selected = view.pageLength == length
                val enabled = !view.storageBusy && view.pageLength != null
                Row(
                    modifier =
                        Modifier.fillMaxWidth()
                            .selectable(
                                selected = selected,
                                enabled = enabled,
                                role = Role.RadioButton,
                                onClick = { dispatch(Event.SetPageLength(length)) },
                            )
                            .padding(vertical = 8.dp)
                            .testTag("page_length_$characters"),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = selected, onClick = null, enabled = enabled)
                    Text(pluralStringResource(R.plurals.page_length_preset, characters, characters))
                }
            }
        }
        if (view.pageLength == null && !view.storageBusy) {
            TextButton(onClick = { dispatch(Event.RefreshLibrary) }) {
                Text(stringResource(R.string.retry))
            }
        }
    }
}
