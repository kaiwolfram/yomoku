package app.yomoku

import android.content.ContentResolver
import android.content.ContentValues
import android.net.Uri
import android.os.Looper

internal sealed interface AnkiModelCreationError {
    data class ReadFailed(val error: AnkiReadError) : AnkiModelCreationError

    data object NameConflict : AnkiModelCreationError

    data object InvalidResponse : AnkiModelCreationError

    data object LayoutMismatch : AnkiModelCreationError

    data object ProviderFailure : AnkiModelCreationError
}

internal sealed interface AnkiModelCreationResult {
    data class Created(val id: AnkiProviderModelId) : AnkiModelCreationResult

    /** No mutating provider call was submitted. */
    data class Failed(val error: AnkiModelCreationError) : AnkiModelCreationResult

    /** A model may remain, including a partially configured one. Never automatically replay. */
    data class Uncertain(val id: AnkiProviderModelId?, val error: AnkiModelCreationError) :
        AnkiModelCreationResult
}

/**
 * Worker-only creation boundary used by the retained typed dispatcher. Only the fresh ID returned
 * by this invocation can receive a template update. There is no repair/delete API or automatic
 * retry/cleanup. Rust supplies the expected layout; creation is not a lease for note insertion. The
 * dispatcher requires a durably committed native permit before any live call.
 */
internal class AnkiProviderModelCreator(
    private val resolver: ContentResolver,
    environment: AnkiProviderEnvironment,
) {
    private val reader = AnkiProviderReader(resolver, environment)

    fun create(
        expected: AnkiProviderModelLayout,
        beforeSubmit: () -> Boolean = { true },
    ): AnkiModelCreationResult {
        check(Looper.myLooper() != Looper.getMainLooper()) { "Anki provider access on main thread" }
        var submitted = false
        var createdId: AnkiProviderModelId? = null
        return try {
            val host = value(reader.availability())
            val existing = value(reader.models())
            // Discovery never repairs or silently renames
            // an existing user's model; discovery/validation can establish reuse separately.
            if (existing.any { it.name == expected.name }) fail(AnkiModelCreationError.NameConflict)
            val values =
                ContentValues().apply {
                    put("name", expected.name)
                    put("field_names", expected.joinedFieldNames)
                    put("num_cards", 1)
                    put("css", expected.css)
                    put("sort_field_index", 0)
                    put("type", 0)
                }
            if (!beforeSubmit()) fail(AnkiModelCreationError.ProviderFailure)
            sameHost(host)
            // From here, even an exception/null response may follow a committed provider write.
            submitted = true
            val inserted = resolver.insert(modelsUri(), values)
            val id = insertedId(inserted)
            if (existing.any { it.id == id }) fail(AnkiModelCreationError.InvalidResponse)
            createdId = id
            // Use a canonical URI, never append paths to an untrusted provider response.
            val modelUri = modelsUri().buildUpon().appendPath(id.value.toString()).build()
            val templateUri = modelUri.buildUpon().appendPath("templates").appendPath("0").build()
            val template =
                ContentValues().apply {
                    put("card_template_name", expected.cardName)
                    put("question_format", expected.questionFormat)
                    put("answer_format", expected.answerFormat)
                    put("browser_question_format", "")
                    put("browser_answer_format", "")
                }
            sameHost(host)
            // AnkiDroid counts changed properties here, not rows; do not require exactly one.
            if (resolver.update(templateUri, template, null, null) <= 0) {
                fail(AnkiModelCreationError.InvalidResponse)
            }
            sameHost(host)
            if (!value(reader.modelMatches(id, expected)))
                fail(AnkiModelCreationError.LayoutMismatch)
            sameHost(host)
            AnkiModelCreationResult.Created(id)
        } catch (error: CreationFailure) {
            failure(submitted, createdId, error.reason)
        } catch (_: SecurityException) {
            failure(
                submitted,
                createdId,
                AnkiModelCreationError.ReadFailed(AnkiReadError.PermissionDenied),
            )
        } catch (_: IllegalArgumentException) {
            failure(submitted, createdId, AnkiModelCreationError.InvalidResponse)
        } catch (_: Exception) {
            failure(submitted, createdId, AnkiModelCreationError.ProviderFailure)
        }
    }

    private fun sameHost(expected: String) {
        if (value(reader.availability()) != expected) {
            fail(AnkiModelCreationError.ReadFailed(AnkiReadError.ProviderUnavailable))
        }
    }

    private fun insertedId(uri: Uri?): AnkiProviderModelId {
        if (
            uri == null ||
                uri.scheme != "content" ||
                uri.authority != AnkiProviderReader.AUTHORITY ||
                uri.query != null ||
                uri.fragment != null ||
                uri.pathSegments.size != 2 ||
                uri.pathSegments[0] != "models"
        )
            fail(AnkiModelCreationError.InvalidResponse)
        val text = uri.pathSegments[1]
        val id = text.toLongOrNull()
        if (id == null || id <= 0 || text != id.toString()) {
            fail(AnkiModelCreationError.InvalidResponse)
        }
        return AnkiProviderModelId(id)
    }

    private fun <T> value(result: AnkiReadResult<T>): T =
        when (result) {
            is AnkiReadResult.Value -> result.value
            is AnkiReadResult.Failed -> fail(AnkiModelCreationError.ReadFailed(result.error))
        }

    private fun failure(
        submitted: Boolean,
        id: AnkiProviderModelId?,
        error: AnkiModelCreationError,
    ): AnkiModelCreationResult =
        if (submitted) AnkiModelCreationResult.Uncertain(id, error)
        else AnkiModelCreationResult.Failed(error)

    private fun modelsUri(): Uri =
        Uri.Builder()
            .scheme("content")
            .authority(AnkiProviderReader.AUTHORITY)
            .appendPath("models")
            .build()

    private class CreationFailure(val reason: AnkiModelCreationError) : RuntimeException()

    private fun fail(reason: AnkiModelCreationError): Nothing = throw CreationFailure(reason)
}
