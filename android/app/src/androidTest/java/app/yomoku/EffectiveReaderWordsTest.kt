package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.text.AnnotatedString
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import app.yomoku.theme.YomokuTheme
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EffectiveReaderWordsTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    private val events = mutableListOf<Event>()
    private val owner = mutableStateOf("session-one")
    private val frame = mutableStateOf(fixture())
    private var failLookup = false
    private var interactions = 0

    private fun anchor(start: Long, end: Long) =
        TextAnchor(
            SourceId(Long.MAX_VALUE),
            ChapterId(Long.MAX_VALUE),
            TextRange(TextOffset(start), TextOffset(end)),
        )

    private fun fixture(): ViewModel {
        val text = "😀 𠮷か\u3099\u0000 猫 猫"
        val page =
            ReaderPage(
                DocumentId(Long.MAX_VALUE),
                "Compact",
                false,
                text,
                anchor(4, 4L + text.toByteArray().size),
                ReadingPosition(SourceId(Long.MAX_VALUE), ChapterId(Long.MAX_VALUE), TextOffset(4)),
                1UL,
                2UL,
                emptyList(),
                emptyList(),
                emptyList(),
            )
        val analysis = ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE))
        val identity =
            EffectivePageIdentity(
                ULong.MAX_VALUE,
                ReaderTokenKey(page.documentId, page.anchor, analysis),
            )
        val tokens =
            listOf("𠮷か\u3099\u0000", "猫", "猫").mapIndexed { index, surface ->
                val charStart = if (index == 2) text.lastIndexOf(surface) else text.indexOf(surface)
                val start = text.substring(0, charStart).toByteArray().size.toLong()
                val end = start + surface.toByteArray().size
                EffectivePageToken(
                    ULong.MAX_VALUE - 2UL + index.toULong(),
                    anchor(4 + start, 4 + end),
                    TextRange(TextOffset(start), TextOffset(end)),
                    surface,
                    if (index == 0) EffectiveTokenEligibility.MANUALJAPANESESPAN
                    else EffectiveTokenEligibility.ANALYZERJAPANESE,
                    if (index == 0) null else "ネコ",
                    if (index == 0) null else "猫",
                    ReaderVocabulary(
                        VocabularyId(if (index == 0) Long.MAX_VALUE else 8),
                        VocabularyStatus.NEW,
                    ),
                )
            }
        return ViewModel(
            ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
            vocabulary =
                VocabularyState(
                    VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                    rows = emptyList(),
                    selectingAll = false,
                    loading = false,
                ),
            statistics = StatisticsState(StatisticsTimeframe.LASTYEAR, loading = false),
            destination = Destination.LIBRARY,
            settingsOpen = false,
            documents = emptyList(),
            libraryTitleQuery = "",
            librarySort = LibrarySort.LASTREAD,
            libraryDocumentCount = 0UL,
            storageBusy = false,
            contentRoute = ContentRoute.READER,
            reader = page,
            readerAnalysis = analysis,
            readerTime = ReadingTimeUpdate(page.documentId, page.position, PendingReadingTime(0)),
            readingTimeSavePending = false,
            completionSavePending = false,
            effectiveReaderTokens = EffectiveReaderPage(identity, tokens, emptyList()),
            effectiveReaderWord =
                EffectiveReaderWordState(busy = false, customEntryPending = false),
            effectiveReaderStatus = EffectiveStatusState(false, false),
            readerFurigana = emptyList(),
        )
    }

    private fun show() {
        compose.setContent {
            YomokuTheme {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    EffectiveReaderWords(frame.value, owner.value, ::dispatch, { interactions++ })
                }
            }
        }
    }

    private fun dispatch(event: Event) {
        events.add(event)
        if (event is Event.LoadEffectiveReaderWord) {
            val token =
                frame.value.effectiveReaderTokens!!.tokens.single {
                    it.segmentIndex == event.value.segmentIndex
                }
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        EffectiveReaderWordState(
                            selection = event.value,
                            details =
                                if (failLookup) null
                                else
                                    EffectiveWordDetails(
                                        event.value,
                                        token,
                                        ULong.MAX_VALUE,
                                        DictionaryCandidates(
                                            token.surface,
                                            token.dictionaryForm,
                                            emptyList(),
                                            emptyList(),
                                        ),
                                        null,
                                    ),
                            error = if (failLookup) DocumentError.UNAVAILABLE else null,
                            busy = false,
                            customEntryPending = false,
                        )
                )
        }
        if (event == Event.DismissReaderWord)
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        frame.value.effectiveReaderWord!!.copy(
                            selection = null,
                            details = null,
                            error = null,
                        )
                )
    }

    private fun text(): AnnotatedString =
        compose
            .onNodeWithTag("reader_text")
            .fetchSemanticsNode()
            .config[SemanticsProperties.Text]
            .single()

    private fun tap(index: Int = 0) {
        val link = text().getLinkAnnotations(0, text().length)[index].item
        compose.runOnUiThread { link.linkInteractionListener!!.onClick(link) }
        compose.waitForIdle()
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun manualUnicodeSelectionUsesCompactIdentityAndAllFiveStatusesWithoutGuessingMetadata() {
        show()
        tap()
        val selection =
            EffectiveReaderSelection(
                frame.value.effectiveReaderTokens!!.identity,
                ULong.MAX_VALUE - 2UL,
            )
        assertEquals(listOf(Event.LoadEffectiveReaderWord(selection)), events)
        compose.onNodeWithTag("word_surface").assertTextEquals("𠮷か\u3099\u0000")
        compose.onNodeWithText("Reading: ネコ").assertDoesNotExist()
        compose.onNodeWithText("Dictionary form: 猫").assertDoesNotExist()
        compose
            .onNodeWithTag("word_encountered")
            .performScrollTo()
            .assertTextEquals("Encountered ${ULong.MAX_VALUE} times")
        compose.onNodeWithTag("word_no_match").assertExists()
        for (status in VocabularyStatus.entries) {
            click("word_status_$status")
            val event = events.last() as Event.SetEffectiveReaderStatus
            assertEquals(selection, event.selection)
            assertEquals(VocabularyId(Long.MAX_VALUE), event.vocabularyId)
            assertEquals(status, event.status)
            assertTrue(event.occurredAtUnixMs > 0)
        }
        assertTrue(interactions > 0)
        assertTrue(events.none { it is Event.LoadReaderWord || it is Event.SetReaderStatus })
    }

    @Test
    fun loadsRespectTimeBusyGatesAndFailuresOnlyRetryOnExplicitAction() {
        val ready = frame.value
        frame.value = ready.copy(effectiveReaderTokens = null, storageBusy = true)
        show()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        compose.waitForIdle()
        assertEquals(
            listOf(Event.LoadReaderTokens(ready.effectiveReaderTokens!!.identity.base)),
            events,
        )
        compose.runOnUiThread {
            frame.value = frame.value.copy(documentError = DocumentError.UNAVAILABLE)
        }
        compose.runOnUiThread { frame.value = frame.value.copy(documentError = null) }
        compose.waitForIdle()
        assertEquals(1, events.size)
        click("retry_word_lookup")
        assertEquals(2, events.size)
        compose.runOnUiThread { frame.value = ready.copy(readingTimeSavePending = true) }
        tap(1)
        assertTrue(events.none { it is Event.LoadEffectiveReaderWord })
        compose.onNodeWithTag("word_status_KNOWN").assertIsNotEnabled()
        failLookup = true
        compose.runOnUiThread { frame.value = frame.value.copy(readingTimeSavePending = false) }
        compose.waitForIdle()
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        compose.onNodeWithTag("word_lookup_error").assertExists()
        compose.runOnUiThread { frame.value = frame.value.copy(documentError = null) }
        compose.waitForIdle()
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        failLookup = false
        click("retry_word_details")
        assertEquals(2, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        compose.onNodeWithTag("word_no_match").assertExists()
        compose.runOnUiThread { frame.value = frame.value.copy(completionSavePending = true) }
        for (status in VocabularyStatus.entries) compose
            .onNodeWithTag("word_status_$status")
            .assertIsNotEnabled()
    }

    @Test
    fun newRevisionAndSessionReplacementRetireSelectionsDraftsAndOldLinkCallbacks() {
        show()
        val old = text().getLinkAnnotations(0, text().length).first().item
        tap()
        click("word_custom_edit")
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement("discard draft")
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens =
                        frame.value.effectiveReaderTokens!!.let {
                            it.copy(identity = it.identity.copy(revision = 2UL))
                        },
                    effectiveReaderWord =
                        EffectiveReaderWordState(busy = false, customEntryPending = false),
                )
        }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        compose.runOnUiThread { old.linkInteractionListener!!.onClick(old) }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        tap()
        compose.onNodeWithTag("word_custom_meaning_input").assertDoesNotExist()
        compose.runOnUiThread { owner.value = "session-two" }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertEquals(2, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        tap()
        compose.onNodeWithTag("word_custom_meaning_input").assertDoesNotExist()
    }

    @Test
    fun customLiteralWritesUseNativeRetryAndCancelAfterDismissalAndInvalidation() {
        show()
        tap()
        val selection = frame.value.effectiveReaderWord!!.selection!!
        click("word_custom_edit")
        compose.onNodeWithTag("word_custom_save").assertIsNotEnabled()
        val meaning = " literal meaning 📖 "
        val reading = " か\u3099𠮷 "
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement(meaning)
        compose
            .onNodeWithTag("word_custom_reading_input")
            .performScrollTo()
            .performTextReplacement(reading)
        click("word_custom_save")
        assertEquals(
            Event.SaveEffectiveReaderCustomEntry(selection, meaning, reading),
            events.last(),
        )
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        frame.value.effectiveReaderWord!!.copy(
                            customEntryPending = true,
                            customEntryError = DocumentError.UNAVAILABLE,
                        )
                )
        }
        compose.onNodeWithTag("word_custom_meaning_input").assertIsNotEnabled()
        click("effective_custom_retry")
        assertEquals(Event.RetryEffectiveReaderCustomEntry, events.last())
        compose.onNodeWithText("Close").performScrollTo().performClick()
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(effectiveReaderTokens = null) }
        click("effective_custom_retry")
        assertEquals(Event.RetryEffectiveReaderCustomEntry, events.last())
        click("effective_custom_cancel")
        assertEquals(Event.CancelEffectiveReaderCustomEntry, events.last())
        assertEquals(1, events.filterIsInstance<Event.SaveEffectiveReaderCustomEntry>().size)
    }

    @Test
    fun committedCustomEditsCancelLocallyAndDeleteUsesExactCompactSelection() {
        show()
        tap()
        val selected = frame.value.effectiveReaderWord!!
        val entry = ReaderCustomEntry(selected.details!!.token.surface, " committed ", " ぬ ")
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        selected.copy(details = selected.details.copy(customEntry = entry))
                )
        }
        click("word_custom_edit")
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .assertTextEquals("English meaning", entry.englishMeaning)
        compose
            .onNodeWithTag("word_custom_reading_input")
            .assertTextEquals("Reading", entry.reading)
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement("discard")
        click("word_custom_cancel")
        compose.onNodeWithTag("word_custom_meaning").assertTextEquals(entry.englishMeaning)
        click("word_custom_delete")
        assertEquals(Event.DeleteEffectiveReaderCustomEntry(selected.selection!!), events.last())
        assertTrue(events.none { it is Event.SaveEffectiveReaderCustomEntry })
    }

    @Test
    fun pendingStatusControlsSurviveDismissalAndBlockLookupsAndChoicesUntilExplicitCancel() {
        show()
        tap(1)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderStatus =
                        EffectiveStatusState(true, false, DocumentError.UNAVAILABLE)
                )
        }
        for (status in VocabularyStatus.entries) compose
            .onNodeWithTag("word_status_$status")
            .assertIsNotEnabled()
        click("effective_status_retry")
        assertEquals(Event.RetryEffectiveReaderStatus, events.last())
        compose.onNodeWithText("Close").performScrollTo().performClick()
        tap(2)
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        compose.runOnUiThread { frame.value = frame.value.copy(readingTimeSavePending = true) }
        compose.onNodeWithTag("effective_status_retry").assertIsNotEnabled()
        compose.onNodeWithTag("effective_status_cancel").assertIsNotEnabled()
        compose.runOnUiThread { frame.value = frame.value.copy(readingTimeSavePending = false) }
        click("effective_status_cancel")
        assertEquals(Event.CancelEffectiveReaderStatus, events.last())
    }

    @Test
    fun repeatedStatusesAndDeletionUpdateHighlightsAndDisableUnregisteredChoices() {
        show()
        tap(1)
        val snapshot = frame.value.effectiveReaderTokens!!
        val word = frame.value.effectiveReaderWord!!
        for (status in VocabularyStatus.entries) {
            compose.runOnUiThread {
                val tokens =
                    snapshot.tokens.map { token ->
                        if (token.surface == "猫")
                            token.copy(vocabulary = token.vocabulary!!.copy(status = status))
                        else token
                    }
                frame.value =
                    frame.value.copy(
                        effectiveReaderTokens = snapshot.copy(tokens = tokens),
                        effectiveReaderWord =
                            word.copy(details = word.details!!.copy(token = tokens[1])),
                    )
            }
            compose.onNodeWithTag("word_status_$status").assertIsSelected()
            val text = text()
            val cats =
                effectiveDisplayTokens(
                        frame.value.reader!!,
                        snapshot.identity,
                        frame.value.effectiveReaderTokens!!,
                    )!!
                    .filter { it.token.surface == "猫" }
            for (cat in cats) assertEquals(
                vocabularyBackground(
                    status,
                    compose.activity.resources.configuration.uiMode and
                        android.content.res.Configuration.UI_MODE_NIGHT_MASK ==
                        android.content.res.Configuration.UI_MODE_NIGHT_YES,
                ),
                text
                    .getLinkAnnotations(cat.start, cat.end)
                    .single()
                    .item
                    .styles!!
                    .style!!
                    .background,
            )
        }
        compose.runOnUiThread {
            val tokens =
                snapshot.tokens.map { if (it.surface == "猫") it.copy(vocabulary = null) else it }
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens = snapshot.copy(tokens = tokens),
                    effectiveReaderWord =
                        word.copy(
                            details = word.details!!.copy(token = tokens[1], encountered = 0UL)
                        ),
                )
        }
        for (status in VocabularyStatus.entries) compose
            .onNodeWithTag("word_status_$status")
            .assertIsNotEnabled()
        compose
            .onNodeWithTag("word_encountered")
            .performScrollTo()
            .assertTextEquals("Encountered 0 times")
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
    }

    @Test
    fun builtInCandidatesHideCustomActionsAndCrossingTokenKeepsFullSurfaceIdentity() {
        val base = frame.value
        val full = "𠮷か\u3099"
        val page =
            base.reader!!.copy(
                text = "か\u3099",
                anchor = anchor(8, 14),
                position = base.reader.position.copy(offset = TextOffset(8)),
            )
        val identity =
            base.effectiveReaderTokens!!
                .identity
                .copy(base = base.effectiveReaderTokens.identity.base.copy(page = page.anchor))
        val token =
            base.effectiveReaderTokens.tokens
                .first()
                .copy(
                    anchor = anchor(4, 14),
                    displayRange = TextRange(TextOffset(0), TextOffset(6)),
                    surface = full,
                )
        frame.value =
            base.copy(
                reader = page,
                effectiveReaderTokens = EffectiveReaderPage(identity, listOf(token), emptyList()),
            )
        show()
        assertEquals("か\u3099", text().text)
        tap()
        compose.onNodeWithTag("word_surface").assertTextEquals(full)
        val word = frame.value.effectiveReaderWord!!
        val candidate =
            WordCandidate(1, WordEntry(emptyList(), emptyList(), emptyList()), emptyList())
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        word.copy(
                            details =
                                word.details!!.copy(
                                    candidates =
                                        word.details.candidates.copy(words = listOf(candidate))
                                )
                        )
                )
        }
        compose.onNodeWithTag("word_definitions").assertExists()
        compose.onNodeWithTag("word_custom_entry").assertDoesNotExist()
        assertEquals(
            EffectiveReaderSelection(identity, token.segmentIndex),
            (events.single() as Event.LoadEffectiveReaderWord).value,
        )
    }

    @Test
    fun successfulLiteralCustomSaveClosesDraftAndWhitespaceMatchesRustValidation() {
        show()
        tap()
        click("word_custom_edit")
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement("\u0085")
        compose
            .onNodeWithTag("word_custom_reading_input")
            .performScrollTo()
            .performTextReplacement("ぬ")
        compose.onNodeWithTag("word_custom_save").assertIsNotEnabled()
        // Rust accepts U+001C as content; Kotlin's isBlank would incorrectly reject it.
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement("\u001C")
        compose.onNodeWithTag("word_custom_save").assertIsEnabled()
        val meaning = " literal meaning 📖 "
        val reading = " か\u3099𠮷 "
        compose
            .onNodeWithTag("word_custom_meaning_input")
            .performScrollTo()
            .performTextReplacement(meaning)
        compose
            .onNodeWithTag("word_custom_reading_input")
            .performScrollTo()
            .performTextReplacement(reading)
        click("word_custom_save")
        val word = frame.value.effectiveReaderWord!!
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        word.copy(
                            details =
                                word.details!!.copy(
                                    customEntry =
                                        ReaderCustomEntry(
                                            word.details.token.surface,
                                            meaning,
                                            reading,
                                        )
                                )
                        )
                )
        }
        compose.onNodeWithTag("word_custom_meaning_input").assertDoesNotExist()
        compose.onNodeWithTag("word_custom_meaning").performScrollTo().assertTextEquals(meaning)
        compose.onNodeWithTag("word_custom_reading").assertTextEquals("Reading: $reading")
    }

    @Test
    fun overrideRetirementNeedsExplicitPageReloadAndPendingWritesBlockWordReads() {
        val ready = frame.value
        frame.value =
            ready.copy(
                effectiveReaderTokens = null,
                effectiveReaderOverride =
                    EffectiveReaderOverrideState(
                        identity = null,
                        prompt = null,
                        busy = false,
                        pending = false,
                        error = null,
                        reloadRequired = true,
                    ),
            )
        show()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        click("retry_word_lookup")
        assertEquals(
            Event.LoadReaderTokens(ready.effectiveReaderTokens!!.identity.base),
            events.last(),
        )
        compose.runOnUiThread {
            frame.value =
                ready.copy(
                    effectiveReaderOverride =
                        frame.value.effectiveReaderOverride!!.copy(pending = true)
                )
        }
        tap()
        assertTrue(events.none { it is Event.LoadEffectiveReaderWord })
        compose.onNodeWithTag("word_status_KNOWN").assertIsNotEnabled()
        // A successful explicit page reload is usable even before reopening a boundary prompt.
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderOverride =
                        frame.value.effectiveReaderOverride!!.copy(pending = false)
                )
        }
        compose.waitForIdle()
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        // The retained reload marker must not block automatic token loads for a new page.
        compose.runOnUiThread {
            val page = ready.reader!!
            frame.value =
                frame.value.copy(
                    reader =
                        page.copy(
                            anchor = anchor(100, 100 + page.text.toByteArray().size.toLong())
                        ),
                    effectiveReaderTokens = null,
                )
        }
        compose.waitForIdle()
        assertEquals(2, events.filterIsInstance<Event.LoadReaderTokens>().size)
    }

    @Test
    fun malformedAndMissingEffectivePagesStayPlainWithoutLegacyFallback() {
        val ready = frame.value
        val snapshot = ready.effectiveReaderTokens!!
        frame.value =
            ready.copy(
                effectiveReaderTokens =
                    snapshot.copy(
                        tokens =
                            snapshot.tokens.map {
                                it.copy(displayRange = TextRange(TextOffset(1), TextOffset(2)))
                            }
                    )
            )
        show()
        assertTrue(text().getLinkAnnotations(0, text().length).isEmpty())
        click("retry_word_lookup")
        assertEquals(Event.LoadReaderTokens(snapshot.identity.base), events.last())
        compose.runOnUiThread {
            frame.value =
                ready.copy(
                    readerAnalysis = ReaderAnalysis.Missing,
                    effectiveReaderTokens =
                        snapshot.copy(
                            identity =
                                snapshot.identity.copy(
                                    base =
                                        snapshot.identity.base.copy(
                                            analysis = ReaderAnalysis.Missing
                                        )
                                ),
                            tokens = emptyList(),
                        ),
                )
        }
        assertEquals(ready.reader!!.text, text().text)
        assertTrue(text().getLinkAnnotations(0, text().length).isEmpty())
        assertTrue(events.none { it is Event.LoadEffectiveReaderWord })
    }

    @Test
    fun samePagePositionAndPresetChangesReloadTokensWithoutReplayingFailedReads() {
        val ready = frame.value.copy(pageLength = PageLength.CHARACTERS500)
        frame.value = ready
        show()
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        compose.runOnUiThread {
            frame.value = ready.copy(effectiveReaderTokens = null, storageBusy = true)
        }
        compose.waitForIdle()
        assertTrue(events.isEmpty())
        val page = ready.reader!!
        val position = page.position.copy(offset = TextOffset(9))
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    reader = page.copy(position = position),
                    readerTime = ready.readerTime!!.copy(position = position),
                )
        }
        compose.waitForIdle()
        val load = Event.LoadReaderTokens(ready.effectiveReaderTokens!!.identity.base)
        assertEquals(listOf(load), events)
        // A failed read on this same context still needs an explicit Retry.
        compose.runOnUiThread {
            frame.value = frame.value.copy(documentError = DocumentError.UNAVAILABLE)
        }
        compose.waitForIdle()
        compose.runOnUiThread { frame.value = frame.value.copy(documentError = null) }
        compose.waitForIdle()
        assertEquals(listOf(load), events)
        // Reflow can retain the same page anchor and exact position.
        compose.runOnUiThread {
            frame.value = frame.value.copy(pageLength = PageLength.CHARACTERS250)
        }
        compose.waitForIdle()
        assertEquals(listOf(load, load), events)
    }
}
