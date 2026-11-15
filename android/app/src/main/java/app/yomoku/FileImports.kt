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
import java.util.concurrent.CancellationException
import java.util.concurrent.Executors

/** Takes ownership of a private file buffer without retaining one boxed UByte per byte. */
internal class FileImportBytes(private val bytes: ByteArray) : AbstractList<UByte>(), RandomAccess {
    override val size: Int
        get() = bytes.size

    override fun get(index: Int): UByte = bytes[index].toUByte()
}

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

private sealed interface FileImportPhase {
    data object Idle : FileImportPhase

    data class Reading(val filename: String) : FileImportPhase

    data class ChoosingEncoding(val input: FileInput, val error: DocumentError) : FileImportPhase

    data class Importing(val input: FileInput) : FileImportPhase

    // Native cancellation and its storage result can return in either order.
    data class Cancelling(
        val filename: String,
        val waitingForCore: Boolean,
        val cancellationReturned: Boolean,
        val result: FileImportResult? = null,
    ) : FileImportPhase

    data class Finished(val result: FileImportResult?) : FileImportPhase
}

/** Only platform URI reads and coordination live here. Rust owns the import transaction. */
internal class FileImports(
    private val resolver: ContentResolver,
    private val core: CoreSession,
    private val observer: FileImportObserver? = null,
    private val notificationsAvailable: () -> Boolean = { false },
    private val availableImportMemory: () -> Long = FileImportMemory::availableBytes,
) : AutoCloseable {
    private val handler = Handler(Looper.getMainLooper())
    private var reader = Executors.newSingleThreadExecutor()
    @Volatile private var readGeneration = 0L
    private var closed = false
    private var phase: FileImportPhase by mutableStateOf(FileImportPhase.Idle)

    val active: Boolean
        get() =
            when (phase) {
                FileImportPhase.Idle,
                is FileImportPhase.Finished -> false
                else -> true
            }

    val choosingEncoding: Boolean
        get() = phase is FileImportPhase.ChoosingEncoding

    val cancelling: Boolean
        get() = phase is FileImportPhase.Cancelling

    val filename: String
        get() =
            when (val current = phase) {
                is FileImportPhase.Reading -> current.filename
                is FileImportPhase.ChoosingEncoding -> current.input.filename
                is FileImportPhase.Importing -> current.input.filename
                is FileImportPhase.Cancelling -> current.filename
                else -> ""
            }

    val encodingError: DocumentError?
        get() = (phase as? FileImportPhase.ChoosingEncoding)?.error

    val result: FileImportResult?
        get() =
            when (val current = phase) {
                is FileImportPhase.Finished -> current.result
                is FileImportPhase.Cancelling -> current.result
                else -> null
            }

    var notificationsEnabled by mutableStateOf(false)
        private set

    init {
        // Post to avoid dispatching recursively while CoreSession is rendering/driving its clock.
        core.onViewChanged = { handler.post { receive() } }
    }

    fun start(uri: Uri) {
        checkMainThread()
        if (closed || active) return
        val view = core.view
        if (
            view.storageBusy ||
                view.completionSavePending ||
                view.contentRoute != ContentRoute.MAIN ||
                view.destination != Destination.LIBRARY ||
                view.settingsOpen
        )
            return
        core.dispatch(Event.DismissImportSummary)
        phase = FileImportPhase.Reading(uri.lastPathSegment.orEmpty())
        val generation = ++readGeneration
        refreshNotifications()
        reader.execute {
            fun checkCurrent() {
                if (generation != readGeneration) throw CancellationException()
            }
            var name = uri.lastPathSegment.orEmpty()
            val loaded =
                try {
                    checkCurrent()
                    name =
                        resolver
                            .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                            ?.use { cursor ->
                                checkCurrent()
                                val hasName = cursor.moveToFirst()
                                checkCurrent()
                                if (hasName) cursor.getString(0) else null
                            }
                            ?.takeIf { it.isNotBlank() } ?: error("Missing display name")
                    checkCurrent()
                    // Publish the display name before a potentially slow provider read.
                    val displayName = name
                    handler.post {
                        if (generation == readGeneration) {
                            phase = FileImportPhase.Reading(displayName)
                            refreshNotifications()
                        }
                    }
                    val format = FileFormat.fromFilename(name)
                    if (format == null) {
                        null
                    } else {
                        val limit = FileImportMemory.sourceLimit(availableImportMemory())
                        checkCurrent()
                        val bytes =
                            resolver.openInputStream(uri)?.use {
                                FileImportMemory.read(it, limit) { generation == readGeneration }
                            } ?: error("Cannot open document")
                        checkCurrent()
                        FileInput(FileImportBytes(bytes), name, format)
                    }
                } catch (error: Exception) {
                    // Resources unwind on this worker, even if their close blocks.
                    if (generation != readGeneration) return@execute
                    handler.post {
                        if (generation == readGeneration)
                            finish(
                                FileImportResult(
                                    name,
                                    message =
                                        if (error is ImportMemoryExceeded)
                                            R.string.import_file_too_large
                                        else R.string.import_file_unreadable,
                                )
                            )
                    }
                    return@execute
                }
            handler.post {
                if (generation == readGeneration) {
                    if (loaded == null) {
                        finish(FileImportResult(name, message = R.string.import_supported_files))
                    } else {
                        submit(loaded)
                    }
                }
            }
        }
    }

    private fun submit(input: FileInput) {
        try {
            FileImportMemory.requireSubmissionRoom(
                input.bytes.size,
                input.filename,
                availableImportMemory(),
            )
        } catch (_: ImportMemoryExceeded) {
            finish(FileImportResult(input.filename, message = R.string.import_file_too_large))
            return
        }
        phase = FileImportPhase.Importing(input)
        core.dispatch(input.event())
    }

    private fun receive() {
        if (closed || core.view.storageBusy) return
        val current = phase
        if (
            current !is FileImportPhase.Importing &&
                !(current is FileImportPhase.Cancelling && current.waitingForCore)
        )
            return
        val error = core.view.documentError
        val summary = core.view.importSummary
        if (current is FileImportPhase.Cancelling) {
            // A completed COMMIT wins cancellation. A rolled-back/encoding attempt adds no row.
            phase =
                current.copy(
                    waitingForCore = false,
                    result =
                        summary?.let { FileImportResult(current.filename, it.outcome, it.title) },
                )
            core.dispatch(Event.DismissImportSummary)
            refreshNotifications()
            settleCancellation()
            return
        }
        val input = (current as FileImportPhase.Importing).input
        if (input.needsEncoding(error)) {
            phase = FileImportPhase.ChoosingEncoding(input, checkNotNull(error))
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
        if (closed) return
        val current = phase as? FileImportPhase.ChoosingEncoding ?: return
        submit(current.input.copy(encoding = encoding))
        refreshNotifications()
    }

    private fun finish(result: FileImportResult) {
        complete(result)
    }

    fun cancel() {
        checkMainThread()
        if (closed || !active || cancelling) return
        val waitingForCore = phase is FileImportPhase.Importing
        phase = FileImportPhase.Cancelling(filename, waitingForCore, !waitingForCore)
        refreshNotifications()
        ++readGeneration
        // A provider can ignore interruption. Retire its executor and ignore all late callbacks;
        // a fresh import must not wait behind that provider's blocked read/query.
        reader.shutdownNow()
        reader = Executors.newSingleThreadExecutor()
        if (waitingForCore) {
            core.cancelFileImport {
                val current = phase as? FileImportPhase.Cancelling
                if (!closed && current != null) {
                    phase = current.copy(cancellationReturned = true)
                    settleCancellation()
                }
            }
        }
        settleCancellation()
    }

    private fun settleCancellation() {
        val current = phase as? FileImportPhase.Cancelling ?: return
        if (current.cancellationReturned && !current.waitingForCore) complete(current.result)
    }

    private fun complete(result: FileImportResult?) {
        phase = FileImportPhase.Finished(result)
        observer?.finished(result)
        // Clear transient errors; retain the terminal outcome for the result dialog.
        core.dispatch(Event.RefreshLibrary)
    }

    fun dismissSummary() {
        checkMainThread()
        if (!active) {
            phase = FileImportPhase.Idle
        }
    }

    fun refreshNotifications() {
        checkMainThread()
        notificationsEnabled = notificationsAvailable()
        if (!closed && active)
            observer?.update(
                FileImportProgress(
                    filename,
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
        phase = FileImportPhase.Idle
        reader.shutdownNow()
        observer?.close()
        // Already accepted Rust work still drains through CoreSession.close(). No import resumes.
    }

    private fun checkMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper())
    }
}
