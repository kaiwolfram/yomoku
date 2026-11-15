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
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Assume.assumeFalse
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TestName
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ImportNotificationsTest {
    @get:Rule val testName = TestName()
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
        if (
            testName.methodName ==
                "deniedNotificationsKeepImportDialogWithoutPermissionActionOrPrompt"
        ) {
            assumeFalse(
                "Run this case with POST_NOTIFICATIONS revoked.",
                manager.areNotificationsEnabled(),
            )
        } else if (Build.VERSION.SDK_INT >= 33) {
            instrumentation.uiAutomation.grantRuntimePermission(
                context.packageName,
                Manifest.permission.POST_NOTIFICATIONS,
            )
        }
        manager.cancelAll()
        compose.waitUntil(30_000) {
            val view = owner().session.view
            !view.storageBusy &&
                (view.libraryWordDistributions != null || view.libraryWordError != null)
        }
    }

    @Test
    fun deniedNotificationsKeepImportDialogWithoutPermissionActionOrPrompt() {
        val imports = owner().fileImports
        assertFalse(manager.areNotificationsEnabled())
        val uri =
            fixture("denied-${System.nanoTime()}.srt", byteArrayOf(0xc2.toByte(), 0xa9.toByte()))
        compose.runOnIdle { imports.start(uri) }
        compose.waitUntil(30_000) { imports.choosingEncoding }
        compose.onNodeWithText("Enable import notifications").assertDoesNotExist()
        compose
            .onNodeWithText(
                "Progress and completion notifications are unavailable. You can follow the import here."
            )
            .assertDoesNotExist()
        compose.onNodeWithText("Cancel").assertIsDisplayed()
        assertNull(notice(ImportNotifications.WAIT_ID))
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        compose.onNodeWithText("Cancel").assertIsDisplayed()
        compose.onNodeWithText("Enable import notifications").assertDoesNotExist()
        compose.runOnIdle { imports.cancel() }
        compose.waitUntil(30_000) { !imports.active && !owner().session.view.storageBusy }
        assertNull(notice(ImportNotifications.COMPLETE_ID))
    }

    @Test
    fun backgroundImportUsesIndeterminateProgressAndCompletionAndTapKeepsReaderClosed() {
        val prefix = "notifications-${System.nanoTime()}"
        val blocked =
            FileImportFixtures.uri("$prefix.epub", FileImportFixtures.epub(prefix), prefix)
        val imports = owner().fileImports
        try {
            compose.runOnIdle { imports.start(blocked) }
            compose.waitUntil(90_000) {
                notice(ImportNotifications.PROGRESS_ID)
                    ?.extras
                    ?.getBoolean(Notification.EXTRA_PROGRESS_INDETERMINATE) == true &&
                    notice(ImportNotifications.PROGRESS_ID)
                        ?.extras
                        ?.getString(Notification.EXTRA_TEXT)
                        ?.contains("$prefix.epub") == true
            }
            compose.onNodeWithText("Enable import notifications").assertDoesNotExist()
            compose
                .onNodeWithText(
                    "Progress and completion notifications are available while you use another app."
                )
                .assertDoesNotExist()
            compose.onNodeWithText("Importing book...").assertIsDisplayed()
            compose.onNodeWithText("Import book").assertDoesNotExist()
            assertTrue(foregroundService())
            val progress = checkNotNull(notice(ImportNotifications.PROGRESS_ID))
            assertEquals("Importing book...", progress.extras.getString(Notification.EXTRA_TITLE))
            assertTrue(
                progress.extras.getString(Notification.EXTRA_TEXT)!!.contains("$prefix.epub")
            )
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
            assertEquals(app.yomoku.shared.ImportOutcome.CREATED, imports.result?.outcome)
            assertNull(notice(ImportNotifications.PROGRESS_ID))
            val completed = checkNotNull(notice(ImportNotifications.COMPLETE_ID))
            assertEquals("Import complete", completed.extras.getString(Notification.EXTRA_TITLE))
            assertEquals("Imported: $prefix", completed.extras.getString(Notification.EXTRA_TEXT))
            completed.contentIntent.send()
            compose.waitUntil(30_000) {
                compose.activity.lifecycle.currentState == Lifecycle.State.RESUMED
            }
            assertNull(owner().session.view.reader)
            assertEquals(imports.result, owner().fileImports.result)
            compose.runOnIdle { imports.dismissSummary() }
        } finally {
            context.contentResolver.call(blocked, "release", prefix, null)
        }
    }

    @Test
    fun visibleCompletionAndBackgroundReturnSuppressResultsIncludingRecreationGap() {
        val retained = owner()
        val observer =
            ImportNotifications(context, isActivityVisible = { retained.activityVisible }) {}
        val result = FileImportResult("book.txt", app.yomoku.shared.ImportOutcome.CREATED, "Book")
        try {
            compose.runOnIdle { observer.finished(result) }
            assertNull(notice(ImportNotifications.COMPLETE_ID))
            // Finish in the old Activity's stopped callback, before the replacement starts.
            var stoppedDuringRecreation = false
            compose.runOnIdle {
                val activity = compose.activity
                activity.lifecycle.addObserver(
                    LifecycleEventObserver { _, event ->
                        if (event == Lifecycle.Event.ON_STOP && activity.isChangingConfigurations) {
                            stoppedDuringRecreation = true
                            assertTrue(retained.activityVisible)
                            observer.finished(result)
                        }
                    }
                )
            }
            compose.activityRule.scenario.recreate()
            assertTrue(stoppedDuringRecreation)
            assertSame(retained, owner())
            assertNull(notice(ImportNotifications.COMPLETE_ID))
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            instrumentation.runOnMainSync {
                assertFalse(retained.activityVisible)
                observer.finished(result)
            }
            compose.waitUntil(30_000) { notice(ImportNotifications.COMPLETE_ID) != null }
            manager.cancel(ImportNotifications.COMPLETE_ID)
            compose.waitUntil(30_000) { notice(ImportNotifications.COMPLETE_ID) == null }
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            compose.runOnIdle { observer.finished(result) }
            assertNull(notice(ImportNotifications.COMPLETE_ID))
        } finally {
            instrumentation.runOnMainSync { observer.close() }
        }
    }

    @Test
    fun foregroundReturnKeepsConciseDialogAndDoesNotPostCompletion() {
        val prefix = "visible-${System.nanoTime()}"
        val uri = FileImportFixtures.uri("$prefix.epub", FileImportFixtures.epub(prefix), prefix)
        val imports = owner().fileImports
        try {
            compose.runOnIdle { imports.start(uri) }
            compose.waitUntil(30_000) { notice(ImportNotifications.PROGRESS_ID) != null }
            compose.onNodeWithText("$prefix.epub").assertIsDisplayed()
            compose
                .onNodeWithText(
                    "Android may stop an import in the background. If Android ends the app process, committed books are kept but an unfinished import will not resume."
                )
                .assertDoesNotExist()
            compose.onNodeWithText("Cancel").assertIsDisplayed()
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            assertTrue(foregroundService())
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            context.contentResolver.call(uri, "release", prefix, null)
            compose.waitUntil(90_000) { !imports.active }
            assertEquals(app.yomoku.shared.ImportOutcome.CREATED, imports.result?.outcome)
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            assertNull(notice(ImportNotifications.COMPLETE_ID))
        } finally {
            context.contentResolver.call(uri, "release", prefix, null)
        }
    }

    @Test
    fun terminalNotificationsDescribeTheSingleOutcome() {
        val observer = ImportNotifications(context, isActivityVisible = { false }) {}
        try {
            val outcomes =
                listOf(
                    FileImportResult(
                        "book.epub",
                        app.yomoku.shared.ImportOutcome.CREATED,
                        "Book",
                    ) to "Imported: Book",
                    FileImportResult(
                        "duplicate.epub",
                        app.yomoku.shared.ImportOutcome.EXISTING,
                        "Book",
                    ) to "Already in Library: Book",
                    FileImportResult("unreadable.txt", message = R.string.import_file_unreadable) to
                        context.getString(R.string.import_file_unreadable),
                    FileImportResult("large.epub", message = R.string.import_file_too_large) to
                        context.getString(R.string.import_file_too_large),
                )
            for ((result, text) in outcomes) {
                compose.runOnIdle { observer.finished(result) }
                compose.waitUntil(30_000) {
                    notice(ImportNotifications.COMPLETE_ID)
                        ?.extras
                        ?.getString(Notification.EXTRA_TEXT) == text
                }
                val completed = checkNotNull(notice(ImportNotifications.COMPLETE_ID))
                assertEquals(
                    "Import complete",
                    completed.extras.getString(Notification.EXTRA_TITLE),
                )
                assertEquals(text, completed.extras.getString(Notification.EXTRA_TEXT))
            }
        } finally {
            compose.runOnIdle { observer.close() }
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
            compose.runOnIdle { imports.start(ambiguous) }
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
            // Cancellation refreshes Library; wait for its queued distribution read too.
            compose.waitUntil(30_000) {
                val view = owner().session.view
                !imports.active &&
                    !view.storageBusy &&
                    view.libraryWordDistributions != null &&
                    notice(ImportNotifications.WAIT_ID) == null
            }
            assertNull(notice(ImportNotifications.WAIT_ID))
            assertNull(notice(ImportNotifications.COMPLETE_ID))
            compose.runOnIdle {
                imports.start(blocked)
                assertTrue(imports.active)
            }
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
            assertNull(imports.result)
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
