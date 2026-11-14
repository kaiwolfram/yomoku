package app.yomoku

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*

@Composable
internal fun ReaderTypographyControls(view: ViewModel, dispatch: (Event) -> Unit) {
    val typography = view.readerTypography
    val typographyEnabled =
        typography != null &&
            !view.storageBusy &&
            !view.completionSavePending &&
            view.effectiveReaderStatus?.let { !it.busy && !it.pending } != false
    TypographyControl(
        label = stringResource(R.string.reader_font_size, typography?.fontSize?.spValue ?: 16),
        tag = "reader_font_size",
        index = typography?.fontSize?.ordinal ?: 1,
        count = ReaderFontSize.entries.size,
        enabled = typographyEnabled,
    ) { index ->
        typography?.let {
            dispatch(Event.SetReaderTypography(it.copy(fontSize = ReaderFontSize.entries[index])))
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
                Event.SetReaderTypography(it.copy(lineSpacing = ReaderLineSpacing.entries[index]))
            )
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
internal fun ReaderTypographyMenu(view: ViewModel, dispatch: (Event) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        IconButton(
            onClick = { expanded = true },
            modifier = Modifier.testTag("reader_settings"),
        ) {
            Icon(
                painterResource(R.drawable.ic_settings),
                contentDescription = stringResource(R.string.reader_typography_settings),
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            Column(
                Modifier.width(360.dp).padding(16.dp).testTag("reader_typography_popup"),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    stringResource(R.string.settings_reader),
                    style = MaterialTheme.typography.titleMedium,
                )
                StorageStatus(view)
                ReaderTypographyControls(view, dispatch)
            }
        }
    }
}
