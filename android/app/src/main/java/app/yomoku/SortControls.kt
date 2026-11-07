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
    var expanded by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box {
            TextButton(
                onClick = { expanded = true },
                enabled = enabled,
                contentPadding = PaddingValues(start = 12.dp, end = 0.dp),
                modifier = Modifier.testTag("${tag}_sort"),
            ) {
                Text(
                    stringResource(R.string.library_sort, stringResource(labels.getValue(baseSort)))
                )
            }
            DropdownMenu(expanded = expanded && enabled, onDismissRequest = { expanded = false }) {
                labels.forEach { (option, label) ->
                    DropdownMenuItem(
                        text = { Text(stringResource(label)) },
                        onClick = {
                            expanded = false
                            onSort(option)
                        },
                        modifier = Modifier.testTag("${tag}_sort_${option.name}"),
                    )
                }
            }
        }
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
