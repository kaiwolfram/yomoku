package app.yomoku

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.sizeIn
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import app.yomoku.shared.ReaderFontSize
import app.yomoku.shared.ReaderLineSpacing
import app.yomoku.shared.ViewModel

@Composable
internal fun SettingsScreen(
    view: ViewModel,
    dispatch: (Event) -> Unit,
    ankiDecks: AnkiDeckDiscoveryState = AnkiDeckDiscoveryState.Unchecked,
    checkAnkiProvider: () -> Unit = {},
    ankiBrowser: AnkiNoteBrowser? = null,
) {
    var showNotices by rememberSaveable { mutableStateOf(false) }
    var showWordNotices by rememberSaveable { mutableStateOf(false) }
    var showSvgNotices by rememberSaveable { mutableStateOf(false) }
    var showFontNotices by rememberSaveable { mutableStateOf(false) }
    if (showNotices) {
        SudachiNotices(onDismiss = { showNotices = false })
    }
    if (showWordNotices) {
        WordDictionaryNotices(onDismiss = { showWordNotices = false })
    }
    if (showSvgNotices) {
        SvgNotices(onDismiss = { showSvgNotices = false })
    }
    if (showFontNotices) {
        ReaderFontNotices(onDismiss = { showFontNotices = false })
    }
    Column(
        modifier =
            Modifier.widthIn(max = 720.dp)
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .testTag("settings_scroll")
                .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StorageStatus(view)
        ReaderSaveRetry(view, dispatch)
        if (view.pageLength == null && !view.storageBusy) {
            TextButton(onClick = { dispatch(Event.RefreshLibrary) }) {
                Text(stringResource(R.string.retry))
            }
        }
        val typography = view.readerTypography
        val typographyEnabled =
            typography != null && !view.storageBusy && !view.completionSavePending

        SettingsSectionHeading(stringResource(R.string.settings_reader))
        SettingsDescription(stringResource(R.string.reader_typography_description))
        TypographyControl(
            label = stringResource(R.string.reader_font_size, typography?.fontSize?.spValue ?: 16),
            tag = "reader_font_size",
            index = typography?.fontSize?.ordinal ?: 1,
            count = ReaderFontSize.entries.size,
            enabled = typographyEnabled,
        ) { index ->
            typography?.let {
                dispatch(
                    Event.SetReaderTypography(it.copy(fontSize = ReaderFontSize.entries[index]))
                )
            }
        }
        TypographyControl(
            label =
                stringResource(
                    R.string.reader_line_spacing,
                    typography?.lineSpacing?.percent ?: 150,
                ),
            tag = "reader_line_spacing",
            index = typography?.lineSpacing?.ordinal ?: 1,
            count = ReaderLineSpacing.entries.size,
            enabled = typographyEnabled,
        ) { index ->
            typography?.let {
                dispatch(
                    Event.SetReaderTypography(
                        it.copy(lineSpacing = ReaderLineSpacing.entries[index])
                    )
                )
            }
        }
        Row(
            Modifier.fillMaxWidth()
                .toggleable(
                    value = typography?.furigana == true,
                    enabled = typographyEnabled,
                    role = Role.Switch,
                ) { enabled ->
                    typography?.let {
                        dispatch(Event.SetReaderTypography(it.copy(furigana = enabled)))
                    }
                }
                .testTag("reader_furigana")
                .padding(vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(stringResource(R.string.reader_furigana))
            Switch(
                checked = typography?.furigana == true,
                onCheckedChange = null,
                enabled = typographyEnabled,
            )
        }
        Text(
            stringResource(R.string.page_length_title),
            style = MaterialTheme.typography.titleMedium,
        )
        SettingsDescription(stringResource(R.string.page_length_description))
        Column(Modifier.selectableGroup()) {
            for ((length, characters) in
                listOf(
                    PageLength.CHARACTERS250 to 250,
                    PageLength.CHARACTERS500 to 500,
                    PageLength.CHARACTERS750 to 750,
                    PageLength.CHARACTERS1000 to 1000,
                )) {
                val selected = view.pageLength == length
                val enabled =
                    !view.storageBusy && !view.completionSavePending && view.pageLength != null

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
        HorizontalDivider()
        AnkiSettingsPanel(
            view.ankiSettings,
            ankiDecks,
            dispatch,
            checkAnkiProvider,
            setupBusy =
                view.ankiSetup == app.yomoku.shared.AnkiSetupPhase.Checking ||
                    view.ankiExport != null,
        )
        AnkiModelSetupPanel(view.ankiSetup)
        view.ankiExport?.let { AnkiExportPanel(it, dispatch, ankiBrowser) }
        HorizontalDivider()
        SettingsSectionHeading(stringResource(R.string.settings_about))
        SettingsDescription(stringResource(R.string.settings_about_description))
        NoticeRow(stringResource(R.string.sudachi_notices_title)) { showNotices = true }
        NoticeRow(stringResource(R.string.word_dictionary_notices_title)) { showWordNotices = true }
        NoticeRow(stringResource(R.string.svg_notices_title)) { showSvgNotices = true }
        NoticeRow(stringResource(R.string.reader_font_notices_title)) { showFontNotices = true }
    }
}

@Composable
private fun TypographyControl(
    label: String,
    tag: String,
    index: Int,
    count: Int,
    enabled: Boolean,
    onChange: (Int) -> Unit,
) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, modifier = Modifier.weight(1f).testTag(tag))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(
                onClick = { onChange(index - 1) },
                enabled = enabled && index > 0,
                modifier = Modifier.testTag("${tag}_decrease"),
            ) {
                Text(stringResource(R.string.reader_typography_decrease))
            }
            TextButton(
                onClick = { onChange(index + 1) },
                enabled = enabled && index < count - 1,
                modifier = Modifier.testTag("${tag}_increase"),
            ) {
                Text(stringResource(R.string.reader_typography_increase))
            }
        }
    }
}

@Composable
internal fun SettingsSectionHeading(label: String) {
    Text(
        label,
        modifier = Modifier.semantics { heading() }.padding(top = 8.dp),
        style = MaterialTheme.typography.titleLarge,
    )
}

@Composable
internal fun SettingsDescription(description: String) {
    Text(
        description,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

@Composable
private fun NoticeRow(label: String, onClick: () -> Unit) {
    Text(
        label,
        modifier =
            Modifier.fillMaxWidth()
                .sizeIn(minHeight = 48.dp)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(vertical = 16.dp),
        style = MaterialTheme.typography.bodyLarge,
        color = MaterialTheme.colorScheme.primary,
    )
}
