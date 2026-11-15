package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.DocumentError
import app.yomoku.shared.ImportOutcome
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MixedFilePickerTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun owner() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java]

    @Test
    fun mixedPickerPreservesFormatsEncodingAndFailuresWithoutOpeningReader() {
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            instrumentation.uiAutomation.grantRuntimePermission(
                instrumentation.targetContext.packageName,
                android.Manifest.permission.POST_NOTIFICATIONS,
            )
        }
        compose.waitUntil(30_000) {
            val view = owner().session.view
            !view.storageBusy &&
                (view.libraryWordDistributions != null || view.libraryWordError != null)
        }
        val prefix = "mixed-${System.nanoTime()}"
        val book = FileImportFixtures.epub("$prefix metadata title")
        val ambiguous = "$prefix choice ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte())
        val uris =
            listOf(
                FileImportFixtures.uri("$prefix.EpUb", book),
                FileImportFixtures.uri("$prefix-duplicate.txt", book),
                FileImportFixtures.uri(
                    "$prefix.SRT",
                    "\uFEFF1\n00:00:01,000 --> 00:00:02,000\n$prefix <i>猫</i>。\n\n2\n00:00:03,000 --> 00:00:04,000\n犬。"
                        .toByteArray(),
                ),
                FileImportFixtures.uri("$prefix-invalid.epub", byteArrayOf(0xff.toByte())),
                FileImportFixtures.uri("$prefix-invalid.srt", byteArrayOf(0xff.toByte())),
                FileImportFixtures.uri("$prefix-unsupported.ass", "猫".toByteArray()),
                FileImportFixtures.uri("$prefix-choice.srt", ambiguous),
                FileImportFixtures.uri("$prefix-cancel.srt", ambiguous + " cancel".toByteArray()),
                FileImportFixtures.uri("$prefix-shift.srt", ambiguous + " shift".toByteArray()),
                FileImportFixtures.uri("$prefix-last.txt", "\uFEFF$prefix last 猫。".toByteArray()),
            )
        val imports = owner().fileImports
        for ((index, uri) in uris.withIndex()) {
            val monitor =
                instrumentation.addMonitor(
                    IntentFilter(Intent.ACTION_OPEN_DOCUMENT).apply {
                        addCategory(Intent.CATEGORY_OPENABLE)
                        addDataType("*/*")
                    },
                    ActivityResult(Activity.RESULT_OK, Intent().setData(uri)),
                    true,
                )
            try {
                // Each import invalidates Library distributions; wait for that
                // asynchronous refresh before tapping the enabled menu again.
                compose.waitUntil(30_000) {
                    val view = owner().session.view
                    !imports.active &&
                        !view.storageBusy &&
                        owner().importBookRequest.step == null &&
                        (view.libraryWordDistributions != null || view.libraryWordError != null)
                }
                compose.onNodeWithTag("library_add").assertIsEnabled().performClick()
                compose.onNodeWithText("Import book").performClick()
                compose.waitUntil(90_000) { imports.result != null || imports.choosingEncoding }
                assertEquals(1, monitor.hits)
                if (index in 6..8) {
                    assertEquals(DocumentError.SRTENCODINGREQUIRED, imports.encodingError)
                    compose.onNodeWithText("Skip file").assertDoesNotExist()
                    if (index == 6) {
                        compose.activityRule.scenario.recreate()
                        assertSame(imports, owner().fileImports)
                    }
                    compose
                        .onNodeWithText(
                            when (index) {
                                7 -> "Cancel"
                                8 -> "Shift-JIS"
                                else -> "UTF-8"
                            }
                        )
                        .performClick()
                }
                compose.waitUntil(90_000) { !imports.active && !owner().session.view.storageBusy }
                when (index) {
                    1 -> {
                        assertEquals(ImportOutcome.EXISTING, imports.result?.outcome)
                        assertEquals("$prefix metadata title", imports.result?.title)
                        compose
                            .onNodeWithText("Already in Library: $prefix metadata title")
                            .assertIsDisplayed()
                    }
                    3 -> {
                        assertEquals(DocumentError.INVALIDEPUB, imports.result?.error)
                        compose
                            .onNodeWithText(
                                "This EPUB is invalid, unsupported, or exceeds import limits."
                            )
                            .assertIsDisplayed()
                    }
                    4 -> assertEquals(DocumentError.SRTUNDECODABLE, imports.result?.error)
                    5 -> assertEquals(R.string.import_supported_files, imports.result?.message)
                    7 -> assertNull(imports.result)
                    else ->
                        assertEquals(
                            "File $index: ${imports.result}",
                            ImportOutcome.CREATED,
                            imports.result?.outcome,
                        )
                }
                assertEquals(ContentRoute.MAIN, owner().session.view.contentRoute)
                assertNull(owner().session.view.reader)
                if (imports.result != null && imports.result?.outcome != ImportOutcome.CREATED) {
                    compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
                    compose.onNodeWithText("Close").performClick()
                } else {
                    compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
                }
                compose.onNodeWithTag("import_summary").assertDoesNotExist()
            } finally {
                instrumentation.removeMonitor(monitor)
            }
        }
        val core = owner().session
        assertEquals(ContentRoute.MAIN, core.view.contentRoute)
        assertNull(core.view.reader)
        assertNull(core.view.documentError)
        assertNull(core.view.importSummary)
        val documents = core.view.documents.filter { it.title.startsWith(prefix) }
        assertEquals(5, documents.size)
        SQLiteDatabase.openDatabase(
                File(compose.activity.filesDir, "yomoku.sqlite3").path,
                null,
                SQLiteDatabase.OPEN_READONLY,
            )
            .use { db ->
                val expected =
                    listOf(
                        Triple("$prefix metadata title", "epub", 2),
                        Triple("$prefix.SRT", "utf8", 1),
                        Triple("$prefix-choice.srt", "utf8", 1),
                        Triple("$prefix-shift.srt", "shift_jis", 1),
                        Triple("$prefix-last.txt", "utf8", 1),
                    )
                for ((title, encoding, chapters) in expected) {
                    val id = documents.single { it.title == title }.documentId.value.toString()
                    db.rawQuery(
                            "SELECT f.text_encoding, s.kind, COUNT(c.id), COUNT(a.id) FROM imported_files f JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id JOIN chapters c ON c.source_id = s.id LEFT JOIN chapter_analyses a ON a.chapter_id = c.id AND a.split_mode = 'c' WHERE d.id = ? GROUP BY d.id",
                            arrayOf(id),
                        )
                        .use {
                            assertTrue(it.moveToFirst())
                            assertEquals(encoding, it.getString(0))
                            assertEquals(
                                if (title.endsWith(".srt", true)) "srt"
                                else if (title.endsWith(".txt")) "txt" else "epub",
                                it.getString(1),
                            )
                            assertEquals(chapters, it.getInt(2))
                            assertEquals(chapters, it.getInt(3))
                        }
                    db.rawQuery(
                            "SELECT COUNT(*) FROM analyzed_tokens t JOIN chapter_analyses a ON a.id = t.analysis_id JOIN chapters c ON c.id = a.chapter_id JOIN documents d ON d.source_id = c.source_id LEFT JOIN vocabulary v ON v.surface = t.surface WHERE d.id = ? AND t.is_japanese = 1 AND v.id IS NULL",
                            arrayOf(id),
                        )
                        .use {
                            assertTrue(it.moveToFirst())
                            assertEquals(
                                "Every imported Japanese surface is registered",
                                0,
                                it.getInt(0),
                            )
                        }
                    db.rawQuery(
                            "SELECT COUNT(*) FROM reading_completions WHERE document_id = ?",
                            arrayOf(id),
                        )
                        .use {
                            assertTrue(it.moveToFirst())
                            assertEquals(
                                "Import creates no reading encounters",
                                0,
                                it.getInt(0),
                            )
                        }
                }
                val srt = documents.single { it.title == "$prefix.SRT" }
                db.rawQuery(
                        "SELECT t.text FROM chapter_texts t JOIN chapters c ON c.id = t.chapter_id JOIN documents d ON d.source_id = c.source_id WHERE d.id = ?",
                        arrayOf(srt.documentId.value.toString()),
                    )
                    .use {
                        assertTrue(it.moveToFirst())
                        assertEquals("$prefix 猫。\n\n犬。", it.getString(0))
                    }
            }
        assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
        compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
        compose.onNodeWithTag("reader_text").assertDoesNotExist()
    }
}
