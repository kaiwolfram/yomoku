package app.yomoku

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ProviderInfo
import android.database.Cursor
import android.net.Uri
import android.os.Looper

/** Shell-only prerequisites; these values are not yet an Event/FFI protocol. */
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

/**
 * Read-only ContentResolver boundary. Call on a worker. No provider state is cached and no
 * permission dialog, Activity launch, model creation or note mutation occurs here. Keep unconnected
 * until the app owns durable pending/uncertain export recovery.
 */
internal class AnkiProviderReader(
    private val resolver: ContentResolver,
    private val environment: AnkiProviderEnvironment,
) {
    constructor(
        context: Context
    ) : this(
        context.applicationContext.contentResolver,
        object : AnkiProviderEnvironment {
            private val app = context.applicationContext

            @Suppress("DEPRECATION")
            override fun provider(): ProviderInfo? =
                app.packageManager.resolveContentProvider(AUTHORITY, PackageManager.GET_META_DATA)

            override fun hasPermission(): Boolean =
                app.checkSelfPermission(PERMISSION) == PackageManager.PERMISSION_GRANTED

            override fun resolves(intent: Intent): Boolean =
                intent.resolveActivity(app.packageManager) != null
        },
    )

    fun availability(): AnkiReadResult<String> = read(AnkiReadError.ProviderFailure) { host() }

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
        read(AnkiReadError.ProviderFailure) {
            val packageName = host()
            val link =
                Uri.Builder()
                    .scheme("anki")
                    .authority("x-callback-url")
                    .appendPath("browser")
                    .appendQueryParameter("search", "nid:${id.value}")
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
        private fun escapeSurface(surface: String): String = buildString {
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
