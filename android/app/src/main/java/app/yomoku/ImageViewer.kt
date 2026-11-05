package app.yomoku

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlin.math.roundToInt

/** Normalized source focus survives viewport/rotation changes without retaining pixel offsets. */
internal data class ImageViewerTransform(
    val zoom: Float = 1f,
    val focusX: Float = 0.5f,
    val focusY: Float = 0.5f,
) {
    companion object {
        val Saver =
            listSaver<ImageViewerTransform, Float>(
                save = { listOf(it.zoom, it.focusX, it.focusY) },
                restore = { ImageViewerTransform(it[0], it[1], it[2]) },
            )
    }
}

internal fun imageViewerRect(
    viewport: IntSize,
    source: Size,
    transform: ImageViewerTransform,
): Rect? {
    if (
        viewport.width <= 0 ||
            viewport.height <= 0 ||
            !source.width.isFinite() ||
            !source.height.isFinite() ||
            source.width <= 0 ||
            source.height <= 0
    )
        return null
    val fit =
        minOf(viewport.width.toFloat() / source.width, viewport.height.toFloat() / source.height)
    val width = source.width * fit * transform.zoom
    val height = source.height * fit * transform.zoom
    fun start(frame: Int, edge: Float, focus: Float): Float =
        if (edge <= frame) (frame - edge) / 2f
        else (frame / 2f - focus * edge).coerceIn(frame - edge, 0f)
    val left = start(viewport.width, width, transform.focusX)
    val top = start(viewport.height, height, transform.focusY)
    return Rect(left, top, left + width, top + height)
}

/** Zoom about the gesture centroid, then clamp panning so the image cannot be lost offscreen. */
internal fun transformImageViewer(
    viewport: IntSize,
    source: Size,
    transform: ImageViewerTransform,
    centroid: Offset,
    pan: Offset,
    zoomChange: Float,
): ImageViewerTransform {
    if (
        !centroid.x.isFinite() ||
            !centroid.y.isFinite() ||
            !pan.x.isFinite() ||
            !pan.y.isFinite() ||
            !zoomChange.isFinite() ||
            zoomChange <= 0f
    )
        return transform
    val before = imageViewerRect(viewport, source, transform) ?: return transform
    val zoom = (transform.zoom * zoomChange).coerceIn(1f, 8f)
    val ratio = zoom / transform.zoom
    val width = before.width * ratio
    val height = before.height * ratio
    fun focus(frame: Int, edge: Float, start: Float): Float =
        if (edge <= frame) 0.5f else (frame / 2f - start.coerceIn(frame - edge, 0f)) / edge
    return ImageViewerTransform(
        zoom,
        focus(viewport.width, width, centroid.x - (centroid.x - before.left) * ratio + pan.x),
        focus(viewport.height, height, centroid.y - (centroid.y - before.top) * ratio + pan.y),
    )
}

/** Source-agnostic viewer: callers own loading/retries and retain the already bounded bitmap. */
@Composable
internal fun ImageViewer(
    bitmap: ImageBitmap?,
    sourceSize: Size,
    description: String,
    loading: Boolean,
    onRetry: (() -> Unit)?,
    onClose: () -> Unit,
) {
    var transform by
        rememberSaveable(stateSaver = ImageViewerTransform.Saver) {
            mutableStateOf(ImageViewerTransform())
        }
    var viewport by remember { mutableStateOf(IntSize.Zero) }

    val close = {
        onClose()
    }
    val percent = stringResource(R.string.image_viewer_zoom, (transform.zoom * 100).roundToInt())
    val colors =
        ButtonDefaults.textButtonColors(
            contentColor = Color.White,
            disabledContentColor = Color.White.copy(alpha = 0.38f),
        )
    Dialog(
        onDismissRequest = close,
        properties =
            DialogProperties(
                usePlatformDefaultWidth = false,
                decorFitsSystemWindows = false,
                dismissOnClickOutside = false,
            ),
    ) {
        Surface(
            color = Color.Black,
            contentColor = Color.White,
            modifier = Modifier.fillMaxSize().testTag("image_viewer"),
        ) {
            Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    TextButton(
                        onClick = {
                            transform = ImageViewerTransform()
                        },
                        colors = colors,
                        modifier = Modifier.testTag("image_viewer_reset"),
                    ) {
                        Text(stringResource(R.string.image_viewer_reset))
                    }
                    TextButton(
                        onClick = close,
                        colors = colors,
                        modifier = Modifier.testTag("image_viewer_close"),
                    ) {
                        Text(stringResource(R.string.close))
                    }
                }
                Box(
                    Modifier.weight(1f).fillMaxWidth().clipToBounds(),
                    contentAlignment = Alignment.Center,
                ) {
                    if (bitmap != null) {
                        Canvas(
                            Modifier.fillMaxSize()
                                .onSizeChanged { viewport = it }
                                .testTag("image_viewer_canvas")
                                .semantics {
                                    contentDescription = description
                                    role = Role.Image
                                    stateDescription = percent
                                }
                                .pointerInput(sourceSize) {
                                    detectTransformGestures { centroid, pan, zoom, _ ->
                                        transform =
                                            transformImageViewer(
                                                size,
                                                sourceSize,
                                                transform,
                                                centroid,
                                                pan,
                                                zoom,
                                            )
                                    }
                                }
                        ) {
                            imageViewerRect(viewport, sourceSize, transform)?.let { rect ->
                                drawImage(
                                    bitmap,
                                    dstOffset =
                                        IntOffset(rect.left.roundToInt(), rect.top.roundToInt()),
                                    dstSize =
                                        IntSize(
                                            maxOf(1, rect.width.roundToInt()),
                                            maxOf(1, rect.height.roundToInt()),
                                        ),
                                    filterQuality = FilterQuality.High,
                                )
                            }
                        }
                    } else
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text(description)
                            Text(
                                stringResource(
                                    if (loading) R.string.reader_image_loading
                                    else R.string.reader_image_unavailable
                                )
                            )
                            if (!loading && onRetry != null) {
                                TextButton(
                                    onClick = {
                                        onRetry()
                                    },
                                    colors = colors,
                                ) {
                                    Text(stringResource(R.string.retry))
                                }
                            }
                        }
                }
                Row(
                    Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceEvenly,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TextButton(
                        onClick = {
                            transform =
                                transformImageViewer(
                                    viewport,
                                    sourceSize,
                                    transform,
                                    Offset(viewport.width / 2f, viewport.height / 2f),
                                    Offset.Zero,
                                    0.5f,
                                )
                        },
                        colors = colors,
                        enabled = bitmap != null && transform.zoom > 1f,
                        modifier = Modifier.testTag("image_viewer_zoom_out"),
                    ) {
                        Text(stringResource(R.string.image_viewer_zoom_out))
                    }
                    Text(percent, Modifier.testTag("image_viewer_zoom"))
                    TextButton(
                        onClick = {
                            transform =
                                transformImageViewer(
                                    viewport,
                                    sourceSize,
                                    transform,
                                    Offset(viewport.width / 2f, viewport.height / 2f),
                                    Offset.Zero,
                                    2f,
                                )
                        },
                        colors = colors,
                        enabled = bitmap != null && transform.zoom < 8f,
                        modifier = Modifier.testTag("image_viewer_zoom_in"),
                    ) {
                        Text(stringResource(R.string.image_viewer_zoom_in))
                    }
                }
            }
        }
    }
}
