package app.yomoku

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp

@Composable
internal fun LibraryCover(id: Long, loader: EpubCoverLoader?) {
    var image by remember(id, loader) { mutableStateOf<DecodedEpubImage?>(null) }
    DisposableEffect(id, loader) {
        val request = loader?.load(id) { image = it }
        onDispose { request?.close() }
    }
    Box(Modifier.size(48.dp, 64.dp), contentAlignment = Alignment.Center) {
        val decoded = image
        if (decoded == null) {
            Icon(
                painterResource(R.drawable.ic_document),
                contentDescription = null,
                modifier = Modifier.size(32.dp).testTag("library_cover_fallback_$id"),
            )
        } else {
            val ratio = decoded.sourceSize.width / decoded.sourceSize.height
            val width = minOf(48f, 64f * ratio)
            Image(
                decoded.bitmap.asImageBitmap(),
                contentDescription = null,
                contentScale = ContentScale.FillBounds,
                modifier = Modifier.size(width.dp, (width / ratio).dp).testTag("library_cover_$id"),
            )
        }
    }
}
