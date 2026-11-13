package app.yomoku

import android.graphics.Bitmap
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File

/** Optional full-window captures for the phone/tablet UI trial, including popup windows. */
internal fun captureUiTrial(compose: ComposeContentTestRule, name: String) {
    val instrumentation = InstrumentationRegistry.getInstrumentation()
    if (InstrumentationRegistry.getArguments().getString("captureUi") != "true") return
    compose.waitForIdle()
    // Let SurfaceFlinger present the last test-clock frame before capturing popup windows.
    instrumentation.waitForIdleSync()
    Thread.sleep(150)
    val screenshot = checkNotNull(instrumentation.uiAutomation.takeScreenshot())
    File(instrumentation.targetContext.cacheDir, "ui-$name.png").outputStream().use {
        screenshot.compress(Bitmap.CompressFormat.PNG, 100, it)
    }
    screenshot.recycle()
}
