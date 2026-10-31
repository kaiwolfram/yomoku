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

        override fun insert(uri: Uri, values: ContentValues?): Uri = error("Unexpected insert")

        override fun update(
            uri: Uri,
            values: ContentValues?,
            selection: String?,
            args: Array<out String>?,
        ): Int = error("Unexpected update")

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

        override fun close() = provider.close()
    }

    private fun <T> value(result: AnkiReadResult<T>): T = (result as AnkiReadResult.Value).value

    private fun failed(reason: AnkiReadError, result: AnkiReadResult<*>) {
        assertEquals(AnkiReadResult.Failed(reason), result)
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
                        { f.reader.decks() },
                        { f.reader.duplicates("猫") },
                        { f.reader.reconcile("猫") },
                        { f.reader.existingNoteIntent(AnkiProviderNoteId(7)) },
                    )
                operations.forEach { operation ->
                    assertThrows(IllegalStateException::class.java) { operation() }
                }
            }
            assertEquals(0, f.provider.queries)
        }
    }
}
