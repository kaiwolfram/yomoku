package app.yomoku

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.yomoku.shared.DictionaryLookup

@Composable
internal fun DictionaryPartChoices(
    lookup: DictionaryLookup,
    enabled: Boolean,
    statusWritePending: Boolean = false,
    onChoose: (ULong?) -> Unit,
) {
    if (lookup.parts.isEmpty()) return
    var expanded by remember(lookup.whole) { mutableStateOf(false) }
    val canChoose = enabled && !lookup.loading
    val preserveAppearance = statusWritePending && !lookup.loading
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        DictionaryPartChip(
            selected = lookup.active == null,
            onClick = { if (lookup.active != null) onChoose(null) },
            enabled = canChoose,
            preserveAppearance = preserveAppearance,
            label = stringResource(R.string.word_whole),
            tag = "dictionary_whole",
        )
        for ((index, part) in lookup.parts.withIndex()) {
            if (expanded || part.primary || lookup.active == index.toULong()) {
                DictionaryPartChip(
                    selected = lookup.active == index.toULong(),
                    onClick = { if (lookup.active != index.toULong()) onChoose(index.toULong()) },
                    enabled = canChoose,
                    preserveAppearance = preserveAppearance,
                    label = part.surface,
                    tag = "dictionary_part_$index",
                )
            }
        }
        if (!expanded && lookup.parts.any { !it.primary }) {
            TextButton(
                onClick = { expanded = true },
                enabled = canChoose,
                colors =
                    ButtonDefaults.textButtonColors().let { colors ->
                        if (preserveAppearance)
                            colors.copy(disabledContentColor = colors.contentColor)
                        else colors
                    },
                modifier = Modifier.testTag("dictionary_more_parts"),
            ) {
                Text(stringResource(R.string.word_more_parts))
            }
        }
    }
}

@Composable
private fun DictionaryPartChip(
    selected: Boolean,
    onClick: () -> Unit,
    enabled: Boolean,
    preserveAppearance: Boolean,
    label: String,
    tag: String,
) {
    // Keep disabled semantics and input guards while status writes retain settled lookup controls.
    FilterChip(
        selected = selected,
        onClick = onClick,
        enabled = enabled,
        label = { Text(label) },
        modifier = Modifier.testTag(tag),
        colors =
            FilterChipDefaults.filterChipColors(
                disabledLabelColor =
                    if (preserveAppearance) {
                        if (selected) MaterialTheme.colorScheme.onSecondaryContainer
                        else MaterialTheme.colorScheme.onSurfaceVariant
                    } else Color.Unspecified,
                disabledSelectedContainerColor =
                    if (preserveAppearance) MaterialTheme.colorScheme.secondaryContainer
                    else Color.Unspecified,
            ),
        border = FilterChipDefaults.filterChipBorder(enabled || preserveAppearance, selected),
    )
}

internal val DictionaryLookup.copyText: String
    get() = active?.let { parts[it.toInt()].surface } ?: whole.surface
