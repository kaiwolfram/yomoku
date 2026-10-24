package app.yomoku

import android.database.sqlite.SQLiteDatabase
import androidx.activity.compose.setContent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.swipeRight
import androidx.compose.ui.test.swipeUp
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.CompletionCommandId
import app.yomoku.shared.CompletionNotice
import app.yomoku.shared.Event
import app.yomoku.shared.PageLength
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderNavigationTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core(): CoreSession =
        ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun awaitReady() {
        compose.waitUntil(30_000) {
            val view = core().view
            !view.storageBusy && !view.readingTimeSavePending && !view.completionSavePending
        }
        compose.waitForIdle()
    }

    private fun openDocument(): Long {
        awaitReady()
        compose.runOnUiThread { core().dispatch(Event.OpenSettings) }
        compose.runOnUiThread { core().dispatch(Event.SetPageLength(PageLength.CHARACTERS500)) }
        awaitReady()
        compose.runOnUiThread { core().dispatch(Event.CloseSettings) }
        val title = "Navigation ${System.nanoTime()}"
        compose.runOnUiThread {
            core()
                .dispatch(
                    Event.CreatePaste("$title\n${"猫".repeat(3000)}。\n${"犬".repeat(3000)}。\n終。")
                )
        }
        awaitReady()
        val id = core().view.documents.single { it.title == title }.documentId
        compose.runOnUiThread { core().dispatch(Event.OpenReader(id)) }
        compose.waitUntil(30_000) { !core().view.storageBusy && core().view.readerTime != null }
        return id.value
    }

    private fun database(): SQLiteDatabase =
        SQLiteDatabase.openDatabase(
            File(compose.activity.filesDir, "yomoku.sqlite3").path,
            null,
            SQLiteDatabase.OPEN_READWRITE,
        )

    private fun completions(db: SQLiteDatabase, id: Long): Long =
        db.rawQuery(
                "SELECT count(*) FROM reading_completions WHERE document_id = ?",
                arrayOf(id.toString()),
            )
            .use {
                it.moveToFirst()
                it.getLong(0)
            }

    private fun assertTop() {
        val scroll =
            compose
                .onNodeWithTag("reader_scroll")
                .fetchSemanticsNode()
                .config[SemanticsProperties.VerticalScrollAxisRange]
        assertEquals(0f, scroll.value(), 0f)
    }

    @Test
    fun controlsAndSwipesPersistBothDirectionsResetScrollAndCountOnlyForward() {
        val id = openDocument()
        database().use { db ->
            compose.onNodeWithTag("previous_page").performScrollTo().assertIsNotEnabled()
            compose.onNodeWithTag("page_counter").assertTextEquals("1 / 3")
            val next =
                compose
                    .onNodeWithTag("next_page")
                    .fetchSemanticsNode()
                    .config[SemanticsActions.OnClick]
                    .action!!
            // Two callbacks from the same rendered button before asynchronous storage returns.
            compose.runOnUiThread {
                next()
                next()
            }
            awaitReady()
            assertEquals(2UL, core().view.reader!!.pageNumber)
            assertTop()
            assertEquals(1L, completions(db, id))
            val forwardProgress =
                core().view.documents.single { it.documentId.value == id }.progressPercent
            assertTrue(forwardProgress > 0UL)
            compose.onNodeWithTag("previous_page").performScrollTo().performClick()
            awaitReady()
            assertEquals(1UL, core().view.reader!!.pageNumber)
            assertTop()
            assertEquals(1L, completions(db, id))
            assertEquals(
                0UL,
                core().view.documents.single { it.documentId.value == id }.progressPercent,
            )

            // A short horizontal movement and vertical scrolling must not complete a page.
            compose.onNodeWithTag("reader_scroll").performTouchInput {
                swipe(Offset(center.x + 5f, center.y), Offset(center.x - 5f, center.y), 200)
            }
            compose.onNodeWithTag("reader_scroll").performTouchInput { swipeUp() }
            assertEquals(1UL, core().view.reader!!.pageNumber)
            assertEquals(1L, completions(db, id))
            // Swipes do not require reaching the content bottom.
            compose.onNodeWithTag("reader_scroll").performTouchInput { swipeLeft() }
            awaitReady()
            assertEquals(2UL, core().view.reader!!.pageNumber)
            assertTop()
            assertEquals(2L, completions(db, id)) // deliberate reread
            compose.onNodeWithTag("reader_scroll").performTouchInput { swipeRight() }
            awaitReady()
            assertEquals(1UL, core().view.reader!!.pageNumber)
            assertTop()
            assertEquals(2L, completions(db, id))
            compose.onNodeWithText("Back").performClick()
            awaitReady()
            compose
                .onNodeWithTag("progress_$id", useUnmergedTree = true)
                .performScrollTo()
                .assertTextEquals("0%")
            compose.onNodeWithTag("document_$id").performClick()
            awaitReady()
            assertTop()
            compose.activityRule.scenario.recreate()
            assertTop()
            assertEquals(1UL, core().view.reader!!.pageNumber)
        }
    }

    @Test
    fun failedForwardDisablesControlsAndRetryCommitsExactlyOnce() {
        val id = openDocument()
        database().use { db ->
            try {
                db.execSQL(
                    "CREATE TRIGGER fail_navigation BEFORE INSERT ON reading_completions WHEN NEW.document_id = $id BEGIN SELECT RAISE(ABORT, 'test completion'); END"
                )
                compose.onNodeWithTag("next_page").performScrollTo().performClick()
                compose.waitUntil(30_000) {
                    !core().view.storageBusy && core().view.documentError != null
                }
                assertEquals(1UL, core().view.reader!!.pageNumber)
                assertEquals(0L, completions(db, id))
                compose.onNodeWithTag("next_page").performScrollTo().assertIsNotEnabled()
                db.execSQL("DROP TRIGGER fail_navigation")
                compose.onNodeWithText("Retry").performScrollTo().performClick()
                awaitReady()
                assertEquals(2UL, core().view.reader!!.pageNumber)
                assertEquals(1L, completions(db, id))
                assertTop()
            } finally {
                db.execSQL("DROP TRIGGER IF EXISTS fail_navigation")
            }
        }
    }

    @Test
    fun positiveCompletionToastIsConsumedOnceAndZeroIsSilent() {
        awaitReady()
        var snapshot by
            mutableStateOf(
                core().view.copy(completionNotice = CompletionNotice(CompletionCommandId(1), 3UL))
            )
        val messages = mutableListOf<String>()
        var dismissals = 0
        compose.runOnUiThread {
            compose.activity.setContent {
                ReaderCompletionToast(
                    snapshot,
                    {
                        assertEquals(Event.DismissCompletionNotice, it)
                        dismissals++
                        snapshot = snapshot.copy(completionNotice = null)
                    },
                    { messages.add(it) },
                )
            }
        }
        compose.waitForIdle()
        assertEquals(listOf("3 new words marked as Known"), messages)
        assertEquals(1, dismissals)
        compose.runOnUiThread { snapshot = snapshot.copy(storageBusy = true) }
        compose.waitForIdle()
        assertEquals(1, messages.size)
        compose.runOnUiThread {
            snapshot =
                snapshot.copy(completionNotice = CompletionNotice(CompletionCommandId(2), 0UL))
        }
        compose.waitForIdle()
        assertEquals(1, messages.size)
        assertEquals(2, dismissals)
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertEquals(1, messages.size)
    }
}
