package app.yomoku

import android.content.Intent
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
import app.yomoku.shared.AnkiExportError
import app.yomoku.shared.AnkiExportPhase
import app.yomoku.shared.AnkiLiveOperation
import app.yomoku.shared.AnkiLiveResult
import app.yomoku.shared.AnkiPreflightOperation
import app.yomoku.shared.AnkiPreflightResult
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.shared.ReaderClockAction
import app.yomoku.shared.ReaderClockSample
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.ViewModel
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

internal enum class CoreRenderSource {
    DISPATCH,
    CALLBACK,
    REPLAY,
}

/** Main-thread render phases; publication includes structural equality and state notification. */
internal data class CoreRenderMeasurement(
    val source: CoreRenderSource,
    val nativeViewNanos: Long,
    val decodeNanos: Long,
    val publishNanos: Long,
    val payloadBytes: Int,
    val loadedRows: Int,
    val selectedIds: Int,
    val confirmationIds: Int,
    val published: Boolean,
)

/** Callback native work/queue delay and subsequent renders are outside these intervals. */
internal data class CoreRequestsMeasurement(
    val source: CoreRenderSource,
    val eventEncodeNanos: Long?,
    val nativeUpdateNanos: Long?,
    val requestDecodeNanos: Long,
    val payloadBytes: Int,
    val requestCount: Int,
)

/** Arrival at Kotlin excludes preceding native work; handling includes render/observer work. */
internal data class CoreCallbackMeasurement(
    val arrivedNanos: Long,
    val queueNanos: Long,
    val handleNanos: Long,
    val payloadBytes: Int,
)

/** Owns a native handle and a render snapshot; application state stays in Rust. */
internal class CoreSession(
    databasePath: String,
    dictionary: DictionaryProvider,
    private val monotonicMs: () -> Long = SystemClock::elapsedRealtime,
    private val checkpointIntervalMs: Long = 15_000L,
    private val cancellationExecutor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val ankiExecutor: ExecutorService = Executors.newSingleThreadExecutor(),
    private val readAnkiPreflight: (AnkiPreflightOperation) -> AnkiPreflightResult = {
        AnkiPreflightResult.Failed(AnkiExportError.PROVIDERUNAVAILABLE)
    },
    private val invokeAnkiLive: (AnkiLiveOperation) -> AnkiLiveResult = {
        AnkiLiveResult.NoSubmission
    },
    readAnkiDecks: () -> AnkiReadResult<List<AnkiProviderDeck>> = {
        AnkiReadResult.Failed(AnkiReadError.ProviderUnavailable)
    },
    resolveAnkiNotes: (List<AnkiProviderNoteId>) -> AnkiReadResult<Intent?> = {
        AnkiReadResult.Failed(AnkiReadError.ProviderUnavailable)
    },
    resolveAnkiNote: (AnkiProviderNoteId) -> AnkiReadResult<Intent?> = {
        AnkiReadResult.Failed(AnkiReadError.ProviderUnavailable)
    },
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
    private var scheduledAnkiPreparation: ULong? = null
    private var checkedAnkiDeck: Long? = null
    private var ankiSetupScheduled = false
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
                    val observer = onCallbackMeasured
                    val arrived = if (observer != null) System.nanoTime() else 0L
                    mainHandler.post {
                        if (!isClosed) {
                            val start = if (observer != null) System.nanoTime() else 0L
                            render(requests, fromCallback = true)
                            driveReaderClock()
                            if (observer != null && onCallbackMeasured === observer) {
                                val end = System.nanoTime()
                                observer(
                                    CoreCallbackMeasurement(
                                        arrived,
                                        start - arrived,
                                        end - start,
                                        requests.size,
                                    )
                                )
                            }
                        }
                    }
                }

                override fun onError(error: FfiError) {
                    mainHandler.post { if (!isClosed) throw error }
                }
            },
            dictionary,
        )

    private val ankiLiveWorker =
        AnkiLiveWorker(
            ankiExecutor,
            invokeAnkiLive,
            { mainHandler.post(it) },
        ) { id, result ->
            if (!closing && !isClosed) {
                render(bridge.resolve(id, result.bincodeSerialize()))
                driveReaderClock()
            }
        }

    var view: ViewModel by mutableStateOf(ViewModel.bincodeDeserialize(bridge.view()))
        private set

    val ankiDecks = AnkiDeckDiscovery(ankiExecutor, readAnkiDecks)
    val ankiBrowser =
        AnkiNoteBrowser(ankiExecutor, resolveAnkiNote, { view.ankiEntries }, resolveAnkiNotes) {
            view.ankiExport?.takeIf {
                view.settingsOpen ||
                    (view.contentRoute == ContentRoute.READER &&
                        view.effectiveReaderWord?.selection != null)
            }
        }

    fun checkAnkiDecks() {
        requireMainThread()
        if (!closing && !isClosed && view.settingsOpen) {
            ankiDecks.check()
            refreshAnkiSetup()
        }
    }

    fun refreshAnkiSetup() {
        requireMainThread()
        if (!closing && !isClosed) {
            checkedAnkiDeck = null
            scheduleAnkiSetup()
        }
    }

    private fun scheduleAnkiSetup() {
        val deck = view.ankiSettings.defaultDeckId
        if (deck == null) {
            checkedAnkiDeck = null
            return
        }
        if (
            closing ||
                view.ankiExport != null ||
                ankiSetupScheduled ||
                deck == checkedAnkiDeck ||
                view.ankiSettings.phase != app.yomoku.shared.AnkiSettingsPhase.READY
        )
            return
        ankiSetupScheduled = true
        mainHandler.post {
            ankiSetupScheduled = false
            if (
                !closing &&
                    !isClosed &&
                    view.ankiSettings.defaultDeckId == deck &&
                    view.ankiExport == null
            ) {
                checkedAnkiDeck = deck
                send(Event.CheckAnkiSetup)
            }
        }
    }

    // Platform import sequencing observes committed core outcomes, on the main thread.
    var onViewChanged: (() -> Unit)? = null

    // Opt-in diagnostics only. Normal renders read no clocks and retain no samples.
    var onRenderMeasured: ((CoreRenderMeasurement) -> Unit)? = null
        set(value) {
            requireMainThread()
            field = value
        }

    var onRequestsMeasured: ((CoreRequestsMeasurement) -> Unit)? = null
        set(value) {
            requireMainThread()
            field = value
        }

    // Read on the native worker at arrival; install/remove and delivery stay on the main thread.
    @Volatile
    var onCallbackMeasured: ((CoreCallbackMeasurement) -> Unit)? = null
        set(value) {
            requireMainThread()
            field = value
        }

    /**
     * Re-read the current native snapshot through the production publication path, diagnostics
     * only.
     */
    fun measureCurrentView() {
        requireMainThread()
        check(!closing && onRenderMeasured != null)
        renderView(CoreRenderSource.REPLAY)
    }

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
        dispatch(Event.LoadAnkiSettings)
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
            is Event.CompleteEffectiveReaderPage,
            is Event.PreviousReaderPage,
            is Event.OpenReader,
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
        if (onRequestsMeasured == null) {
            render(bridge.update(event.bincodeSerialize()))
            return
        }
        val start = System.nanoTime()
        val bytes = event.bincodeSerialize()
        val encoded = System.nanoTime()
        val requests = bridge.update(bytes)
        val updated = System.nanoTime()
        render(requests, eventEncodeNanos = encoded - start, nativeUpdateNanos = updated - encoded)
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
                ankiExecutor.shutdown()
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

    private fun render(
        requests: ByteArray,
        fromCallback: Boolean = false,
        eventEncodeNanos: Long? = null,
        nativeUpdateNanos: Long? = null,
    ) {
        val observer = onRequestsMeasured
        val start = if (observer != null) System.nanoTime() else 0L
        val decoded = Requests.bincodeDeserialize(requests).value
        val decodeNanos = if (observer != null) System.nanoTime() - start else 0L
        val source = if (fromCallback) CoreRenderSource.CALLBACK else CoreRenderSource.DISPATCH
        for (request in decoded) {
            when (request.effect) {
                is ShellEffect.Render -> renderView(source)

                is ShellEffect.AnkiLive -> {
                    val operation = request.effect.value
                    ankiLiveWorker.dispatch(request.id, operation)
                }
                is ShellEffect.AnkiExportPreflight -> {
                    val operation = request.effect.value
                    readAnki(request.id, { readAnkiPreflight(operation).bincodeSerialize() }) {
                        AnkiPreflightResult.Failed(AnkiExportError.PROVIDERFAILURE)
                            .bincodeSerialize()
                    }
                }
            }
        }
        onViewChanged?.invoke()
        observer?.invoke(
            CoreRequestsMeasurement(
                source,
                eventEncodeNanos,
                nativeUpdateNanos,
                decodeNanos,
                requests.size,
                decoded.size,
            )
        )
        dispatchReadyAnki()
        ankiBrowser.refresh()
        scheduleAnkiSetup()
    }

    /** Only an active native owner can expose Ready; dispatch consumes its permit. */
    private fun dispatchReadyAnki() {
        val export = view.ankiExport ?: return
        if (
            closing ||
                export.phase != AnkiExportPhase.Ready ||
                scheduledAnkiPreparation == export.preparationId
        )
            return
        scheduledAnkiPreparation = export.preparationId
        // Leave the render/observer stack before consuming the permit. Recheck the retained
        // owner on delivery so close or an intervening state change cannot launch queued work.
        mainHandler.post {
            if (
                !closing &&
                    !isClosed &&
                    view.ankiExport?.preparationId == export.preparationId &&
                    view.ankiExport?.phase == AnkiExportPhase.Ready
            ) {
                send(Event.DispatchAnkiLive)
                driveReaderClock()
            }
        }
    }

    /** Both read protocols share worker/main delivery and the retained handle's close gate. */
    private fun readAnki(id: UInt, read: () -> ByteArray, failed: () -> ByteArray) {
        if (closing) return
        ankiExecutor.execute {
            val result =
                try {
                    read()
                } catch (_: Exception) {
                    failed()
                }
            mainHandler.post {
                // Read callbacks never keep a closing handle alive or reach a restarted bridge.
                if (!closing && !isClosed) {
                    render(bridge.resolve(id, result))
                    driveReaderClock()
                }
            }
        }
    }

    private fun renderView(source: CoreRenderSource) {
        val observer = onRenderMeasured
        if (observer == null) {
            view = ViewModel.bincodeDeserialize(bridge.view())
            return
        }
        val start = System.nanoTime()
        val bytes = bridge.view()
        val retrieved = System.nanoTime()
        val decoded = ViewModel.bincodeDeserialize(bytes)
        val deserialized = System.nanoTime()
        view = decoded
        val published = System.nanoTime()
        observer(
            CoreRenderMeasurement(
                source,
                retrieved - start,
                deserialized - retrieved,
                published - deserialized,
                bytes.size,
                decoded.vocabulary.rows.size,
                decoded.vocabulary.selection?.size ?: 0,
                decoded.vocabulary.confirmation?.ids?.size ?: 0,
                view === decoded,
            )
        )
    }

    override fun close() {
        requireMainThread()
        if (!closing) {
            covers.close()
            images.close()
            pauseClock()
            closing = true
            ankiDecks.close()
            ankiBrowser.close()
            ankiLiveWorker.close()
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
