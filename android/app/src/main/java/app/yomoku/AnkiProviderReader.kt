package app.yomoku

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ProviderInfo
import android.database.Cursor
import android.net.Uri
import android.os.Looper

/** Provider-local failures, translated to the typed recovery protocol by AnkiRecoveryReader. */
internal enum class AnkiReadError {
    ProviderUnavailable,
    UnsupportedProvider,
    PermissionDenied,
    CollectionUnavailable,
    DuplicateLookupFailed,
    InvalidResponse,
    ProviderFailure,
}

internal sealed interface AnkiReadResult<out T> {
    data class Value<T>(val value: T) : AnkiReadResult<T>

    data class Failed(val error: AnkiReadError) : AnkiReadResult<Nothing>
}

internal data class AnkiProviderNoteId(val value: Long) {
    init {
        require(value > 0)
    }
}

internal data class AnkiProviderDeck(val id: Long, val name: String, val filtered: Boolean)

internal sealed interface AnkiReadReconciliation {
    data class Existing(val ids: List<AnkiProviderNoteId>) : AnkiReadReconciliation {
        init {
            require(ids.isNotEmpty())
        }
    }

    data object Unresolved : AnkiReadReconciliation

    data class Failed(val error: AnkiReadError) : AnkiReadReconciliation
}

internal interface AnkiProviderEnvironment {
    fun provider(): ProviderInfo?

    fun hasPermission(): Boolean

    fun resolves(intent: Intent): Boolean
}

internal class AndroidAnkiProviderEnvironment(context: Context) : AnkiProviderEnvironment {
    private val app = context.applicationContext

    @Suppress("DEPRECATION")
    override fun provider(): ProviderInfo? =
        app.packageManager.resolveContentProvider(
            AnkiProviderReader.AUTHORITY,
            PackageManager.GET_META_DATA,
        )

    override fun hasPermission(): Boolean =
        app.checkSelfPermission(AnkiProviderReader.PERMISSION) == PackageManager.PERMISSION_GRANTED

    override fun resolves(intent: Intent): Boolean =
        intent.resolveActivity(app.packageManager) != null
}

/**
 * Read-only ContentResolver boundary. Call on a worker. No provider state is cached and no
 * permission dialog, Activity launch, model creation or note mutation occurs here. Explicit
 * recovery dispatch uses the app's captured handle and durable evidence commit gate.
 */
internal class AnkiProviderReader(
    private val resolver: ContentResolver,
    private val environment: AnkiProviderEnvironment,
) {
    constructor(
        context: Context
    ) : this(
        context.applicationContext.contentResolver,
        AndroidAnkiProviderEnvironment(context),
    )

    fun availability(): AnkiReadResult<String> = read(AnkiReadError.ProviderFailure) { host() }

    /** Discovery only. A name (even an exact match) never establishes a reusable layout. */
    fun models(): AnkiReadResult<List<AnkiProviderModel>> =
        read(AnkiReadError.CollectionUnavailable) {
            host()
            val cursor =
                resolver.query(uri("models"), arrayOf("_id", "name"), null, null, null)
                    ?: fail(AnkiReadError.CollectionUnavailable)
            try {
                val id = cursor.getColumnIndexOrThrow("_id")
                val name = cursor.getColumnIndexOrThrow("name")
                val models = linkedMapOf<AnkiProviderModelId, AnkiProviderModel>()
                while (cursor.moveToNext()) {
                    val modelId = AnkiProviderModelId(cursor.positiveId(id))
                    val label = cursor.requiredString(name)
                    if (label.isEmpty() || models.containsKey(modelId)) {
                        fail(AnkiReadError.InvalidResponse)
                    }
                    models[modelId] = AnkiProviderModel(modelId, label)
                }
                models.values.toList()
            } finally {
                cursor.close()
            }
        }

    /**
     * Fresh read-only validation by ID, including the actual template rows. A false result means
     * missing/incompatible, never permission to overwrite a same-name user model. This observation
     * is not a lease: a future mutating adapter must validate again before submission.
     */
    fun modelMatches(
        id: AnkiProviderModelId,
        expected: AnkiProviderModelLayout,
    ): AnkiReadResult<Boolean> =
        read(AnkiReadError.CollectionUnavailable) {
            val packageName = host()
            val modelUri = uri("models").buildUpon().appendPath(id.value.toString()).build()
            val cursor =
                resolver.query(
                    modelUri,
                    arrayOf(
                        "_id",
                        "name",
                        "field_names",
                        "num_cards",
                        "css",
                        "sort_field_index",
                        "type",
                    ),
                    null,
                    null,
                    null,
                ) ?: fail(AnkiReadError.CollectionUnavailable)
            val metadataMatches =
                try {
                    // Resolve all columns even for an empty cursor: a malformed schema is failure.
                    val columns =
                        listOf(
                                "_id",
                                "name",
                                "field_names",
                                "num_cards",
                                "css",
                                "sort_field_index",
                                "type",
                            )
                            .map { cursor.getColumnIndexOrThrow(it) }
                    if (!cursor.moveToNext()) false
                    else {
                        if (cursor.positiveId(columns[0]) != id.value)
                            fail(AnkiReadError.InvalidResponse)
                        val name = cursor.requiredString(columns[1])
                        val fields = cursor.requiredString(columns[2])
                        val count = cursor.requiredLong(columns[3])
                        val css = cursor.requiredString(columns[4])
                        val sort = cursor.requiredLong(columns[5])
                        val type = cursor.requiredLong(columns[6])
                        if (cursor.moveToNext()) fail(AnkiReadError.InvalidResponse)
                        name == expected.name &&
                            fields == expected.joinedFieldNames &&
                            count == 1L &&
                            css == expected.css &&
                            sort == 0L &&
                            type == 0L
                    }
                } finally {
                    cursor.close()
                }
            if (!metadataMatches) return@read false
            // Permission/provider observations must remain fresh between the two provider calls.
            if (host() != packageName) fail(AnkiReadError.ProviderUnavailable)
            val templates =
                resolver.query(
                    modelUri.buildUpon().appendPath("templates").build(),
                    arrayOf(
                        "model_id",
                        "ord",
                        "card_template_name",
                        "question_format",
                        "answer_format",
                        "browser_question_format",
                        "browser_answer_format",
                    ),
                    null,
                    null,
                    null,
                ) ?: fail(AnkiReadError.CollectionUnavailable)
            try {
                val columns =
                    listOf(
                            "model_id",
                            "ord",
                            "card_template_name",
                            "question_format",
                            "answer_format",
                            "browser_question_format",
                            "browser_answer_format",
                        )
                        .map { templates.getColumnIndexOrThrow(it) }
                if (!templates.moveToNext()) false
                else {
                    if (templates.positiveId(columns[0]) != id.value)
                        fail(AnkiReadError.InvalidResponse)
                    val ordinal = templates.requiredLong(columns[1])
                    val name = templates.requiredString(columns[2])
                    val question = templates.requiredString(columns[3])
                    val answer = templates.requiredString(columns[4])
                    val browserQuestion = templates.requiredString(columns[5])
                    val browserAnswer = templates.requiredString(columns[6])
                    // Do not confuse the provider's synthetic, one-based _id with zero-based ord.
                    val extra = templates.moveToNext()
                    !extra &&
                        ordinal == 0L &&
                        name == expected.cardName &&
                        question == expected.questionFormat &&
                        answer == expected.answerFormat &&
                        browserQuestion.isEmpty() &&
                        browserAnswer.isEmpty()
                }
            } finally {
                templates.close()
            }
        }

    fun decks(): AnkiReadResult<List<AnkiProviderDeck>> =
        read(AnkiReadError.CollectionUnavailable) {
            host()
            val cursor =
                resolver.query(
                    uri("decks"),
                    arrayOf("deck_id", "deck_name", "deck_dyn"),
                    null,
                    null,
                    null,
                ) ?: fail(AnkiReadError.CollectionUnavailable)
            try {
                val id = cursor.getColumnIndexOrThrow("deck_id")
                val name = cursor.getColumnIndexOrThrow("deck_name")
                val filtered = cursor.getColumnIndexOrThrow("deck_dyn")
                val decks = linkedMapOf<Long, AnkiProviderDeck>()
                while (cursor.moveToNext()) {
                    val value = cursor.positiveId(id)
                    val label = cursor.requiredString(name)
                    val dynamic = cursor.filteredDeck(filtered)
                    if (label.isEmpty() || decks.containsKey(value)) {
                        fail(AnkiReadError.InvalidResponse)
                    }
                    decks[value] = AnkiProviderDeck(value, label, dynamic)
                }
                decks.values.toList()
            } finally {
                cursor.close()
            }
        }

    /**
     * Query every model/deck, using bound whole-first-field candidates. Accept only literal plain
     * text or Yomoku's exact escaping; arbitrary HTML, alternate entities and combined fields are
     * not decoded into identities. Never normalize, trim, case-fold or use a checksum alone.
     */
    fun duplicates(surface: String): AnkiReadResult<List<AnkiProviderNoteId>> =
        read(AnkiReadError.DuplicateLookupFailed) {
            host()
            // Android SQLite string transport cannot establish lossless identity for embedded NUL.
            if (surface.isEmpty() || '\u0000' in surface) fail(AnkiReadError.InvalidResponse)
            val candidates = linkedSetOf(escapeSurface(surface))
            if (surface.none { it in "&<>[]\u001f" }) candidates.add(surface)
            val selection =
                candidates.joinToString(" OR ") {
                    "(flds = ? COLLATE BINARY OR instr(flds, ?) = 1)"
                }
            val arguments = candidates.flatMap { listOf(it, "$it\u001f") }.toTypedArray()
            val cursor =
                resolver.query(
                    uri("notes_v2"),
                    arrayOf("_id", "mid", "flds"),
                    selection,
                    arguments,
                    null,
                ) ?: fail(AnkiReadError.DuplicateLookupFailed)
            try {
                val id = cursor.getColumnIndexOrThrow("_id")
                val model = cursor.getColumnIndexOrThrow("mid")
                val fields = cursor.getColumnIndexOrThrow("flds")
                val notes = linkedSetOf<AnkiProviderNoteId>()
                while (cursor.moveToNext()) {
                    val noteId = AnkiProviderNoteId(cursor.positiveId(id))
                    cursor.positiveId(model)
                    val first = cursor.requiredString(fields).substringBefore('\u001f')
                    // Verify returned rows independently of SQL filtering/provider
                    // implementation.
                    if (first in candidates) notes.add(noteId)
                }
                notes.toList()
            } finally {
                cursor.close()
            }
        }

    /** Empty reads never authorize another insertion, even after a successful query. */
    fun reconcile(surface: String): AnkiReadReconciliation =
        when (val result = duplicates(surface)) {
            is AnkiReadResult.Failed -> AnkiReadReconciliation.Failed(result.error)
            is AnkiReadResult.Value ->
                if (result.value.isEmpty()) AnkiReadReconciliation.Unresolved
                else AnkiReadReconciliation.Existing(result.value)
        }

    /**
     * Resolve the supported browser link only in the current provider's package. Does not launch.
     */
    fun existingNoteIntent(id: AnkiProviderNoteId): AnkiReadResult<Intent?> =
        existingNotesIntent(listOf(id))

    fun existingNotesIntent(ids: List<AnkiProviderNoteId>): AnkiReadResult<Intent?> =
        read(AnkiReadError.ProviderFailure) {
            val packageName = host()
            if (ids.isEmpty()) fail(AnkiReadError.InvalidResponse)
            val link =
                Uri.Builder()
                    .scheme("anki")
                    .authority("x-callback-url")
                    .appendPath("browser")
                    .appendQueryParameter(
                        "search",
                        "nid:${ids.distinct().joinToString(",") { it.value.toString() }}",
                    )
                    .build()
            val intent = Intent(Intent.ACTION_VIEW, link).setPackage(packageName)
            intent.takeIf { environment.resolves(it) }
        }

    private fun host(): String {
        val provider = environment.provider() ?: fail(AnkiReadError.ProviderUnavailable)
        if (!provider.enabled || provider.applicationInfo?.enabled == false || !provider.exported) {
            fail(AnkiReadError.ProviderUnavailable)
        }
        if ((provider.metaData?.getInt(SPEC_KEY, 1) ?: 1) < 2)
            fail(AnkiReadError.UnsupportedProvider)
        if (!environment.hasPermission()) fail(AnkiReadError.PermissionDenied)
        return provider.packageName?.takeIf { it.isNotEmpty() }
            ?: fail(AnkiReadError.InvalidResponse)
    }

    private inline fun <T> read(fallback: AnkiReadError, block: () -> T): AnkiReadResult<T> {
        check(Looper.myLooper() != Looper.getMainLooper()) { "Anki provider access on main thread" }
        return try {
            AnkiReadResult.Value(block())
        } catch (error: ReadFailure) {
            AnkiReadResult.Failed(error.reason)
        } catch (_: SecurityException) {
            AnkiReadResult.Failed(AnkiReadError.PermissionDenied)
        } catch (_: IllegalArgumentException) {
            AnkiReadResult.Failed(AnkiReadError.InvalidResponse)
        } catch (_: Exception) {
            // Includes cursor iteration/close failures and transport/collection errors. Partial
            // results must not escape as an empty or successful duplicate observation.
            AnkiReadResult.Failed(fallback)
        }
    }

    private fun Cursor.requiredString(column: Int): String {
        if (getType(column) != Cursor.FIELD_TYPE_STRING) fail(AnkiReadError.InvalidResponse)
        return getString(column) ?: fail(AnkiReadError.InvalidResponse)
    }

    private fun Cursor.requiredLong(column: Int): Long {
        if (getType(column) != Cursor.FIELD_TYPE_INTEGER) fail(AnkiReadError.InvalidResponse)
        return getLong(column)
    }

    private fun Cursor.positiveId(column: Int): Long =
        requiredLong(column).also { if (it <= 0) fail(AnkiReadError.InvalidResponse) }

    private fun Cursor.filteredDeck(column: Int): Boolean =
        // The contract documents 0/1, but 2.25.3 adds a Boolean to its MatrixCursor. That
        // reaches string cursor access as "true"/"false". Reject all other representations.
        when (getType(column)) {
            Cursor.FIELD_TYPE_INTEGER ->
                when (getLong(column)) {
                    0L -> false
                    1L -> true
                    else -> fail(AnkiReadError.InvalidResponse)
                }
            Cursor.FIELD_TYPE_STRING ->
                when (getString(column)) {
                    "false" -> false
                    "true" -> true
                    else -> fail(AnkiReadError.InvalidResponse)
                }
            else -> fail(AnkiReadError.InvalidResponse)
        }

    private fun uri(path: String): Uri =
        Uri.Builder().scheme("content").authority(AUTHORITY).appendPath(path).build()

    private class ReadFailure(val reason: AnkiReadError) : RuntimeException()

    private fun fail(reason: AnkiReadError): Nothing = throw ReadFailure(reason)

    companion object {
        const val AUTHORITY = "com.ichi2.anki.flashcards"
        const val PERMISSION = "com.ichi2.anki.permission.READ_WRITE_DATABASE"
        const val SPEC_KEY = "com.ichi2.anki.provider.spec"

        // Identity representation only; note content/templates remain owned by Rust. Keep parity
        // with yomoku-core::anki::escape, including inert media syntax and field delimiters.
        internal fun escapeSurface(surface: String): String = buildString {
            for (character in surface) {
                append(
                    when (character) {
                        '&' -> "&amp;"
                        '<' -> "&lt;"
                        '>' -> "&gt;"
                        '"' -> "&quot;"
                        '\'' -> "&#39;"
                        '[' -> "&#91;"
                        ']' -> "&#93;"
                        '\u001f' -> "&#31;"
                        else -> character.toString()
                    }
                )
            }
        }
    }
}
