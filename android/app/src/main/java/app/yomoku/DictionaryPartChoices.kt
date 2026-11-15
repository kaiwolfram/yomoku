package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.DictionaryLookup

@Composable
internal fun DictionaryPartChoices(
    lookup: DictionaryLookup,
    enabled: Boolean,
    onChoose: (ULong?) -> Unit,
) {
    if (lookup.parts.isEmpty()) return
    var expanded by remember(lookup.whole) { mutableStateOf(false) }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(
            selected = lookup.active == null,
            onClick = { if (lookup.active != null) onChoose(null) },
            enabled = enabled && !lookup.loading,
            label = { Text(stringResource(R.string.word_whole)) },
            modifier = Modifier.testTag("dictionary_whole"),
        )
        for ((index, part) in lookup.parts.withIndex()) {
            if (expanded || part.primary || lookup.active == index.toULong()) {
                FilterChip(
                    selected = lookup.active == index.toULong(),
                    onClick = { if (lookup.active != index.toULong()) onChoose(index.toULong()) },
                    enabled = enabled && !lookup.loading,
                    label = { Text(part.surface) },
                    modifier = Modifier.testTag("dictionary_part_$index"),
                )
            }
        }
        if (!expanded && lookup.parts.any { !it.primary }) {
            TextButton(
                onClick = { expanded = true },
                enabled = enabled && !lookup.loading,
                modifier = Modifier.testTag("dictionary_more_parts"),
            ) {
                Text(stringResource(R.string.word_more_parts))
            }
        }
    }
}
