package app.yomoku

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.ServiceInfo
import android.os.Binder
import android.os.IBinder
import app.yomoku.shared.ImportOutcome

internal data class FileImportProgress(
    val filename: String,
    val choosingEncoding: Boolean,
    val cancelling: Boolean,
)

/** Main-thread platform observer. It never receives bytes, URIs or database work. */
internal interface FileImportObserver : AutoCloseable {
    fun update(progress: FileImportProgress)

    /** A null result means cancellation completed without a committed outcome. */
    fun finished(result: FileImportResult?)
}

/** The lease and callbacks exist only in this process; there is no restartable work. */
internal class ImportNotifications(context: Context, private val interrupt: () -> Unit) :
    FileImportObserver {
    private val context = context.applicationContext
    private val manager = this.context.getSystemService(NotificationManager::class.java)
    private var progress: FileImportProgress? = null
    private var service: ImportService? = null
    private var bound = false
    private var started = false
    private val connection =
        object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                val current = progress ?: return
                service = (binder as ImportService.LocalBinder).service
                service?.interrupted = {
                    releaseService()
                    interrupt()
                }
                service?.show(current)
            }

            override fun onServiceDisconnected(name: ComponentName) {
                service = null
                releaseService()
                if (progress != null) interrupt()
            }
        }

    init {
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL,
                this.context.getString(R.string.import_notifications),
                NotificationManager.IMPORTANCE_LOW,
            )
        )
    }

    override fun update(progress: FileImportProgress) {
        val previous = this.progress
        this.progress = progress
        if (previous == null) manager.cancel(COMPLETE_ID)
        if (progress.choosingEncoding) {
            // Waiting for user input needs no background execution lease.
            releaseService()
            if (manager.areNotificationsEnabled())
                manager.notify(WAIT_ID, progressNotification(context, progress))
            return
        }
        manager.cancel(WAIT_ID)
        if (!started && !progress.cancelling && (previous == null || previous.choosingEncoding)) {
            // Start only for a user-started import/encoding choice. Never restart on a timer,
            // boot, service redelivery, process launch or background notification update.
            try {
                context.startForegroundService(Intent(context, ImportService::class.java))
                started = true
                bound =
                    context.bindService(
                        Intent(context, ImportService::class.java),
                        connection,
                        Context.BIND_AUTO_CREATE,
                    )
                if (!bound) releaseService()
            } catch (_: IllegalStateException) {
                // Android can reject background execution. Persistence remains per-file atomic;
                // no service retry or promise of process survival is made.
                releaseService()
            } catch (_: SecurityException) {
                releaseService()
            }
        }
        service?.show(progress)
        if (!started && manager.areNotificationsEnabled())
            manager.notify(PROGRESS_ID, progressNotification(context, progress))
    }

    override fun finished(result: FileImportResult?) {
        progress = null
        releaseService()
        manager.cancel(PROGRESS_ID)
        manager.cancel(WAIT_ID)
        if (manager.areNotificationsEnabled()) {
            manager.notify(
                COMPLETE_ID,
                notification(context)
                    .setContentTitle(
                        context.getString(
                            if (result == null) R.string.import_cancelled
                            else R.string.import_complete
                        )
                    )
                    .setContentText(
                        when {
                            result == null -> ""
                            result.error != null ->
                                context.getString(result.error.messageResource())
                            result.message != null -> context.getString(result.message)
                            result.outcome == ImportOutcome.EXISTING ->
                                context.getString(
                                    R.string.import_existing,
                                    result.title ?: result.filename,
                                )
                            result.outcome == ImportOutcome.CREATED ->
                                context.getString(
                                    R.string.import_created,
                                    result.title ?: result.filename,
                                )
                            else -> context.getString(R.string.error_storage_unavailable)
                        }
                    )
                    .setAutoCancel(true)
                    .build(),
            )
        }
    }

    private fun releaseService() {
        service?.interrupted = null
        service = null
        if (bound) {
            bound = false
            context.unbindService(connection)
        }
        if (started) {
            started = false
            context.stopService(Intent(context, ImportService::class.java))
        }
    }

    override fun close() {
        progress = null
        releaseService()
        manager.cancel(PROGRESS_ID)
        manager.cancel(WAIT_ID)
    }

    companion object {
        const val CHANNEL = "imports"
        const val PROGRESS_ID = 1001
        const val COMPLETE_ID = 1002
        const val WAIT_ID = 1003
    }
}

/** Android foreground lease only. All import work remains in the existing FIFO/helper. */
class ImportService : Service() {
    internal var interrupted: (() -> Unit)? = null

    internal inner class LocalBinder : Binder() {
        val service: ImportService
            get() = this@ImportService
    }

    override fun onCreate() {
        super.onCreate()
        startForeground(
            ImportNotifications.PROGRESS_ID,
            notification(this)
                .setContentTitle(getString(R.string.import_book))
                .setProgress(0, 0, true)
                .build(),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
        )
    }

    override fun onBind(intent: Intent): IBinder = LocalBinder()

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_NOT_STICKY

    internal fun show(progress: FileImportProgress) {
        getSystemService(NotificationManager::class.java)
            .notify(ImportNotifications.PROGRESS_ID, progressNotification(this, progress))
    }

    override fun onTimeout(startId: Int, fgsType: Int) {
        val callback = interrupted
        interrupted = null
        // Stop within Android's deadline; native cancellation/COMMIT may take longer.
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
        callback?.invoke()
    }
}

private fun notification(context: Context): Notification.Builder =
    Notification.Builder(context, ImportNotifications.CHANNEL)
        .setSmallIcon(R.drawable.ic_library)
        .setCategory(Notification.CATEGORY_PROGRESS)
        .setOnlyAlertOnce(true)
        .setContentIntent(
            PendingIntent.getActivity(
                context,
                0,
                Intent(context, MainActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        )

private fun progressNotification(context: Context, progress: FileImportProgress): Notification {
    val title =
        when {
            progress.cancelling -> context.getString(R.string.import_cancelling)
            progress.choosingEncoding -> context.getString(R.string.import_choose_encoding)
            else -> context.getString(R.string.import_book)
        }
    val detail = progress.filename
    return notification(context)
        .setContentTitle(title)
        .setContentText(detail)
        .setStyle(Notification.BigTextStyle().bigText(detail))
        .setProgress(0, 0, true)
        .setOngoing(!progress.choosingEncoding)
        .build()
}
