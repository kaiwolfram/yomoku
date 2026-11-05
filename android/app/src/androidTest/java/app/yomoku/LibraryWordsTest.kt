package app.yomoku

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
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
class LibraryWordsTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun core() = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session

    private fun idle() {
        // Flush composition's lazy read before waiting on native state. A test
        // can otherwise observe an invalidated cache before its effect runs.
        compose.waitForIdle()
        try {
            compose.waitUntil(90_000) {
                val view = core().view
                !view.storageBusy &&
                    !view.completionSavePending &&
                    (view.contentRoute != ContentRoute.MAIN ||
                        view.destination != Destination.LIBRARY ||
                        view.settingsOpen ||
                        view.libraryWordDistributions != null ||
                        view.libraryWordError != null)
            }
        } catch (error: Throwable) {
            val view = core().view
            throw AssertionError(
                "Library idle: busy=${view.storageBusy}, counts=${view.libraryWordDistributions}, error=${view.libraryWordError}, route=${view.contentRoute}, documentError=${view.documentError}",
                error,
            )
        }
    }

    private fun open(id: DocumentId): EffectiveReaderPage {
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
        compose.onNodeWithTag("document_${id.value}").performClick()
        compose.waitUntil(90_000) {
            val view = core().view
            view.reader?.documentId == id && view.effectiveReaderTokens != null && !view.storageBusy
        }
        return checkNotNull(core().view.effectiveReaderTokens)
    }

    private fun checkBar(id: DocumentId, counts: WordStatusCounts) {
        compose.onNodeWithTag("library_documents").performScrollToKey(id.value)
        val node = compose.onNodeWithTag("word_distribution_${id.value}", useUnmergedTree = true)
        node.assertContentDescriptionEquals(
            "Distinct words: New ${counts.new}, Learning ${counts.learning}, Familiar ${counts.familiar}, Known ${counts.known}, Ignored ${counts.ignored}"
        )
        compose.mainClock.advanceTimeBy(500)
        compose.waitForIdle()
        val dark =
            (compose.activity.resources.configuration.uiMode and
                android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
                android.content.res.Configuration.UI_MODE_NIGHT_YES
        val colors =
            listOf(
                vocabularyBackground(VocabularyStatus.NEW, dark),
                vocabularyBackground(VocabularyStatus.LEARNING, dark),
                vocabularyBackground(VocabularyStatus.FAMILIAR, dark),
                Color(if (dark) 0xFF438A60 else 0xFF7CCB96),
                vocabularyBackground(VocabularyStatus.IGNORED, dark),
            )
        val values =
            listOf(counts.new, counts.learning, counts.familiar, counts.known, counts.ignored)
        val total = values.sumOf { it.toDouble() }
        val pixels = node.captureToImage().toPixelMap()
        var before = 0.0
        values.forEachIndexed { index, value ->
            if (value > 0UL) {
                val middle =
                    ((before + value.toDouble() / 2) / total * pixels.width)
                        .toInt()
                        .coerceAtMost(pixels.width - 1)
                val pixel = pixels[middle, pixels.height / 2]
                assertEquals(colors[index].red, pixel.red, 0.03f)
                assertEquals(colors[index].green, pixel.green, 0.03f)
                assertEquals(colors[index].blue, pixel.blue, 0.03f)
            }
            before += value.toDouble()
        }
    }

    @Test
    fun distinctBarsRefreshBothDocumentsAfterManualChoicesCompletionAndRecreation() {
        idle()
        compose.runOnUiThread {
            core().dispatch(Event.CloseSettings)
            core().dispatch(Event.CloseDocument)
            core().dispatch(Event.SelectDestination(Destination.LIBRARY))
            core().dispatch(Event.SearchLibraryTitles(""))
        }
        idle()
        val prefix = "Distribution-${System.nanoTime()}"
        val ids =
            listOf("a", "b").map { suffix ->
                compose.runOnUiThread {
                    core().dispatch(Event.CreatePaste("$prefix-$suffix\n猫 猫 犬 鳥 魚 虫 花"))
                }
                idle()
                checkNotNull(core().view.importSummary).documentId
            }
        compose.runOnUiThread {
            core().dispatch(Event.DismissImportSummary)
            core().dispatch(Event.SearchLibraryTitles(prefix))
        }
        val snapshot = open(ids[0])

        val surfaces = snapshot.tokens.distinctBy { it.surface }
        assertEquals(6, surfaces.size)
        surfaces.forEachIndexed { index, token ->
            compose.runOnUiThread {
                core()
                    .dispatch(
                        Event.SetEffectiveReaderStatus(
                            EffectiveReaderSelection(snapshot.identity, token.segmentIndex),
                            checkNotNull(token.vocabulary).id,
                            VocabularyStatus.entries[index % 5],
                            System.currentTimeMillis(),
                        )
                    )
            }
            idle()
        }
        compose.runOnUiThread { core().dispatch(Event.CloseDocument) }
        idle()
        val expected = WordStatusCounts(2UL, 1UL, 1UL, 1UL, 1UL)
        ids.forEach { id ->
            assertEquals(
                expected,
                core().view.libraryWordDistributions!!.single { it.documentId == id }.counts,
            )
            checkBar(id, expected)
        }
        val session = core()
        compose.activityRule.scenario.recreate()
        compose.waitForIdle()
        idle()
        assertSame(session, core())
        checkBar(ids[0], expected)
        open(ids[1])
        compose.onNodeWithTag("reader_scroll").performScrollToNode(hasTestTag("next_page"))
        compose.onNodeWithTag("next_page").performClick()
        compose.waitUntil(90_000) { core().view.contentRoute == ContentRoute.MAIN }
        idle()
        val completed = WordStatusCounts(0UL, 1UL, 1UL, 3UL, 1UL)
        ids.forEach { id ->
            assertEquals(
                completed,
                core().view.libraryWordDistributions!!.single { it.documentId == id }.counts,
            )
            checkBar(id, completed)
        }
    }
}
