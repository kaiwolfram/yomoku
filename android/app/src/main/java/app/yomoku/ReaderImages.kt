package app.yomoku

import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.ViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.DpSize
import app.yomoku.shared.ReaderInlineImage
import app.yomoku.shared.ReaderPage

internal sealed interface ReaderImagePart {
    data class Text(val start: Int, val end: Int) : ReaderImagePart

    data class Image(val index: Int) : ReaderImagePart
}

/** Inline links must not expand over adjacent inert text or another word's glyphs. */
@Composable
private fun ReaderTextBlock(text: AnnotatedString, modifier: Modifier) {
    val original = LocalViewConfiguration.current
    val configuration =
        remember(original) {
            object : ViewConfiguration by original {
                override val minimumTouchTargetSize = DpSize.Zero
            }
        }
    CompositionLocalProvider(LocalViewConfiguration provides configuration) {
        Text(text, style = MaterialTheme.typography.bodyLarge, modifier = modifier)
    }
}

/** Keep canonical text/anchors intact; only replace verified whole placeholder scalars. */
internal fun readerImageParts(
    text: String,
    images: List<ReaderInlineImage>,
): List<ReaderImagePart>? {
    val boundaries = utf8Boundaries(text) ?: return null
    var previous = 0
    return buildList {
        for ((index, image) in images.withIndex()) {
            val start = boundaries[image.range.start.value] ?: return null
            val end = boundaries[image.range.end.value] ?: return null
            if (start < previous || end != start + 1 || text.substring(start, end) != "\uFFFC")
                return null
            if (previous < start) add(ReaderImagePart.Text(previous, start))
            add(ReaderImagePart.Image(index))
            previous = end
        }
        if (previous < text.length) add(ReaderImagePart.Text(previous, text.length))
    }
}

@Composable
internal fun ReaderImageContent(
    page: ReaderPage,
    text: AnnotatedString,
    loader: EpubImageLoader?,
    onInteraction: () -> Unit,
) {
    val parts = remember(page.text, page.images) { readerImageParts(page.text, page.images) }
    if (parts == null || page.images.isEmpty()) {
        ReaderTextBlock(
            text,
            modifier = Modifier.testTag("reader_text"),
        )
        return
    }
    val assets = remember(page.images) { page.images.map { it.asset }.distinct() }
    val results =
        remember(page.documentId, page.anchor, assets) {
            mutableStateMapOf<app.yomoku.shared.ReaderImageAsset, DecodedEpubImage?>()
        }
    var retry by remember(page.documentId, page.anchor) { mutableIntStateOf(0) }
    var selectedImage by rememberSaveable(page.documentId, page.anchor) { mutableIntStateOf(-1) }
    DisposableEffect(loader, page.documentId, page.anchor, assets, retry) {
        results.clear()
        val requests = assets.map { asset ->
            if (loader == null) {
                results[asset] = null
                null
            } else
                loader.load(
                    EpubImageKey(page.documentId.value, asset),
                    maxOf(1, EPUB_PAGE_PIXELS / assets.size),
                ) {
                    results[asset] = it
                }
        }
        onDispose { requests.forEach { it?.close() } }
    }
    Column(Modifier.fillMaxWidth().testTag("reader_image_content")) {
        for (part in parts) {
            when (part) {
                is ReaderImagePart.Text ->
                    ReaderTextBlock(
                        text.subSequence(part.start, part.end),
                        modifier = Modifier.testTag("reader_text_${part.start}"),
                    )
                is ReaderImagePart.Image -> {
                    val reference = page.images[part.index]
                    val image = results[reference.asset]
                    if (image != null) {
                        val bitmap = remember(image) { image.bitmap.asImageBitmap() }
                        val density = LocalDensity.current
                        BoxWithConstraints(
                            Modifier.fillMaxWidth(),
                            contentAlignment = Alignment.Center,
                        ) {
                            // Extremely tall sources still retain their ratio inside a bounded
                            // frame.
                            val height =
                                with(density) {
                                    minOf(
                                            maxWidth.toPx() * image.sourceSize.height /
                                                image.sourceSize.width,
                                            8192f,
                                        )
                                        .toDp()
                                }
                            val width = height * image.sourceSize.width / image.sourceSize.height
                            Image(
                                bitmap,
                                contentDescription =
                                    reference.alt.ifBlank { stringResource(R.string.reader_image) },
                                contentScale = ContentScale.FillBounds,
                                modifier =
                                    Modifier.width(width)
                                        .height(height)
                                        .clickable(
                                            onClickLabel =
                                                stringResource(R.string.image_viewer_open)
                                        ) {
                                            onInteraction()
                                            selectedImage = part.index
                                        }
                                        .testTag("reader_image_${part.index}"),
                            )
                        }
                    } else {
                        Column(
                            Modifier.fillMaxWidth()
                                .testTag("reader_image_placeholder_${part.index}")
                        ) {
                            if (reference.alt.isNotBlank()) Text(reference.alt)
                            Text(
                                stringResource(
                                    if (results.containsKey(reference.asset))
                                        R.string.reader_image_unavailable
                                    else R.string.reader_image_loading
                                )
                            )
                            if (results.containsKey(reference.asset) && loader != null) {
                                TextButton(
                                    onClick = {
                                        onInteraction()
                                        retry++
                                    }
                                ) {
                                    Text(stringResource(R.string.retry))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    page.images.getOrNull(selectedImage)?.let { reference ->
        val image = results[reference.asset]
        key(page.documentId, page.anchor, selectedImage) {
            ImageViewer(
                bitmap = remember(image) { image?.bitmap?.asImageBitmap() },
                sourceSize = image?.sourceSize ?: Size.Zero,
                description = reference.alt.ifBlank { stringResource(R.string.reader_image) },
                loading = !results.containsKey(reference.asset),
                onRetry = if (loader == null) null else ({ retry++ }),
                onClose = { selectedImage = -1 },
                onInteraction = onInteraction,
            )
        }
    }
}
