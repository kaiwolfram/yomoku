package app.yomoku

import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderStatusBordersTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun selectedSegmentsKeepStraightInteriorEdgesAndOuterPillCorners() {
        val status = mutableStateOf(VocabularyStatus.NEW)
        compose.setContent {
            YomokuTheme {
                ReaderWordPanel(
                    ReaderWordHeader(
                        "猫",
                        null,
                        null,
                        ReaderVocabulary(VocabularyId(1), status.value),
                    ),
                    enabled = true,
                    onClose = {},
                    onStatus = { status.value = it },
                    details = {},
                )
            }
        }
        for (value in VocabularyStatus.entries) {
            compose.runOnUiThread { status.value = value }
            compose.onNodeWithTag("word_status_$value").assertIsSelected()
            val bitmap =
                compose.onNodeWithTag("word_status_pill").captureToImage().asAndroidBitmap()
            File(compose.activity.cacheDir, "status-$value.png").outputStream().use {
                bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
            }
            // Both inner border corners meet the top edge, while the outer pill stays clipped.
            val segment = compose.onNodeWithTag("word_status_$value").getUnclippedBoundsInRoot()
            assertTrue((segment.right - segment.left).value > 0)
        }
    }
}
