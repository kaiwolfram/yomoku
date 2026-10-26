package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.ClipData
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import android.net.Uri
import android.util.Base64
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertRangeInfoEquals
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToNode
import androidx.lifecycle.Lifecycle
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
class TxtPickerTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()

    private fun owner() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java]

    private fun awaitIdle() {
        compose.waitUntil(30_000) { !owner().session.view.storageBusy }
        compose.waitForIdle()
    }

    private fun fixture(name: String, bytes: ByteArray, gate: String? = null): Uri =
        Uri.Builder()
            .scheme("content")
            .authority("app.yomoku.test.txt-fixtures")
            .appendPath(name)
            .appendQueryParameter("bytes", Base64.encodeToString(bytes, Base64.NO_WRAP))
            .apply { if (gate != null) appendQueryParameter("gate", gate) }
            .build()

    @Test
    fun cancelBlockedReadRetainsCompletedFilesAndFreshBatchIgnoresLateProviderResult() {
        awaitIdle()
        val prefix = "cancel-read-${System.nanoTime()}"
        val first = fixture("$prefix-first.txt", "\uFEFF$prefix first 猫".toByteArray())
        val blocked =
            fixture("$prefix-blocked.txt", "\uFEFF$prefix partial 犬".toByteArray(), prefix)
        val remaining = fixture("$prefix-remaining.txt", "$prefix remaining".toByteArray())
        val fresh = fixture("$prefix-fresh.txt", byteArrayOf(0xc2.toByte(), 0xa9.toByte()))
        val resolver = instrumentation.targetContext.contentResolver
        try {
            compose.runOnIdle { owner().fileImports.start(listOf(first, blocked, remaining)) }
            compose.waitUntil(90_000) {
                owner().fileImports.filename == "$prefix-blocked.txt" &&
                    owner().fileImports.results.size == 1
            }
            val retained = owner().fileImports
            compose.onNodeWithText("Cancel").performClick()
            compose.waitUntil(30_000) { !retained.active }
            awaitIdle()
            assertEquals(listOf(ImportOutcome.CREATED), retained.results.map { it.outcome })
            assertEquals(3, retained.totalFiles)
            assertEquals(1f / 3f, retained.progress)
            compose.activityRule.scenario.recreate()
            assertSame(retained, owner().fileImports)
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            compose.onNodeWithText("Close").performClick()
            // This must reach encoding selection while the old provider read is still blocked.
            compose.runOnIdle { retained.start(listOf(fresh, remaining)) }
            compose.waitUntil(30_000) { retained.choosingEncoding }
            resolver.call(blocked, "release", prefix, null)
            compose.waitUntil(30_000) {
                resolver.call(blocked, "finished", prefix, null)?.getBoolean("finished") == true
            }
            compose.waitForIdle()
            assertEquals("$prefix-fresh.txt", retained.filename)
            assertTrue(retained.results.isEmpty())
            compose.activityRule.scenario.recreate()
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            compose.onNodeWithText("Cancel").performClick()
            compose.waitUntil(30_000) { !retained.active }
            awaitIdle()
            assertTrue(retained.results.isEmpty())
            compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
            assertTrue(
                owner().session.view.documents.none {
                    it.title in
                        listOf("$prefix-blocked.txt", "$prefix-remaining.txt", "$prefix-fresh.txt")
                }
            )
            // A later explicit batch is independent, including exact-source duplicate detection.
            compose.runOnIdle { retained.start(listOf(first)) }
            compose.waitUntil(30_000) { !retained.active && retained.results.isNotEmpty() }
            awaitIdle()
            assertEquals(ImportOutcome.EXISTING, retained.results.single().outcome)
            assertEquals(1, retained.totalFiles)
            assertNull(owner().session.view.documentError)
            assertNull(owner().session.view.reader)
            compose.onNodeWithText("Close").performClick()
        } finally {
            resolver.call(blocked, "release", prefix, null)
        }
    }

    @Test
    fun pickerBatchRetainsEncodingAndSummarizesEveryFileWithoutOpeningReader() {
        awaitIdle()
        val prefix = "TXT-${System.nanoTime()}"
        fun fixture(suffix: String, bytes: ByteArray?): Uri {
            return Uri.Builder()
                .scheme("content")
                .authority("app.yomoku.test.txt-fixtures")
                .appendPath("$prefix-$suffix")
                .apply {
                    if (bytes != null)
                        appendQueryParameter("bytes", Base64.encodeToString(bytes, Base64.NO_WRAP))
                }
                .build()
        }
        val original = "\uFEFF$prefix\n｜猫《ねこ》と犬。\r\n".toByteArray()
        val uris =
            listOf(
                fixture("first.txt", original),
                fixture("duplicate.txt", original),
                fixture("unreadable.txt", null),
                fixture("unsupported.vtt", "WEBVTT\n猫".toByteArray()),
                fixture("invalid.txt", byteArrayOf(0xff.toByte())),
                fixture(
                    "choice.txt",
                    "$prefix choice ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte()),
                ),
                fixture(
                    "skip.txt",
                    "$prefix skip ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte()),
                ),
                fixture(
                    "shift.txt",
                    "$prefix shift ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte()),
                ),
                fixture("last.TXT", "$prefix last file".toByteArray()),
            )
        val clip = ClipData.newRawUri("TXT fixtures", uris.first())
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
            // Fail clearly if the test APK's provider is unavailable before testing the picker.
            assertArrayEquals(
                original,
                instrumentation.targetContext.contentResolver.openInputStream(uris.first())?.use {
                    it.readBytes()
                },
            )
            compose.onNodeWithText("Import files").performClick()
            try {
                compose.waitUntil(90_000) { owner().fileImports.choosingEncoding }
            } catch (error: Throwable) {
                val imports = owner().fileImports
                throw AssertionError(
                    "Picker hits=${monitor.hits}, active=${imports.active}, file=${imports.filename}, results=${imports.results}, busy=${owner().session.view.storageBusy}, error=${owner().session.view.documentError}",
                    error,
                )
            }
            assertEquals(1, monitor.hits)
            val retained = owner().fileImports
            assertEquals("$prefix-choice.txt", retained.filename)
            assertEquals(
                listOf(ImportOutcome.CREATED, ImportOutcome.EXISTING),
                retained.results.take(2).map { it.outcome },
            )
            assertEquals(R.string.import_file_unreadable, retained.results[2].message)
            assertEquals(R.string.import_supported_files, retained.results[3].message)
            assertEquals(DocumentError.TXTUNDECODABLE, retained.results[4].error)
            assertEquals(9, retained.totalFiles)
            assertEquals(6, retained.currentFileNumber)
            compose.onNodeWithText("File 6 of 9").assertIsDisplayed()
            compose.onNodeWithText("Processed: 5 of 9").assertIsDisplayed()
            compose
                .onNodeWithTag("file_import_progress")
                .assertRangeInfoEquals(ProgressBarRangeInfo(5f / 9f, 0f..1f))
            compose.onNodeWithText("UTF-8").assertIsDisplayed()
            compose.activityRule.scenario.recreate()
            assertSame(retained, owner().fileImports)
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            compose.onNodeWithText("File 6 of 9").assertIsDisplayed()
            compose.onNodeWithText("Processed: 5 of 9").assertIsDisplayed()
            compose.onNodeWithText("UTF-8").performClick()
            compose.waitUntil(30_000) {
                retained.choosingEncoding && retained.filename == "$prefix-skip.txt"
            }
            compose.onNodeWithText("File 7 of 9").assertIsDisplayed()
            compose.onNodeWithText("Processed: 6 of 9").assertIsDisplayed()
            compose.onNodeWithText("Skip file").performClick()
            compose.waitUntil(30_000) {
                retained.choosingEncoding && retained.filename == "$prefix-shift.txt"
            }
            compose.onNodeWithText("File 8 of 9").assertIsDisplayed()
            compose.onNodeWithText("Processed: 7 of 9").assertIsDisplayed()
            compose.onNodeWithText("Shift-JIS").performClick()
            compose.waitUntil(30_000) { !retained.active && retained.results.size == 9 }
            awaitIdle()
            assertEquals(0, retained.currentFileNumber)
            assertEquals(1f, retained.progress)
            assertEquals("", retained.filename)
            assertEquals(ImportOutcome.CREATED, retained.results[5].outcome)
            assertEquals(R.string.import_file_skipped, retained.results[6].message)
            assertEquals(ImportOutcome.CREATED, retained.results[7].outcome)
            assertEquals(ImportOutcome.CREATED, retained.results[8].outcome)
            val core = owner().session
            assertEquals(ContentRoute.MAIN, core.view.contentRoute)
            assertNull(core.view.reader)
            assertNull(core.view.documentError)
            assertNull(core.view.importSummary)
            assertEquals(1, core.view.documents.count { it.title == "$prefix-first.txt" })
            assertEquals(0, core.view.documents.count { it.title == "$prefix-duplicate.txt" })
            assertEquals(1, core.view.documents.count { it.title == "$prefix-last.TXT" })
            SQLiteDatabase.openDatabase(
                    File(compose.activity.filesDir, "yomoku.sqlite3").path,
                    null,
                    SQLiteDatabase.OPEN_READONLY,
                )
                .use { db ->
                    for ((suffix, encoding) in
                        listOf("choice.txt" to "utf8", "shift.txt" to "shift_jis")) {
                        val document = core.view.documents.single { it.title == "$prefix-$suffix" }
                        db.rawQuery(
                                "SELECT text_encoding FROM imported_files WHERE document_id = ?",
                                arrayOf(document.documentId.value.toString()),
                            )
                            .use { cursor ->
                                assertTrue(cursor.moveToFirst())
                                assertEquals(encoding, cursor.getString(0))
                            }
                    }
                }
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            compose.onNodeWithText("Already in Library: $prefix-first.txt").assertIsDisplayed()
            compose
                .onNodeWithTag("file_import_summary")
                .performScrollToNode(hasText("$prefix-unreadable.txt"))
            compose
                .onNodeWithText("Could not read this file. Select it again to retry.")
                .assertIsDisplayed()
            compose
                .onNodeWithTag("file_import_summary")
                .performScrollToNode(hasText("$prefix-last.TXT"))
            compose
                .onNode(
                    hasText("$prefix-last.TXT") and
                        hasAnyAncestor(hasTestTag("file_import_summary"))
                )
                .assertIsDisplayed()
            compose.onNodeWithTag("reader_text").assertDoesNotExist()
            compose.activityRule.scenario.recreate()
            assertEquals(9, owner().fileImports.results.size)
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            compose.onNodeWithText("Close").performClick()
            assertEquals(0, retained.totalFiles)
            assertEquals(0f, retained.progress)
            compose.activityRule.scenario.recreate()
            compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }

    @Test
    fun currentDisplayNameAndProgressSurviveRecreationDuringBlockedProviderRead() {
        awaitIdle()
        val token = "slow-${System.nanoTime()}"
        val uri =
            Uri.Builder()
                .scheme("content")
                .authority("app.yomoku.test.txt-fixtures")
                .appendPath(token)
                .appendQueryParameter("name", "$token.txt")
                .appendQueryParameter("gate", token)
                .appendQueryParameter(
                    "bytes",
                    "wqk=",
                ) // Ambiguous UTF-8/Shift-JIS, no import needed.
                .build()
        val resolver = instrumentation.targetContext.contentResolver
        try {
            compose.runOnIdle { owner().fileImports.start(listOf(uri)) }
            compose.waitUntil(30_000) { owner().fileImports.filename == "$token.txt" }
            val retained = owner().fileImports
            assertTrue(retained.active)
            assertFalse(retained.choosingEncoding)
            compose.onNodeWithText("$token.txt").assertIsDisplayed()
            compose.onNodeWithText("File 1 of 1").assertIsDisplayed()
            compose.onNodeWithText("Processed: 0 of 1").assertIsDisplayed()
            compose
                .onNodeWithTag("file_import_progress")
                .assertRangeInfoEquals(ProgressBarRangeInfo(0f, 0f..1f))
            compose.activityRule.scenario.recreate()
            assertSame(retained, owner().fileImports)
            compose.onNodeWithText("$token.txt").assertIsDisplayed()
            compose.onNodeWithText("Processed: 0 of 1").assertIsDisplayed()
            resolver.call(uri, "release", token, null)
            compose.waitUntil(30_000) { retained.choosingEncoding }
            compose.onNodeWithText("File 1 of 1").assertIsDisplayed()
            compose.onNodeWithText("Processed: 0 of 1").assertIsDisplayed()
            compose.onNodeWithText("Skip file").performClick()
            compose.waitUntil(30_000) { !retained.active }
            awaitIdle()
            assertEquals(1, retained.results.size)
            assertEquals(R.string.import_file_skipped, retained.results.single().message)
            assertEquals(1f, retained.progress)
            compose.onNodeWithText("Close").performClick()
            assertEquals(0, retained.totalFiles)
            assertEquals(0f, retained.progress)
        } finally {
            resolver.call(uri, "release", token, null)
        }
    }

    @Test
    fun cancellingSystemPickerLeavesLibraryAndPreviousStateAlone() {
        awaitIdle()
        val documents = owner().session.view.documents
        val monitor =
            instrumentation.addMonitor(
                IntentFilter(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    addDataType("*/*")
                },
                ActivityResult(Activity.RESULT_CANCELED, null),
                true,
            )
        try {
            compose.onNodeWithText("Import files").performClick()
            compose.waitForIdle()
            assertEquals(1, monitor.hits)
            assertFalse(owner().fileImports.active)
            assertTrue(owner().fileImports.results.isEmpty())
            assertEquals(documents, owner().session.view.documents)
            compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
            compose.onNodeWithTag("reader_text").assertDoesNotExist()
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }
}
