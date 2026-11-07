package app.yomoku

import android.app.ActivityManager
import android.app.NotificationManager
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.Process
import android.os.SystemClock
import android.util.Base64
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.Text
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.ImportOutcome
import java.io.File
import org.json.JSONObject

/** ADB-only experiment with an isolated database and the separately installed test provider. */
class TxtImportRecoveryProbeActivity : ComponentActivity() {
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var directory: File
    private lateinit var phase: String
    private lateinit var core: CoreSession
    private lateinit var imports: FileImports

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val run = checkNotNull(intent.getStringExtra("run"))
        require(run.matches(Regex("[0-9a-f]{32}")))
        phase = checkNotNull(intent.getStringExtra("phase"))
        require(phase in listOf("before", "after"))
        directory = File(noBackupFilesDir, "txt-recovery-$run")
        check(directory.isDirectory || directory.mkdir())
        core = CoreSession(File(directory, "reader.sqlite3").path, sudachiDictionaryProvider(this))
        imports = FileImports(contentResolver, core, ImportNotifications(this) { imports.cancel() })
        setContent { Text("TXT process-death acceptance: $phase") }
        val deadline = SystemClock.elapsedRealtime() + 90_000
        var started = false
        val checkProgress =
            object : Runnable {
                override fun run() {
                    try {
                        check(SystemClock.elapsedRealtime() < deadline) { "Probe timed out" }
                        if (!core.view.storageBusy && !started) {
                            started = true
                            if (phase == "before") {
                                check(core.view.documents.isEmpty())
                                imports.start(fixture(run, "finished"))
                            } else {
                                val before = JSONObject(File(directory, "before.json").readText())
                                check(before.getInt("pid") != Process.myPid())
                                check(!File(directory, "before-closed.json").exists())
                                check(core.view.documents.single().title == "$run-finished.txt")
                                check(core.view.contentRoute == ContentRoute.MAIN)
                                check(core.view.reader == null)
                                check(!imports.active && imports.result == null)
                                check(!foregroundService())
                                val notifications =
                                    getSystemService(NotificationManager::class.java)
                                        .activeNotifications
                                check(
                                    notifications.none {
                                        it.id == ImportNotifications.PROGRESS_ID ||
                                            it.id == ImportNotifications.WAIT_ID
                                    }
                                )
                                publish("PASS", documents = 1)
                                return
                            }
                        }
                        if (
                            phase == "before" &&
                                !imports.active &&
                                imports.result?.outcome == ImportOutcome.CREATED
                        ) {
                            imports.start(fixture(run, "unfinished", gated = true))
                        }
                        if (
                            phase == "before" &&
                                imports.active &&
                                imports.filename == "$run-unfinished.txt" &&
                                foregroundService()
                        ) {
                            check(core.view.documents.single().title == "$run-finished.txt")
                            publish("PASS", documents = core.view.documents.size)
                            return // Keep the real provider read and service alive for SIGKILL.
                        }
                        handler.postDelayed(this, 50)
                    } catch (failure: Exception) {
                        publish("FAIL", failure = failure.toString())
                    }
                }
            }
        handler.post(checkProgress)
    }

    private fun fixture(run: String, suffix: String, gated: Boolean = false): Uri =
        Uri.Builder()
            .scheme("content")
            .authority("app.yomoku.test.txt-fixtures")
            .appendPath("$run-$suffix.txt")
            .appendQueryParameter(
                "bytes",
                Base64.encodeToString("\uFEFF$run $suffix 猫。".toByteArray(), Base64.NO_WRAP),
            )
            .apply { if (gated) appendQueryParameter("gate", run) }
            .build()

    @Suppress("DEPRECATION")
    private fun foregroundService(): Boolean =
        getSystemService(ActivityManager::class.java).getRunningServices(100).any {
            it.service.className == ImportService::class.java.name && it.foreground
        }

    private fun publish(status: String, documents: Int = 0, failure: String? = null) {
        val report =
            JSONObject()
                .put("phase", phase)
                .put("status", status)
                .put("pid", Process.myPid())
                .put("documents", documents)
                .put("active", imports.active)
                .put("has_result", imports.result != null)
                .put("foreground_service", foregroundService())
                .put(
                    "notifications_enabled",
                    getSystemService(NotificationManager::class.java).areNotificationsEnabled(),
                )
                .put("error", failure)
        val temporary = File(directory, "$phase.partial")
        temporary.writeText(report.toString(2))
        check(temporary.renameTo(File(directory, "$phase.json")))
    }

    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        File(directory, "$phase-closed.json").writeText("closed")
        imports.close()
        core.close()
        super.onDestroy()
    }

    override fun onStart() {
        super.onStart()
        core.setForeground(true)
    }

    override fun onStop() {
        if (!isChangingConfigurations) core.setForeground(false)
        super.onStop()
    }
}
