package app.yomoku

import android.content.ContentValues
import android.content.Intent
import android.net.Uri
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Opt in only on an isolated emulator with a disposable, initialized Anki collection. */
@RunWith(AndroidJUnit4::class)
class AnkiInstalledProviderTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var core: CoreSession

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private fun send(event: Event) = main { core.dispatch(event) }

    private fun view(): ViewModel {
        lateinit var value: ViewModel
        main { value = core.view }
        return value
    }

    private fun await(predicate: (ViewModel) -> Boolean): ViewModel {
        val deadline = System.nanoTime() + 90_000_000_000L
        while (true) {
            val value = view()
            if (predicate(value)) return value
            check(System.nanoTime() < deadline) {
                "Installed Anki trial timed out: setup=${value.ankiSetup}, entries=${value.ankiEntries}, error=${value.documentError}"
            }
            Thread.sleep(20)
        }
    }

    private fun <T> value(result: AnkiReadResult<T>): T {
        check(result is AnkiReadResult.Value) { "Installed Anki provider failed: $result" }
        return result.value
    }

    private fun close() {
        main { core.close() }
        val deadline = System.nanoTime() + 30_000_000_000L
        while (!core.isClosed) {
            check(System.nanoTime() < deadline)
            Thread.sleep(20)
        }
    }

    @Test
    fun realCollectionAddRefreshExternalChangesAndRestartWithoutReplay() {
        assumeTrue(
            InstrumentationRegistry.getArguments().getString("disposableAnkiTrial") == "true"
        )
        val context = instrumentation.targetContext
        val provider =
            AnkiProviderReader(context.contentResolver, AndroidAnkiProviderEnvironment(context))
        val deck = value(provider.decks()).first { !it.filtered }.id
        assertTrue(
            "Use a fresh disposable collection with no cat notes",
            value(provider.duplicates("猫", "ねこ")).isEmpty(),
        )
        val directory =
            File(context.noBackupFilesDir, "installed-anki-${System.nanoTime()}").apply { mkdirs() }
        val database = File(directory, "db.sqlite3")
        val writes = AtomicInteger()
        val inserted = AtomicReference<AnkiLiveOperation.InsertNote>()
        val dispatcher = AnkiLiveDispatcher(context)
        fun open() {
            main {
                core =
                    CoreSession(
                        database.path,
                        sudachiDictionaryProvider(context),
                        readAnkiPreflight = AnkiPreflightReader(provider)::read,
                        readAnkiDecks = provider::decks,
                        resolveAnkiNote = provider::existingNoteIntent,
                        resolveAnkiNotes = provider::existingNotesIntent,
                        invokeAnkiLive = {
                            if (it is AnkiLiveOperation.InsertNote) {
                                writes.incrementAndGet()
                                inserted.set(it)
                            }
                            dispatcher.invoke(it)
                        },
                    )
            }
            await { !it.storageBusy && it.ankiSettings.phase == AnkiSettingsPhase.READY }
        }
        fun select(id: DocumentId): AnkiSelection {
            send(Event.OpenReader(id))
            val opened = await { !it.storageBusy && it.reader != null }
            send(
                Event.LoadReaderTokens(
                    ReaderTokenKey(id, opened.reader!!.anchor, opened.readerAnalysis!!)
                )
            )
            val page =
                await { !it.storageBusy && it.effectiveReaderTokens != null }
                    .effectiveReaderTokens!!
            val selection =
                EffectiveReaderSelection(
                    page.identity,
                    page.tokens.first { it.surface == "猫" }.segmentIndex,
                )
            send(Event.LoadEffectiveReaderWord(selection))
            await { !it.storageBusy && it.effectiveReaderWord?.details != null }
            return AnkiSelection.Effective(selection)
        }
        fun refresh() {
            main {
                core.setForeground(false)
                core.setForeground(true)
                core.refreshAnkiSetup()
            }
            instrumentation.waitForIdleSync()
            await { it.ankiSetup == AnkiSetupPhase.Ready && it.ankiEntries.isNotEmpty() }
        }
        fun phase(predicate: (AnkiEntryPhase) -> Boolean): AnkiEntryView = await {
            it.ankiEntries.any { entry -> entry.headword == "猫" && predicate(entry.phase) }
        }
            .ankiEntries
            .first { it.headword == "猫" && predicate(it.phase) }
        open()
        try {
            send(Event.OpenSettings)
            send(Event.SetAnkiDefaultDeck(deck))
            val setup = await {
                it.ankiSetup == AnkiSetupPhase.Ready ||
                    it.ankiSetup == AnkiSetupPhase.Missing ||
                    it.ankiSetup is AnkiSetupPhase.Failed
            }
            check(setup.ankiSetup !is AnkiSetupPhase.Failed) {
                "Real provider setup failed: ${setup.ankiSetup}"
            }
            if (setup.ankiSetup == AnkiSetupPhase.Missing) send(Event.CreateAnkiModel)
            await { it.ankiSetup == AnkiSetupPhase.Ready }
            send(Event.CloseSettings)
            send(Event.CreatePaste("猫。"))
            val id = await {
                !it.storageBusy && it.documents.size == 1
            }
                .documents
                .single()
                .documentId
            val selection = select(id)
            send(Event.LoadReaderAnkiEntries(selection))
            val entry = phase { it == AnkiEntryPhase.Add }
            main {
                core.dispatch(Event.AddReaderAnkiEntry(selection, entry.key))
                core.dispatch(Event.AddReaderAnkiEntry(selection, entry.key))
            }
            val existing = phase { it is AnkiEntryPhase.Existing }.phase as AnkiEntryPhase.Existing
            assertEquals(1, writes.get())
            val noteId = existing.value.single()
            assertEquals(listOf(AnkiProviderNoteId(noteId)), value(provider.duplicates("猫", "ねこ")))
            val intent =
                checkNotNull(value(provider.existingNoteIntent(AnkiProviderNoteId(noteId))))
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            // The real provider must permit identical first fields when readings differ.
            val template = checkNotNull(inserted.get())
            val trialNotes = mutableListOf<Uri>()
            try {
                for (reading in listOf("せい", "なま")) {
                    assertTrue(value(provider.duplicates("生", reading)).isEmpty())
                    val operation =
                        template.copy(
                            surface = "生",
                            reading = reading,
                            fields =
                                listOf(
                                    "生",
                                    reading,
                                    "<ol><li>life</li></ol>",
                                    "<strong>生</strong>。",
                                ),
                        )
                    val added = dispatcher.invoke(operation) as AnkiLiveResult.NoteAdded
                    trialNotes.add(
                        Uri.parse("content://${AnkiProviderReader.AUTHORITY}/notes/${added.value}")
                    )
                    assertEquals(
                        listOf(AnkiProviderNoteId(added.value)),
                        value(provider.duplicates("生", reading)),
                    )
                    assertEquals(
                        AnkiLiveResult.NoteExisting(listOf(added.value)),
                        dispatcher.invoke(operation),
                    )
                }
                assertNotEquals(
                    value(provider.duplicates("生", "せい")),
                    value(provider.duplicates("生", "なま")),
                )
            } finally {
                trialNotes.forEach { context.contentResolver.delete(it, null, null) }
            }
            val noteUri = Uri.parse("content://${AnkiProviderReader.AUTHORITY}/notes/$noteId")
            assertTrue(context.contentResolver.delete(noteUri, null, null) > 0)
            refresh()
            phase { it == AnkiEntryPhase.Add }
            assertTrue(value(provider.duplicates("猫", "ねこ")).isEmpty())
            val frozen = checkNotNull(inserted.get())
            val external =
                checkNotNull(
                    context.contentResolver.insert(
                        Uri.parse("content://${AnkiProviderReader.AUTHORITY}/notes"),
                        ContentValues().apply {
                            put("mid", frozen.modelId)
                            put("flds", frozen.fields.joinToString("\u001f"))
                            put("tags", "yomoku-disposable-trial")
                        },
                    )
                )
            refresh()
            phase { it is AnkiEntryPhase.Existing }
            close()
            open()
            val reopened = select(id)
            send(Event.LoadReaderAnkiEntries(reopened))
            phase { it is AnkiEntryPhase.Existing }
            assertEquals(1, writes.get())
            assertTrue(context.contentResolver.delete(external, null, null) > 0)
        } finally {
            close()
            directory.deleteRecursively()
        }
    }
}
