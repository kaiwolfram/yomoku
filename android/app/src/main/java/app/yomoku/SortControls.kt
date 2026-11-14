package app.yomoku

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

@Composable
internal fun <T : Enum<T>> SortControls(
    labels: Map<T, Int>,
    baseSort: T,
    ascending: Boolean,
    tag: String,
    enabled: Boolean = true,
    onReverse: () -> Unit,
    onSort: (T) -> Unit,
) {
    Row(Modifier.testTag("${tag}_sort_controls"), verticalAlignment = Alignment.CenterVertically) {
        SelectionDropdown(
            labels = labels,
            selection = baseSort,
            label =
                stringResource(R.string.library_sort, stringResource(labels.getValue(baseSort))),
            tag = "${tag}_sort",
            enabled = enabled,
            onSelect = onSort,
        )
        IconButton(
            onClick = onReverse,
            enabled = enabled,
            modifier = Modifier.testTag("${tag}_reverse_sort"),
        ) {
            Icon(
                painterResource(
                    if (ascending) R.drawable.ic_arrow_up else R.drawable.ic_arrow_down
                ),
                contentDescription =
                    stringResource(
                        if (ascending) R.string.library_sort_ascending
                        else R.string.library_sort_descending
                    ),
                modifier = Modifier.size(16.dp),
            )
        }
    }
}

/** Shared anchored selector; reverse ordering remains a separate sort action. */
@Composable
internal fun <T : Enum<T>> SelectionDropdown(
    labels: Map<T, Int>,
    selection: T,
    label: String,
    tag: String,
    enabled: Boolean = true,
    onSelect: (T) -> Unit,
) {
    var expanded by remember(enabled) { mutableStateOf(false) }
    Box {
        TextButton(
            onClick = { expanded = true },
            enabled = enabled,
            contentPadding = PaddingValues(start = 12.dp, end = 0.dp),
            modifier = Modifier.testTag(tag),
        ) {
            Text(label)
        }
        DropdownMenu(expanded = expanded && enabled, onDismissRequest = { expanded = false }) {
            labels.forEach { (option, resource) ->
                DropdownMenuItem(
                    text = { Text(stringResource(resource)) },
                    onClick = {
                        expanded = false
                        onSelect(option)
                    },
                    enabled = enabled,
                    modifier =
                        Modifier.testTag("${tag}_${option.name}").semantics {
                            selected = option == selection
                        },
                )
            }
        }
    }
}
