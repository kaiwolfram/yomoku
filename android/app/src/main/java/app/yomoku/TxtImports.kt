package app.yomoku

import android.content.ContentResolver
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Destination
import app.yomoku.shared.DocumentError
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.TextEncoding
import app.yomoku.shared.TxtImport
import java.util.concurrent.Executors

internal data class TxtFileResult(
    val filename: String,
    val outcome: ImportOutcome? = null,
    val title: String? = null,
    val error: DocumentError? = null,
    val message: Int? = null,
)

/** Only platform URI reads and sequencing live here. Rust owns every per-file transaction. */
internal class TxtImports(private val resolver: ContentResolver, private val core: CoreSession) :
    AutoCloseable {
    private val handler = Handler(Looper.getMainLooper())
    private val reader = Executors.newSingleThreadExecutor()
    private val queue = ArrayDeque<Uri>()
    private var closed = false
    private var waitingForCore = false
    private var input: TxtImport? = null
    var active by mutableStateOf(false)
        private set

    var choosingEncoding by mutableStateOf(false)
        private set

    var filename by mutableStateOf("")
        private set

    var totalFiles by mutableStateOf(0)
        private set

    val currentFileNumber: Int
        get() = if (active) results.size + 1 else 0

    // Count terminal outcomes, including errors, duplicates and skips. Never estimate
    // byte/analyzer progress or count an encoding prompt/retry as a completed file.
    val progress: Float
        get() = if (totalFiles == 0) 0f else results.size.toFloat() / totalFiles

    var encodingError: DocumentError? by mutableStateOf(null)
        private set

    var results: List<TxtFileResult> by mutableStateOf(emptyList())
        private set

    init {
        // Post to avoid dispatching recursively while CoreSession is rendering/driving its clock.
        core.onViewChanged = { handler.post { receive() } }
    }

    fun start(uris: List<Uri>) {
        checkMainThread()
        if (closed || active || uris.isEmpty()) return
        val view = core.view
        if (
            view.storageBusy ||
                view.readingTimeSavePending ||
                view.completionSavePending ||
                view.contentRoute != ContentRoute.MAIN ||
                view.destination != Destination.LIBRARY ||
                view.settingsOpen
        )
            return
        results = emptyList()
        totalFiles = uris.size
        core.dispatch(Event.DismissImportSummary)
        queue.addAll(uris)
        active = true
        next()
    }

    private fun next() {
        input = null
        choosingEncoding = false
        encodingError = null
        if (queue.isEmpty()) {
            active = false
            filename = ""
            // Clear the last file's transient error; each result remains in the batch summary.
            core.dispatch(Event.RefreshLibrary)
            return
        }
        val uri = queue.removeFirst()
        filename = uri.lastPathSegment.orEmpty()
        reader.execute {
            var name = uri.lastPathSegment.orEmpty()
            val loaded =
                try {
                    name =
                        resolver
                            .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                            ?.use { cursor ->
                                if (cursor.moveToFirst()) cursor.getString(0) else null
                            }
                            ?.takeIf { it.isNotBlank() } ?: error("Missing display name")
                    // Publish the display name before a potentially slow provider read.
                    val displayName = name
                    handler.post { if (!closed) filename = displayName }
                    if (!name.endsWith(".txt", ignoreCase = true)) {
                        null
                    } else {
                        val bytes =
                            resolver.openInputStream(uri)?.use { it.readBytes() }
                                ?: error("Cannot open document")
                        TxtImport(bytes.map { it.toUByte() }, name)
                    }
                } catch (_: Exception) {
                    handler.post {
                        if (!closed)
                            finish(TxtFileResult(name, message = R.string.import_file_unreadable))
                    }
                    return@execute
                }
            handler.post {
                if (!closed) {
                    if (loaded == null) {
                        finish(TxtFileResult(name, message = R.string.import_txt_only))
                    } else {
                        input = loaded
                        filename = loaded.filename
                        submit()
                    }
                }
            }
        }
    }

    private fun submit() {
        waitingForCore = true
        core.dispatch(Event.ImportTxt(checkNotNull(input)))
    }

    private fun receive() {
        if (closed || !waitingForCore || core.view.storageBusy) return
        waitingForCore = false
        val error = core.view.documentError
        if (
            error == DocumentError.TXTENCODINGREQUIRED ||
                (input?.encoding != null &&
                    (error == DocumentError.TXTINVALIDUTF8 ||
                        error == DocumentError.TXTINVALIDSHIFTJIS))
        ) {
            encodingError = error
            choosingEncoding = true
            return
        }
        val summary = core.view.importSummary
        finish(
            TxtFileResult(
                filename,
                outcome = summary?.outcome,
                title = summary?.title,
                error = error,
                message =
                    if (error == null && summary == null) R.string.error_storage_unavailable
                    else null,
            )
        )
        core.dispatch(Event.DismissImportSummary)
    }

    fun selectEncoding(encoding: TextEncoding) {
        checkMainThread()
        if (closed || !choosingEncoding) return
        input = checkNotNull(input).copy(encoding = encoding)
        choosingEncoding = false
        submit()
    }

    fun skipEncoding() {
        checkMainThread()
        if (!closed && choosingEncoding)
            finish(TxtFileResult(filename, message = R.string.import_file_skipped))
    }

    private fun finish(result: TxtFileResult) {
        results = results + result
        next()
    }

    fun dismissSummary() {
        checkMainThread()
        if (!active) {
            results = emptyList()
            totalFiles = 0
        }
    }

    override fun close() {
        checkMainThread()
        closed = true
        core.onViewChanged = null
        queue.clear()
        input = null
        reader.shutdownNow()
        // Already accepted Rust work still drains through CoreSession.close(). No batch is
        // persisted.
    }

    private fun checkMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper())
    }
}
