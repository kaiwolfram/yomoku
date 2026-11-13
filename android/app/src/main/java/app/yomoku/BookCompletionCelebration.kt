package app.yomoku

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.SemanticsPropertyKey
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.yomoku.shared.CompletionCommandId
import app.yomoku.shared.Event
import app.yomoku.shared.ViewModel
import kotlin.math.cos
import kotlin.math.sin
import kotlin.random.Random

internal val ConfettiOrigin = SemanticsPropertyKey<Offset>("ConfettiOrigin")

private data class ConfettiPiece(
    val velocity: Offset,
    val size: Size,
    val rotation: Float,
    val spin: Float,
    val color: Color,
)

private data class Celebration(val command: CompletionCommandId, val origin: Offset)

@Composable
internal fun BookCompletionCelebration(
    view: ViewModel,
    origin: Offset?,
    onOriginConsumed: () -> Unit,
    dispatch: (Event) -> Unit,
) {
    var playing by remember { mutableStateOf<Celebration?>(null) }
    val progress = remember { Animatable(0f) }
    val notice = view.bookCompletionNotice
    val pieces = remember {
        val random = Random(71)
        val colors = listOf(0xFFFF5577, 0xFFFFCA45, 0xFF47CFA6, 0xFF5599FF, 0xFFB877EF)
        List(90) {
            val angle = (-150f + random.nextFloat() * 120f) * Math.PI.toFloat() / 180f
            val speed = 420f + random.nextFloat() * 520f
            ConfettiPiece(
                Offset(cos(angle) * speed, sin(angle) * speed),
                Size(4f + random.nextFloat() * 5f, 7f + random.nextFloat() * 7f),
                random.nextFloat() * 360f,
                -500f + random.nextFloat() * 1000f,
                Color(colors[random.nextInt(colors.size)]),
            )
        }
    }
    LaunchedEffect(notice) {
        if (notice != null) {
            // Consume even if recreation lost the local button position. Never guess an origin.
            dispatch(Event.DismissBookCompletionNotice(notice))
            playing = origin?.let { Celebration(notice, it) }
            onOriginConsumed()
        }
    }
    LaunchedEffect(playing) {
        if (playing != null) {
            progress.snapTo(0f)
            progress.animateTo(1f, tween(1_600, easing = LinearEasing))
            playing = null
        }
    }
    playing?.let { celebration ->
        val start = celebration.origin
        val description = stringResource(R.string.book_completed)
        // Canvas has no input handlers: Library remains usable during the celebration.
        Canvas(
            Modifier.fillMaxSize().testTag("book_confetti").semantics {
                contentDescription = description
                this[ConfettiOrigin] = start
            }
        ) {
            val fraction = progress.value
            val seconds = fraction * 1.6f
            val scale = 1.dp.toPx()
            val alpha = ((1f - fraction) / 0.25f).coerceIn(0f, 1f)
            pieces.forEach { piece ->
                val center =
                    start +
                        Offset(
                            piece.velocity.x * seconds,
                            piece.velocity.y * seconds + 0.5f * 900f * seconds * seconds,
                        ) * scale
                val dimensions = Size(piece.size.width * scale, piece.size.height * scale)
                rotate(piece.rotation + piece.spin * seconds, center) {
                    drawRect(
                        piece.color,
                        center - Offset(dimensions.width / 2, dimensions.height / 2),
                        dimensions,
                        alpha = alpha,
                    )
                }
            }
        }
    }
}
