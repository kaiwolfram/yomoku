package app.yomoku

import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.yomoku.shared.AnkiEntryPhase
import app.yomoku.shared.AnkiEntryView
import app.yomoku.shared.AnkiExportPhase
import app.yomoku.shared.AnkiExportView
import java.util.concurrent.ExecutorService

internal fun AnkiExportView.openableNoteIds(): List<Long> =
    when (val value = phase) {
        is AnkiExportPhase.Existing -> value.value
        is AnkiExportPhase.NoteAdded -> listOf(value.value)
        else -> emptyList()
    }

internal sealed interface AnkiBrowserPhase {
    data object Resolving : AnkiBrowserPhase

    data object Unavailable : AnkiBrowserPhase

    data class Failed(val error: AnkiReadError) : AnkiBrowserPhase
}

internal data class AnkiBrowserState(
    val export: AnkiExportView?,
    val noteId: Long,
    val phase: AnkiBrowserPhase,
    val entry: AnkiEntryView? = null,
)

/** Platform navigation only. Native export identity and acknowledged IDs remain authoritative. */
internal class AnkiNoteBrowser(
    private val executor: ExecutorService,
    private val resolve: (AnkiProviderNoteId) -> AnkiReadResult<Intent?>,
    private val currentEntries: () -> List<AnkiEntryView> = { emptyList() },
    private val resolveEntries: (List<AnkiProviderNoteId>) -> AnkiReadResult<Intent?> = { ids ->
        resolve(ids.first())
    },
    private val current: () -> AnkiExportView?,
) : AutoCloseable {
    private val main = Handler(Looper.getMainLooper())
    @Volatile private var pending: AnkiBrowserState? = null
    private var closed = false
    private var launch: ((Intent) -> Unit)? = null
    var state: AnkiBrowserState? by mutableStateOf(null)
        private set

    /** Hold an Activity callback only while resumed; attaching never replays a launch. */
    fun attach(launch: (Intent) -> Unit) {
        requireMain()
        if (!closed) this.launch = launch
    }

    fun detach() {
        requireMain()
        launch = null
        pending = null
        if (state?.phase == AnkiBrowserPhase.Resolving) state = null
    }

    fun refresh() {
        requireMain()
        if (state?.let { !isCurrent(it) } == true) {
            pending = null
            state = null
        }
    }

    fun open(export: AnkiExportView, noteId: Long) {
        requireMain()
        if (closed || launch == null || pending != null || current() != export) return
        if (noteId <= 0 || noteId !in export.openableNoteIds()) return
        val request = AnkiBrowserState(export, noteId, AnkiBrowserPhase.Resolving)
        openRequest(request)
    }

    fun openEntry(entry: AnkiEntryView) {
        requireMain()
        val ids = (entry.phase as? AnkiEntryPhase.Existing)?.value ?: return
        if (
            closed ||
                launch == null ||
                pending != null ||
                entry !in currentEntries() ||
                ids.isEmpty()
        )
            return
        openRequest(AnkiBrowserState(null, ids.first(), AnkiBrowserPhase.Resolving, entry))
    }

    private fun isCurrent(request: AnkiBrowserState): Boolean =
        request.entry?.let { it in currentEntries() } ?: (request.export == current())

    private fun openRequest(request: AnkiBrowserState) {
        pending = request
        state = request
        executor.execute {
            if (pending !== request) return@execute
            val result =
                try {
                    request.entry?.let { entry ->
                        val ids = (entry.phase as AnkiEntryPhase.Existing).value
                        resolveEntries(ids.map(::AnkiProviderNoteId))
                    } ?: resolve(AnkiProviderNoteId(request.noteId))
                } catch (_: SecurityException) {
                    AnkiReadResult.Failed(AnkiReadError.PermissionDenied)
                } catch (_: Exception) {
                    AnkiReadResult.Failed(AnkiReadError.ProviderFailure)
                }
            main.post {
                if (pending !== request) return@post
                pending = null
                if (closed || !isCurrent(request) || launch == null) {
                    state = null
                    return@post
                }
                when (result) {
                    is AnkiReadResult.Failed ->
                        state = request.copy(phase = AnkiBrowserPhase.Failed(result.error))
                    is AnkiReadResult.Value -> {
                        val intent = result.value
                        if (intent == null) {
                            state = request.copy(phase = AnkiBrowserPhase.Unavailable)
                        } else {
                            // Consume before calling Android. A browser redirect/return conveys
                            // no mutation acknowledgement and must never trigger another export.
                            state = null
                            try {
                                launch?.invoke(intent)
                            } catch (_: ActivityNotFoundException) {
                                state = request.copy(phase = AnkiBrowserPhase.Unavailable)
                            } catch (_: SecurityException) {
                                state =
                                    request.copy(
                                        phase =
                                            AnkiBrowserPhase.Failed(AnkiReadError.PermissionDenied)
                                    )
                            } catch (_: Exception) {
                                state =
                                    request.copy(
                                        phase =
                                            AnkiBrowserPhase.Failed(AnkiReadError.ProviderFailure)
                                    )
                            }
                        }
                    }
                }
            }
        }
    }

    override fun close() {
        requireMain()
        closed = true
        detach()
        state = null
    }

    private fun requireMain() {
        check(Looper.myLooper() == Looper.getMainLooper())
    }
}
