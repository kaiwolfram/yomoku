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
import app.yomoku.shared.EpubImport
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.SrtImport
import app.yomoku.shared.TextEncoding
import app.yomoku.shared.TxtImport
import java.util.concurrent.Executors

internal data class FileImportResult(
    val filename: String,
    val outcome: ImportOutcome? = null,
    val title: String? = null,
    val error: DocumentError? = null,
    val message: Int? = null,
)

private enum class FileFormat {
    TXT,
    SRT,
    EPUB;

    companion object {
        fun fromFilename(name: String): FileFormat? = entries.firstOrNull {
            name.endsWith(".${it.name}", ignoreCase = true)
        }
    }
}

private data class FileInput(
    val bytes: List<UByte>,
    val filename: String,
    val format: FileFormat,
    val encoding: TextEncoding? = null,
) {
    fun event(): Event =
        when (format) {
            FileFormat.TXT -> Event.ImportTxt(TxtImport(bytes, filename, encoding))
            FileFormat.SRT -> Event.ImportSrt(SrtImport(bytes, filename, encoding))
            FileFormat.EPUB -> Event.ImportEpub(EpubImport(bytes, filename))
        }

    fun needsEncoding(error: DocumentError?): Boolean =
        when (format) {
            FileFormat.TXT ->
                error == DocumentError.TXTENCODINGREQUIRED ||
                    (encoding != null &&
                        (error == DocumentError.TXTINVALIDUTF8 ||
                            error == DocumentError.TXTINVALIDSHIFTJIS))
            FileFormat.SRT ->
                error == DocumentError.SRTENCODINGREQUIRED ||
                    (encoding != null &&
                        (error == DocumentError.SRTINVALIDUTF8 ||
                            error == DocumentError.SRTINVALIDSHIFTJIS))
            FileFormat.EPUB -> false
        }
}

/** Only platform URI reads and sequencing live here. Rust owns every per-file transaction. */
internal class FileImports(
    private val resolver: ContentResolver,
    private val core: CoreSession,
    private val observer: FileImportObserver? = null,
) : AutoCloseable {
    private val handler = Handler(Looper.getMainLooper())
    private var reader = Executors.newSingleThreadExecutor()
    @Volatile private var readGeneration = 0L
    private val queue = ArrayDeque<Uri>()
    private var closed = false
    private var waitingForCore = false
    private var cancellationReturned = false
    private var cancellingFileNumber = 0
    private var input: FileInput? = null
    var active by mutableStateOf(false)
        private set

    var choosingEncoding by mutableStateOf(false)
        private set

    var cancelling by mutableStateOf(false)
        private set

    var filename by mutableStateOf("")
        private set

    var totalFiles by mutableStateOf(0)
        private set

    val currentFileNumber: Int
        get() = if (!active) 0 else if (cancelling) cancellingFileNumber else results.size + 1

    // Count terminal outcomes, including errors, duplicates and skips. Never estimate
    // byte/analyzer progress or count an encoding prompt/retry as a completed file.
    val progress: Float
        get() = if (totalFiles == 0) 0f else results.size.toFloat() / totalFiles

    var encodingError: DocumentError? by mutableStateOf(null)
        private set

    var results: List<FileImportResult> by mutableStateOf(emptyList())
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
            completeBatch()
            return
        }
        val generation = ++readGeneration
        val uri = queue.removeFirst()
        filename = uri.lastPathSegment.orEmpty()
        refreshNotifications()
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
                    if (generation != readGeneration) return@execute
                    // Publish the display name before a potentially slow provider read.
                    val displayName = name
                    handler.post {
                        if (generation == readGeneration) {
                            filename = displayName
                            refreshNotifications()
                        }
                    }
                    val format = FileFormat.fromFilename(name)
                    if (format == null) {
                        null
                    } else {
                        val bytes =
                            resolver.openInputStream(uri)?.use { it.readBytes() }
                                ?: error("Cannot open document")
                        if (generation != readGeneration) return@execute
                        FileInput(bytes.map { it.toUByte() }, name, format)
                    }
                } catch (_: Exception) {
                    handler.post {
                        if (generation == readGeneration)
                            finish(
                                FileImportResult(name, message = R.string.import_file_unreadable)
                            )
                    }
                    return@execute
                }
            handler.post {
                if (generation == readGeneration) {
                    if (loaded == null) {
                        finish(FileImportResult(name, message = R.string.import_supported_files))
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
        core.dispatch(checkNotNull(input).event())
    }

    private fun receive() {
        if (closed || !waitingForCore || core.view.storageBusy) return
        waitingForCore = false
        val error = core.view.documentError
        val summary = core.view.importSummary
        if (cancelling) {
            // A completed COMMIT wins cancellation. A rolled-back/encoding attempt adds no row.
            if (summary != null)
                results = results + FileImportResult(filename, summary.outcome, summary.title)
            core.dispatch(Event.DismissImportSummary)
            refreshNotifications()
            settleCancellation()
            return
        }
        if (input?.needsEncoding(error) == true) {
            encodingError = error
            choosingEncoding = true
            refreshNotifications()
            return
        }
        core.dispatch(Event.DismissImportSummary)
        finish(
            FileImportResult(
                filename,
                outcome = summary?.outcome,
                title = summary?.title,
                error = error,
                message =
                    if (error == null && summary == null) R.string.error_storage_unavailable
                    else null,
            )
        )
    }

    fun selectEncoding(encoding: TextEncoding) {
        checkMainThread()
        if (closed || cancelling || !choosingEncoding) return
        input = checkNotNull(input).copy(encoding = encoding)
        choosingEncoding = false
        refreshNotifications()
        submit()
    }

    fun skipEncoding() {
        checkMainThread()
        if (!closed && !cancelling && choosingEncoding)
            finish(FileImportResult(filename, message = R.string.import_file_skipped))
    }

    private fun finish(result: FileImportResult) {
        results = results + result
        next()
    }

    fun cancel() {
        checkMainThread()
        if (closed || !active || cancelling) return
        cancellingFileNumber = currentFileNumber
        cancelling = true
        choosingEncoding = false
        encodingError = null
        queue.clear()
        input = null
        refreshNotifications()
        ++readGeneration
        // A provider can ignore interruption. Retire its executor and ignore all late callbacks;
        // a fresh batch must not wait behind that provider's blocked read/query.
        reader.shutdownNow()
        reader = Executors.newSingleThreadExecutor()
        cancellationReturned = !waitingForCore
        if (waitingForCore) {
            core.cancelFileImport {
                cancellationReturned = true
                if (!closed) settleCancellation()
            }
        }
        settleCancellation()
    }

    private fun settleCancellation() {
        if (cancellationReturned && !waitingForCore) completeBatch()
    }

    private fun completeBatch() {
        observer?.finished(results.size, totalFiles, cancelling)
        active = false
        cancelling = false
        filename = ""
        input = null
        // Clear transient errors; completed outcomes stay in the retained batch summary.
        core.dispatch(Event.RefreshLibrary)
    }

    fun dismissSummary() {
        checkMainThread()
        if (!active) {
            results = emptyList()
            totalFiles = 0
        }
    }

    fun refreshNotifications() {
        checkMainThread()
        if (!closed && active)
            observer?.update(
                FileImportProgress(
                    filename,
                    currentFileNumber,
                    results.size,
                    totalFiles,
                    choosingEncoding,
                    cancelling,
                )
            )
    }

    override fun close() {
        checkMainThread()
        closed = true
        ++readGeneration
        core.onViewChanged = null
        queue.clear()
        input = null
        reader.shutdownNow()
        observer?.close()
        // Already accepted Rust work still drains through CoreSession.close(). No batch is
        // persisted.
    }

    private fun checkMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper())
    }
}
