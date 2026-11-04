package app.yomoku

import android.content.ContentResolver
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.Looper

/** Frozen provider transport; core owns content/layout and the in-memory operation. */
internal class AnkiProviderNoteRequest(
    val surface: String,
    fields: List<String>,
    val modelId: AnkiProviderModelId,
    val deckId: Long,
    val layout: AnkiProviderModelLayout,
) {
    val joinedFields: String

    init {
        require(surface.isNotEmpty() && '\u0000' !in surface)
        require(deckId > 0)
        require(fields.size == 4 && fields.none { '\u001f' in it || '\u0000' in it })
        require(fields[0] == AnkiProviderReader.escapeSurface(surface))
        joinedFields = fields.joinToString("\u001f")
    }
}

internal sealed interface AnkiNoteInsertionError {
    data class ReadFailed(val error: AnkiReadError) : AnkiNoteInsertionError

    data object LayoutMismatch : AnkiNoteInsertionError

    data object DeckUnavailable : AnkiNoteInsertionError

    data object InvalidResponse : AnkiNoteInsertionError

    data object ProviderFailure : AnkiNoteInsertionError
}

internal sealed interface AnkiNoteInsertionResult {
    data class Inserted(val id: AnkiProviderNoteId) : AnkiNoteInsertionResult

    data class Existing(val ids: List<AnkiProviderNoteId>) : AnkiNoteInsertionResult

    /** No mutation submitted; the owner may offer an explicit pre-insert retry. */
    data class Failed(val error: AnkiNoteInsertionError) : AnkiNoteInsertionResult

    /** Preserve the payload/deck even when insertion or later placement lost its response. */
    data class Uncertain(
        val request: AnkiProviderNoteRequest,
        val id: AnkiProviderNoteId?,
        val error: AnkiNoteInsertionError,
    ) : AnkiNoteInsertionResult
}

/**
 * Worker-only insertion boundary. The retained dispatcher consumes an in-memory invocation before
 * invoking this boundary, invokes once per request and never replays on restart. There is no
 * retry/repair/delete here. A later explicit attempt queries the provider afresh; it cannot
 * establish that a previous interrupted write never completed.
 */
internal class AnkiProviderNoteInserter(
    private val resolver: ContentResolver,
    environment: AnkiProviderEnvironment,
) {
    private val reader = AnkiProviderReader(resolver, environment)

    fun insert(request: AnkiProviderNoteRequest): AnkiNoteInsertionResult {
        check(Looper.myLooper() != Looper.getMainLooper()) { "Anki provider access on main thread" }
        var submitted = false
        var insertedId: AnkiProviderNoteId? = null
        return try {
            val host = value(reader.availability())
            if (!value(reader.modelMatches(request.modelId, request.layout))) {
                fail(AnkiNoteInsertionError.LayoutMismatch)
            }
            sameHost(host)
            val deck = value(reader.decks()).singleOrNull { it.id == request.deckId }
            if (deck == null || deck.filtered) fail(AnkiNoteInsertionError.DeckUnavailable)
            sameHost(host)
            // Last collection read before submission. No supported atomic check-and-add exists.
            val existing = value(reader.duplicates(request.surface))
            sameHost(host)
            if (existing.isNotEmpty()) return AnkiNoteInsertionResult.Existing(existing)
            val values =
                ContentValues().apply {
                    put("mid", request.modelId.value)
                    put("flds", request.joinedFields)
                }
            submitted = true
            val id = noteId(resolver.insert(notesUri(), values))
            insertedId = id
            val noteUri = notesUri().buildUpon().appendPath(id.value.toString()).build()
            // Validate returned note content before touching any card, using canonical URIs only.
            sameHost(host)
            verifyNote(noteUri, id, request)
            sameHost(host)
            val cardsUri = noteUri.buildUpon().appendPath("cards").build()
            verifyCard(cardsUri, id, null)
            sameHost(host)
            // insert(notes) uses the model's default deck. Only the returned note's ordinal-zero
            // card is moved; never change the model default, selected deck or discovered
            // duplicates.
            val placement = ContentValues().apply { put("deck_id", request.deckId) }
            val cardUri = cardsUri.buildUpon().appendPath("0").build()
            if (resolver.update(cardUri, placement, null, null) != 1) {
                fail(AnkiNoteInsertionError.InvalidResponse)
            }
            sameHost(host)
            verifyNote(noteUri, id, request)
            sameHost(host)
            verifyCard(cardsUri, id, request.deckId)
            sameHost(host)
            AnkiNoteInsertionResult.Inserted(id)
        } catch (error: InsertionFailure) {
            failure(submitted, request, insertedId, error.reason)
        } catch (_: SecurityException) {
            failure(
                submitted,
                request,
                insertedId,
                AnkiNoteInsertionError.ReadFailed(AnkiReadError.PermissionDenied),
            )
        } catch (_: IllegalArgumentException) {
            failure(submitted, request, insertedId, AnkiNoteInsertionError.InvalidResponse)
        } catch (_: Exception) {
            failure(submitted, request, insertedId, AnkiNoteInsertionError.ProviderFailure)
        }
    }

    private fun verifyNote(uri: Uri, id: AnkiProviderNoteId, request: AnkiProviderNoteRequest) {
        val cursor =
            resolver.query(uri, arrayOf("_id", "mid", "flds"), null, null, null)
                ?: fail(AnkiNoteInsertionError.InvalidResponse)
        try {
            val note = cursor.getColumnIndexOrThrow("_id")
            val model = cursor.getColumnIndexOrThrow("mid")
            val fields = cursor.getColumnIndexOrThrow("flds")
            if (
                !cursor.moveToNext() ||
                    cursor.integer(note) != id.value ||
                    cursor.integer(model) != request.modelId.value ||
                    cursor.getType(fields) != Cursor.FIELD_TYPE_STRING ||
                    cursor.getString(fields) != request.joinedFields ||
                    cursor.moveToNext()
            ) {
                fail(AnkiNoteInsertionError.InvalidResponse)
            }
        } finally {
            cursor.close()
        }
    }

    private fun verifyCard(uri: Uri, id: AnkiProviderNoteId, deckId: Long?) {
        val cursor =
            resolver.query(uri, arrayOf("note_id", "ord", "deck_id"), null, null, null)
                ?: fail(AnkiNoteInsertionError.InvalidResponse)
        try {
            val note = cursor.getColumnIndexOrThrow("note_id")
            val ordinal = cursor.getColumnIndexOrThrow("ord")
            val deck = cursor.getColumnIndexOrThrow("deck_id")
            if (
                !cursor.moveToNext() ||
                    cursor.integer(note) != id.value ||
                    cursor.integer(ordinal) != 0L
            )
                fail(AnkiNoteInsertionError.InvalidResponse)
            val actualDeck = cursor.integer(deck)
            if (
                actualDeck <= 0 || (deckId != null && actualDeck != deckId) || cursor.moveToNext()
            ) {
                fail(AnkiNoteInsertionError.InvalidResponse)
            }
        } finally {
            cursor.close()
        }
    }

    private fun Cursor.integer(column: Int): Long {
        if (getType(column) != Cursor.FIELD_TYPE_INTEGER)
            fail(AnkiNoteInsertionError.InvalidResponse)
        return getLong(column)
    }

    private fun noteId(uri: Uri?): AnkiProviderNoteId {
        if (
            uri == null ||
                uri.scheme != "content" ||
                uri.authority != AnkiProviderReader.AUTHORITY ||
                uri.query != null ||
                uri.fragment != null ||
                uri.pathSegments.size != 2 ||
                uri.pathSegments[0] != "notes"
        ) {
            fail(AnkiNoteInsertionError.InvalidResponse)
        }
        val text = uri.pathSegments[1]
        val id = text.toLongOrNull()
        if (id == null || id <= 0 || text != id.toString())
            fail(AnkiNoteInsertionError.InvalidResponse)
        return AnkiProviderNoteId(id)
    }

    private fun sameHost(expected: String) {
        if (value(reader.availability()) != expected) {
            fail(AnkiNoteInsertionError.ReadFailed(AnkiReadError.ProviderUnavailable))
        }
    }

    private fun <T> value(result: AnkiReadResult<T>): T =
        when (result) {
            is AnkiReadResult.Value -> result.value
            is AnkiReadResult.Failed -> fail(AnkiNoteInsertionError.ReadFailed(result.error))
        }

    private fun failure(
        submitted: Boolean,
        request: AnkiProviderNoteRequest,
        id: AnkiProviderNoteId?,
        error: AnkiNoteInsertionError,
    ): AnkiNoteInsertionResult =
        if (submitted) AnkiNoteInsertionResult.Uncertain(request, id, error)
        else AnkiNoteInsertionResult.Failed(error)

    private fun notesUri(): Uri =
        Uri.Builder()
            .scheme("content")
            .authority(AnkiProviderReader.AUTHORITY)
            .appendPath("notes")
            .build()

    private class InsertionFailure(val reason: AnkiNoteInsertionError) : RuntimeException()

    private fun fail(reason: AnkiNoteInsertionError): Nothing = throw InsertionFailure(reason)
}
