package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.io.File
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal fun verifyAnkiSettingsBridge(fixtures: String) {
    val root = File(fixtures)
    val entrySelection =
        AnkiSelection.Effective(
            EffectiveReaderSelection(
                EffectivePageIdentity(
                    ULong.MAX_VALUE,
                    ReaderTokenKey(
                        DocumentId(Long.MAX_VALUE),
                        TextAnchor(
                            SourceId(9007199254740993L),
                            ChapterId(Long.MAX_VALUE),
                            TextRange(TextOffset(0), TextOffset(3)),
                        ),
                        ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE)),
                    ),
                ),
                ULong.MAX_VALUE,
            )
        )
    val entryActions =
        listOf(
            Event.CheckAnkiSetup(AnkiSetupRequest.REFRESH),
            Event.CheckAnkiSetup(AnkiSetupRequest.CONFIGURE),
            Event.LoadReaderAnkiEntries(entrySelection),
            Event.AddReaderAnkiEntry(
                entrySelection,
                AnkiEntryKey.Word(Long.MAX_VALUE, ReadingMatch(0UL, 0UL)),
            ),
            Event.AddReaderAnkiEntry(
                entrySelection,
                AnkiEntryKey.Name(9007199254740993L, ReadingMatch(0UL, 0UL)),
            ),
            Event.AddReaderAnkiEntry(
                entrySelection,
                AnkiEntryKey.Word(1, ReadingMatch(0UL, 0UL)),
            ),
        )
    for ((index, expected) in entryActions.withIndex()) {
        val bytes = root.resolve("anki-entry-action-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(bytes)
        check(event == expected)
        root.resolve("anki-entry-action-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
    val entryPhases =
        listOf(
            AnkiEntryPhase.Checking,
            AnkiEntryPhase.Add,
            AnkiEntryPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L)),
            AnkiEntryPhase.Export(AnkiExportPhase.NoteUncertain(Long.MAX_VALUE)),
            AnkiEntryPhase.Failed(AnkiExportError.PERMISSIONDENIED),
        )
    for ((index, phase) in entryPhases.withIndex()) {
        val view =
            AnkiEntryView.bincodeDeserialize(root.resolve("anki-entry-view-$index.bin").readBytes())
        check(
            view ==
                AnkiEntryView(
                    entrySelection,
                    AnkiEntryKey.Word(Long.MAX_VALUE, ReadingMatch(0UL, 0UL)),
                    "𠮷<&",
                    "よし",
                    phase,
                )
        )
        root.resolve("anki-entry-view-$index-return.bin").writeBytes(view.bincodeSerialize())
    }
    val events =
        listOf(
            Event.LoadAnkiSettings,
            Event.RetryAnkiSettings,
            Event.SetAnkiDefaultDeck(null),
            Event.SetAnkiDefaultDeck(9007199254740993L),
            Event.SetAnkiDefaultDeck(Long.MAX_VALUE),
            Event.SetAnkiDefaultDeck(0),
            Event.SetAnkiDefaultDeck(Long.MIN_VALUE),
        )
    for ((index, expected) in events.withIndex()) {
        val bytes = root.resolve("anki-settings-action-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(bytes)
        check(event == expected)
        check(event.bincodeSerialize().contentEquals(bytes))
        root.resolve("anki-settings-action-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
    for ((index, phase) in AnkiSettingsPhase.entries.withIndex()) {
        val bytes = root.resolve("anki-settings-view-$index.bin").readBytes()
        val view = AnkiSettingsView.bincodeDeserialize(bytes)
        check(
            view ==
                AnkiSettingsView(
                    phase,
                    if (index < 2) null else Long.MAX_VALUE,
                    if (phase == AnkiSettingsPhase.FAILED) DocumentError.INVALIDSETTINGS else null,
                )
        )
        root.resolve("anki-settings-view-$index-return.bin").writeBytes(view.bincodeSerialize())
    }
    val directory = Files.createTempDirectory("yomoku-anki-settings-jni-").toFile()
    val callbacks = LinkedBlockingQueue<ByteArray>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                callbacks.add(requests)
            }

            override fun onError(error: FfiError) {
                error("Unexpected bridge error: $error")
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
    fun send(bridge: NativeBridge, event: Event) {
        renders(bridge, bridge.update(event.bincodeSerialize()))
    }
    fun awaitReady(bridge: NativeBridge, expected: Long?): ViewModel {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(30)
        while (true) {
            val view = view(bridge)
            if (view.ankiSettings.phase == AnkiSettingsPhase.READY) {
                check(view.ankiSettings.defaultDeckId == expected)
                check(view.ankiSettings.storageError == null)
                return view
            }
            val remaining = deadline - System.nanoTime()
            check(remaining > 0) { "Settings timed out: ${view.ankiSettings}" }
            renders(bridge, checkNotNull(callbacks.poll(remaining, TimeUnit.NANOSECONDS)))
        }
    }
    try {
        for (kind in listOf("settings")) {
            val database = directory.resolve("$kind.sqlite3")
            NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
                check(view(bridge).ankiSettings.phase == AnkiSettingsPhase.UNLOADED)
                send(bridge, Event.LoadAnkiSettings)
                awaitReady(bridge, null)
                send(bridge, Event.OpenSettings)
                val before = view(bridge)
                send(bridge, Event.LoadAnkiSettings)
                send(bridge, Event.LoadAnkiSettings)
                awaitReady(bridge, null)
                for (invalid in listOf(0L, -1L, Long.MIN_VALUE)) send(
                    bridge,
                    Event.SetAnkiDefaultDeck(invalid),
                )
                check(view(bridge).ankiSettings.defaultDeckId == null)
                send(bridge, Event.SetAnkiDefaultDeck(Long.MAX_VALUE))
                val saved = awaitReady(bridge, Long.MAX_VALUE)
                check(
                    saved.copy(ankiSettings = before.ankiSettings, ankiSetup = before.ankiSetup) ==
                        before
                )
                send(bridge, Event.CloseSettings)
                send(bridge, Event.OpenSettings)
                send(bridge, Event.LoadAnkiSettings)
                check(view(bridge).ankiSettings.defaultDeckId == Long.MAX_VALUE)
            }
            callbacks.clear()
            NativeBridge(database.path, listener, HostDictionary()).use { bridge ->
                check(view(bridge).ankiSettings.phase == AnkiSettingsPhase.UNLOADED)
                send(bridge, Event.OpenSettings)
                send(bridge, Event.LoadAnkiSettings)
                awaitReady(bridge, Long.MAX_VALUE)
                send(bridge, Event.SetAnkiDefaultDeck(9007199254740993L))
                awaitReady(bridge, 9007199254740993L)
                send(bridge, Event.SetAnkiDefaultDeck(null))
                awaitReady(bridge, null)
            }
        }
    } finally {
        directory.deleteRecursively()
    }
}
