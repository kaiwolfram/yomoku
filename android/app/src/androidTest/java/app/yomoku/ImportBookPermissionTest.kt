package app.yomoku

import android.Manifest
import android.accessibilityservice.AccessibilityServiceInfo
import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.os.Build
import android.view.accessibility.AccessibilityNodeInfo
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Run grant/deny/blocked separately with permission reset before instrumentation. */
@RunWith(AndroidJUnit4::class)
class ImportBookPermissionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun owner() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java]

    @Test
    fun permissionResultOpensOnePickerAcrossRecreationAndCancellationKeepsLibrary() {
        val automation = instrumentation.uiAutomation
        automation.serviceInfo =
            automation.serviceInfo.apply {
                flags = flags or AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS
            }
        val mode =
            InstrumentationRegistry.getArguments().getString("notificationChoice") ?: "granted"
        if (Build.VERSION.SDK_INT >= 33 && mode == "granted") {
            instrumentation.uiAutomation.grantRuntimePermission(
                instrumentation.targetContext.packageName,
                Manifest.permission.POST_NOTIFICATIONS,
            )
        }
        compose.waitUntil(30_000) {
            val view = owner().session.view
            !view.storageBusy && view.libraryWordDistributions != null
        }
        val before = owner().session.view.documents
        val request = owner().importBookRequest
        val monitor =
            instrumentation.addMonitor(
                IntentFilter(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    addDataType("*/*")
                },
                ActivityResult(Activity.RESULT_CANCELED, null),
                true,
            )
        try {
            compose.onNodeWithTag("library_add").performClick()
            compose.onNodeWithText("Import book").performClick()
            if (Build.VERSION.SDK_INT >= 33 && mode in listOf("grant", "deny")) {
                compose.waitUntil(10_000) { request.step == ImportBookRequest.Step.PERMISSION }
                assertEquals(0, monitor.hits)
                compose.runOnUiThread { assertNull(request.begin(true)) }
                // ActivityScenario.recreate waits for RESUMED, but the system prompt
                // keeps this activity paused. Recreate directly under the prompt.
                val previousActivity = compose.activity
                compose.runOnUiThread { previousActivity.recreate() }
                instrumentation.waitForIdleSync()
                compose.waitUntil(10_000) {
                    runCatching { compose.activity }.getOrNull()?.let { it !== previousActivity } ==
                        true
                }
                assertSame(request, owner().importBookRequest)
                val id =
                    if (mode == "grant") "permission_allow_button" else "permission_deny_button"
                var button: AccessibilityNodeInfo? = null
                compose.waitUntil(10_000) {
                    button =
                        instrumentation.uiAutomation.rootInActiveWindow
                            ?.findAccessibilityNodeInfosByViewId(
                                "com.android.permissioncontroller:id/$id"
                            )
                            ?.firstOrNull()
                    button != null
                }
                assertTrue(button!!.performAction(AccessibilityNodeInfo.ACTION_CLICK))
            }
            compose.waitUntil(15_000) { monitor.hits == 1 && request.step == null }
            compose.waitForIdle()
            assertEquals(1, monitor.hits)
            assertFalse(owner().fileImports.active)
            assertNull(owner().fileImports.result)
            assertEquals(before, owner().session.view.documents)
            assertNull(owner().session.view.reader)
            if (Build.VERSION.SDK_INT >= 33) {
                val granted =
                    instrumentation.targetContext.checkSelfPermission(
                        Manifest.permission.POST_NOTIFICATIONS
                    ) == PackageManager.PERMISSION_GRANTED
                assertEquals(mode in listOf("grant", "granted"), granted)
            }
            compose.activityRule.scenario.recreate()
            assertNull(owner().importBookRequest.step)
            assertEquals(1, monitor.hits)
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }
}
