package app.yomoku.tests

import app.yomoku.shared.*
import java.io.File

internal fun verifyAnkiProtocol(fixtures: String) {
    verifyAnkiExportBridge()
    val root = File(fixtures)
    for (index in 0..5) {
        val bytes = root.resolve("anki-action-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(bytes)
        when (event) {
            is Event.PrepareReaderAnkiExport -> {
                when (val selected = event.selection) {
                    is AnkiSelection.Vocabulary -> {
                        check(index == 5 && selected.value.vocabularyId.value == Long.MAX_VALUE)
                        check(selected.value.generation == ULong.MAX_VALUE)
                    }
                    is AnkiSelection.Effective -> {
                        check(index in 0..1 && selected.selection.segmentIndex == ULong.MAX_VALUE)
                        check(
                            selected.selection.page.revision ==
                                if (index == 0) 1UL else ULong.MAX_VALUE
                        )
                        check(selected.selection.page.base.documentId.value == Long.MAX_VALUE)
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
            AnkiExportPhase.Ready,
            AnkiExportPhase.Invoking,
            AnkiExportPhase.NoteAdded(Long.MAX_VALUE),
            AnkiExportPhase.ModelCreated(9007199254740993L),
            AnkiExportPhase.NotSubmitted,
            AnkiExportPhase.NoteUncertain(null),
            AnkiExportPhase.NoteUncertain(Long.MAX_VALUE),
            AnkiExportPhase.ModelUncertain(null),
            AnkiExportPhase.ModelUncertain(9007199254740993L),
            AnkiExportPhase.ProviderFailed(AnkiExportError.INCOMPATIBLEMODEL),
        )
    for ((index, phase) in exportPhases.withIndex()) {
        val view =
            AnkiExportView.bincodeDeserialize(
                root.resolve("anki-export-view-$index.bin").readBytes()
            )
        check(view == AnkiExportView(ULong.MAX_VALUE, "𠮷猫", Long.MAX_VALUE, phase))
        root.resolve("anki-export-view-$index-return.bin").writeBytes(view.bincodeSerialize())
    }

    val frozen = AnkiReadModelLayout.bincodeDeserialize(root.resolve("anki-layout.bin").readBytes())
    root.resolve("anki-layout-return.bin").writeBytes(frozen.bincodeSerialize())
    val preflight =
        AnkiPreflightOperation.bincodeDeserialize(
            root.resolve("anki-preflight-operation.bin").readBytes()
        )
    check(preflight == AnkiPreflightOperation("𠮷猫", "よしねこ", Long.MAX_VALUE, frozen))
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
                    operation.reading == "よしねこ" &&
                    operation.deckId == Long.MAX_VALUE &&
                    operation.modelId == 9007199254740993L
            )
            check(operation.layout == frozen && operation.fields.size == 4)
        } else
            check(operation == AnkiLiveOperation.CreateModel("𠮷猫", "よしねこ", Long.MAX_VALUE, frozen))
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

    println("PASS: Anki typed provider codecs and in-memory write ownership")
}
