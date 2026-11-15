package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.compose.ui.unit.dp
import app.yomoku.shared.*
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class LibraryPresentationTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test
    fun emptyStateIsCenteredBelowMessagesAndUsesThemeColors() {
        val dark = mutableStateOf(false)
        val view =
            scalarVocabularyView(emptyList())
                .copy(
                    destination = Destination.LIBRARY,
                    libraryWordError = DocumentError.UNAVAILABLE,
                )
        compose.setContent {
            MaterialTheme(colorScheme = if (dark.value) darkColorScheme() else lightColorScheme()) {
                Surface(Modifier.fillMaxSize()) {
                    LibraryScreen(view, {}, libraryActionsEnabled = false)
                }
            }
        }
        for (night in listOf(false, true)) {
            compose.runOnUiThread { dark.value = night }
            val area = compose.onNodeWithTag("library_empty_area").fetchSemanticsNode().boundsInRoot
            val state =
                compose.onNodeWithTag("library_empty_state").fetchSemanticsNode().boundsInRoot
            assertEquals(area.center.x, state.center.x, 1f)
            assertEquals(area.center.y, state.center.y, 1f)
            compose.onNodeWithTag("library_empty_message").assertTextEquals("No documents yet")
            compose
                .onNodeWithTag("library_empty_hint")
                .assertTextEquals("Tap + to import a book or paste text.")
            val expected = (if (night) darkColorScheme() else lightColorScheme()).onSurfaceVariant
            for (tag in listOf("library_empty_message", "library_empty_hint")) {
                val pixels = compose.onNodeWithTag(tag).captureToImage().toPixelMap()
                assertTrue(
                    (0 until pixels.width).any { x ->
                        (0 until pixels.height).any { y ->
                            val pixel = pixels[x, y]
                            kotlin.math.abs(pixel.red - expected.red) < 0.02f &&
                                kotlin.math.abs(pixel.green - expected.green) < 0.02f &&
                                kotlin.math.abs(pixel.blue - expected.blue) < 0.02f
                        }
                    }
                )
            }
            compose.onNodeWithTag("library_sort").assertDoesNotExist()
            compose.onNodeWithTag("library_reverse_sort").assertDoesNotExist()
        }
    }

    @Test
    fun sortControlsFollowCountAndRetainOrderingAcrossDeletionThreshold() {
        val books =
            (1L..2L).map {
                LibraryDocument(
                    documentId = DocumentId(it),
                    title = "Book $it",
                    progressPercent = 0UL,
                    finishedAtLeastOnce = false,
                )
            }
        val frame =
            mutableStateOf(
                scalarVocabularyView(emptyList()).copy(destination = Destination.LIBRARY)
            )
        val events = mutableListOf<Event>()
        compose.setContent {
            MaterialTheme {
                LibraryScreen(frame.value, events::add, libraryActionsEnabled = false)
            }
        }
        for (count in listOf(0, 1, 2, 1, 0, 2)) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        documents = books.take(count),
                        librarySort = LibrarySort.TITLEDESCENDING,
                    )
            }
            if (count < 2) {
                compose.onNodeWithTag("library_sort").assertDoesNotExist()
                compose.onNodeWithTag("library_reverse_sort").assertDoesNotExist()
            } else {
                compose.onNodeWithTag("library_sort").assertTextContains("Sort: Title")
                compose
                    .onNodeWithTag("library_reverse_sort")
                    .assertContentDescriptionEquals("Descending. Reverse sort order")
            }
        }
        assertTrue(events.isEmpty())
    }

    @Test
    fun sortingSitsImmediatelyBelowToolbarWithoutImportSummary() {
        val books =
            (1L..2L).map {
                LibraryDocument(
                    documentId = DocumentId(it),
                    title = "Book $it",
                    progressPercent = 0UL,
                    finishedAtLeastOnce = false,
                )
            }
        val view =
            scalarVocabularyView(emptyList())
                .copy(
                    destination = Destination.LIBRARY,
                    documents = books,
                    libraryWordDistributions = emptyList(),
                    importSummary = ImportSummary(DocumentId(1), "Book 1", ImportOutcome.CREATED),
                )
        compose.setContent { MaterialTheme { YomokuApp(view, "library-presentation", {}) } }
        compose.onNodeWithTag("import_summary").assertDoesNotExist()
        compose.onNodeWithText("Imported: Book 1").assertDoesNotExist()
        compose.onNodeWithText("Close").assertDoesNotExist()
        val toolbar = compose.onNodeWithTag("app_toolbar").fetchSemanticsNode().boundsInRoot
        val sorting =
            compose.onNodeWithTag("library_sort_controls").fetchSemanticsNode().boundsInRoot
        assertEquals(toolbar.bottom, sorting.top, 1f)
        compose.onNodeWithTag("library_sort_controls").assertHeightIsAtLeast(48.dp)
        captureUiTrial(compose, "library")
    }
}
