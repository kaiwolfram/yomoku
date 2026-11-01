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
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.verticalScroll
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
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import app.yomoku.shared.ReaderFontSize
import app.yomoku.shared.ReaderLineSpacing
import app.yomoku.shared.ViewModel

@Composable
internal fun SettingsScreen(view: ViewModel, dispatch: (Event) -> Unit) {
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
                .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StorageStatus(view)
        ReaderSaveRetry(view, dispatch)
        view.ankiRecovery?.let { AnkiRecoveryPanel(it, dispatch) }
        val typography = view.readerTypography
        val typographyEnabled =
            typography != null &&
                !view.storageBusy &&
                !view.readingTimeSavePending &&
                !view.completionSavePending
        Text(
            stringResource(R.string.reader_typography_title),
            style = MaterialTheme.typography.titleMedium,
        )
        Text(stringResource(R.string.reader_typography_description))
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
                val enabled =
                    !view.storageBusy &&
                        !view.readingTimeSavePending &&
                        !view.completionSavePending &&
                        view.pageLength != null
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
        TextButton(onClick = { showNotices = true }) {
            Text(stringResource(R.string.sudachi_notices_title))
        }
        TextButton(onClick = { showWordNotices = true }) {
            Text(stringResource(R.string.word_dictionary_notices_title))
        }
        TextButton(onClick = { showSvgNotices = true }) {
            Text(stringResource(R.string.svg_notices_title))
        }
        TextButton(onClick = { showFontNotices = true }) {
            Text(stringResource(R.string.reader_font_notices_title))
        }
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
    Column(Modifier.fillMaxWidth()) {
        Text(label, modifier = Modifier.testTag(tag))
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
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
