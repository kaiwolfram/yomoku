package app.yomoku

import android.content.ContentProvider
import android.content.ContentResolver
import android.content.ContentValues
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.ProviderInfo
import android.database.Cursor
import android.database.CursorWrapper
import android.database.MatrixCursor
import android.database.sqlite.SQLiteDatabase
import android.net.Uri
import android.os.Bundle
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.AnkiExportError
import app.yomoku.shared.AnkiLiveOperation
import app.yomoku.shared.AnkiLiveResult
import app.yomoku.shared.AnkiPreflightDeck
import app.yomoku.shared.AnkiPreflightModel
import app.yomoku.shared.AnkiPreflightOperation
import app.yomoku.shared.AnkiPreflightResult
import app.yomoku.shared.AnkiReadModelLayout
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiProviderReaderTest {
    private class Environment : AnkiProviderEnvironment {
        var info: ProviderInfo? =
            ProviderInfo().apply {
                authority = AnkiProviderReader.AUTHORITY
                packageName = "test.anki.host"
                enabled = true
                exported = true
                applicationInfo = ApplicationInfo().apply { enabled = true }
                metaData = Bundle().apply { putInt(AnkiProviderReader.SPEC_KEY, 2) }
            }
        var permitted = true
        var canOpen = true
        var resolved: Intent? = null

        override fun provider() = info

        override fun hasPermission() = permitted

        override fun resolves(intent: Intent): Boolean {
            resolved = intent
            return canOpen
        }
    }

    private class Provider : ContentProvider(), AutoCloseable {
        val database =
            SQLiteDatabase.create(null).apply {
                execSQL(
                    "CREATE TABLE notes(id INTEGER PRIMARY KEY, mid INTEGER, flds TEXT, did INTEGER)"
                )
                execSQL("CREATE TABLE decks(deck_id INTEGER, deck_name TEXT, deck_dyn INTEGER)")
                execSQL("CREATE TABLE cards(note_id INTEGER, ord INTEGER, deck_id INTEGER)")
            }
        var queries = 0
        var replacement: (() -> Cursor?)? = null
        var lastUri: Uri? = null
        var lastProjection: List<String>? = null
        var lastSelection: String? = null
        var lastArguments: List<String>? = null
        var lastNoteProjection: List<String>? = null
        var lastNoteUri: Uri? = null
        var lastNoteSelection: String? = null
        var lastNoteArguments: List<String>? = null
        val models = linkedMapOf<Long, Array<Any?>>()
        val templates = linkedMapOf<Long, List<Array<Any?>>>()
        val inserts = mutableListOf<Pair<Uri, ContentValues>>()
        val updates = mutableListOf<Pair<Uri, ContentValues>>()
        var insertReplacement: ((Uri, ContentValues) -> Uri?)? = null
        var updateReplacement: ((Uri, ContentValues) -> Int)? = null
        var beforeQuery: ((Uri) -> Unit)? = null
        var queryResult: ((Uri, Cursor?) -> Cursor?)? = null
        val operations = mutableListOf<String>()

        override fun onCreate() = true

        override fun getType(uri: Uri) = "vnd.android.cursor.dir/test"

        override fun query(
            uri: Uri,
            projection: Array<out String>?,
            selection: String?,
            args: Array<out String>?,
            order: String?,
        ): Cursor? {
            check(Looper.myLooper() != Looper.getMainLooper())
            queries++
            lastUri = uri
            lastProjection = projection?.toList()
            lastSelection = selection
            lastArguments = args?.toList()
            if (uri.lastPathSegment == "notes_v2") {
                lastNoteProjection = projection?.toList()
                lastNoteUri = uri
                lastNoteSelection = selection
                lastNoteArguments = args?.toList()
            }
            operations.add("query:${uri.path}")
            beforeQuery?.invoke(uri)
            val cursor = queryRows(uri, projection, selection, args, order)
            val transform = queryResult
            return if (transform == null) cursor else transform(uri, cursor)
        }

        private fun queryRows(
            uri: Uri,
            projection: Array<out String>?,
            selection: String?,
            args: Array<out String>?,
            order: String?,
        ): Cursor? {
            replacement?.let {
                return it()
            }
            if (uri.pathSegments.first() == "notes") {
                val noteId = uri.pathSegments[1]
                return if (uri.lastPathSegment == "cards") {
                    database.query(
                        "cards",
                        projection,
                        "note_id = ?",
                        arrayOf(noteId),
                        null,
                        null,
                        null,
                    )
                } else {
                    val columns =
                        projection?.map { if (it == "_id") "id AS _id" else it }?.toTypedArray()
                    database.query("notes", columns, "id = ?", arrayOf(noteId), null, null, null)
                }
            }
            if (uri.pathSegments.first() == "models") {
                val isTemplate = uri.lastPathSegment == "templates"
                val columns = if (isTemplate) TEMPLATE_COLUMNS else MODEL_COLUMNS
                val rows =
                    if (isTemplate) templates[uri.pathSegments[1].toLong()].orEmpty()
                    else if (uri.pathSegments.size == 1) models.values.toList()
                    else listOfNotNull(models[uri.pathSegments[1].toLong()])
                val requested = projection ?: columns
                return MatrixCursor(requested).apply {
                    rows.forEach { row ->
                        addRow(requested.map { row[columns.indexOf(it)] }.toTypedArray())
                    }
                }
            }
            val table =
                when (uri.lastPathSegment) {
                    "notes_v2" -> "notes"
                    "decks" -> "decks"
                    else -> error("Unexpected provider URI $uri")
                }
            // Same _id projection mapping as AnkiDroid's notes_v2 implementation. Run the actual
            // bound selection in Android SQLite, rather than teaching the fake the expected match.
            val columns = projection?.map { if (it == "_id") "id AS _id" else it }?.toTypedArray()
            return database.query(table, columns, selection, args, null, null, order)
        }

        override fun insert(uri: Uri, values: ContentValues?): Uri? {
            check(Looper.myLooper() != Looper.getMainLooper())
            val handler = insertReplacement ?: error("Unexpected insert")
            val snapshot = ContentValues(checkNotNull(values))
            inserts.add(uri to snapshot)
            operations.add("insert:${uri.path}")
            return handler(uri, snapshot)
        }

        override fun update(
            uri: Uri,
            values: ContentValues?,
            selection: String?,
            args: Array<out String>?,
        ): Int {
            check(Looper.myLooper() != Looper.getMainLooper())
            check(selection == null && args == null)
            val handler = updateReplacement ?: error("Unexpected update")
            val snapshot = ContentValues(checkNotNull(values))
            updates.add(uri to snapshot)
            operations.add("update:${uri.path}")
            return handler(uri, snapshot)
        }

        override fun delete(uri: Uri, selection: String?, args: Array<out String>?): Int =
            error("Unexpected delete")

        override fun close() = database.close()

        fun note(
            id: Long,
            first: String,
            model: Long = 1,
            deck: Long = 1,
            rest: String = "ねこ",
        ) {
            models.putIfAbsent(
                model,
                arrayOf(model, "External", "Surface\u001fReading", 1L, "", 0L, 0L),
            )
            database.execSQL(
                "INSERT INTO notes VALUES(?, ?, ?, ?)",
                arrayOf<Any?>(id, model, "$first\u001f$rest", deck),
            )
        }

        fun rows(): List<List<String?>> =
            database.rawQuery("SELECT * FROM notes ORDER BY id", null).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) add(
                        (0 until cursor.columnCount).map { cursor.getString(it) }
                    )
                }
            }
    }

    private class Fixture : AutoCloseable {
        val environment = Environment()
        val provider =
            Provider().apply {
                attachInfo(
                    InstrumentationRegistry.getInstrumentation().targetContext,
                    environment.info,
                )
            }
        val reader = AnkiProviderReader(ContentResolver.wrap(provider), environment)
        val creator = AnkiProviderModelCreator(ContentResolver.wrap(provider), environment)
        val inserter = AnkiProviderNoteInserter(ContentResolver.wrap(provider), environment)
        // Comparison inputs stand in for the future Rust-owned layout transport, not production
        // copies of the core's HTML/CSS constants.
        val layout =
            AnkiProviderModelLayout(
                "Yomoku",
                listOf("Surface", "Reading", "Meanings", "Sentence"),
                ".card { color: black; }",
                "Japanese word",
                "{{Surface}}",
                "{{Sentence}}",
            )

        fun model(id: Long = 7, name: String = layout.name) {
            provider.models[id] =
                arrayOf(
                    id,
                    name,
                    layout.joinedFieldNames,
                    1L,
                    layout.css,
                    0L,
                    0L,
                )
            provider.templates[id] =
                listOf(
                    arrayOf(
                        id,
                        0L,
                        layout.cardName,
                        layout.questionFormat,
                        layout.answerFormat,
                        "",
                        "",
                    )
                )
        }

        fun enableCreation(id: Long = Long.MAX_VALUE) {
            provider.insertReplacement = { uri, values ->
                check(uri.pathSegments == listOf("models"))
                check(id !in provider.models)
                provider.models[id] =
                    arrayOf(
                        id,
                        values.getAsString("name"),
                        values.getAsString("field_names"),
                        values.getAsLong("num_cards"),
                        values.getAsString("css"),
                        values.getAsLong("sort_field_index"),
                        values.getAsLong("type"),
                    )
                provider.templates[id] =
                    listOf(arrayOf(id, 0L, "Dummy", "dummy question", "dummy answer", "", ""))
                uri.buildUpon().appendPath(id.toString()).build()
            }
            provider.updateReplacement = { uri, values ->
                check(uri.pathSegments == listOf("models", id.toString(), "templates", "0"))
                provider.templates[id] =
                    listOf(
                        arrayOf(
                            id,
                            0L,
                            values.getAsString("card_template_name"),
                            values.getAsString("question_format"),
                            values.getAsString("answer_format"),
                            values.getAsString("browser_question_format"),
                            values.getAsString("browser_answer_format"),
                        )
                    )
                // The real provider increments once per property, not once per template row.
                values.size()
            }
        }

        fun noteRequest(
            surface: String = "猫",
            fields: List<String> =
                listOf(
                    AnkiProviderReader.escapeSurface(surface),
                    "ねこ",
                    "<ol><li>cat</li></ol>",
                    "<strong>猫</strong>。",
                ),
            deckId: Long = Long.MAX_VALUE,
            modelId: Long = 7,
            reading: String = "ねこ",
        ) =
            AnkiProviderNoteRequest(
                surface,
                reading,
                fields,
                AnkiProviderModelId(modelId),
                deckId,
                layout,
            )

        fun enableNoteInsertion(id: Long = Long.MAX_VALUE) {
            model()
            provider.database.execSQL(
                "INSERT INTO decks VALUES(?, 'Frozen deck', 0)",
                arrayOf<Any?>(Long.MAX_VALUE),
            )
            provider.insertReplacement = { uri, values ->
                check(uri.pathSegments == listOf("notes"))
                provider.database.execSQL(
                    "INSERT INTO notes VALUES(?, ?, ?, 1)",
                    arrayOf<Any?>(id, values.getAsLong("mid"), values.getAsString("flds")),
                )
                provider.database.execSQL("INSERT INTO cards VALUES(?, 0, 1)", arrayOf<Any?>(id))
                uri.buildUpon().appendPath(id.toString()).build()
            }
            provider.updateReplacement = { uri, values ->
                check(uri.pathSegments == listOf("notes", id.toString(), "cards", "0"))
                check(values.keySet() == setOf("deck_id"))
                provider.database.update(
                    "cards",
                    values,
                    "note_id = ? AND ord = 0",
                    arrayOf(id.toString()),
                )
            }
        }

        override fun close() = provider.close()
    }

    private fun Fixture.liveNote(): AnkiLiveOperation.InsertNote {
        val request = noteRequest("𠮷猫")
        return AnkiLiveOperation.InsertNote(
            request.surface,
            request.reading,
            request.joinedFields.split('\u001f'),
            request.deckId,
            request.modelId.value,
            modelLayout(),
        )
    }

    private fun Fixture.liveModel(): AnkiLiveOperation.CreateModel =
        AnkiLiveOperation.CreateModel("𠮷猫", "ねこ", Long.MAX_VALUE, modelLayout())

    private fun Fixture.dispatch(operation: AnkiLiveOperation): AnkiLiveResult =
        AnkiLiveDispatcher(ContentResolver.wrap(provider), environment).invoke(operation)

    @Test
    fun liveDispatchPreservesFrozenNoteAndMapsAddedExistingAndUncertainResults() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            val operation = f.liveNote()
            assertEquals(AnkiLiveResult.NoteAdded(Long.MAX_VALUE), f.dispatch(operation))
            assertEquals(1, f.provider.inserts.size)
            assertEquals(AnkiLiveResult.NoteExisting(listOf(Long.MAX_VALUE)), f.dispatch(operation))
            assertEquals(1, f.provider.inserts.size)
        }
        Fixture().use { f ->
            f.enableNoteInsertion()
            f.provider.updateReplacement = { _, _ -> throw IllegalStateException("lost placement") }
            assertEquals(AnkiLiveResult.NoteUncertain(Long.MAX_VALUE), f.dispatch(f.liveNote()))
            assertEquals(1, f.provider.inserts.size)
        }
    }

    @Test
    fun liveDispatchMapsCreationAndRetainsPartialModelIds() {
        for (uncertain in listOf(false, true)) Fixture().use { f ->
            f.enableCreation(9007199254740993L)
            f.provider.database.execSQL(
                "INSERT INTO decks VALUES(?, 'Frozen', 0)",
                arrayOf<Any?>(Long.MAX_VALUE),
            )
            if (uncertain) f.provider.updateReplacement = { _, _ -> 0 }
            val expected =
                if (uncertain) AnkiLiveResult.ModelUncertain(9007199254740993L)
                else AnkiLiveResult.ModelCreated(9007199254740993L)
            assertEquals(expected, f.dispatch(f.liveModel()))
            assertEquals(1, f.provider.inserts.size)
            assertEquals(AnkiLiveResult.NoSubmission, f.dispatch(f.liveModel()))
            assertEquals(1, f.provider.inserts.size)
        }
    }

    @Test
    fun liveDispatchRejectsMissingFilteredDeckDuplicatesAndConflictsBeforeCreating() {
        for (gate in 0..4) Fixture().use { f ->
            f.enableCreation()
            if (gate != 0)
                f.provider.database.execSQL(
                    "INSERT INTO decks VALUES(?, 'Frozen', ?)",
                    arrayOf<Any?>(Long.MAX_VALUE, if (gate == 1) 1 else 0),
                )
            when (gate) {
                2 -> f.provider.database.execSQL("INSERT INTO notes VALUES(8, 9, '𠮷猫', 1)")
                3 -> f.model()
                4 -> f.environment.permitted = false
            }
            assertEquals(AnkiLiveResult.NoSubmission, f.dispatch(f.liveModel()))
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun liveDispatchRefreshesDeckAndDuplicatesImmediatelyBeforeModelSubmission() {
        for (duplicate in listOf(false, true)) Fixture().use { f ->
            f.enableCreation()
            f.provider.database.execSQL(
                "INSERT INTO decks VALUES(?, 'Frozen', 0)",
                arrayOf<Any?>(Long.MAX_VALUE),
            )
            var modelReads = 0
            f.provider.beforeQuery = { uri ->
                if (uri.pathSegments == listOf("models") && ++modelReads == 2) {
                    if (duplicate)
                        f.provider.database.execSQL("INSERT INTO notes VALUES(8, 9, '𠮷猫', 1)")
                    else f.provider.database.execSQL("UPDATE decks SET deck_dyn = 1")
                }
            }
            assertEquals(AnkiLiveResult.NoSubmission, f.dispatch(f.liveModel()))
            assertEquals(2, modelReads)
            assertTrue(f.provider.inserts.isEmpty())
        }
    }

    @Test
    fun liveDispatchClassifiesProviderLossBeforeAndAfterSubmission() {
        for (after in listOf(false, true)) Fixture().use { f ->
            f.enableNoteInsertion()
            if (after)
                f.provider.insertReplacement = { _, _ -> throw SecurityException("lost response") }
            else f.environment.permitted = false
            assertEquals(
                if (after) AnkiLiveResult.NoteUncertain(null) else AnkiLiveResult.NoSubmission,
                f.dispatch(f.liveNote()),
            )
            assertEquals(if (after) 1 else 0, f.provider.inserts.size)
        }
    }

    private fun <T> value(result: AnkiReadResult<T>): T = (result as AnkiReadResult.Value).value

    private fun failed(reason: AnkiReadError, result: AnkiReadResult<*>) {
        assertEquals(AnkiReadResult.Failed(reason), result)
    }

    private fun Fixture.modelLayout() =
        AnkiReadModelLayout(
            layout.name,
            layout.joinedFieldNames.split('\u001f'),
            layout.css,
            layout.cardName,
            layout.questionFormat,
            layout.answerFormat,
        )

    private fun Fixture.preflight() =
        AnkiPreflightOperation("𠮷猫", "ねこ", Long.MAX_VALUE, modelLayout())

    @Test
    fun typedPreflightKeepsDecksExactDuplicatesAndAllNamedModelsWithoutWrites() {
        Fixture().use { f ->
            f.provider.database.execSQL(
                "INSERT INTO decks VALUES(?, 'Frozen', 0)",
                arrayOf<Any?>(Long.MAX_VALUE),
            )
            f.provider.database.execSQL("INSERT INTO decks VALUES(1, 'Filtered', 1)")
            f.provider.database.execSQL(
                "INSERT INTO notes VALUES(?, 1, ?, 1)",
                arrayOf<Any?>(Long.MAX_VALUE, "𠮷猫\u001fねこ"),
            )
            f.model(9007199254740993L)
            f.model(Long.MAX_VALUE)
            f.provider.templates[Long.MAX_VALUE] = emptyList()
            f.model(1, "User model")
            val before = f.provider.rows()
            val result = AnkiPreflightReader(f.reader).read(f.preflight())
            val observed = (result as AnkiPreflightResult.Complete).value
            assertEquals(
                listOf(AnkiPreflightDeck(Long.MAX_VALUE, false), AnkiPreflightDeck(1, true)),
                observed.decks,
            )
            assertEquals(listOf(Long.MAX_VALUE), observed.duplicateNoteIds)
            assertEquals(
                listOf(
                    AnkiPreflightModel(9007199254740993L, f.layout.name, true),
                    AnkiPreflightModel(Long.MAX_VALUE, f.layout.name, false),
                    AnkiPreflightModel(1, "User model", false),
                ),
                observed.models,
            )
            assertEquals(result, AnkiPreflightResult.bincodeDeserialize(result.bincodeSerialize()))
            assertEquals(before, f.provider.rows())
            assertFalse(f.provider.operations.contains("query:/models/1/templates"))
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun typedPreflightEmptyCompleteReadsRemainEvidenceForCoreEvaluation() {
        Fixture().use { f ->
            val result =
                AnkiPreflightReader(f.reader).read(f.preflight()) as AnkiPreflightResult.Complete
            assertTrue(result.value.decks.isEmpty())
            assertTrue(result.value.models.isEmpty())
            assertTrue(result.value.duplicateNoteIds.isEmpty())
        }
    }

    @Test
    fun typedPreflightDiscardsAllEvidenceOnEachLaterQueryFailure() {
        for (path in
            listOf(
                "/decks",
                "/notes_v2",
                "/models",
                "/models/7",
                "/models/7/templates",
                "/models/${Long.MAX_VALUE}",
            )) {
            Fixture().use { f ->
                f.model(7)
                f.model(Long.MAX_VALUE)
                f.provider.database.execSQL(
                    "INSERT INTO notes VALUES(1, 7, ?, 1)",
                    arrayOf<Any?>("𠮷猫"),
                )
                f.provider.beforeQuery = { uri ->
                    if (uri.path == path) throw IllegalStateException("lost collection")
                }
                assertEquals(
                    AnkiPreflightResult.Failed(
                        if (path in listOf("/notes_v2", "/models/7"))
                            AnkiExportError.DUPLICATELOOKUPFAILED
                        else AnkiExportError.COLLECTIONUNAVAILABLE
                    ),
                    AnkiPreflightReader(f.reader).read(f.preflight()),
                )
                assertTrue(f.provider.inserts.isEmpty())
                assertTrue(f.provider.updates.isEmpty())
            }
        }
    }

    @Test
    fun typedPreflightPermissionRevocationAndMalformedLaterCursorDiscardEarlierMatches() {
        for (permission in listOf(false, true)) {
            Fixture().use { f ->
                f.model(7)
                f.model(Long.MAX_VALUE)
                f.provider.beforeQuery = { uri ->
                    if (uri.path == "/models/${Long.MAX_VALUE}") {
                        if (permission) f.environment.permitted = false
                        else throw IllegalArgumentException("bad cursor")
                    }
                }
                assertEquals(
                    AnkiPreflightResult.Failed(
                        if (permission) AnkiExportError.PERMISSIONDENIED
                        else AnkiExportError.INVALIDRESPONSE
                    ),
                    AnkiPreflightReader(f.reader).read(f.preflight()),
                )
            }
        }
    }

    @Test
    fun typedPreflightMissingUnsupportedOrUnpermittedProviderAndInvalidInputFailClosed() {
        Fixture().use { f ->
            val reader = AnkiPreflightReader(f.reader)
            assertEquals(
                AnkiPreflightResult.Failed(AnkiExportError.INVALIDRESPONSE),
                reader.read(f.preflight().copy(deckId = 0)),
            )
            assertEquals(0, f.provider.queries)
            f.environment.permitted = false
            assertEquals(
                AnkiPreflightResult.Failed(AnkiExportError.PERMISSIONDENIED),
                reader.read(f.preflight()),
            )
            f.environment.permitted = true
            f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
            assertEquals(
                AnkiPreflightResult.Failed(AnkiExportError.UNSUPPORTEDPROVIDER),
                reader.read(f.preflight()),
            )
            f.environment.info = null
            assertEquals(
                AnkiPreflightResult.Failed(AnkiExportError.PROVIDERUNAVAILABLE),
                reader.read(f.preflight()),
            )
            assertEquals(0, f.provider.queries)
        }
    }

    @Test
    fun noteInsertionValidatesThenPlacesOnlyTheNewCardInTheFrozenDeck() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            val modelId = Long.MAX_VALUE - 1
            f.model(modelId)
            f.provider.note(1, "犬", model = 9, deck = 3)
            val before = f.provider.rows()
            val models = f.provider.models.mapValues { it.value.toList() }
            val templates = f.provider.templates.mapValues { it.value.map { row -> row.toList() } }
            val request = f.noteRequest(modelId = modelId)
            assertEquals(
                AnkiNoteInsertionResult.Inserted(AnkiProviderNoteId(Long.MAX_VALUE)),
                f.inserter.insert(request),
            )
            assertEquals(before.single(), f.provider.rows().first())
            assertEquals(models, f.provider.models.mapValues { it.value.toList() })
            assertEquals(
                templates,
                f.provider.templates.mapValues { it.value.map { row -> row.toList() } },
            )
            val (uri, values) = f.provider.inserts.single()
            assertEquals("content://${AnkiProviderReader.AUTHORITY}/notes", uri.toString())
            assertEquals(setOf("mid", "flds", "tags"), values.keySet())
            assertEquals("yomoku", values.getAsString("tags"))
            assertEquals(modelId, values.getAsLong("mid"))
            assertEquals(request.joinedFields, values.getAsString("flds"))
            val (card, placement) = f.provider.updates.single()
            assertEquals("/notes/${Long.MAX_VALUE}/cards/0", card.path)
            assertEquals(setOf("deck_id"), placement.keySet())
            assertEquals(Long.MAX_VALUE, placement.getAsLong("deck_id"))
            f.provider.database.rawQuery("SELECT deck_id FROM cards", null).use {
                assertTrue(it.moveToFirst())
                assertEquals(Long.MAX_VALUE, it.getLong(0))
            }
            assertEquals(
                listOf(
                    "query:/models/$modelId",
                    "query:/models/$modelId/templates",
                    "query:/decks",
                    "query:/notes_v2",
                    "insert:/notes",
                    "query:/notes/${Long.MAX_VALUE}",
                    "query:/notes/${Long.MAX_VALUE}/cards",
                    "update:/notes/${Long.MAX_VALUE}/cards/0",
                    "query:/notes/${Long.MAX_VALUE}",
                    "query:/notes/${Long.MAX_VALUE}/cards",
                ),
                f.provider.operations,
            )
        }
    }

    @Test
    fun insertionRechecksNewExactDuplicatesAcrossModelsAndDecksImmediatelyBeforeSubmission() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            f.provider.beforeQuery = { uri ->
                if (uri.lastPathSegment == "notes_v2") {
                    f.provider.note(8, "猫", model = 4, deck = 9)
                    f.provider.note(9, "猫", model = 5, deck = 10)
                }
            }
            assertEquals(
                AnkiNoteInsertionResult.Existing(
                    listOf(AnkiProviderNoteId(8), AnkiProviderNoteId(9))
                ),
                f.inserter.insert(f.noteRequest()),
            )
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun missingOrIncompatibleModelAndMissingOrFilteredFrozenDeckPreventSubmission() {
        for (scenario in 0..3) Fixture().use { f ->
            f.enableNoteInsertion()
            when (scenario) {
                0 -> f.provider.models.clear()
                1 -> f.provider.templates[7]!!.single()[3] = "User-edited question"
                2 -> f.provider.database.execSQL("DELETE FROM decks")
                3 -> f.provider.database.execSQL("UPDATE decks SET deck_dyn = 1")
            }
            val reason =
                if (scenario < 2) AnkiNoteInsertionError.LayoutMismatch
                else AnkiNoteInsertionError.DeckUnavailable
            assertEquals(AnkiNoteInsertionResult.Failed(reason), f.inserter.insert(f.noteRequest()))
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun insertionUnavailableHostUnsupportedSpecAndDeniedPermissionNeverSubmit() {
        for (scenario in 0..2) Fixture().use { f ->
            f.enableNoteInsertion()
            val error =
                when (scenario) {
                    0 -> {
                        f.environment.info = null
                        AnkiReadError.ProviderUnavailable
                    }
                    1 -> {
                        f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
                        AnkiReadError.UnsupportedProvider
                    }
                    else -> {
                        f.environment.permitted = false
                        AnkiReadError.PermissionDenied
                    }
                }
            assertEquals(
                AnkiNoteInsertionResult.Failed(AnkiNoteInsertionError.ReadFailed(error)),
                f.inserter.insert(f.noteRequest()),
            )
            assertEquals(0, f.provider.queries)
            assertTrue(f.provider.inserts.isEmpty())
        }
    }

    @Test
    fun preInsertReadNullTransportMalformedAndCloseFailuresNeverMeanNoDuplicates() {
        for (path in listOf("/models/7", "/models/7/templates", "/decks", "/notes_v2")) {
            for (failure in 0..3) Fixture().use { f ->
                f.enableNoteInsertion()
                var closed = false
                f.provider.queryResult = { uri, cursor ->
                    if (uri.path != path) cursor
                    else {
                        cursor!!.close()
                        when (failure) {
                            0 -> null
                            1 -> throw IllegalStateException("collection locked")
                            2 -> MatrixCursor(arrayOf("wrong_schema"))
                            else ->
                                object : CursorWrapper(MatrixCursor(cursor.columnNames)) {
                                    override fun close() {
                                        super.close()
                                        closed = true
                                        throw IllegalStateException("close failed")
                                    }
                                }
                        }
                    }
                }
                assertTrue(
                    "$path failure=$failure",
                    f.inserter.insert(f.noteRequest()) is AnkiNoteInsertionResult.Failed,
                )
                if (failure == 3) assertTrue(closed)
                assertTrue(f.provider.inserts.isEmpty())
                assertTrue(f.provider.updates.isEmpty())
            }
        }
    }

    @Test
    fun insertionRefreshesHostSpecAndPermissionBetweenEveryPreflightCall() {
        for (path in listOf("/models/7", "/models/7/templates", "/decks", "/notes_v2")) {
            for (change in 0..2) Fixture().use { f ->
                f.enableNoteInsertion()
                f.provider.queryResult = { uri, cursor ->
                    if (uri.path == path)
                        when (change) {
                            0 -> f.environment.permitted = false
                            1 -> f.environment.info!!.packageName = "replacement.host"
                            else ->
                                f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
                        }
                    cursor
                }
                val error =
                    when (change) {
                        0 -> AnkiReadError.PermissionDenied
                        1 -> AnkiReadError.ProviderUnavailable
                        else -> AnkiReadError.UnsupportedProvider
                    }
                assertEquals(
                    "$path change=$change",
                    AnkiNoteInsertionResult.Failed(AnkiNoteInsertionError.ReadFailed(error)),
                    f.inserter.insert(f.noteRequest()),
                )
                assertTrue(f.provider.inserts.isEmpty())
                assertTrue(f.provider.updates.isEmpty())
            }
        }
    }

    @Test
    fun nullThrownAndCommittedButLostNoteInsertResponsesAreAlwaysUncertain() {
        for (committed in listOf(false, true)) for (failure in 0..3) Fixture().use { f ->
            f.enableNoteInsertion()
            val insert = f.provider.insertReplacement!!
            f.provider.insertReplacement = { uri, values ->
                if (committed) insert(uri, values)
                when (failure) {
                    0 -> null
                    1 -> throw IllegalStateException("transport lost")
                    2 -> throw SecurityException("permission revoked during insert")
                    else -> throw IllegalArgumentException("provider rejected request")
                }
            }
            val request = f.noteRequest()
            val result = f.inserter.insert(request) as AnkiNoteInsertionResult.Uncertain
            assertSame(request, result.request)
            assertNull(result.id)
            assertEquals(if (committed) 1 else 0, f.provider.rows().size)
            assertEquals(1, f.provider.inserts.size)
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun untrustedReturnedNoteUrisNeverSelectACardForMutation() {
        val base = "content://${AnkiProviderReader.AUTHORITY}"
        for (response in
            listOf(
                "$base/notes/0",
                "$base/notes/-1",
                "$base/notes/01",
                "$base/notes/+1",
                "$base/notes/9223372036854775808",
                "$base/models/1",
                "$base/notes/1/cards",
                "$base/notes/1?x=y",
                "$base/notes/1#fragment",
                "content://other/notes/1",
                "https://${AnkiProviderReader.AUTHORITY}/notes/1",
            )) {
            Fixture().use { f ->
                f.enableNoteInsertion()
                val insert = f.provider.insertReplacement!!
                f.provider.insertReplacement = { uri, values ->
                    insert(uri, values)
                    Uri.parse(response)
                }
                val result = f.inserter.insert(f.noteRequest()) as AnkiNoteInsertionResult.Uncertain
                assertNull(result.id)
                assertEquals(AnkiNoteInsertionError.InvalidResponse, result.error)
                assertTrue(f.provider.updates.isEmpty())
                assertEquals(1, f.provider.rows().size)
            }
        }
    }

    @Test
    fun returnedUnrelatedNoteCannotHaveItsCardMoved() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            f.provider.note(8, "User note", model = 7, deck = 99)
            val before = f.provider.rows().single()
            val insert = f.provider.insertReplacement!!
            f.provider.insertReplacement = { uri, values ->
                insert(uri, values)
                Uri.parse("content://${AnkiProviderReader.AUTHORITY}/notes/8")
            }
            val result = f.inserter.insert(f.noteRequest()) as AnkiNoteInsertionResult.Uncertain
            assertEquals(AnkiProviderNoteId(8), result.id)
            assertEquals(AnkiNoteInsertionError.InvalidResponse, result.error)
            assertEquals(before, f.provider.rows().first())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun missingMalformedAndExtraNoteOrCardRowsFailClosedAndCloseBeforePlacement() {
        for (card in listOf(false, true)) {
            val columns =
                if (card) arrayOf("note_id", "ord", "deck_id") else arrayOf("_id", "mid", "flds")
            for (failure in 0..8) Fixture().use { f ->
                f.enableNoteInsertion()
                var closed = false
                val request = f.noteRequest()
                f.provider.queryResult = { uri, cursor ->
                    if (
                        uri.pathSegments.first() != "notes" ||
                            (uri.lastPathSegment == "cards") != card
                    )
                        cursor
                    else {
                        cursor!!.close()
                        if (failure == 0) null
                        else {
                            val row =
                                if (card) arrayOf<Any?>(Long.MAX_VALUE, 0L, 1L)
                                else arrayOf<Any?>(Long.MAX_VALUE, 7L, request.joinedFields)
                            when (failure) {
                                2 -> row[0] = 8L
                                3 -> row[1] = if (card) 1L else 9L
                                4 -> row[2] = if (card) 0L else "different fields"
                                5 -> row[0] = Long.MAX_VALUE.toString()
                                6 -> row[2] = null
                            }
                            val matrix =
                                MatrixCursor(columns).apply {
                                    if (failure != 1) addRow(row)
                                    if (failure == 7) addRow(row)
                                }
                            object : CursorWrapper(matrix) {
                                override fun close() {
                                    super.close()
                                    closed = true
                                    if (failure == 8) throw IllegalStateException("close failed")
                                }
                            }
                        }
                    }
                }
                val result = f.inserter.insert(request) as AnkiNoteInsertionResult.Uncertain
                assertEquals(AnkiProviderNoteId(Long.MAX_VALUE), result.id)
                if (failure != 0) assertTrue(closed)
                assertTrue(f.provider.updates.isEmpty())
                assertEquals(1, f.provider.inserts.size)
            }
        }
    }

    @Test
    fun failedOrLostPlacementRetainsKnownNoteAndNeverReplaysOrDeletes() {
        for (committed in listOf(false, true)) for (failure in 0..4) Fixture().use { f ->
            f.enableNoteInsertion()
            val update = f.provider.updateReplacement!!
            f.provider.updateReplacement = { uri, values ->
                if (committed) update(uri, values)
                when (failure) {
                    0 -> 0
                    1 -> 2
                    2 -> throw SecurityException("permission revoked")
                    3 -> throw IllegalStateException("lost response")
                    else -> throw IllegalArgumentException("deck disappeared")
                }
            }
            val request = f.noteRequest()
            val result = f.inserter.insert(request) as AnkiNoteInsertionResult.Uncertain
            assertSame(request, result.request)
            assertEquals(AnkiProviderNoteId(Long.MAX_VALUE), result.id)
            assertEquals(request.joinedFields, f.provider.rows().single()[2])
            assertEquals(1, f.provider.inserts.size)
            assertEquals(1, f.provider.updates.size)
        }
    }

    @Test
    fun successfulPlacementCountNeedsExactNoteAndDeckReadback() {
        for (failure in 0..5) Fixture().use { f ->
            f.enableNoteInsertion()
            val update = f.provider.updateReplacement!!
            f.provider.updateReplacement = { uri, values ->
                if (failure != 0) update(uri, values)
                when (failure) {
                    1 -> f.provider.database.execSQL("UPDATE notes SET flds = 'edited'")
                    2 -> f.provider.database.execSQL("DELETE FROM cards")
                    3 -> f.provider.database.execSQL("INSERT INTO cards SELECT * FROM cards")
                    4 -> f.provider.queryResult = { _, cursor ->
                            cursor!!.close()
                            null
                        }
                    5 -> f.provider.queryResult = { _, cursor ->
                            object : CursorWrapper(cursor) {
                                override fun close() {
                                    super.close()
                                    throw IllegalStateException("close failed")
                                }
                            }
                        }
                }
                1
            }
            val result = f.inserter.insert(f.noteRequest()) as AnkiNoteInsertionResult.Uncertain
            assertEquals(AnkiProviderNoteId(Long.MAX_VALUE), result.id)
            assertEquals(1, f.provider.inserts.size)
            assertEquals(1, f.provider.updates.size)
        }
    }

    @Test
    fun hostAndPermissionChangesAfterSubmissionOrPlacementRemainUncertain() {
        for (afterUpdate in listOf(false, true)) for (change in 0..2) Fixture().use { f ->
            f.enableNoteInsertion()
            val changeEnvironment = {
                when (change) {
                    0 -> f.environment.permitted = false
                    1 -> f.environment.info!!.packageName = "replacement.host"
                    else -> f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
                }
            }
            if (afterUpdate) {
                val update = f.provider.updateReplacement!!
                f.provider.updateReplacement = { uri, values ->
                    update(uri, values).also { changeEnvironment() }
                }
            } else {
                val insert = f.provider.insertReplacement!!
                f.provider.insertReplacement = { uri, values ->
                    insert(uri, values).also { changeEnvironment() }
                }
            }
            val result = f.inserter.insert(f.noteRequest()) as AnkiNoteInsertionResult.Uncertain
            assertEquals(AnkiProviderNoteId(Long.MAX_VALUE), result.id)
            assertEquals(1, f.provider.inserts.size)
            assertEquals(if (afterUpdate) 1 else 0, f.provider.updates.size)
        }
    }

    @Test
    fun uncertainPayloadAndDeckSurviveReadOnlyUnresolvedAndExistingReconciliation() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            f.provider.insertReplacement = { _, _ -> null }
            val fields = mutableListOf("猫", "ねこ", "", "猫。<strong>猫</strong>。")
            val request = f.noteRequest(fields = fields)
            fields[0] = "犬"
            val result = f.inserter.insert(request) as AnkiNoteInsertionResult.Uncertain
            repeat(2) {
                assertEquals(
                    AnkiReadResult.Value(emptyList<AnkiProviderNoteId>()),
                    f.reader.duplicates(result.request.surface, result.request.reading),
                )
            }
            f.provider.note(9, "猫", model = 88, deck = 99)
            assertEquals(
                AnkiReadResult.Value(listOf(AnkiProviderNoteId(9))),
                f.reader.duplicates(result.request.surface, result.request.reading),
            )
            assertEquals("猫\u001fねこ\u001f\u001f猫。<strong>猫</strong>。", result.request.joinedFields)
            assertEquals(Long.MAX_VALUE, result.request.deckId)
            assertEquals(1, f.provider.inserts.size)
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun noteRequestFreezesFieldsAndRejectsInvalidIdentityOrTransport() {
        Fixture().use { f ->
            val raw = "<&\"'>[sound:猫.mp3]\u001f"
            val escaped = "&lt;&amp;&quot;&#39;&gt;&#91;sound:猫.mp3&#93;&#31;"
            val fields = mutableListOf(escaped, "", "", "sentence")
            val request = f.noteRequest(surface = raw, fields = fields, reading = "")
            fields[3] = "changed"
            assertEquals("$escaped\u001f\u001f\u001fsentence", request.joinedFields)
            for (bad in
                listOf(
                    listOf("猫"),
                    listOf("犬", "", "", ""),
                    listOf("猫", "\u001f", "", ""),
                    listOf("猫", "", "\u0000", ""),
                )) {
                assertThrows(IllegalArgumentException::class.java) { f.noteRequest(fields = bad) }
            }
            assertThrows(IllegalArgumentException::class.java) { f.noteRequest(surface = "") }
            assertThrows(IllegalArgumentException::class.java) {
                f.noteRequest(surface = "猫\u0000")
            }
            assertThrows(IllegalArgumentException::class.java) { f.noteRequest(deckId = 0) }
        }
    }

    @Test
    fun noteInsertionRefusesMainThreadBeforeAnyProviderCall() {
        Fixture().use { f ->
            val request = f.noteRequest()
            InstrumentationRegistry.getInstrumentation().runOnMainSync {
                assertThrows(IllegalStateException::class.java) { f.inserter.insert(request) }
            }
            assertEquals(0, f.provider.queries)
            assertTrue(f.provider.inserts.isEmpty())
        }
    }

    @Test
    fun creationWritesOnlyFreshModelThenValidatesTheEntireLayout() {
        Fixture().use { f ->
            f.model(7, "User model")
            val before = f.provider.models[7]!!.toList()
            val templateBefore = f.provider.templates[7]!!.map { it.toList() }
            f.provider.note(1, "猫", model = 7)
            val notesBefore = f.provider.rows()
            f.enableCreation()
            assertEquals(
                AnkiModelCreationResult.Created(AnkiProviderModelId(Long.MAX_VALUE)),
                f.creator.create(f.layout),
            )
            assertEquals(before, f.provider.models[7]!!.toList())
            assertEquals(templateBefore, f.provider.templates[7]!!.map { it.toList() })
            val (uri, values) = f.provider.inserts.single()
            assertEquals("content://${AnkiProviderReader.AUTHORITY}/models", uri.toString())
            assertEquals(
                setOf("name", "field_names", "num_cards", "css", "sort_field_index", "type"),
                values.keySet(),
            )
            assertEquals(f.layout.joinedFieldNames, values.getAsString("field_names"))
            assertEquals(1, values.getAsInteger("num_cards"))
            assertEquals(0, values.getAsInteger("type"))
            assertEquals(0, values.getAsInteger("sort_field_index"))
            val (templateUri, template) = f.provider.updates.single()
            assertEquals(
                "content://${AnkiProviderReader.AUTHORITY}/models/${Long.MAX_VALUE}/templates/0",
                templateUri.toString(),
            )
            assertEquals(5, template.size())
            assertEquals("", template.getAsString("browser_question_format"))
            assertEquals("", template.getAsString("browser_answer_format"))
            assertTrue(value(f.reader.modelMatches(AnkiProviderModelId(Long.MAX_VALUE), f.layout)))
            assertEquals(notesBefore, f.provider.rows())
        }
    }

    @Test
    fun creationNeverOverwritesOrRenamesSameNameModelsEvenWhenCompatible() {
        for (compatible in listOf(false, true)) Fixture().use { f ->
            f.model()
            if (!compatible) f.provider.models[7]!![2] = "User fields"
            val before = f.provider.models[7]!!.toList()
            f.enableCreation()
            assertEquals(
                AnkiModelCreationResult.Failed(AnkiModelCreationError.NameConflict),
                f.creator.create(f.layout),
            )
            assertEquals(before, f.provider.models[7]!!.toList())
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun creationGatesAndFailedDiscoveryNeverSubmitWrites() {
        val cases =
            listOf<(Fixture) -> Unit>(
                { it.environment.info = null },
                { it.environment.info!!.metaData!!.putInt(AnkiProviderReader.SPEC_KEY, 1) },
                { it.environment.permitted = false },
                { it.provider.replacement = { null } },
                { it.provider.replacement = { throw android.os.DeadObjectException() } },
                { it.provider.replacement = { MatrixCursor(arrayOf("_id")) } },
            )
        for (configure in cases) Fixture().use { f ->
            f.enableCreation()
            configure(f)
            assertTrue(f.creator.create(f.layout) is AnkiModelCreationResult.Failed)
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun creationRechecksHostAndPermissionAfterDiscoveryBeforeSubmission() {
        for (revoke in listOf(false, true)) Fixture().use { f ->
            f.enableCreation()
            f.provider.replacement = {
                if (revoke) f.environment.permitted = false
                else f.environment.info!!.packageName = "replacement.host"
                MatrixCursor(arrayOf("_id", "name"))
            }
            assertTrue(f.creator.create(f.layout) is AnkiModelCreationResult.Failed)
            assertTrue(f.provider.inserts.isEmpty())
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun creationRejectsMalformedOrPreviouslyDiscoveredReturnedIdsWithoutUpdatingThem() {
        val base = "content://${AnkiProviderReader.AUTHORITY}"
        val responses =
            listOf(
                "$base/models/0",
                "$base/models/-1",
                "$base/models/07",
                "$base/models/+8",
                "$base/models/9223372036854775808",
                "$base/models/current",
                "$base/models/7",
                "$base/notes/8",
                "$base/models/8/templates/0",
                "$base/models/8?other=1",
                "$base/models/8#fragment",
                "content://other.host/models/8",
                "https://${AnkiProviderReader.AUTHORITY}/models/8",
            )
        for (response in responses) Fixture().use { f ->
            f.model(7, "User model")
            f.enableCreation()
            f.provider.insertReplacement = { _, _ -> Uri.parse(response) }
            assertEquals(
                AnkiModelCreationResult.Uncertain(null, AnkiModelCreationError.InvalidResponse),
                f.creator.create(f.layout),
            )
            assertEquals(1, f.provider.inserts.size)
            assertTrue(f.provider.updates.isEmpty())
            assertEquals("User model", f.provider.models[7]!![1])
        }
    }

    @Test
    fun creationLostInsertResponseStaysUncertainEvenIfModelWasCommitted() {
        for (mode in 0..3) Fixture().use { f ->
            f.enableCreation()
            val insert = f.provider.insertReplacement!!
            f.provider.insertReplacement = { uri, values ->
                insert(uri, values)
                when (mode) {
                    0 -> null
                    1 -> throw android.os.DeadObjectException()
                    2 -> throw SecurityException("permission lost")
                    else -> throw IllegalArgumentException("failure after commit")
                }
            }
            val result = f.creator.create(f.layout)
            assertTrue(result is AnkiModelCreationResult.Uncertain)
            assertNull((result as AnkiModelCreationResult.Uncertain).id)
            assertEquals(1, f.provider.models.size)
            assertEquals(1, f.provider.inserts.size)
            assertTrue(f.provider.updates.isEmpty())
            // Read-only discovery cannot cause a replay or repair a dummy model.
            value(f.reader.models())
            assertFalse(value(f.reader.modelMatches(AnkiProviderModelId(Long.MAX_VALUE), f.layout)))
            assertEquals(1, f.provider.inserts.size)
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun creationHostChangeOrRevocationAfterInsertStopsTemplateSubmission() {
        for (revoke in listOf(false, true)) Fixture().use { f ->
            f.enableCreation()
            val insert = f.provider.insertReplacement!!
            f.provider.insertReplacement = { uri, values ->
                insert(uri, values).also {
                    if (revoke) f.environment.permitted = false
                    else f.environment.info!!.packageName = "replacement.host"
                }
            }
            val result = f.creator.create(f.layout) as AnkiModelCreationResult.Uncertain
            assertEquals(AnkiProviderModelId(Long.MAX_VALUE), result.id)
            assertTrue(f.provider.updates.isEmpty())
        }
    }

    @Test
    fun creationFailedTemplateUpdateRetainsIdWithoutRetryOrCleanup() {
        for (mode in 0..3) Fixture().use { f ->
            f.enableCreation()
            val update = f.provider.updateReplacement!!
            f.provider.updateReplacement = { uri, values ->
                when (mode) {
                    0 -> 0
                    1 -> -1
                    else -> {
                        update(uri, values)
                        if (mode == 2) throw android.os.DeadObjectException()
                        throw SecurityException("permission lost after update")
                    }
                }
            }
            val result = f.creator.create(f.layout) as AnkiModelCreationResult.Uncertain
            assertEquals(AnkiProviderModelId(Long.MAX_VALUE), result.id)
            assertEquals(1, f.provider.inserts.size)
            assertEquals(1, f.provider.updates.size)
            assertEquals(1, f.provider.models.size)
        }
    }

    @Test
    fun creationValidatesReadbackInsteadOfTrustingPositiveUpdateCounts() {
        for (mode in 0..3) Fixture().use { f ->
            f.enableCreation()
            val update = f.provider.updateReplacement!!
            f.provider.updateReplacement = { uri, values ->
                if (mode != 0) update(uri, values)
                when (mode) {
                    1 -> f.provider.models[Long.MAX_VALUE]!![1] = "Provider renamed model"
                    2 -> f.provider.templates[Long.MAX_VALUE]!![0][5] = "browser override"
                    3 -> f.provider.replacement = { null }
                }
                5
            }
            val result = f.creator.create(f.layout) as AnkiModelCreationResult.Uncertain
            assertEquals(AnkiProviderModelId(Long.MAX_VALUE), result.id)
            if (mode != 3) assertEquals(AnkiModelCreationError.LayoutMismatch, result.error)
            else
                assertEquals(
                    AnkiModelCreationError.ReadFailed(AnkiReadError.CollectionUnavailable),
                    result.error,
                )
            assertEquals(1, f.provider.inserts.size)
            assertEquals(1, f.provider.updates.size)
        }
    }

    @Test
    fun creationRejectsRevocationAndHostChangeDuringReadback() {
        for (duringTemplate in listOf(false, true)) for (revoke in listOf(false, true)) Fixture()
            .use { f ->
                f.enableCreation()
                val update = f.provider.updateReplacement!!
                f.provider.updateReplacement = { uri, values ->
                    update(uri, values).also {
                        f.provider.replacement = {
                            val template = f.provider.lastUri!!.lastPathSegment == "templates"
                            if (template == duringTemplate) {
                                if (revoke) f.environment.permitted = false
                                else f.environment.info!!.packageName = "replacement.host"
                            }
                            val columns = if (template) TEMPLATE_COLUMNS else MODEL_COLUMNS
                            MatrixCursor(columns).apply {
                                addRow(
                                    if (template) f.provider.templates[Long.MAX_VALUE]!!.single()
                                    else f.provider.models[Long.MAX_VALUE]!!
                                )
                            }
                        }
                    }
                }
                assertTrue(f.creator.create(f.layout) is AnkiModelCreationResult.Uncertain)
                assertEquals(1, f.provider.updates.size)
            }
    }

    @Test
    fun creationReadbackCloseFailureCannotReportSuccess() {
        Fixture().use { f ->
            f.enableCreation()
            val update = f.provider.updateReplacement!!
            f.provider.updateReplacement = { uri, values ->
                update(uri, values).also {
                    f.provider.replacement = {
                        object :
                            CursorWrapper(
                                MatrixCursor(MODEL_COLUMNS).apply {
                                    addRow(f.provider.models[Long.MAX_VALUE]!!)
                                }
                            ) {
                            override fun close() {
                                super.close()
                                throw IllegalStateException("close failed after model read")
                            }
                        }
                    }
                }
            }
            assertEquals(
                AnkiModelCreationResult.Uncertain(
                    AnkiProviderModelId(Long.MAX_VALUE),
                    AnkiModelCreationError.ReadFailed(AnkiReadError.CollectionUnavailable),
                ),
                f.creator.create(f.layout),
            )
            assertEquals(1, f.provider.inserts.size)
            assertEquals(1, f.provider.updates.size)
        }
    }

    @Test
    fun modelsPreserveSameNamesAndLongIdsWithoutAssumingCompatibility() {
        Fixture().use { f ->
            assertEquals(emptyList<AnkiProviderModel>(), value(f.reader.models()))
            f.model(7)
            f.model(Long.MAX_VALUE)
            f.provider.models[7]!![2] = "Front\u001fBack"
            assertEquals(
                listOf(
                    AnkiProviderModel(AnkiProviderModelId(7), f.layout.name),
                    AnkiProviderModel(AnkiProviderModelId(Long.MAX_VALUE), f.layout.name),
                ),
                value(f.reader.models()),
            )
            assertFalse(value(f.reader.modelMatches(AnkiProviderModelId(7), f.layout)))
            assertTrue(value(f.reader.modelMatches(AnkiProviderModelId(Long.MAX_VALUE), f.layout)))
            assertEquals("Front\u001fBack", f.provider.models[7]!![2])
        }
    }

    @Test
    fun modelValidationReadsExactIdAndDoesNotCacheTemplatesOrNames() {
        Fixture().use { f ->
            val id = AnkiProviderModelId(Long.MAX_VALUE)
            assertFalse(value(f.reader.modelMatches(id, f.layout)))
            f.model(id.value)
            assertTrue(value(f.reader.modelMatches(id, f.layout)))
            assertEquals(
                listOf("models", id.value.toString(), "templates"),
                f.provider.lastUri!!.pathSegments,
            )
            assertNull(f.provider.lastSelection)
            assertNull(f.provider.lastArguments)
            f.provider.templates[id.value]!!.single()[3] = "{{Reading}}"
            assertFalse(value(f.reader.modelMatches(id, f.layout)))
            f.model(id.value, "Renamed")
            assertFalse(value(f.reader.modelMatches(id, f.layout)))
            f.provider.models.clear()
            assertFalse(value(f.reader.modelMatches(id, f.layout)))
        }
    }

    @Test
    fun everyMetadataLayoutDifferenceRejectsReuseWithoutWriting() {
        Fixture().use { f ->
            val changes =
                listOf(
                    1 to "yomoku v1",
                    2 to "Reading\u001fSurface\u001fMeanings\u001fSentence",
                    2 to "Surface\u001fReading\u001fMeanings\u001fSentence\u001fExtra",
                    2 to "Surface\u001fReading\u001fMeanings\u001fSentence\u001f",
                    3 to 0L,
                    3 to 2L,
                    4 to "${f.layout.css}\n",
                    5 to 1L,
                    6 to 1L,
                )
            changes.forEach { (column, changed) ->
                f.model()
                f.provider.models[7]!![column] = changed
                assertFalse(
                    "Column $column: $changed",
                    value(f.reader.modelMatches(AnkiProviderModelId(7), f.layout)),
                )
                assertEquals(changed, f.provider.models[7]!![column])
            }
        }
    }

    @Test
    fun everyTemplateDifferenceAndActualTemplateCountRejectReuse() {
        Fixture().use { f ->
            val changes =
                listOf(
                    1 to 1L,
                    2 to "Renamed",
                    3 to "{{Reading}}",
                    4 to "{{Meanings}}",
                    5 to "{{Surface}}",
                    6 to "{{Sentence}}",
                )
            changes.forEach { (column, changed) ->
                f.model()
                f.provider.templates[7]!!.single()[column] = changed
                assertFalse(
                    "Column $column: $changed",
                    value(f.reader.modelMatches(AnkiProviderModelId(7), f.layout)),
                )
                assertEquals(changed, f.provider.templates[7]!!.single()[column])
            }
            f.provider.templates[7] = emptyList()
            assertFalse(value(f.reader.modelMatches(AnkiProviderModelId(7), f.layout)))
            f.model()
            f.provider.templates[7] = List(2) { f.provider.templates[7]!!.single().clone() }
            assertFalse(value(f.reader.modelMatches(AnkiProviderModelId(7), f.layout)))
        }
    }

    @Test
    fun malformedModelListsAndSchemasNeverPublishPartialSuccess() {
        Fixture().use { f ->
            for (row in
                listOf(
                    arrayOf<Any?>(0L, "Model"),
                    arrayOf<Any?>("7", "Model"),
                    arrayOf<Any?>(7L, null),
                    arrayOf<Any?>(7L, ""),
                    arrayOf<Any?>(1L, "Repeated"),
                )) {
                val cursor =
                    MatrixCursor(arrayOf("_id", "name")).apply {
                        addRow(arrayOf<Any?>(1L, "Valid"))
                        addRow(row)
                    }
                f.provider.replacement = { cursor }
                failed(AnkiReadError.InvalidResponse, f.reader.models())
                assertTrue(cursor.isClosed)
            }
            val cursor = MatrixCursor(arrayOf("_id"))
            f.provider.replacement = { cursor }
            failed(AnkiReadError.InvalidResponse, f.reader.models())
            assertTrue(cursor.isClosed)
        }
    }

    @Test
    fun malformedModelAndTemplateResponsesFailInsteadOfClaimingIncompatibility() {
        Fixture().use { f ->
            for (templates in listOf(false, true)) {
                val columns = if (templates) TEMPLATE_COLUMNS else MODEL_COLUMNS
                val changes =
                    if (templates) listOf(0 to 8L, 0 to "7", 1 to "0", 3 to null)
                    else listOf(0 to 8L, 0 to "7", 3 to "1", 5 to null, 2 to null)
                changes.forEach { (column, changed) ->
                    f.model()
                    val row =
                        if (templates) f.provider.templates[7]!!.single()
                        else f.provider.models[7]!!
                    row[column] = changed
                    failed(
                        AnkiReadError.InvalidResponse,
                        f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
                    )
                }
                f.model()
                var closed = false
                f.provider.replacement = {
                    val isTemplateQuery = f.provider.lastUri!!.lastPathSegment == "templates"
                    if (isTemplateQuery != templates) {
                        MatrixCursor(MODEL_COLUMNS).apply { addRow(f.provider.models[7]) }
                    } else
                        object : CursorWrapper(MatrixCursor(columns.dropLast(1).toTypedArray())) {
                            override fun close() {
                                super.close()
                                closed = true
                            }
                        }
                }
                failed(
                    AnkiReadError.InvalidResponse,
                    f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
                )
                assertTrue(closed)
                f.provider.replacement = null
            }
            f.model()
            f.provider.replacement = {
                MatrixCursor(MODEL_COLUMNS).apply {
                    repeat(2) { addRow(f.provider.models[7]) }
                }
            }
            failed(
                AnkiReadError.InvalidResponse,
                f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
            )
        }
    }

    @Test
    fun modelReadsDiscardNullTransportPartialAndCloseFailuresAtEveryStage() {
        Fixture().use { f ->
            f.model()
            for (stage in listOf("models", "7", "templates")) {
                for (failure in listOf("null", "transport", "iteration", "close")) {
                    var closed = false
                    f.provider.replacement = {
                        val columns =
                            if (stage == "models") arrayOf("_id", "name")
                            else if (stage == "templates") TEMPLATE_COLUMNS else MODEL_COLUMNS
                        if (f.provider.lastUri!!.lastPathSegment != stage) {
                            MatrixCursor(MODEL_COLUMNS).apply { addRow(f.provider.models[7]) }
                        } else
                            when (failure) {
                                "null" -> null
                                "transport" -> throw android.os.DeadObjectException()
                                else ->
                                    object :
                                        CursorWrapper(
                                            MatrixCursor(columns).apply {
                                                addRow(
                                                    when (stage) {
                                                        "models" -> arrayOf<Any?>(7L, f.layout.name)
                                                        "templates" ->
                                                            f.provider.templates[7]!!.single()
                                                        else -> f.provider.models[7]!!
                                                    }
                                                )
                                            }
                                        ) {
                                        var moves = 0

                                        override fun moveToNext(): Boolean {
                                            if (++moves == 2 && failure == "iteration")
                                                error("Partial read")
                                            return super.moveToNext()
                                        }

                                        override fun close() {
                                            super.close()
                                            closed = true
                                            if (failure == "close") error("Close failed")
                                        }
                                    }
                            }
                    }
                    val result =
                        if (stage == "models") f.reader.models()
                        else f.reader.modelMatches(AnkiProviderModelId(7), f.layout)
                    failed(AnkiReadError.CollectionUnavailable, result)
                    if (failure in listOf("iteration", "close")) assertTrue(closed)
                }
            }
        }
    }

    @Test
    fun modelOperationsRecheckPermissionSpecAndProviderBetweenReads() {
        Fixture().use { f ->
            f.environment.permitted = false
            failed(AnkiReadError.PermissionDenied, f.reader.models())
            failed(
                AnkiReadError.PermissionDenied,
                f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
            )
            assertEquals(0, f.provider.queries)
            f.environment.permitted = true
            f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
            failed(AnkiReadError.UnsupportedProvider, f.reader.models())
            failed(
                AnkiReadError.UnsupportedProvider,
                f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
            )
            f.environment.info!!.metaData.putInt(AnkiProviderReader.SPEC_KEY, 2)
            f.model()
            for (replacePackage in listOf(false, true)) {
                f.environment.permitted = true
                f.environment.info!!.packageName = "test.anki.host"
                f.provider.replacement = {
                    if (replacePackage) f.environment.info!!.packageName = "test.other.host"
                    else f.environment.permitted = false
                    MatrixCursor(MODEL_COLUMNS).apply { addRow(f.provider.models[7]) }
                }
                val before = f.provider.queries
                failed(
                    if (replacePackage) AnkiReadError.ProviderUnavailable
                    else AnkiReadError.PermissionDenied,
                    f.reader.modelMatches(AnkiProviderModelId(7), f.layout),
                )
                assertEquals(before + 1, f.provider.queries)
            }
        }
    }

    @Test
    fun expectedModelLayoutSnapshotsFieldsAndRejectsInvalidTransportInputs() {
        val fields = mutableListOf("Surface", "Reading", "Meanings", "Sentence")
        fun layout() = AnkiProviderModelLayout("Model", fields, "css", "Card", "question", "answer")
        val expected = layout()
        fields.reverse()
        assertEquals("Surface\u001fReading\u001fMeanings\u001fSentence", expected.joinedFieldNames)
        fields[0] = "bad\u001ffield"
        assertThrows(IllegalArgumentException::class.java) { layout() }
        fields[0] = "bad\u0000field"
        assertThrows(IllegalArgumentException::class.java) { layout() }
        fields[0] = fields[1]
        assertThrows(IllegalArgumentException::class.java) { layout() }
        fields.removeAt(0)
        assertThrows(IllegalArgumentException::class.java) { layout() }
        assertThrows(IllegalArgumentException::class.java) { AnkiProviderModelId(0) }
    }

    @Test
    fun providerSpecPermissionAndEnabledGatesPrecedeEveryRead() {
        Fixture().use { f ->
            val supported = f.environment.info!!
            f.environment.info = null
            failed(AnkiReadError.ProviderUnavailable, f.reader.duplicates("猫", "ねこ"))
            f.environment.info = supported
            supported.metaData = null
            failed(AnkiReadError.UnsupportedProvider, f.reader.decks())
            supported.metaData = Bundle().apply { putInt(AnkiProviderReader.SPEC_KEY, 1) }
            failed(AnkiReadError.UnsupportedProvider, f.reader.duplicates("猫", "ねこ"))
            supported.metaData.putInt(AnkiProviderReader.SPEC_KEY, 2)
            supported.enabled = false
            failed(AnkiReadError.ProviderUnavailable, f.reader.availability())
            supported.enabled = true
            supported.exported = false
            failed(AnkiReadError.ProviderUnavailable, f.reader.decks())
            supported.exported = true
            supported.applicationInfo.enabled = false
            failed(AnkiReadError.ProviderUnavailable, f.reader.availability())
            supported.applicationInfo.enabled = true
            f.environment.permitted = false
            failed(AnkiReadError.PermissionDenied, f.reader.duplicates("猫", "ねこ"))
            assertEquals(0, f.provider.queries)
            f.environment.permitted = true
            assertEquals("test.anki.host", value(f.reader.availability()))
            assertTrue(value(f.reader.duplicates("猫", "ねこ")).isEmpty())
            f.environment.permitted = false
            failed(AnkiReadError.PermissionDenied, f.reader.decks())
            f.environment.permitted = true
            supported.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
            assertEquals(
                AnkiReadResult.Failed(AnkiReadError.UnsupportedProvider),
                f.reader.duplicates("猫", "ねこ"),
            )
            assertEquals(1, f.provider.queries)
        }
    }

    @Test
    fun decksPreserveLongIdsNamesAndFilteredFlagWithoutSelectingOrCreating() {
        Fixture().use { f ->
            assertTrue(value(f.reader.decks()).isEmpty())
            f.provider.database.execSQL(
                "INSERT INTO decks VALUES(?, ?, ?)",
                arrayOf<Any?>(Long.MAX_VALUE, " 日本語::単語 ", 0),
            )
            f.provider.database.execSQL("INSERT INTO decks VALUES(9, 'Filtered', 1)")
            assertEquals(
                listOf(
                    AnkiProviderDeck(Long.MAX_VALUE, " 日本語::単語 ", false),
                    AnkiProviderDeck(9, "Filtered", true),
                ),
                value(f.reader.decks()),
            )
            assertEquals(listOf("deck_id", "deck_name", "deck_dyn"), f.provider.lastProjection)
            assertNull(f.provider.lastSelection)
            assertNull(f.provider.lastArguments)
            assertEquals(
                "content://${AnkiProviderReader.AUTHORITY}/decks",
                f.provider.lastUri.toString(),
            )
            f.provider.replacement = {
                MatrixCursor(arrayOf("deck_id", "deck_name", "deck_dyn")).apply {
                    // Match the real provider's Boolean-in-MatrixCursor representation too.
                    addRow(arrayOf<Any?>(1L, "Normal", false))
                    addRow(arrayOf<Any?>(2L, "Filtered", true))
                }
            }
            assertEquals(
                listOf(AnkiProviderDeck(1, "Normal", false), AnkiProviderDeck(2, "Filtered", true)),
                value(f.reader.decks()),
            )
        }
    }

    @Test
    fun duplicateIdentityRequiresTheNamedReadingAcrossModelsAndDoesNotSplitReadings() {
        Fixture().use { f ->
            f.provider.note(1, "生", model = 11, rest = "せい")
            f.provider.note(2, "生", model = 12, rest = "なま")
            f.provider.note(3, "生", model = 13, rest = "meaning\u001fせい")
            f.provider.models[13]!![2] = "Expression\u001fMeaning\u001freading"
            f.provider.note(4, "生", model = 14, rest = "せい")
            f.provider.models[14]!![2] = "Expression\u001fMeaning"
            f.provider.note(5, "生", model = 15, rest = "せい、なま")
            assertEquals(
                listOf(AnkiProviderNoteId(1), AnkiProviderNoteId(3)),
                value(f.reader.duplicates("生", "せい")),
            )
            assertEquals(listOf(AnkiProviderNoteId(2)), value(f.reader.duplicates("生", "なま")))
            assertTrue(value(f.reader.duplicates("生", "セイ")).isEmpty())
            assertTrue(value(f.reader.duplicates("生", " せい")).isEmpty())
            f.provider.note(6, "生", model = 16, rest = "&lt;reading&gt;")
            assertEquals(
                listOf(AnkiProviderNoteId(6)),
                value(f.reader.duplicates("生", "<reading>")),
            )
            f.provider.queryResult = { uri, cursor ->
                if (uri.pathSegments.first() == "models") {
                    cursor?.close()
                    null
                } else cursor
            }
            failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("生", "せい"))
        }
    }

    @Test
    fun finalInsertionCheckDistinguishesReadingsAndRejectsMismatchedFrozenFields() {
        Fixture().use { f ->
            f.enableNoteInsertion()
            f.provider.note(8, "猫", rest = "びょう")
            assertTrue(f.inserter.insert(f.noteRequest()) is AnkiNoteInsertionResult.Inserted)
            assertEquals(
                listOf(AnkiProviderNoteId(Long.MAX_VALUE)),
                value(f.reader.duplicates("猫", "ねこ")),
            )
            assertEquals(listOf(AnkiProviderNoteId(8)), value(f.reader.duplicates("猫", "びょう")))
            assertTrue(f.inserter.insert(f.noteRequest()) is AnkiNoteInsertionResult.Existing)
            assertEquals(1, f.provider.inserts.size)
            assertThrows(IllegalArgumentException::class.java) {
                AnkiProviderNoteRequest(
                    "猫",
                    "びょう",
                    listOf("猫", "ねこ", "", ""),
                    AnkiProviderModelId(7),
                    Long.MAX_VALUE,
                    f.layout,
                )
            }
        }
    }

    @Test
    fun duplicatesRetainEveryExactMatchAcrossModelsAndDecksReadOnly() {
        Fixture().use { f ->
            f.provider.note(Long.MAX_VALUE, "猫", model = 5, deck = 8)
            f.provider.note(7, "猫", model = 9, deck = 3)
            f.provider.note(8, "犬", rest = "猫")
            f.provider.note(9, "猫と犬")
            val before = f.provider.rows()
            assertEquals(
                setOf(AnkiProviderNoteId(7), AnkiProviderNoteId(Long.MAX_VALUE)),
                value(f.reader.duplicates("猫", "ねこ")).toSet(),
            )
            assertEquals(listOf("_id", "mid", "flds"), f.provider.lastNoteProjection)
            assertEquals(
                "content://${AnkiProviderReader.AUTHORITY}/notes_v2",
                f.provider.lastNoteUri.toString(),
            )
            assertFalse(f.provider.lastNoteSelection!!.contains("mid"))
            assertFalse(f.provider.lastNoteSelection!!.contains("did"))
            assertEquals(before, f.provider.rows())
        }
    }

    @Test
    fun identityDoesNotNormalizeTrimDecodeArbitraryHtmlOrSearchOtherFields() {
        Fixture().use { f ->
            val surfaces =
                listOf(
                    "ガ",
                    "ｶﾞ",
                    "é",
                    "e\u0301",
                    "猫",
                    " 猫 ",
                    "食べた",
                    "食べる",
                    "Cat",
                    "cat",
                    "𠮷",
                    "猫\n\t犬",
                )
            surfaces.forEachIndexed { index, surface ->
                f.provider.note(index.toLong() + 1, surface)
            }
            f.provider.note(100, "<b>猫</b>")
            f.provider.note(101, "&#29483;")
            f.provider.note(102, "猫[ねこ]")
            f.provider.note(103, "other", rest = "猫")
            surfaces.forEachIndexed { index, surface ->
                assertEquals(
                    listOf(AnkiProviderNoteId(index.toLong() + 1)),
                    value(f.reader.duplicates(surface, "ねこ")),
                )
            }
            assertTrue(value(f.reader.duplicates("ねこ", "ねこ")).isEmpty())
            // For a literal-markup surface, only the escaped representation proves identity.
            f.provider.note(104, "&lt;b&gt;猫&lt;/b&gt;")
            assertEquals(
                listOf(AnkiProviderNoteId(104)),
                value(f.reader.duplicates("<b>猫</b>", "ねこ")),
            )
        }
    }

    @Test
    fun escapedSurfaceParityAndBoundArgumentsKeepMarkupMediaAndSqlLiteral() {
        Fixture().use { f ->
            val raw = "<&\"'>[sound:猫.mp3]\u001f"
            val escaped = "&lt;&amp;&quot;&#39;&gt;&#91;sound:猫.mp3&#93;&#31;"
            f.provider.note(1, escaped)
            assertEquals(listOf(AnkiProviderNoteId(1)), value(f.reader.duplicates(raw, "ねこ")))
            assertFalse(f.provider.lastNoteSelection!!.contains(raw))
            val quoted = "猫' OR 1=1 --_%"
            f.provider.note(2, quoted)
            f.provider.note(3, "猫&#39; OR 1=1 --_%")
            f.provider.note(4, "unrelated")
            assertEquals(
                setOf(AnkiProviderNoteId(2), AnkiProviderNoteId(3)),
                value(f.reader.duplicates(quoted, "ねこ")).toSet(),
            )
            assertTrue(f.provider.lastNoteArguments!!.contains(quoted))
            assertFalse(f.provider.lastNoteSelection!!.contains(quoted))
            val queries = f.provider.queries
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫\u0000犬", "ねこ"))
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates("", "ねこ"))
            assertEquals(queries, f.provider.queries)
        }
    }

    @Test
    fun nullDeniedTransportAndUnconfiguredCollectionAreNeverEmptyResults() {
        Fixture().use { f ->
            f.provider.replacement = { null }
            failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫", "ねこ"))
            failed(AnkiReadError.CollectionUnavailable, f.reader.decks())
            f.provider.replacement = { throw SecurityException("revoked after gate") }
            failed(AnkiReadError.PermissionDenied, f.reader.duplicates("猫", "ねこ"))
            f.provider.replacement = {
                throw IllegalStateException("unconfigured/locked collection")
            }
            failed(AnkiReadError.CollectionUnavailable, f.reader.decks())
            f.provider.replacement = { throw android.os.DeadObjectException() }
            failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫", "ねこ"))
        }
    }

    @Test
    fun malformedRowsAndSchemasFailClosedAndCloseCursors() {
        Fixture().use { f ->
            for (row in
                listOf(
                    arrayOf<Any?>(0L, 1L, "猫"),
                    arrayOf<Any?>(null, 1L, "猫"),
                    arrayOf<Any?>("7", 1L, "猫"),
                    arrayOf<Any?>(7L, -1L, "猫"),
                    arrayOf<Any?>(7L, 1L, null),
                )) {
                val cursor = MatrixCursor(arrayOf("_id", "mid", "flds")).apply { addRow(row) }
                f.provider.replacement = { cursor }
                failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫", "ねこ"))
                assertTrue(cursor.isClosed)
            }
            val cursor = MatrixCursor(arrayOf("_id"))
            f.provider.replacement = { cursor }
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫", "ねこ"))
            assertTrue(cursor.isClosed)
            // Even if a provider ignores the predicate, compare the entire first field locally.
            f.provider.replacement = {
                if (f.provider.lastUri?.pathSegments?.first() == "models") {
                    MatrixCursor(arrayOf("_id", "field_names")).apply {
                        addRow(arrayOf<Any?>(2L, "Surface\u001fReading"))
                    }
                } else
                    MatrixCursor(arrayOf("_id", "mid", "flds")).apply {
                        addRow(arrayOf<Any?>(7L, 1L, "犬\u001f猫"))
                        addRow(arrayOf<Any?>(8L, 2L, "猫\u001fねこ"))
                        addRow(arrayOf<Any?>(8L, 2L, "猫\u001fねこ"))
                    }
            }
            assertEquals(listOf(AnkiProviderNoteId(8)), value(f.reader.duplicates("猫", "ねこ")))
        }
    }

    @Test
    fun partialIterationAndCloseFailuresDiscardAllObservedIds() {
        Fixture().use { f ->
            for (failDuringClose in listOf(false, true)) {
                var closed = false
                val cursor =
                    object :
                        CursorWrapper(
                            MatrixCursor(arrayOf("_id", "mid", "flds")).apply {
                                addRow(arrayOf<Any?>(7L, 1L, "猫"))
                            }
                        ) {
                        var moves = 0

                        override fun moveToNext(): Boolean {
                            if (++moves == 2 && !failDuringClose)
                                throw IllegalStateException("cursor transport lost")
                            return super.moveToNext()
                        }

                        override fun close() {
                            super.close()
                            closed = true
                            if (failDuringClose) throw IllegalStateException("close failed")
                        }
                    }
                f.provider.replacement = { cursor }
                failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫", "ねこ"))
                assertTrue(closed)
            }
        }
    }

    @Test
    fun malformedDecksFailWithoutPublishingPartialLists() {
        Fixture().use { f ->
            for (row in
                listOf(
                    arrayOf<Any?>(0L, "deck", 0L),
                    arrayOf<Any?>(7L, null, 0L),
                    arrayOf<Any?>(7L, "deck", 2L),
                )) {
                val cursor =
                    MatrixCursor(arrayOf("deck_id", "deck_name", "deck_dyn")).apply {
                        addRow(arrayOf<Any?>(1L, "Valid", 0L))
                        addRow(row)
                    }
                f.provider.replacement = { cursor }
                failed(AnkiReadError.InvalidResponse, f.reader.decks())
                assertTrue(cursor.isClosed)
            }
        }
    }

    @Test
    fun freshEmptyAndFailedDuplicateReadsStayDistinctWithoutAnyWrite() {
        Fixture().use { f ->
            repeat(2) {
                assertEquals(
                    AnkiReadResult.Value(emptyList<AnkiProviderNoteId>()),
                    f.reader.duplicates("猫", "ねこ"),
                )
            }
            f.provider.replacement = { null }
            assertEquals(
                AnkiReadResult.Failed(AnkiReadError.DuplicateLookupFailed),
                f.reader.duplicates("猫", "ねこ"),
            )
            f.provider.replacement = null
            f.provider.note(7, "猫", model = 9, deck = 10)
            f.provider.note(8, "猫", model = 11, deck = 12)
            val before = f.provider.rows()
            assertEquals(
                AnkiReadResult.Value(listOf(AnkiProviderNoteId(7), AnkiProviderNoteId(8))),
                f.reader.duplicates("猫", "ねこ"),
            )
            assertEquals(before, f.provider.rows())
        }
    }

    @Test
    fun browserLinkUsesExactLongNoteIdAndOnlyResolvedProviderPackage() {
        Fixture().use { f ->
            val intent = value(f.reader.existingNoteIntent(AnkiProviderNoteId(Long.MAX_VALUE)))!!
            assertEquals(Intent.ACTION_VIEW, intent.action)
            assertEquals("test.anki.host", intent.`package`)
            assertEquals("anki", intent.data!!.scheme)
            assertEquals("x-callback-url", intent.data!!.authority)
            assertEquals("/browser", intent.data!!.path)
            assertEquals("nid:${Long.MAX_VALUE}", intent.data!!.getQueryParameter("search"))
            val entries =
                value(
                    f.reader.existingNotesIntent(
                        listOf(
                            AnkiProviderNoteId(Long.MAX_VALUE),
                            AnkiProviderNoteId(9007199254740993L),
                            AnkiProviderNoteId(Long.MAX_VALUE),
                        )
                    )
                )!!
            assertEquals(
                "nid:${Long.MAX_VALUE},9007199254740993",
                entries.data!!.getQueryParameter("search"),
            )
            assertNull(intent.component)
            f.environment.canOpen = false
            assertNull(value(f.reader.existingNoteIntent(AnkiProviderNoteId(7))))
            f.environment.info!!.packageName = "test.replaced.host"
            f.environment.canOpen = true
            assertEquals(
                "test.replaced.host",
                value(f.reader.existingNoteIntent(AnkiProviderNoteId(7)))!!.`package`,
            )
            f.environment.permitted = false
            failed(
                AnkiReadError.PermissionDenied,
                f.reader.existingNoteIntent(AnkiProviderNoteId(7)),
            )
            assertEquals(0, f.provider.queries)
            assertThrows(IllegalArgumentException::class.java) { AnkiProviderNoteId(0) }
        }
    }

    @Test
    fun everyEntryPointRejectsMainThreadProviderAccess() {
        Fixture().use { f ->
            InstrumentationRegistry.getInstrumentation().runOnMainSync {
                val operations =
                    listOf<() -> Any?>(
                        { f.reader.availability() },
                        { f.reader.models() },
                        { f.reader.modelMatches(AnkiProviderModelId(7), f.layout) },
                        { f.reader.decks() },
                        { f.reader.duplicates("猫", "ねこ") },
                        { f.reader.existingNoteIntent(AnkiProviderNoteId(7)) },
                        { f.creator.create(f.layout) },
                    )
                operations.forEach { operation ->
                    assertThrows(IllegalStateException::class.java) { operation() }
                }
            }
            assertEquals(0, f.provider.queries)
        }
    }

    companion object {
        private val MODEL_COLUMNS =
            arrayOf("_id", "name", "field_names", "num_cards", "css", "sort_field_index", "type")
        private val TEMPLATE_COLUMNS =
            arrayOf(
                "model_id",
                "ord",
                "card_template_name",
                "question_format",
                "answer_format",
                "browser_question_format",
                "browser_answer_format",
            )
    }
}
