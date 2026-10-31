package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.io.File
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyAnkiRecoveryBridge(fixtures: String) {
    val root = File(fixtures)
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
                listOf(Event.BeginAnkiExportPreflight, Event.RetryAnkiExportPreflight)) {
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
