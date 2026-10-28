package app.yomoku

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class LibrarySearchTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() =
        compose.waitUntil(90_000) {
            !core().view.storageBusy &&
                !core().view.readingTimeSavePending &&
                !core().view.completionSavePending
        }

    private fun search(query: String) {
        compose.onNodeWithTag("library_title_search").performTextReplacement(query)
        compose.waitUntil { core().view.libraryTitleQuery == query }
    }

    @Test
    fun titleOnlySearchClearNavigationAndRecreationPreserveLibraryRows() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.SearchLibraryTitles(""))
        }
        val prefix = "Search-${System.nanoTime()}"
        val bodyOnly = "$prefix-body-only"
        val imported = mutableListOf<LibraryDocument>()
        for (title in listOf("$prefix RUST", "$prefix 猫𠮷", "$prefix %_")) {
            compose.runOnUiThread { core().dispatch(Event.CreatePaste("$title\n$bodyOnly。")) }
            idle()
            val id = checkNotNull(core().view.importSummary).documentId
            imported += core().view.documents.single { it.documentId == id }
        }
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        val all = core().view.documents
        search(prefix.lowercase())
        assertEquals(imported, core().view.documents)
        compose.onNodeWithTag("clear_library_search").assertIsDisplayed()
        search("$prefix 猫𠮷")
        assertEquals(listOf(imported[1]), core().view.documents)
        compose.onNodeWithTag("document_${imported[1].documentId.value}").assertIsDisplayed()
        compose.onNodeWithTag("document_${imported[0].documentId.value}").assertDoesNotExist()

        val original = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertSame(original, core())
        assertEquals("$prefix 猫𠮷", core().view.libraryTitleQuery)
        compose.onNodeWithTag("library_title_search").assertTextContains("$prefix 猫𠮷")
        compose.onNodeWithTag("document_${imported[1].documentId.value}").assertIsDisplayed()
        compose.runOnUiThread {
            core().dispatch(Event.SelectDestination(Destination.VOCABULARY))
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.RefreshLibrary)
        }
        idle()
        assertEquals(listOf(imported[1]), core().view.documents)
        search(bodyOnly)
        assertTrue(core().view.documents.isEmpty())
        assertEquals(all.size.toULong(), core().view.libraryDocumentCount)
        compose.onNodeWithTag("library_empty_message").assertTextEquals("No matching titles.")
        search("$prefix %_")
        assertEquals(listOf(imported[2]), core().view.documents)
        compose.onNodeWithTag("clear_library_search").performClick()
        compose.waitUntil { core().view.libraryTitleQuery.isEmpty() }
        assertEquals(all, core().view.documents)
        compose.onNodeWithTag("clear_library_search").assertDoesNotExist()
        compose.onNodeWithTag("library_empty_message").assertDoesNotExist()
    }
}
