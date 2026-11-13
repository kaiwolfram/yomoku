package app.yomoku

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.util.concurrent.ExecutorService

/** A platform observation, never a native export permit or a persisted preference. */
internal sealed interface AnkiDeckDiscoveryState {
    data object Unchecked : AnkiDeckDiscoveryState

    data object Loading : AnkiDeckDiscoveryState

    data class Ready(val decks: List<AnkiProviderDeck>) : AnkiDeckDiscoveryState

    data class Failed(val error: AnkiReadError) : AnkiDeckDiscoveryState
}

/**
 * Retained by CoreSession; explicit reads share its provider FIFO and survive Activity recreation.
 */
internal class AnkiDeckDiscovery(
    private val executor: ExecutorService,
    private val read: () -> AnkiReadResult<List<AnkiProviderDeck>>,
) : AutoCloseable {
    private val main = Handler(Looper.getMainLooper())
    @Volatile private var closed = false
    var state: AnkiDeckDiscoveryState by mutableStateOf(AnkiDeckDiscoveryState.Unchecked)
        private set

    fun check() {
        check(Looper.myLooper() == Looper.getMainLooper())
        if (closed || state == AnkiDeckDiscoveryState.Loading) return
        state = AnkiDeckDiscoveryState.Loading
        executor.execute {
            if (closed) return@execute
            val result =
                try {
                    read()
                } catch (_: SecurityException) {
                    AnkiReadResult.Failed(AnkiReadError.PermissionDenied)
                } catch (_: Exception) {
                    AnkiReadResult.Failed(AnkiReadError.ProviderFailure)
                }
            main.post {
                if (!closed) {
                    state =
                        when (result) {
                            is AnkiReadResult.Failed -> AnkiDeckDiscoveryState.Failed(result.error)
                            is AnkiReadResult.Value ->
                                AnkiDeckDiscoveryState.Ready(result.value.toList())
                        }
                }
            }
        }
    }

    fun permissionDenied() {
        check(Looper.myLooper() == Looper.getMainLooper())
        if (!closed) state = AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied)
    }

    override fun close() {
        check(Looper.myLooper() == Looper.getMainLooper())
        closed = true
    }
}
