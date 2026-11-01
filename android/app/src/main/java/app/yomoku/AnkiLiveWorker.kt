package app.yomoku

import android.os.Looper
import app.yomoku.shared.AnkiLiveOperation
import app.yomoku.shared.AnkiLiveResult
import java.util.concurrent.ExecutorService

/** Session-local dispatch ownership; no persisted payload or restarted session can replay it. */
internal class AnkiLiveWorker(
    private val executor: ExecutorService,
    private val invoke: (AnkiLiveOperation) -> AnkiLiveResult,
    private val post: (() -> Unit) -> Unit,
    private val deliver: (UInt, AnkiLiveResult) -> Unit,
) : AutoCloseable {
    private val issued = mutableSetOf<UInt>()
    @Volatile private var closed = false

    fun dispatch(id: UInt, operation: AnkiLiveOperation) {
        check(Looper.myLooper() == Looper.getMainLooper())
        if (closed || !issued.add(id)) return
        executor.execute {
            // Work still queued when the session closes must not start a mutation.
            if (closed) return@execute
            val result =
                try {
                    invoke(operation)
                } catch (_: Exception) {
                    when (operation) {
                        is AnkiLiveOperation.InsertNote -> AnkiLiveResult.NoteUncertain(null)
                        is AnkiLiveOperation.CreateModel -> AnkiLiveResult.ModelUncertain(null)
                    }
                }
            post { if (!closed) deliver(id, result) }
        }
    }

    override fun close() {
        check(Looper.myLooper() == Looper.getMainLooper())
        closed = true
    }
}
