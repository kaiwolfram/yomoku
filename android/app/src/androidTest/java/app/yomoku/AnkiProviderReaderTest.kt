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
            }
        var queries = 0
        var replacement: (() -> Cursor?)? = null
        var lastUri: Uri? = null
        var lastProjection: List<String>? = null
        var lastSelection: String? = null
        var lastArguments: List<String>? = null
        val models = linkedMapOf<Long, Array<Any?>>()
        val templates = linkedMapOf<Long, List<Array<Any?>>>()
        val inserts = mutableListOf<Pair<Uri, ContentValues>>()
        val updates = mutableListOf<Pair<Uri, ContentValues>>()
        var insertReplacement: ((Uri, ContentValues) -> Uri?)? = null
        var updateReplacement: ((Uri, ContentValues) -> Int)? = null

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
            replacement?.let {
                return it()
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
            rest: String = "meaning",
        ) {
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
        // Comparison inputs stand in for the future Rust-owned layout transport, not production
        // copies of the core's HTML/CSS constants.
        val layout =
            AnkiProviderModelLayout(
                "Yomoku v1",
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

        override fun close() = provider.close()
    }

    private fun <T> value(result: AnkiReadResult<T>): T = (result as AnkiReadResult.Value).value

    private fun failed(reason: AnkiReadError, result: AnkiReadResult<*>) {
        assertEquals(AnkiReadResult.Failed(reason), result)
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
            failed(AnkiReadError.ProviderUnavailable, f.reader.duplicates("猫"))
            f.environment.info = supported
            supported.metaData = null
            failed(AnkiReadError.UnsupportedProvider, f.reader.decks())
            supported.metaData = Bundle().apply { putInt(AnkiProviderReader.SPEC_KEY, 1) }
            failed(AnkiReadError.UnsupportedProvider, f.reader.duplicates("猫"))
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
            failed(AnkiReadError.PermissionDenied, f.reader.duplicates("猫"))
            assertEquals(0, f.provider.queries)
            f.environment.permitted = true
            assertEquals("test.anki.host", value(f.reader.availability()))
            assertTrue(value(f.reader.duplicates("猫")).isEmpty())
            f.environment.permitted = false
            failed(AnkiReadError.PermissionDenied, f.reader.decks())
            f.environment.permitted = true
            supported.metaData.putInt(AnkiProviderReader.SPEC_KEY, 1)
            assertEquals(
                AnkiReadReconciliation.Failed(AnkiReadError.UnsupportedProvider),
                f.reader.reconcile("猫"),
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
    fun duplicatesRetainEveryExactMatchAcrossModelsAndDecksReadOnly() {
        Fixture().use { f ->
            f.provider.note(Long.MAX_VALUE, "猫", model = 5, deck = 8)
            f.provider.note(7, "猫", model = 9, deck = 3)
            f.provider.note(8, "犬", rest = "猫")
            f.provider.note(9, "猫と犬")
            val before = f.provider.rows()
            assertEquals(
                setOf(AnkiProviderNoteId(7), AnkiProviderNoteId(Long.MAX_VALUE)),
                value(f.reader.duplicates("猫")).toSet(),
            )
            assertEquals(listOf("_id", "mid", "flds"), f.provider.lastProjection)
            assertEquals(
                "content://${AnkiProviderReader.AUTHORITY}/notes_v2",
                f.provider.lastUri.toString(),
            )
            assertFalse(f.provider.lastSelection!!.contains("mid"))
            assertFalse(f.provider.lastSelection!!.contains("did"))
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
                    value(f.reader.duplicates(surface)),
                )
            }
            assertTrue(value(f.reader.duplicates("ねこ")).isEmpty())
            // For a literal-markup surface, only the escaped representation proves identity.
            f.provider.note(104, "&lt;b&gt;猫&lt;/b&gt;")
            assertEquals(listOf(AnkiProviderNoteId(104)), value(f.reader.duplicates("<b>猫</b>")))
        }
    }

    @Test
    fun escapedSurfaceParityAndBoundArgumentsKeepMarkupMediaAndSqlLiteral() {
        Fixture().use { f ->
            val raw = "<&\"'>[sound:猫.mp3]\u001f"
            val escaped = "&lt;&amp;&quot;&#39;&gt;&#91;sound:猫.mp3&#93;&#31;"
            f.provider.note(1, escaped)
            assertEquals(listOf(AnkiProviderNoteId(1)), value(f.reader.duplicates(raw)))
            assertFalse(f.provider.lastSelection!!.contains(raw))
            val quoted = "猫' OR 1=1 --_%"
            f.provider.note(2, quoted)
            f.provider.note(3, "猫&#39; OR 1=1 --_%")
            f.provider.note(4, "unrelated")
            assertEquals(
                setOf(AnkiProviderNoteId(2), AnkiProviderNoteId(3)),
                value(f.reader.duplicates(quoted)).toSet(),
            )
            assertTrue(f.provider.lastArguments!!.contains(quoted))
            assertFalse(f.provider.lastSelection!!.contains(quoted))
            val queries = f.provider.queries
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫\u0000犬"))
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates(""))
            assertEquals(queries, f.provider.queries)
        }
    }

    @Test
    fun nullDeniedTransportAndUnconfiguredCollectionAreNeverEmptyResults() {
        Fixture().use { f ->
            f.provider.replacement = { null }
            failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫"))
            failed(AnkiReadError.CollectionUnavailable, f.reader.decks())
            f.provider.replacement = { throw SecurityException("revoked after gate") }
            failed(AnkiReadError.PermissionDenied, f.reader.duplicates("猫"))
            f.provider.replacement = {
                throw IllegalStateException("unconfigured/locked collection")
            }
            failed(AnkiReadError.CollectionUnavailable, f.reader.decks())
            f.provider.replacement = { throw android.os.DeadObjectException() }
            failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫"))
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
                failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫"))
                assertTrue(cursor.isClosed)
            }
            val cursor = MatrixCursor(arrayOf("_id"))
            f.provider.replacement = { cursor }
            failed(AnkiReadError.InvalidResponse, f.reader.duplicates("猫"))
            assertTrue(cursor.isClosed)
            // Even if a provider ignores the predicate, compare the entire first field locally.
            f.provider.replacement = {
                MatrixCursor(arrayOf("_id", "mid", "flds")).apply {
                    addRow(arrayOf<Any?>(7L, 1L, "犬\u001f猫"))
                    addRow(arrayOf<Any?>(8L, 2L, "猫\u001fmeaning"))
                    addRow(arrayOf<Any?>(8L, 2L, "猫\u001fmeaning"))
                }
            }
            assertEquals(listOf(AnkiProviderNoteId(8)), value(f.reader.duplicates("猫")))
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
                failed(AnkiReadError.DuplicateLookupFailed, f.reader.duplicates("猫"))
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
    fun reconciliationEmptyAndFailedReadsRemainUnresolvedWithoutAnyWrite() {
        Fixture().use { f ->
            repeat(2) { assertSame(AnkiReadReconciliation.Unresolved, f.reader.reconcile("猫")) }
            f.provider.replacement = { null }
            assertEquals(
                AnkiReadReconciliation.Failed(AnkiReadError.DuplicateLookupFailed),
                f.reader.reconcile("猫"),
            )
            f.provider.replacement = null
            f.provider.note(7, "猫", model = 9, deck = 10)
            f.provider.note(8, "猫", model = 11, deck = 12)
            val before = f.provider.rows()
            assertEquals(
                AnkiReadReconciliation.Existing(
                    listOf(AnkiProviderNoteId(7), AnkiProviderNoteId(8))
                ),
                f.reader.reconcile("猫"),
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
                        { f.reader.duplicates("猫") },
                        { f.reader.reconcile("猫") },
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
