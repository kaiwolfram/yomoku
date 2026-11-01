package app.yomoku

import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
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
    val export: AnkiExportView,
    val noteId: Long,
    val phase: AnkiBrowserPhase,
)

/** Platform navigation only. Native export identity and acknowledged IDs remain authoritative. */
internal class AnkiNoteBrowser(
    private val executor: ExecutorService,
    private val resolve: (AnkiProviderNoteId) -> AnkiReadResult<Intent?>,
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
        if (state?.export != current()) {
            pending = null
            state = null
        }
    }

    fun open(export: AnkiExportView, noteId: Long) {
        requireMain()
        if (closed || launch == null || pending != null || current() != export) return
        if (noteId <= 0 || noteId !in export.openableNoteIds()) return
        val request = AnkiBrowserState(export, noteId, AnkiBrowserPhase.Resolving)
        pending = request
        state = request
        executor.execute {
            if (pending !== request) return@execute
            val result =
                try {
                    resolve(AnkiProviderNoteId(noteId))
                } catch (_: SecurityException) {
                    AnkiReadResult.Failed(AnkiReadError.PermissionDenied)
                } catch (_: Exception) {
                    AnkiReadResult.Failed(AnkiReadError.ProviderFailure)
                }
            main.post {
                if (pending !== request) return@post
                pending = null
                if (closed || current() != export || launch == null) {
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
