package app.yomoku

import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.ExperimentalComposeRuntimeApi
import androidx.compose.runtime.InternalComposeApi
import androidx.compose.runtime.RecomposeScope
import androidx.compose.runtime.currentComposer
import androidx.compose.runtime.tooling.CompositionObserver
import androidx.compose.runtime.tooling.ObservableComposition
import androidx.compose.runtime.tooling.setObserver
import java.util.IdentityHashMap

internal data class CompositionMeasurement(val startNanos: Long, val endNanos: Long, val depth: Int)

/** Test-only whole-composition intervals, including nested passes; never sum overlapping rows. */
@OptIn(ExperimentalComposeRuntimeApi::class, InternalComposeApi::class)
internal class CompositionMeasurements : CompositionObserver {
    private data class Start(val nanos: Long, val depth: Int)

    private val active = IdentityHashMap<ObservableComposition, Start>()
    private val samples = mutableListOf<CompositionMeasurement>()
    private var recording = false

    @Composable
    fun Observe() {
        val composition = currentComposer.composition
        DisposableEffect(composition) {
            val handle = checkNotNull(composition.setObserver(this@CompositionMeasurements))
            onDispose { handle.dispose() }
        }
    }

    fun start() {
        requireMainThread()
        check(!recording && active.isEmpty())
        samples.clear()
        recording = true
    }

    fun stop(): List<CompositionMeasurement> {
        requireMainThread()
        check(recording && active.isEmpty())
        recording = false
        return samples.toList()
    }

    override fun onBeginComposition(composition: ObservableComposition) {
        if (!recording) return
        requireMainThread()
        check(active.put(composition, Start(System.nanoTime(), active.size)) == null)
    }

    override fun onEndComposition(composition: ObservableComposition) {
        if (!recording) return
        val end = System.nanoTime()
        requireMainThread()
        val start = checkNotNull(active.remove(composition))
        samples.add(CompositionMeasurement(start.nanos, end, start.depth))
    }

    override fun onScopeEnter(scope: RecomposeScope) = Unit

    override fun onReadInScope(scope: RecomposeScope, value: Any) = Unit

    override fun onScopeExit(scope: RecomposeScope) = Unit

    override fun onScopeInvalidated(scope: RecomposeScope, value: Any?) = Unit

    override fun onScopeDisposed(scope: RecomposeScope) = Unit

    private fun requireMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper())
    }
}
