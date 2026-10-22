package app.yomoku

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.Event
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel

/** Owns a native handle and a render snapshot; application state stays in Rust. */
internal class CoreSession : AutoCloseable {
    private val mainHandler = Handler(Looper.getMainLooper())
    private var closed = false

    init {
        requireMainThread()
    }

    private val bridge =
        NativeBridge(
            object : EffectsListener {
                override fun onEffects(requests: ByteArray) {
                    mainHandler.post { if (!closed) render(requests) }
                }

                override fun onError(error: FfiError) {
                    mainHandler.post { if (!closed) throw error }
                }
            }
        )

    var view: ViewModel by mutableStateOf(ViewModel.bincodeDeserialize(bridge.view()))
        private set

    fun dispatch(event: Event) {
        requireMainThread()
        check(!closed) { "CoreSession is closed" }
        render(bridge.update(event.bincodeSerialize()))
    }

    private fun render(requests: ByteArray) {
        for (request in Requests.bincodeDeserialize(requests).value) {
            when (request.effect) {
                is ShellEffect.Render -> view = ViewModel.bincodeDeserialize(bridge.view())
            }
        }
    }

    override fun close() {
        requireMainThread()
        if (!closed) {
            closed = true
            bridge.close()
        }
    }

    private fun requireMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper()) {
            "CoreSession requires the main thread"
        }
    }
}
