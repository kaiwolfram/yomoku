package app.yomoku

import android.app.Activity
import android.app.Instrumentation.ActivityResult
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import android.net.Uri
import android.util.Base64
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertRangeInfoEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
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
        compose.waitUntil(30_000) {
            val view = owner().session.view
            !view.storageBusy &&
                (owner().fileImports.active ||
                    view.libraryWordDistributions != null ||
                    view.libraryWordError != null)
        }
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

    private fun start(uri: Uri) {
        awaitIdle()
        compose.runOnIdle { owner().fileImports.start(uri) }
    }

    private fun finished(): FileImportResult {
        compose.waitUntil(90_000) { !owner().fileImports.active }
        awaitIdle()
        assertNull(owner().session.view.reader)
        return checkNotNull(owner().fileImports.result)
    }

    @Test
    fun cancelBlockedReadAllowsFreshImportAndIgnoresLateProviderResult() {
        val prefix = "cancel-read-${System.nanoTime()}"
        val first = fixture("$prefix-first.txt", "\uFEFF$prefix first 猫".toByteArray())
        val blocked =
            fixture("$prefix-blocked.txt", "\uFEFF$prefix partial 犬".toByteArray(), prefix)
        val fresh = fixture("$prefix-fresh.txt", byteArrayOf(0xc2.toByte(), 0xa9.toByte()))
        val resolver = instrumentation.targetContext.contentResolver
        val imports = owner().fileImports
        try {
            start(first)
            assertEquals(ImportOutcome.CREATED, finished().outcome)
            compose.onNodeWithText("Close").performClick()
            start(blocked)
            compose.waitUntil(30_000) { imports.filename == "$prefix-blocked.txt" }
            compose.onNodeWithText("Cancel").performClick()
            compose.waitUntil(30_000) { !imports.active }
            awaitIdle()
            assertNull(imports.result)
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            // The new reader executor must run while the previous provider is still blocked.
            start(fresh)
            compose.waitUntil(30_000) { imports.choosingEncoding }
            resolver.call(blocked, "release", prefix, null)
            compose.waitUntil(30_000) {
                resolver.call(blocked, "finished", prefix, null)?.getBoolean("finished") == true
            }
            compose.waitForIdle()
            assertEquals("$prefix-fresh.txt", imports.filename)
            assertNull(imports.result)
            compose.activityRule.scenario.recreate()
            compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
            compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
            compose.onNodeWithText("Cancel").performClick()
            compose.waitUntil(30_000) { !imports.active }
            awaitIdle()
            assertNull(imports.result)
            assertTrue(
                owner().session.view.documents.none {
                    it.title in listOf("$prefix-blocked.txt", "$prefix-fresh.txt")
                }
            )
            start(first)
            assertEquals(ImportOutcome.EXISTING, finished().outcome)
            compose.onNodeWithText("Already in Library: $prefix-first.txt").assertIsDisplayed()
            compose.onNodeWithText("Close").performClick()
        } finally {
            resolver.call(blocked, "release", prefix, null)
        }
    }

    @Test
    fun individualTxtImportsRetainErrorsEncodingRetriesAndResultAcrossRecreation() {
        val prefix = "TXT-${System.nanoTime()}"
        val imports = owner().fileImports
        val invalid = fixture("$prefix-invalid.txt", byteArrayOf(0xff.toByte()))
        start(invalid)
        assertEquals(DocumentError.TXTUNDECODABLE, finished().error)
        compose.onNodeWithText("Close").performClick()
        val unreadable = Uri.parse("content://app.yomoku.test.txt-fixtures/$prefix-unreadable.txt")
        start(unreadable)
        assertEquals(R.string.import_file_unreadable, finished().message)
        compose
            .onNodeWithText("Could not read this file. Select it again to retry.")
            .assertIsDisplayed()
        compose.onNodeWithText("Close").performClick()
        start(fixture("$prefix-unsupported.vtt", "WEBVTT".toByteArray()))
        assertEquals(R.string.import_supported_files, finished().message)
        compose.onNodeWithText("Close").performClick()
        for ((suffix, encoding) in listOf("choice" to "UTF-8", "shift" to "Shift-JIS")) {
            val bytes = "$prefix $suffix ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte())
            start(fixture("$prefix-$suffix.txt", bytes))
            compose.waitUntil(30_000) { imports.choosingEncoding }
            assertEquals(DocumentError.TXTENCODINGREQUIRED, imports.encodingError)
            compose.onNodeWithText("Skip file").assertDoesNotExist()
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            compose.onNodeWithText(encoding).performClick()
            val completed = finished()
            assertEquals(completed.toString(), ImportOutcome.CREATED, completed.outcome)
            val result = imports.result
            compose.activityRule.scenario.recreate()
            assertEquals(result, owner().fileImports.result)
            compose.onNodeWithTag("file_import_summary").assertIsDisplayed()
            compose.onNodeWithText("Close").performClick()
            assertNull(imports.result)
            SQLiteDatabase.openDatabase(
                    File(compose.activity.filesDir, "yomoku.sqlite3").path,
                    null,
                    SQLiteDatabase.OPEN_READONLY,
                )
                .use { db ->
                    val document =
                        owner().session.view.documents.single { it.title == "$prefix-$suffix.txt" }
                    db.rawQuery(
                            "SELECT text_encoding FROM imported_files WHERE document_id = ?",
                            arrayOf(document.documentId.value.toString()),
                        )
                        .use {
                            assertTrue(it.moveToFirst())
                            assertEquals(
                                if (suffix == "choice") "utf8" else "shift_jis",
                                it.getString(0),
                            )
                        }
                }
        }
    }

    @Test
    fun currentDisplayNameAndIndeterminateProgressSurviveRecreationDuringBlockedRead() {
        val token = "slow-${System.nanoTime()}"
        val uri = fixture("$token.txt", byteArrayOf(0xc2.toByte(), 0xa9.toByte()), token)
        val resolver = instrumentation.targetContext.contentResolver
        val imports = owner().fileImports
        try {
            start(uri)
            compose.waitUntil(30_000) { imports.filename == "$token.txt" }
            assertTrue(imports.active)
            assertFalse(imports.choosingEncoding)
            compose.onNodeWithText("$token.txt").assertIsDisplayed()
            compose.mainClock.advanceTimeBy(320)
            compose
                .onNodeWithTag("app_loading", useUnmergedTree = true)
                .assertRangeInfoEquals(ProgressBarRangeInfo.Indeterminate)
            compose.activityRule.scenario.recreate()
            assertSame(imports, owner().fileImports)
            compose.onNodeWithText("$token.txt").assertIsDisplayed()
            resolver.call(uri, "release", token, null)
            compose.waitUntil(30_000) { imports.choosingEncoding }
            compose.onNodeWithText("Cancel").performClick()
            compose.waitUntil(30_000) { !imports.active }
            awaitIdle()
            assertNull(imports.result)
            compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
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
            compose.onNodeWithTag("library_add").performClick()
            compose.onNodeWithText("Import book").performClick()
            compose.waitForIdle()
            assertEquals(1, monitor.hits)
            assertFalse(owner().fileImports.active)
            assertNull(owner().fileImports.result)
            assertEquals(documents, owner().session.view.documents)
            compose.onNodeWithTag("file_import_summary").assertDoesNotExist()
            compose.onNodeWithTag("reader_text").assertDoesNotExist()
        } finally {
            instrumentation.removeMonitor(monitor)
        }
    }
}
