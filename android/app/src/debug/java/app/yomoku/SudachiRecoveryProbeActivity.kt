package app.yomoku

import android.content.Context
import android.content.ContextWrapper
import android.os.Bundle
import android.os.Debug
import android.os.ParcelFileDescriptor
import android.os.Process
import android.os.SystemClock
import android.system.Os
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.Text
import app.yomoku.theme.YomokuTheme
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.Executors
import org.json.JSONObject

/** ADB-driven process-death experiment. Never used by the production reader or startup. */
class SudachiRecoveryProbeActivity : ComponentActivity() {
    private val worker = Executors.newSingleThreadExecutor()
    private var generation = 0L // Accessed only on the worker, including normal destruction.
    private lateinit var directory: File
    private lateinit var phase: String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val run = intent.getStringExtra("run") ?: error("Missing probe run")
        require(run.matches(Regex("[0-9a-f]{32}")))
        phase = intent.getStringExtra("phase") ?: error("Missing probe phase")
        require(phase in listOf("before", "after"))
        directory = File(noBackupFilesDir, "process-probe-$run")
        val privateContext =
            object : ContextWrapper(applicationContext) {
                override fun getApplicationContext(): Context = this

                override fun getNoBackupFilesDir(): File = directory
            }
        setContent { YomokuTheme { Text("Sudachi process-death measurement: $phase") } }
        worker.execute {
            val report = JSONObject().put("phase", phase).put("pid", Process.myPid())
            try {
                // The host must have killed the first process without onDestroy/nativeClose.
                if (phase == "after") {
                    check(File(directory, "before.json").isFile)
                    check(!File(directory, "before-closed.json").exists())
                }
                val files = sudachiDictionaryFiles(privateContext)
                val started = SystemClock.elapsedRealtimeNanos()
                val native = SudachiAppProbe()
                files.open().use { input ->
                    val stat = Os.fstat(input.fd)
                    report.put("inode", stat.st_ino).put("mtime", stat.st_mtime)
                    report.put("size", stat.st_size).put("mode", stat.st_mode)
                    ParcelFileDescriptor.dup(input.fd).use {
                        generation = native.nativeOpen(it.fd)
                    }
                }
                check(generation > 0) { "Native open failed: $generation" }
                report.put("open_ms", (SystemClock.elapsedRealtimeNanos() - started) / 1_000_000.0)
                for ((fixture, tokens) in
                    listOf(0 to 3L, 1 to 2L, 2 to 1L, 4 to 330L, 5 to 16002L)) {
                    check(native.nativeValidate(generation, fixture) == tokens)
                }
                check(native.nativeValidate(generation, 3) > 0)
                val mapped =
                    File("/proc/self/maps").useLines { lines ->
                        lines.any { it.contains("/${directory.name}/sudachi/full-") }
                    }
                check(mapped)
                val memory = Debug.MemoryInfo()
                Debug.getMemoryInfo(memory)
                report.put("pss_kib", memory.totalPss).put("mapped", mapped)
                report.put("status", "PASS")
            } catch (failure: Throwable) {
                Log.e("SudachiRecoveryProbe", "Probe failed", failure)
                report.put("status", "FAIL").put("error", failure.toString())
            }
            publish("$phase.json", report)
            // Keep the analyzer mapped until the external driver terminates this process.
        }
    }

    override fun onDestroy() {
        worker.execute {
            val result = if (generation > 0) SudachiAppProbe().nativeClose(generation) else 0L
            publish("$phase-closed.json", JSONObject().put("close_result", result))
        }
        worker.shutdown()
        super.onDestroy()
    }

    private fun publish(name: String, report: JSONObject) {
        check(directory.isDirectory || directory.mkdirs())
        val temporary = File(directory, "$name.partial")
        FileOutputStream(temporary).use {
            it.write(report.toString(2).toByteArray(Charsets.UTF_8))
            it.fd.sync()
        }
        Os.rename(temporary.path, File(directory, name).path)
    }
}
