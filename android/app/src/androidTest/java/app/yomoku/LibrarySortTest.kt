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
class LibrarySortTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() =
        compose.waitUntil(90_000) {
            !core().view.storageBusy && !core().view.completionSavePending
        }

    private fun sort(sort: LibrarySort) {
        compose.onNodeWithTag("library_sort").performTouchInput { click() }
        compose.onNodeWithTag("library_sort_${sort.name}").assertIsEnabled().performTouchInput {
            click()
        }
        compose.waitUntil { core().view.librarySort == sort }
    }

    private fun reverseSort(sort: LibrarySort, expected: List<DocumentId>) {
        compose.onNodeWithTag("library_reverse_sort").performClick()
        compose.waitUntil { core().view.librarySort == sort }
        val ascending =
            sort in
                listOf(
                    LibrarySort.TITLE,
                    LibrarySort.LASTREADASCENDING,
                    LibrarySort.IMPORTDATEASCENDING,
                )
        compose
            .onNodeWithTag("library_reverse_sort")
            .assertContentDescriptionEquals(
                if (ascending) "Ascending. Reverse sort order" else "Descending. Reverse sort order"
            )
        assertOrder(expected)
    }

    private var importedIds = emptyList<DocumentId>()

    private fun assertOrder(expected: List<DocumentId>) {
        assertEquals(
            expected,
            core().view.documents.map { it.documentId }.filter { it in importedIds },
        )
        expected.forEach { id ->
            compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
            compose.onNodeWithTag("document_${id.value}").assertIsDisplayed()
        }
    }

    @Test
    fun sortingRetainsChoiceAndPersistsReadOrder() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.SortLibrary(LibrarySort.LASTREAD))
        }
        val prefix = "Sort-${System.nanoTime()}"
        val texts = listOf("$prefix beta\n本文", "$prefix ALPHA\n本文", "$prefix alpha\n本文")
        val ids = texts.map { text ->
            compose.runOnUiThread { core().dispatch(Event.CreatePaste(text)) }
            idle()
            checkNotNull(core().view.importSummary).documentId
        }
        compose.runOnUiThread { core().dispatch(Event.DismissImportSummary) }
        importedIds = ids
        assertOrder(ids.reversed())
        reverseSort(LibrarySort.LASTREADASCENDING, ids)
        reverseSort(LibrarySort.LASTREAD, ids.reversed())
        sort(LibrarySort.TITLE)
        assertOrder(listOf(ids[2], ids[1], ids[0]))
        compose.onNodeWithTag("library_sort").assertTextContains("Sort: Title")
        reverseSort(LibrarySort.TITLEDESCENDING, listOf(ids[0], ids[1], ids[2]))
        val session = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        assertSame(session, core())
        compose.onNodeWithTag("library_sort").assertTextContains("Sort: Title")
        assertOrder(listOf(ids[0], ids[1], ids[2]))
        assertEquals(LibrarySort.TITLEDESCENDING, core().view.librarySort)
        reverseSort(LibrarySort.TITLE, listOf(ids[2], ids[1], ids[0]))
        sort(LibrarySort.IMPORTDATE)
        assertOrder(ids.reversed())
        reverseSort(LibrarySort.IMPORTDATEASCENDING, ids)
        reverseSort(LibrarySort.IMPORTDATE, ids.reversed())
        compose.onNodeWithTag("library_documents").performScrollToKey(ids[0].value)
        compose.onNodeWithTag("document_${ids[0].value}").performClick()
        idle()
        compose.waitUntil { core().view.reader?.documentId == ids[0] }
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        sort(LibrarySort.LASTREAD)
        assertOrder(listOf(ids[0], ids[2], ids[1]))
        val recency = core().view.documents[0].lastReadSequence
        assertNotNull(recency)
        compose.runOnUiThread { core().dispatch(Event.RefreshLibrary) }
        idle()
        assertOrder(listOf(ids[0], ids[2], ids[1]))
        compose.runOnUiThread { core().dispatch(Event.CreatePaste(texts[0])) }
        idle()
        assertEquals(ImportOutcome.EXISTING, core().view.importSummary?.outcome)
        assertEquals(recency, core().view.documents[0].lastReadSequence)
        assertOrder(listOf(ids[0], ids[2], ids[1]))
    }
}
