package app.yomoku

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createEmptyComposeRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class LibrarySortTest {
    @get:Rule(order = 0) val compose = createEmptyComposeRule()
    @get:Rule(order = 1) internal val isolated = IsolatedMainActivity()

    private fun core() = isolated.core

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

    private fun assertOrder(expected: List<DocumentId>) {
        assertEquals(expected, core().view.documents.map { it.documentId })
        compose.onNodeWithTag("library_documents").performScrollToIndex(0)
        val positions = expected.map { id ->
            compose
                .onNodeWithTag("document_${id.value}")
                .assertIsDisplayed()
                .getUnclippedBoundsInRoot()
                .top
        }
        assertTrue(positions.zipWithNext().all { (above, below) -> above < below })
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
        assertOrder(ids.reversed())
        reverseSort(LibrarySort.LASTREADASCENDING, ids)
        reverseSort(LibrarySort.LASTREAD, ids.reversed())
        sort(LibrarySort.TITLE)
        assertOrder(listOf(ids[2], ids[1], ids[0]))
        compose.onNodeWithTag("library_sort").assertTextContains("Sort: Title")
        reverseSort(LibrarySort.TITLEDESCENDING, listOf(ids[0], ids[1], ids[2]))
        val session = core()
        isolated.recreate()
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
