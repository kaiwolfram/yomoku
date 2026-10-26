package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.ClipData
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToNode
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
        compose.waitUntil(30_000) { !owner().session.view.storageBusy }
        val prefix = "mixed-${System.nanoTime()}"
        val book = FileImportFixtures.epub("$prefix metadata title")
        val ambiguous = "$prefix choice ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte())
        val uris =
            listOf(
                FileImportFixtures.uri("$prefix.EpUb", book),
                FileImportFixtures.uri("$prefix-duplicate.txt", book),
                FileImportFixtures.uri(
                    "$prefix.SRT",
                    "\uFEFF1\n00:00:01,000 --> 00:00:02,000\n<i>猫</i>。\n\n2\n00:00:03,000 --> 00:00:04,000\n犬。"
                        .toByteArray(),
                ),
                FileImportFixtures.uri("$prefix-invalid.epub", byteArrayOf(0xff.toByte())),
                FileImportFixtures.uri("$prefix-invalid.srt", byteArrayOf(0xff.toByte())),
                FileImportFixtures.uri("$prefix-unsupported.ass", "猫".toByteArray()),
                FileImportFixtures.uri("$prefix-choice.srt", ambiguous),
                FileImportFixtures.uri("$prefix-skip.srt", ambiguous + " skip".toByteArray()),
                FileImportFixtures.uri("$prefix-shift.srt", ambiguous + " shift".toByteArray()),
                FileImportFixtures.uri("$prefix-last.txt", "\uFEFF$prefix last 猫。".toByteArray()),
            )
        val clip = ClipData.newRawUri("Mixed file fixtures", uris.first())
        uris.drop(1).forEach { clip.addItem(ClipData.Item(it)) }
        val monitor =
            instrumentation.addMonitor(
                IntentFilter(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    addDataType("*/*")
                },
                ActivityResult(Activity.RESULT_OK, Intent().apply { clipData = clip }),
                true,
            )
        try {
            compose.onNodeWithText("Import files").performClick()
            compose.waitUntil(90_000) { owner().fileImports.choosingEncoding }
            assertEquals(1, monitor.hits)
            val imports = owner().fileImports
            assertEquals("$prefix-choice.srt", imports.filename)
            assertEquals(DocumentError.SRTENCODINGREQUIRED, imports.encodingError)
            assertEquals(
                listOf(ImportOutcome.CREATED, ImportOutcome.EXISTING, ImportOutcome.CREATED),
                imports.results.take(3).map { it.outcome },
            )
            assertEquals("$prefix metadata title", imports.results[1].title)
            assertEquals(DocumentError.INVALIDEPUB, imports.results[3].error)
            assertEquals(DocumentError.SRTUNDECODABLE, imports.results[4].error)
            assertEquals(R.string.import_supported_files, imports.results[5].message)
            compose.onNodeWithText("File 7 of 10").assertIsDisplayed()
            compose.onNodeWithText("UTF-8").assertIsDisplayed()
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            compose.onNodeWithText("UTF-8").performClick()
            compose.waitUntil(30_000) {
                imports.choosingEncoding && imports.filename == "$prefix-skip.srt"
            }
            compose.onNodeWithText("Skip file").performClick()
            compose.waitUntil(30_000) {
                imports.choosingEncoding && imports.filename == "$prefix-shift.srt"
            }
            compose.onNodeWithText("Shift-JIS").performClick()
            compose.waitUntil(90_000) { !imports.active && !owner().session.view.storageBusy }
            assertEquals(10, imports.results.size)
            assertEquals(ImportOutcome.CREATED, imports.results[6].outcome)
            assertEquals(R.string.import_file_skipped, imports.results[7].message)
            assertEquals(ImportOutcome.CREATED, imports.results[8].outcome)
            assertEquals(ImportOutcome.CREATED, imports.results[9].outcome)
            assertEquals(1f, imports.progress)
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
                                "SELECT f.text_encoding, s.kind, COUNT(c.id), COUNT(a.id) FROM imported_files f JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id JOIN chapters c ON c.source_id = s.id LEFT JOIN chapter_analyses a ON a.chapter_id = c.id AND a.split_mode = 'b' WHERE d.id = ? GROUP BY d.id",
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
                            assertEquals("猫。\n\n犬。", it.getString(0))
                        }
                }
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            compose
                .onNodeWithTag("file_import_summary")
                .performScrollToNode(hasText("$prefix-invalid.epub"))
            compose
                .onNodeWithText("This EPUB is invalid, unsupported, or exceeds import limits.")
                .assertIsDisplayed()
            compose.onNodeWithTag("reader_text").assertDoesNotExist()
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            compose.onNodeWithText("Close").performClick()
            assertTrue(imports.results.isEmpty())
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }
}
