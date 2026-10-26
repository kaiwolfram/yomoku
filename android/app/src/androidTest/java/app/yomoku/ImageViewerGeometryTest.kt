package app.yomoku

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.unit.IntSize
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ImageViewerGeometryTest {
    private val viewport = IntSize(400, 600)
    private val source = Size(800f, 400f)

    @Test
    fun fitUsesSourceRatioAndCentersLetterboxingIncludingExtremeImages() {
        assertEquals(
            Rect(0f, 200f, 400f, 400f),
            imageViewerRect(viewport, source, ImageViewerTransform()),
        )
        val tall = checkNotNull(imageViewerRect(viewport, Size(1f, 32768f), ImageViewerTransform()))
        assertEquals(600f, tall.height, 0.001f)
        assertEquals(32768f, tall.height / tall.width, 20f) // Float rounding at centered subpixels.
        assertNull(imageViewerRect(IntSize.Zero, source, ImageViewerTransform()))
        assertNull(imageViewerRect(viewport, Size(1f, 0f), ImageViewerTransform()))
        assertNull(imageViewerRect(viewport, Size(Float.NaN, 1f), ImageViewerTransform()))
        val fractional =
            checkNotNull(imageViewerRect(viewport, Size(160.5f, 80.25f), ImageViewerTransform()))
        assertEquals(2f, fractional.width / fractional.height, 0.0001f)
    }

    @Test
    fun zoomKeepsCentroidStableAndPanClampsAtBothEdgesAndResetScale() {
        val centroid = Offset(100f, 300f)
        val zoomed =
            transformImageViewer(
                viewport,
                source,
                ImageViewerTransform(),
                centroid,
                Offset.Zero,
                2f,
            )
        val rect = checkNotNull(imageViewerRect(viewport, source, zoomed))
        assertEquals(2f, zoomed.zoom)
        assertEquals(0.25f, (centroid.x - rect.left) / rect.width, 0.0001f)
        assertEquals(0.5f, (centroid.y - rect.top) / rect.height, 0.0001f)
        val left =
            transformImageViewer(viewport, source, zoomed, centroid, Offset(-10000f, -10000f), 1f)
        assertEquals(Rect(-400f, 100f, 400f, 500f), imageViewerRect(viewport, source, left))
        val right =
            transformImageViewer(viewport, source, left, centroid, Offset(10000f, 10000f), 1f)
        assertEquals(Rect(0f, 100f, 800f, 500f), imageViewerRect(viewport, source, right))
        val bottom =
            transformImageViewer(
                viewport,
                source,
                ImageViewerTransform(),
                centroid,
                Offset(-10000f, -10000f),
                4f,
            )
        assertEquals(Rect(-1200f, -200f, 400f, 600f), imageViewerRect(viewport, source, bottom))
        val small = transformImageViewer(viewport, source, left, centroid, Offset.Zero, 0.001f)
        assertEquals(ImageViewerTransform(), small)
        val huge =
            transformImageViewer(viewport, source, small, centroid, Offset.Zero, Float.MAX_VALUE)
        assertEquals(8f, huge.zoom)
        assertEquals(
            huge,
            transformImageViewer(viewport, source, huge, centroid, Offset.Zero, Float.NaN),
        )
        assertEquals(
            huge,
            transformImageViewer(viewport, source, huge, centroid, Offset(Float.NaN, 0f), 1f),
        )
    }

    @Test
    fun normalizedFocusRetainsSourceLocationAcrossRotationAndViewportChanges() {
        val source = Size(800f, 800f)
        val transform = ImageViewerTransform(4f, 0.6f, 0.4f)
        for (size in
            listOf(
                IntSize(400, 600),
                IntSize(600, 400),
                IntSize(1600, 1000),
                IntSize(1000, 1600),
            )) {
            val rect = checkNotNull(imageViewerRect(size, source, transform))
            assertEquals(0.6f, (size.width / 2f - rect.left) / rect.width, 0.0001f)
            assertEquals(0.4f, (size.height / 2f - rect.top) / rect.height, 0.0001f)
            assertEquals(1f, rect.width / rect.height, 0.0001f)
        }
    }
}
