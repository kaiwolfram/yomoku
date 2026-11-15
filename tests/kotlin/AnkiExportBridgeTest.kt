package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/** Real JNI/SQLite ownership, with explicit fake provider responses and no Android provider. */
internal fun verifyAnkiExportBridge() {
    val directory = Files.createTempDirectory("yomoku-anki-export-jni-").toFile()
    val callbacks = LinkedBlockingQueue<ByteArray>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                callbacks.add(requests)
            }

            override fun onError(error: FfiError) {
                error("Unexpected bridge failure: $error")
            }
        }
    fun view(bridge: NativeBridge) = ViewModel.bincodeDeserialize(bridge.view())
    fun renders(bridge: NativeBridge, bytes: ByteArray) {
        for (request in Requests.bincodeDeserialize(bytes).value) {
            when (val effect = request.effect) {
                is ShellEffect.Render -> Unit
                is ShellEffect.AnkiExportPreflight -> {
                    // Deck saves now continue Settings setup. This host fixture has no provider.
                    check(effect.value.surface.isEmpty())
                    renders(
                        bridge,
                        bridge.resolve(
                            request.id,
                            AnkiPreflightResult.Failed(AnkiExportError.PROVIDERUNAVAILABLE)
                                .bincodeSerialize(),
                        ),
                    )
                }
                else -> error("Unexpected provider write: ${request.effect}")
            }
        }
    }
    fun send(bridge: NativeBridge, event: Event): List<Request> {
        val bytes = event.bincodeSerialize()
        check(Event.bincodeDeserialize(bytes) == event)
        return Requests.bincodeDeserialize(bridge.update(bytes)).value
    }
    fun awaitView(bridge: NativeBridge, predicate: (ViewModel) -> Boolean): ViewModel {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(60)
        while (true) {
            val snapshot = view(bridge)
            if (predicate(snapshot)) return snapshot
            val remaining = deadline - System.nanoTime()
            check(remaining > 0) { "Native Anki state timed out: ${snapshot.ankiExport}" }
            renders(bridge, checkNotNull(callbacks.poll(remaining, TimeUnit.NANOSECONDS)))
        }
    }
    try {
        for (create in listOf(false, true)) {
            callbacks.clear()
            val database = directory.resolve("export-$create.sqlite3")
            val documentId =
                NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
                    send(bridge, Event.RefreshLibrary)
                    awaitView(bridge) { !it.storageBusy }
                    send(bridge, Event.CreatePaste("猫と猫。"))
                    val documentId =
                        awaitView(bridge) { it.documents.size == 1 && !it.storageBusy }
                            .documents
                            .single()
                            .documentId
                            .value
                    send(bridge, Event.OpenReader(DocumentId(documentId)))
                    val opened = awaitView(bridge) { it.reader != null && !it.storageBusy }
                    val page = checkNotNull(opened.reader)
                    send(
                        bridge,
                        Event.LoadReaderTokens(
                            ReaderTokenKey(
                                DocumentId(documentId),
                                page.anchor,
                                checkNotNull(opened.readerAnalysis),
                            )
                        ),
                    )
                    val tokens =
                        checkNotNull(
                            awaitView(bridge) {
                                    it.effectiveReaderTokens != null && !it.storageBusy
                                }
                                .effectiveReaderTokens
                        )
                    val token = tokens.tokens.last { it.surface == "猫" }
                    val selected = EffectiveReaderSelection(tokens.identity, token.segmentIndex)
                    send(bridge, Event.LoadEffectiveReaderWord(selected))
                    val loaded =
                        awaitView(bridge) {
                            it.effectiveReaderWord?.details != null && !it.storageBusy
                        }
                    val before = checkNotNull(loaded.effectiveReaderWord?.details)
                    var prepare =
                        Event.PrepareReaderAnkiExport(AnkiSelection.Effective(selected, 1UL))
                    check(send(bridge, prepare).isEmpty())
                    check(view(bridge).ankiExport == null)
                    send(bridge, Event.OpenSettings)
                    send(bridge, Event.LoadAnkiSettings)
                    awaitView(bridge) { it.ankiSettings.phase == AnkiSettingsPhase.READY }
                    check(view(bridge).ankiSettings.defaultDeckId == null)
                    send(bridge, Event.CloseSettings)
                    send(bridge, Event.LoadEffectiveReaderWord(selected))
                    awaitView(bridge) { it.effectiveReaderWord?.details != null && !it.storageBusy }
                    check(send(bridge, prepare).isEmpty())
                    check(view(bridge).ankiExport == null)
                    send(bridge, Event.OpenSettings)
                    send(bridge, Event.SetAnkiDefaultDeck(Long.MAX_VALUE))
                    awaitView(bridge) { it.ankiSettings.phase == AnkiSettingsPhase.READY }
                    check(view(bridge).ankiSettings.defaultDeckId == Long.MAX_VALUE)
                    check(send(bridge, prepare).isEmpty()) // The selection is hidden in Settings.
                    send(bridge, Event.CloseSettings)
                    send(bridge, Event.LoadEffectiveReaderWord(selected))
                    awaitView(bridge) { it.effectiveReaderWord?.details != null && !it.storageBusy }
                    check(
                        send(bridge, prepare).isEmpty()
                    ) // A reopened lookup rejects the old revision.
                    check(view(bridge).ankiExport == null)
                    prepare =
                        Event.PrepareReaderAnkiExport(
                            AnkiSelection.Effective(
                                selected,
                                view(bridge).effectiveReaderWord!!.details!!.lookup.revision,
                            )
                        )
                    send(bridge, prepare)
                    val old = checkNotNull(view(bridge).ankiExport)
                    check(old.phase == AnkiExportPhase.Prepared && old.surface == "猫")
                    send(bridge, Event.ClearAnkiExport(old.preparationId))
                    send(bridge, prepare)
                    val frozen = checkNotNull(view(bridge).ankiExport)
                    check(frozen.preparationId > old.preparationId)
                    send(bridge, Event.ClearAnkiExport(old.preparationId))
                    check(view(bridge).ankiExport == frozen)
                    send(bridge, Event.OpenSettings)
                    send(bridge, Event.SetAnkiDefaultDeck(9007199254740993L))
                    awaitView(bridge) { it.ankiSettings.phase == AnkiSettingsPhase.READY }
                    send(bridge, Event.CloseSettings)
                    send(bridge, Event.LoadEffectiveReaderWord(selected))
                    awaitView(bridge) { it.effectiveReaderWord?.details != null && !it.storageBusy }
                    send(bridge, prepare)
                    check(view(bridge).ankiExport == frozen)
                    val preflight =
                        send(bridge, Event.BeginAnkiExportPreflight).single {
                            it.effect is ShellEffect.AnkiExportPreflight
                        }
                    val operation = (preflight.effect as ShellEffect.AnkiExportPreflight).value
                    check(operation.surface == "猫" && operation.deckId == Long.MAX_VALUE)
                    renders(
                        bridge,
                        bridge.resolve(
                            preflight.id,
                            AnkiPreflightResult.Complete(
                                    AnkiPreflightObservation(
                                        listOf(AnkiPreflightDeck(Long.MAX_VALUE, false)),
                                        emptyList(),
                                        if (create) emptyList()
                                        else
                                            listOf(
                                                AnkiPreflightModel(
                                                    9007199254740993L,
                                                    operation.layout.name,
                                                    true,
                                                )
                                            ),
                                    )
                                )
                                .bincodeSerialize(),
                        ),
                    )
                    val checked = checkNotNull(view(bridge).ankiExport)
                    check(
                        checked.phase ==
                            if (create) AnkiExportPhase.CreateModel
                            else AnkiExportPhase.ReuseModel(9007199254740993L)
                    )
                    check(
                        send(
                                bridge,
                                Event.BeginPreparedAnkiExport(old.preparationId, checked.phase),
                            )
                            .isEmpty()
                    )
                    check(
                        send(
                                bridge,
                                Event.BeginPreparedAnkiExport(
                                    checked.preparationId,
                                    AnkiExportPhase.Prepared,
                                ),
                            )
                            .isEmpty()
                    )
                    check(send(bridge, Event.DispatchAnkiLive).isEmpty())
                    send(
                        bridge,
                        Event.BeginPreparedAnkiExport(checked.preparationId, checked.phase),
                    )
                    awaitView(bridge) { it.ankiExport?.phase == AnkiExportPhase.Ready }
                    check(
                        send(
                                bridge,
                                Event.BeginPreparedAnkiExport(checked.preparationId, checked.phase),
                            )
                            .isEmpty()
                    )
                    send(bridge, Event.ClearAnkiExport(checked.preparationId))
                    check(view(bridge).ankiExport?.phase == AnkiExportPhase.Ready)
                    val live =
                        send(bridge, Event.DispatchAnkiLive).single {
                            it.effect is ShellEffect.AnkiLive
                        }
                    check(send(bridge, Event.DispatchAnkiLive).isEmpty())
                    val liveOperation = (live.effect as ShellEffect.AnkiLive).value
                    val result =
                        if (create) {
                            check(
                                liveOperation ==
                                    AnkiLiveOperation.CreateModel(
                                        "猫",
                                        operation.reading,
                                        Long.MAX_VALUE,
                                        operation.layout,
                                    )
                            )
                            AnkiLiveResult.ModelCreated(9007199254740993L)
                        } else {
                            liveOperation as AnkiLiveOperation.InsertNote
                            check(liveOperation.fields[0] == "猫")
                            check(
                                liveOperation.fields[3] ==
                                    "猫と<strong class=\"yomoku-target\">猫</strong>。"
                            )
                            AnkiLiveResult.NoteUncertain(Long.MAX_VALUE)
                        }
                    renders(bridge, bridge.resolve(live.id, result.bincodeSerialize()))
                    val expected =
                        if (create) AnkiExportPhase.ModelCreated(9007199254740993L)
                        else AnkiExportPhase.NoteUncertain(Long.MAX_VALUE)
                    val finished = awaitView(bridge) { it.ankiExport?.phase == expected }
                    val finalWord = checkNotNull(finished.effectiveReaderWord?.details)
                    check(
                        finalWord.copy(
                            lookup = finalWord.lookup.copy(revision = before.lookup.revision)
                        ) == before
                    )
                    check(
                        finished.reader == loaded.reader &&
                            finished.completionNotice == loaded.completionNotice
                    )
                    check(send(bridge, Event.DispatchAnkiLive).isEmpty())
                    documentId
                }
            callbacks.clear()
            NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
                send(bridge, Event.RefreshLibrary)
                awaitView(bridge) { it.documents.size == 1 && !it.storageBusy }
                check(view(bridge).ankiExport == null)
                check(send(bridge, Event.DispatchAnkiLive).isEmpty())
                check(
                    send(bridge, Event.BeginPreparedAnkiExport(2UL, AnkiExportPhase.CreateModel))
                        .isEmpty()
                )
                check(view(bridge).ankiSettings.phase == AnkiSettingsPhase.UNLOADED)
                send(bridge, Event.OpenSettings)
                send(bridge, Event.LoadAnkiSettings)
                awaitView(bridge) { it.ankiSettings.phase == AnkiSettingsPhase.READY }
                check(view(bridge).ankiSettings.defaultDeckId == 9007199254740993L)
                send(bridge, Event.CloseSettings)
                send(bridge, Event.OpenReader(DocumentId(documentId)))
                awaitView(bridge) { it.reader != null && !it.storageBusy }
                check(view(bridge).ankiExport == null)
            }
        }
    } finally {
        check(directory.deleteRecursively())
    }
    println(
        "PASS: native configured-deck preparation, acknowledged-only targets, frozen fields/deck, in-memory model/note outcomes, one dispatch and restart without replay"
    )
}
