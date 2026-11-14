package app.yomoku

import android.app.Activity
import android.os.SystemClock
import android.view.MotionEvent
import androidx.test.platform.app.InstrumentationRegistry

/** Send a window-level outside tap so Popup dismissal is tested through Android input. */
internal fun tapBelowPopup(activity: Activity) {
    val instrumentation = InstrumentationRegistry.getInstrumentation()
    instrumentation.waitForIdleSync()
    var x = 0f
    var y = 0f
    instrumentation.runOnMainSync {
        val decor = activity.window.decorView
        val location = IntArray(2)
        decor.getLocationOnScreen(location)
        x = location[0] + decor.width / 2f
        y = location[1] + decor.height * 0.8f
    }
    val time = SystemClock.uptimeMillis()
    for (action in listOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
        val event = MotionEvent.obtain(time, time + action, action, x, y, 0)
        instrumentation.sendPointerSync(event)
        event.recycle()
    }
}
