package app.yomoku

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.yomoku.nativebridge.DictionaryProvider
import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderClockAction
import app.yomoku.shared.ReaderClockSample
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/** Owns a native handle and a render snapshot; application state stays in Rust. */
internal class CoreSession(
    databasePath: String,
    dictionary: DictionaryProvider,
    private val monotonicMs: () -> Long = SystemClock::elapsedRealtime,
    private val checkpointIntervalMs: Long = 15_000L,
    private val cancellationExecutor: ExecutorService = Executors.newSingleThreadExecutor(),
) : AutoCloseable {
    // Retained with this handle across Activity recreation; never restored into a new handle.
    val sessionKey: String = UUID.randomUUID().toString()
    private val mainHandler = Handler(Looper.getMainLooper())
    var isClosed = false
        private set

    private var closing = false
    private var cancellationPending = false
    private var readerActive = false
    private var clockRunning = false
    private var tickScheduled = false
    private val tick = Runnable {
        tickScheduled = false
        if (closing) {
            if (!view.storageBusy) {
                when {
                    view.readingTimeSavePending -> send(Event.RetryReadingTimeCheckpoint)
                    view.completionSavePending -> send(Event.RetryReaderCompletion)
                }
            }
        } else if (clockRunning) {
            observe(ReaderClockAction.CHECKPOINT)
        }
        driveReaderClock()
    }

    init {
        requireMainThread()
        require(checkpointIntervalMs > 0)
    }

    private val bridge =
        NativeBridge(
            databasePath,
            object : EffectsListener {
                override fun onEffects(requests: ByteArray) {
                    mainHandler.post {
                        if (!isClosed) {
                            render(requests)
                            driveReaderClock()
                        }
                    }
                }

                override fun onError(error: FfiError) {
                    mainHandler.post { if (!isClosed) throw error }
                }
            },
            dictionary,
        )

    var view: ViewModel by mutableStateOf(ViewModel.bincodeDeserialize(bridge.view()))
        private set

    // Platform import sequencing observes committed core outcomes, on the main thread.
    var onViewChanged: (() -> Unit)? = null

    val images =
        EpubImageLoader(
            read = { key, listener ->
                bridge.loadEpubImageAsset(
                    key.documentId,
                    key.asset.path,
                    key.asset.mediaType,
                    listener,
                )
            }
        )

    val covers = EpubCoverLoader(bridge::loadEpubCoverReference, images)

    init {
        dispatch(Event.RefreshLibrary)
    }

    fun dispatch(event: Event) {
        requireMainThread()
        // Compose can dispose its transient selection after ViewModel teardown begins.
        if (closing && event == Event.DismissReaderWord) return
        check(!closing) { "CoreSession is closing or closed" }
        // Sample before the intent: core may freeze the departing position immediately.
        when (event) {
            Event.OpenSettings,
            Event.CloseDocument,
            Event.OpenPaste,
            is Event.SelectDestination,
            is Event.CompleteReaderPage,
            is Event.CompleteEffectiveReaderPage,
            is Event.PreviousReaderPage,
            is Event.OpenReader,
            is Event.LoadPaste,
            is Event.CreatePaste,
            is Event.ImportTxt,
            is Event.ImportSrt,
            is Event.ImportEpub -> pauseClock()
            is Event.SaveReadingPosition -> if (event.value != view.reader?.position) pauseClock()
            else -> Unit
        }
        send(event)
        driveReaderClock()
    }

    fun setForeground(foreground: Boolean) {
        requireMainThread()
        check(!closing) { "CoreSession is closing or closed" }
        if (!foreground) setReaderActive(false)
        bridge.setForeground(foreground)
    }

    /** The native commit gate may wait. Keep the handle alive until this direct call returns. */
    fun cancelFileImport(onComplete: () -> Unit) {
        requireMainThread()
        check(!closing && !cancellationPending)
        cancellationPending = true
        cancellationExecutor.execute {
            val result = runCatching { bridge.cancelFileImport() }
            mainHandler.post {
                cancellationPending = false
                try {
                    result.getOrThrow()
                    onComplete()
                } finally {
                    driveReaderClock()
                }
            }
        }
    }

    /** Activity resume/pause, separate from the retained analyzer's start/stop policy. */
    fun setReaderActive(active: Boolean) {
        requireMainThread()
        if (closing) return
        readerActive = active
        if (!active) pauseClock()
        driveReaderClock()
    }

    fun onReaderInteraction() {
        requireMainThread()
        if (!closing && clockRunning) observe(ReaderClockAction.INTERACT)
    }

    private fun send(event: Event) {
        render(bridge.update(event.bincodeSerialize()))
    }

    private fun observe(action: ReaderClockAction) {
        val reader = view.reader ?: return
        send(
            Event.ObserveReaderClock(
                ReaderClockSample(reader.documentId, reader.position, monotonicMs(), action)
            )
        )
    }

    private fun pauseClock() {
        if (clockRunning) observe(ReaderClockAction.PAUSE)
        clockRunning = false
        cancelTick()
    }

    private fun driveReaderClock() {
        if (closing) {
            if (
                !view.storageBusy &&
                    !view.readingTimeSavePending &&
                    !view.completionSavePending &&
                    !cancellationPending
            ) {
                cancelTick()
                isClosed = true
                bridge.close()
                cancellationExecutor.shutdown()
            } else if (!view.storageBusy) {
                // onCleared has no UI left for Retry. Keep the exact core obligation alive.
                scheduleTick(1_000L)
            }
            return
        }
        val reader = view.reader
        if (
            !readerActive ||
                view.settingsOpen ||
                reader == null ||
                reader.position.offset == reader.anchor.range.end
        )
            return
        if (!view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending) {
            if (view.readerTime == null && view.documentError == null) {
                send(Event.RestoreReadingTime)
            }
            if (view.readerTime != null && !clockRunning) {
                observe(ReaderClockAction.RESUME)
                clockRunning = true
            }
        }
        if (clockRunning) scheduleTick(checkpointIntervalMs)
    }

    private fun scheduleTick(delayMs: Long) {
        if (!tickScheduled) {
            tickScheduled = true
            mainHandler.postDelayed(tick, delayMs)
        }
    }

    private fun cancelTick() {
        mainHandler.removeCallbacks(tick)
        tickScheduled = false
    }

    private fun render(requests: ByteArray) {
        for (request in Requests.bincodeDeserialize(requests).value) {
            when (request.effect) {
                is ShellEffect.Render -> view = ViewModel.bincodeDeserialize(bridge.view())
            }
        }
        onViewChanged?.invoke()
    }

    override fun close() {
        requireMainThread()
        if (!closing) {
            covers.close()
            images.close()
            pauseClock()
            closing = true
            readerActive = false
            driveReaderClock()
        }
    }

    private fun requireMainThread() {
        check(Looper.myLooper() == Looper.getMainLooper()) {
            "CoreSession requires the main thread"
        }
    }
}
