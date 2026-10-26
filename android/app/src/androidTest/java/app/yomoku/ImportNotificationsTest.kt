package app.yomoku

import android.Manifest
import android.app.ActivityManager
import android.app.Notification
import android.app.NotificationManager
import android.content.ComponentName
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.ServiceInfo
import android.net.Uri
import android.os.Build
import android.os.IBinder
import android.util.Base64
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ImportNotificationsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val manager = context.getSystemService(NotificationManager::class.java)

    private fun owner() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java]

    private fun notice(id: Int): Notification? =
        manager.activeNotifications.singleOrNull { it.id == id }?.notification

    @Suppress("DEPRECATION")
    private fun foregroundService(): Boolean =
        context.getSystemService(ActivityManager::class.java).getRunningServices(100).any {
            it.service.className == ImportService::class.java.name && it.foreground
        }

    private fun fixture(name: String, bytes: ByteArray, gate: String? = null): Uri =
        Uri.Builder()
            .scheme("content")
            .authority("app.yomoku.test.txt-fixtures")
            .appendPath(name)
            .appendQueryParameter("bytes", Base64.encodeToString(bytes, Base64.NO_WRAP))
            .apply { if (gate != null) appendQueryParameter("gate", gate) }
            .build()

    @Before
    fun prepare() {
        if (Build.VERSION.SDK_INT >= 33)
            instrumentation.uiAutomation.grantRuntimePermission(
                context.packageName,
                Manifest.permission.POST_NOTIFICATIONS,
            )
        manager.cancelAll()
        compose.waitUntil(30_000) { !owner().session.view.storageBusy }
    }

    @Test
    fun backgroundBatchUpdatesProgressAndCompletionAndTapKeepsReaderClosed() {
        val prefix = "notifications-${System.nanoTime()}"
        val first = FileImportFixtures.uri("$prefix-first.epub", FileImportFixtures.epub(prefix))
        val blocked = fixture("$prefix-second.srt", "\uFEFF$prefix second 猫。".toByteArray(), prefix)
        val third = fixture("$prefix-third.txt", "\uFEFF$prefix third 猫。".toByteArray())
        val imports = owner().fileImports
        try {
            compose.runOnIdle { imports.start(listOf(first, blocked, third)) }
            compose.waitUntil(90_000) {
                imports.results.size == 1 &&
                    notice(ImportNotifications.PROGRESS_ID)
                        ?.extras
                        ?.getInt(Notification.EXTRA_PROGRESS) == 1
            }
            assertTrue(foregroundService())
            val progress = checkNotNull(notice(ImportNotifications.PROGRESS_ID))
            assertEquals("File 2 of 3", progress.extras.getString(Notification.EXTRA_TITLE))
            assertTrue(progress.extras.getString(Notification.EXTRA_TEXT)!!.contains("second.srt"))
            assertEquals(3, progress.extras.getInt(Notification.EXTRA_PROGRESS_MAX))
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            assertTrue(foregroundService())
            context.contentResolver.call(blocked, "release", prefix, null)
            compose.waitUntil(90_000) {
                !imports.active &&
                    notice(ImportNotifications.COMPLETE_ID) != null &&
                    !foregroundService()
            }
            assertEquals(3, imports.results.size)
            assertNull(notice(ImportNotifications.PROGRESS_ID))
            val completed = checkNotNull(notice(ImportNotifications.COMPLETE_ID))
            assertEquals("Import complete", completed.extras.getString(Notification.EXTRA_TITLE))
            assertEquals("Processed: 3 of 3", completed.extras.getString(Notification.EXTRA_TEXT))
            completed.contentIntent.send()
            compose.waitUntil(30_000) {
                compose.activity.lifecycle.currentState == Lifecycle.State.RESUMED
            }
            assertNull(owner().session.view.reader)
            assertEquals(3, owner().fileImports.results.size)
            compose.runOnIdle { imports.dismissSummary() }
        } finally {
            context.contentResolver.call(blocked, "release", prefix, null)
        }
    }

    @Test
    fun encodingReleasesLeaseAndTimeoutCancelsUnfinishedReadWithoutRestart() {
        val prefix = "notification-timeout-${System.nanoTime()}"
        val ambiguous = fixture("$prefix-choice.srt", byteArrayOf(0xc2.toByte(), 0xa9.toByte()))
        val blocked =
            FileImportFixtures.uri("$prefix-blocked.epub", FileImportFixtures.epub(prefix), prefix)
        val imports = owner().fileImports
        var boundService: ImportService? = null
        var bound = false
        val connection =
            object : ServiceConnection {
                override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                    boundService = (binder as ImportService.LocalBinder).service
                }

                override fun onServiceDisconnected(name: ComponentName) {}
            }
        try {
            compose.runOnIdle { imports.start(listOf(ambiguous)) }
            compose.waitUntil(30_000) {
                imports.choosingEncoding &&
                    notice(ImportNotifications.WAIT_ID) != null &&
                    !foregroundService()
            }
            assertEquals(
                "Choose an encoding in Yomoku",
                notice(ImportNotifications.WAIT_ID)!!.extras.getString(Notification.EXTRA_TITLE),
            )
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            compose.runOnIdle { imports.cancel() }
            compose.waitUntil(30_000) {
                !imports.active &&
                    !owner().session.view.storageBusy &&
                    notice(ImportNotifications.COMPLETE_ID) != null &&
                    notice(ImportNotifications.WAIT_ID) == null
            }
            assertNull(notice(ImportNotifications.WAIT_ID))
            assertEquals(
                "Import cancelled",
                notice(ImportNotifications.COMPLETE_ID)!!
                    .extras
                    .getString(Notification.EXTRA_TITLE),
            )
            compose.runOnIdle { imports.start(listOf(blocked, ambiguous)) }
            compose.waitUntil(30_000) { foregroundService() }
            compose.runOnIdle {
                bound =
                    context.bindService(
                        Intent(context, ImportService::class.java),
                        connection,
                        0,
                    )
            }
            assertTrue(bound)
            compose.waitUntil(30_000) { boundService != null }
            compose.runOnIdle {
                checkNotNull(boundService)
                    .onTimeout(1, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
            }
            compose.waitUntil(30_000) {
                !imports.active &&
                    !foregroundService() &&
                    notice(ImportNotifications.PROGRESS_ID) == null
            }
            assertTrue(imports.results.isEmpty())
            assertFalse(owner().session.view.documents.any { it.title.startsWith(prefix) })
            assertNull(notice(ImportNotifications.PROGRESS_ID))
            compose.runOnIdle { imports.refreshNotifications() }
            assertFalse(foregroundService())
        } finally {
            if (bound) compose.runOnIdle { context.unbindService(connection) }
            context.contentResolver.call(blocked, "release", prefix, null)
        }
    }
}
