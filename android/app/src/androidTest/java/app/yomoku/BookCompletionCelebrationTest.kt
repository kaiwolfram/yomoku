package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import app.yomoku.shared.*
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class BookCompletionCelebrationTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val frame = mutableStateOf(readerFixture())
    private val generation = mutableStateOf(0)
    private val consumed = mutableListOf<CompletionCommandId>()

    private fun render() {
        compose.setContent {
            key(generation.value) {
                YomokuTheme {
                    YomokuApp(
                        frame.value,
                        "completion-test",
                        { event ->
                            when (event) {
                                is Event.CompleteEffectiveReaderPage ->
                                    frame.value =
                                        frame.value.copy(
                                            storageBusy = true,
                                            completionSavePending = true,
                                        )
                                is Event.DismissBookCompletionNotice -> {
                                    consumed.add(event.value)
                                    frame.value = frame.value.copy(bookCompletionNotice = null)
                                }
                                else -> Unit
                            }
                        },
                    )
                }
            }
        }
    }

    private fun request(): Offset {
        compose.onNodeWithTag("next_page").performScrollTo()
        val center = compose.onNodeWithTag("next_page").fetchSemanticsNode().boundsInRoot.center
        captureUiTrial(compose, "finish-button")
        compose.onNodeWithTag("next_page").performClick()
        compose.onNodeWithTag("book_confetti").assertDoesNotExist()
        assertTrue(frame.value.completionSavePending)
        return center
    }

    private fun confirm(command: Long) {
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    completionSavePending = false,
                    reader = null,
                    contentRoute = ContentRoute.MAIN,
                    bookCompletionNotice = CompletionCommandId(command),
                )
        }
    }

    @Test
    fun confirmedCompletionStartsAtButtonCenterAndRepeatsThenCleansUp() {
        render()
        repeat(2) { attempt ->
            if (attempt > 0) compose.runOnUiThread { frame.value = readerFixture() }
            val center = request()
            compose.mainClock.autoAdvance = false
            confirm(attempt + 1L)
            compose.mainClock.advanceTimeBy(64)
            val overlay = compose.onNodeWithTag("book_confetti").fetchSemanticsNode()
            val origin = overlay.config[ConfettiOrigin] + overlay.boundsInRoot.topLeft
            assertEquals(center.x, origin.x, 1f)
            assertEquals(center.y, origin.y, 1f)
            compose.onNodeWithTag("book_confetti").assertHasNoClickAction()
            compose.onNodeWithTag("screen_title").assertTextEquals("Library")
            assertEquals(attempt + 1, consumed.size)
            compose.mainClock.advanceTimeBy(300)
            captureUiTrial(compose, "confetti")
            compose.mainClock.advanceTimeBy(700)
            compose.onNodeWithTag("book_confetti").assertExists()
            compose.mainClock.advanceTimeBy(800)
            compose.onNodeWithTag("book_confetti").assertDoesNotExist()
            compose.mainClock.autoAdvance = true
        }
        compose.runOnUiThread { generation.value++ }
        compose.onNodeWithTag("book_confetti").assertDoesNotExist()
        assertEquals(listOf(CompletionCommandId(1), CompletionCommandId(2)), consumed)
    }

    @Test
    fun lostLocalOriginBeforeConfirmationConsumesWithoutCelebrating() {
        render()
        request()
        // Rebuild the composition while the retained core command is still saving.
        compose.runOnUiThread { generation.value++ }
        compose.waitForIdle()
        confirm(9)
        compose.onNodeWithTag("book_confetti").assertDoesNotExist()
        assertEquals(listOf(CompletionCommandId(9)), consumed)
    }
}

private fun readerFixture(): ViewModel {
    val page =
        ReaderPage(
            DocumentId(1),
            "Short book",
            false,
            "終。",
            TextAnchor(
                SourceId(1),
                ChapterId(1),
                app.yomoku.shared.TextRange(TextOffset(0), TextOffset(6)),
            ),
            ReadingPosition(SourceId(1), ChapterId(1), TextOffset(0)),
            1UL,
            1UL,
            emptyList(),
            emptyList(),
        )
    val analysis = ReaderAnalysis.Available(AnalysisId(1))
    return scalarVocabularyView(emptyList())
        .copy(
            destination = Destination.LIBRARY,
            contentRoute = ContentRoute.READER,
            reader = page,
            readerAnalysis = analysis,
            effectiveReaderTokens =
                EffectiveReaderPage(
                    EffectivePageIdentity(
                        1UL,
                        ReaderTokenKey(page.documentId, page.anchor, analysis),
                    ),
                    emptyList(),
                ),
            effectiveReaderWord = EffectiveReaderWordState(busy = false),
            effectiveReaderStatus = EffectiveStatusState(false, false),
            libraryWordDistributions = emptyList(),
        )
}
