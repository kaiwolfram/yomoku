package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.io.File
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyAnkiRecoveryBridge(fixtures: String) {
    verifyAnkiExportBridge()
    val root = File(fixtures)
    for (index in 0..4) {
        val bytes = root.resolve("anki-action-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(bytes)
        when (event) {
            is Event.PrepareReaderAnkiExport -> {
                check(event.deckId == Long.MAX_VALUE)
                when (val selected = event.selection) {
                    is AnkiSelection.Original -> {
                        check(index == 0 && selected.value.tokenIndex == ULong.MAX_VALUE)
                        check(selected.value.key.documentId.value == Long.MAX_VALUE)
                    }
                    is AnkiSelection.Effective -> {
                        check(index == 1 && selected.value.segmentIndex == ULong.MAX_VALUE)
                        check(selected.value.page.revision == ULong.MAX_VALUE)
                        check(selected.value.page.base.documentId.value == Long.MAX_VALUE)
                    }
                }
            }
            is Event.BeginPreparedAnkiExport -> {
                check(event.preparationId == ULong.MAX_VALUE)
                check(
                    event.expectedPhase ==
                        if (index == 2) AnkiExportPhase.CreateModel
                        else AnkiExportPhase.ReuseModel(9007199254740993L)
                )
            }
            is Event.ClearAnkiExport -> check(index == 4 && event.preparationId == ULong.MAX_VALUE)
            else -> error("Unexpected Anki action")
        }
        check(event.bincodeSerialize().contentEquals(bytes))
        root.resolve("anki-action-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
    val exportPhases =
        listOf(
            AnkiExportPhase.Prepared,
            AnkiExportPhase.Checking,
            AnkiExportPhase.CreateModel,
            AnkiExportPhase.ReuseModel(9007199254740993L),
            AnkiExportPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L)),
            AnkiExportPhase.Beginning,
            AnkiExportPhase.Ready,
            AnkiExportPhase.Invoking,
            AnkiExportPhase.Saving,
            AnkiExportPhase.NoteAdded(Long.MAX_VALUE),
            AnkiExportPhase.ModelCreated(9007199254740993L),
            AnkiExportPhase.NotSubmitted,
            AnkiExportPhase.NoteUncertain(null),
            AnkiExportPhase.NoteUncertain(Long.MAX_VALUE),
            AnkiExportPhase.ModelUncertain(null),
            AnkiExportPhase.ModelUncertain(9007199254740993L),
            AnkiExportPhase.ProviderFailed(AnkiExportError.INCOMPATIBLEMODEL),
            AnkiExportPhase.StorageFailed(DocumentError.UNAVAILABLE),
        )
    for ((index, phase) in exportPhases.withIndex()) {
        val view =
            AnkiExportView.bincodeDeserialize(
                root.resolve("anki-export-view-$index.bin").readBytes()
            )
        check(view == AnkiExportView(ULong.MAX_VALUE, "𠮷猫", Long.MAX_VALUE, phase))
        root.resolve("anki-export-view-$index-return.bin").writeBytes(view.bincodeSerialize())
    }
    for ((index, phase) in AnkiRecoveryPhase.entries.withIndex()) {
        val view =
            AnkiRecoveryView.bincodeDeserialize(
                root.resolve("anki-recovery-view-$index.bin").readBytes()
            )
        check(
            view ==
                AnkiRecoveryView(
                    phase,
                    index % 2 == 0,
                    index % 2 != 0,
                    DocumentError.UNAVAILABLE,
                    AnkiExportError.PERMISSIONDENIED,
                    index % 2 == 0,
                    false,
                    index % 2 != 0,
                )
        )
        root.resolve("anki-recovery-view-$index-return.bin").writeBytes(view.bincodeSerialize())
    }
    val frozen = AnkiReadModelLayout.bincodeDeserialize(root.resolve("anki-layout.bin").readBytes())
    root.resolve("anki-layout-return.bin").writeBytes(frozen.bincodeSerialize())
    val preflight =
        AnkiPreflightOperation.bincodeDeserialize(
            root.resolve("anki-preflight-operation.bin").readBytes()
        )
    check(preflight == AnkiPreflightOperation("𠮷猫", Long.MAX_VALUE, frozen))
    root.resolve("anki-preflight-operation-return.bin").writeBytes(preflight.bincodeSerialize())
    for (i in 0..1) {
        val result =
            AnkiPreflightResult.bincodeDeserialize(
                root.resolve("anki-preflight-result-$i.bin").readBytes()
            )
        if (i == 0) {
            val observed = (result as AnkiPreflightResult.Complete).value
            check(observed.decks == listOf(AnkiPreflightDeck(Long.MAX_VALUE, false)))
            check(observed.duplicateNoteIds == listOf(Long.MAX_VALUE, 9007199254740993L))
            check(
                observed.models == listOf(AnkiPreflightModel(9007199254740993L, frozen.name, true))
            )
        } else check(result == AnkiPreflightResult.Failed(AnkiExportError.PERMISSIONDENIED))
        root.resolve("anki-preflight-result-$i-return.bin").writeBytes(result.bincodeSerialize())
    }
    for (i in 0..1) {
        val operation =
            AnkiLiveOperation.bincodeDeserialize(
                root.resolve("anki-live-operation-$i.bin").readBytes()
            )
        if (operation is AnkiLiveOperation.InsertNote) {
            check(
                operation.surface == "𠮷猫" &&
                    operation.deckId == Long.MAX_VALUE &&
                    operation.modelId == 9007199254740993L
            )
            check(operation.layout == frozen && operation.fields.size == 4)
        } else check(operation == AnkiLiveOperation.CreateModel("𠮷猫", Long.MAX_VALUE, frozen))
        root.resolve("anki-live-operation-$i-return.bin").writeBytes(operation.bincodeSerialize())
    }
    val liveResults =
        listOf(
            AnkiLiveResult.NoSubmission,
            AnkiLiveResult.NoteAdded(Long.MAX_VALUE),
            AnkiLiveResult.NoteExisting(listOf(Long.MAX_VALUE, 9007199254740993L)),
            AnkiLiveResult.NoteUncertain(null),
            AnkiLiveResult.NoteUncertain(Long.MAX_VALUE),
            AnkiLiveResult.ModelCreated(9007199254740993L),
            AnkiLiveResult.ModelUncertain(null),
            AnkiLiveResult.ModelUncertain(Long.MAX_VALUE),
        )
    for ((i, expected) in liveResults.withIndex()) {
        val result =
            AnkiLiveResult.bincodeDeserialize(root.resolve("anki-live-result-$i.bin").readBytes())
        check(result == expected)
        root.resolve("anki-live-result-$i-return.bin").writeBytes(result.bincodeSerialize())
    }
    for (model in listOf(false, true)) {
        val name = if (model) "model" else "note"
        val database = root.resolve("anki-$name-jni.sqlite3")
        root.resolve("anki-$name-seed.sqlite3").copyTo(database, overwrite = true)
        val renders = LinkedBlockingQueue<Unit>()
        val listener =
            object : EffectsListener {
                override fun onEffects(requests: ByteArray) {
                    for (request in Requests.bincodeDeserialize(requests).value) {
                        check(request.effect is ShellEffect.Render) // No automatic provider reads.
                        renders.add(Unit)
                    }
                }

                override fun onError(error: FfiError) {
                    throw error
                }
            }
        val target = if (model) AnkiRecoveryReadTarget.MODEL else AnkiRecoveryReadTarget.NOTE
        val event = Event.BeginAnkiRecoveryRead(target)
        fun begin(bridge: NativeBridge): Request {
            val bytes = bridge.update(event.bincodeSerialize())
            val requests = Requests.bincodeDeserialize(bytes)
            check(requests.bincodeSerialize().contentEquals(bytes))
            return requests.value.single().also {
                val operation = (it.effect as ShellEffect.AnkiRecoveryRead).value
                if (model) {
                    check(operation == AnkiRecoveryReadOperation.Model(null, frozen))
                } else {
                    check(operation == AnkiRecoveryReadOperation.Note("𠮷猫"))
                }
            }
        }
        fun barrier(bridge: NativeBridge) {
            bridge.update(Event.RefreshLibrary.bincodeSerialize())
            checkNotNull(renders.poll(60, TimeUnit.SECONDS))
        }
        fun resolve(bridge: NativeBridge, request: Request, result: AnkiRecoveryReadResult) {
            val bytes = result.bincodeSerialize()
            check(AnkiRecoveryReadResult.bincodeDeserialize(bytes) == result)
            check(Requests.bincodeDeserialize(bridge.resolve(request.id, bytes)).value.isEmpty())
        }
        NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
            check(
                Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.isEmpty()
            )
            barrier(bridge)
            for (preflightEvent in
                listOf(
                    Event.DispatchAnkiLive,
                    Event.BeginAnkiExportPreflight,
                    Event.RetryAnkiExportPreflight,
                )) {
                check(
                    Requests.bincodeDeserialize(bridge.update(preflightEvent.bincodeSerialize()))
                        .value
                        .isEmpty()
                )
            }
            val lost = begin(bridge)
            check(
                Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.isEmpty()
            )
            bridge.update(Event.RetryAnkiRecovery.bincodeSerialize())
            barrier(bridge)
            val fresh = begin(bridge)
            resolve(
                bridge,
                lost,
                if (model) AnkiRecoveryReadResult.Model(Long.MAX_VALUE)
                else AnkiRecoveryReadResult.Notes(listOf(Long.MAX_VALUE)),
            )
            check(
                Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.isEmpty()
            )
            resolve(bridge, fresh, AnkiRecoveryReadResult.Failed(AnkiExportError.PERMISSIONDENIED))
            barrier(bridge)
            val empty = begin(bridge)
            resolve(
                bridge,
                empty,
                if (model) AnkiRecoveryReadResult.Model(null)
                else AnkiRecoveryReadResult.Notes(emptyList()),
            )
            barrier(bridge)
            begin(bridge) // Lose a read on close: restart must retain the obligation.
        }
        NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
            barrier(bridge)
            val request = begin(bridge)
            val result =
                if (model) AnkiRecoveryReadResult.Model(9007199254740993L)
                else AnkiRecoveryReadResult.Notes(listOf(Long.MAX_VALUE))
            resolve(bridge, request, result)
            barrier(bridge)
            check(
                Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize())).value.isEmpty()
            )
        }
    }
    println(
        "PASS: Anki read-only JNI dispatch, frozen layout/Unicode/long IDs, lost/stale reads, permission/empty results, restart and durable recovery commits"
    )
}
