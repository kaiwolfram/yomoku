package app.yomoku

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.CompletionCommandId
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel
import kotlin.math.sin

@Composable
internal fun BookCompletionCelebration(view: ViewModel, dispatch: (Event) -> Unit) {
    var playing by remember { mutableStateOf<CompletionCommandId?>(null) }
    val progress = remember { Animatable(0f) }
    val notice = view.bookCompletionNotice
    LaunchedEffect(notice) {
        if (notice != null) {
            // Consume before displaying. Retained core state prevents recreation
            // from replaying a celebration; animation state is deliberately local.
            dispatch(Event.DismissBookCompletionNotice(notice))
            playing = notice
        }
    }
    LaunchedEffect(playing) {
        if (playing != null) {
            progress.snapTo(0f)
            progress.animateTo(1f, tween(1_400, easing = LinearEasing))
            playing = null
        }
    }
    if (playing != null) {
        val description = stringResource(R.string.book_completed)
        val colors =
            listOf(
                MaterialTheme.colorScheme.primary,
                MaterialTheme.colorScheme.tertiary,
                MaterialTheme.colorScheme.secondary,
            )
        Canvas(
            Modifier.fillMaxSize().testTag("book_confetti").semantics {
                contentDescription = description
            }
        ) {
            val fraction = progress.value
            repeat(64) { index ->
                val x =
                    size.width * ((index * 37 % 101) / 100f) +
                        sin(fraction * 8f + index) * 24.dp.toPx()
                val y = size.height * (-0.25f + fraction * (1.2f + (index % 5) * 0.12f))
                val center = Offset(x, y)
                rotate(index * 17f + fraction * 300f, center) {
                    drawRect(
                        colors[index % colors.size],
                        center,
                        Size(6.dp.toPx(), 11.dp.toPx()),
                        alpha = (1f - fraction).coerceIn(0f, 1f),
                    )
                }
            }
        }
    }
}
