package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
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

    private fun anchor(start: Long, end: Long) =
        TextAnchor(
            SourceId(Long.MAX_VALUE),
            ChapterId(Long.MAX_VALUE),
            TextRange(TextOffset(start), TextOffset(end)),
        )

    private fun fixture(firstSurface: String = "𠮷か\u3099\u0000"): ViewModel {
        val text = "😀 $firstSurface 猫 猫"
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
            )
        val analysis = ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE))
        val identity =
            EffectivePageIdentity(
                ULong.MAX_VALUE,
                ReaderTokenKey(page.documentId, page.anchor, analysis),
            )
        val tokens =
            listOf(firstSurface, "猫", "猫").mapIndexed { index, surface ->
                val charStart = if (index == 2) text.lastIndexOf(surface) else text.indexOf(surface)
                val start = text.substring(0, charStart).toByteArray().size.toLong()
                val end = start + surface.toByteArray().size
                EffectivePageToken(
                    ULong.MAX_VALUE - 2UL + index.toULong(),
                    anchor(4 + start, 4 + end),
                    TextRange(TextOffset(start), TextOffset(end)),
                    surface,
                    EffectiveTokenEligibility.ANALYZERJAPANESE,
                    if (index == 0) null else "ネコ",
                    if (index == 0) null else "猫",
                    ReaderVocabulary(
                        VocabularyId(if (index == 0) Long.MAX_VALUE else 8),
                        VocabularyStatus.NEW,
                    ),
                )
            }
        return ViewModel(
            ankiAddedNotice = null,
            ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
            ankiSetup = app.yomoku.shared.AnkiSetupPhase.Unchecked,
            ankiEntries = emptyList(),
            vocabulary =
                VocabularyState(
                    VocabularyQuery("", VocabularyStatus.entries.toList(), VocabularySort.NEWEST),
                    rows = emptyList(),
                    loading = false,
                ),
            statistics = StatisticsState(StatisticsTimeframe.LASTWEEK, loading = false),
            destination = Destination.LIBRARY,
            settingsOpen = false,
            documents = emptyList(),
            librarySort = LibrarySort.LASTREAD,
            storageBusy = false,
            contentRoute = ContentRoute.READER,
            reader = page,
            readerAnalysis = analysis,
            completionSavePending = false,
            effectiveReaderTokens = EffectiveReaderPage(identity, tokens),
            effectiveReaderWord = EffectiveReaderWordState(busy = false),
            effectiveReaderStatus = EffectiveStatusState(false, false),
        )
    }

    private fun show() {
        compose.setContent {
            YomokuTheme {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    EffectiveReaderWords(
                        frame.value,
                        owner.value,
                        ::dispatch,
                    )
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
                    readerWordMarker = token.anchor,
                    effectiveReaderWord =
                        EffectiveReaderWordState(
                            selection = event.value,
                            details =
                                if (failLookup) null
                                else
                                    EffectiveWordDetails(
                                        DictionaryLookup(
                                            DictionaryQuery(token.surface, token.dictionaryForm),
                                            emptyList(),
                                            revision = 1UL,
                                            loading = false,
                                        ),
                                        event.value,
                                        token,
                                        ULong.MAX_VALUE,
                                        null,
                                        DictionaryCandidates(
                                            token.surface,
                                            token.dictionaryForm,
                                            emptyList(),
                                            emptyList(),
                                        ),
                                    ),
                            error = if (failLookup) DocumentError.UNAVAILABLE else null,
                            busy = false,
                        ),
                )
        }
        if (event == Event.DismissReaderWord)
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        frame.value.effectiveReaderWord?.copy(
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

    private fun exposeDictionaryAnki(selection: AnkiSelection) {
        compose.runOnUiThread {
            val candidate =
                WordCandidate(
                    1,
                    WordEntry(
                        listOf(Spelling("猫", emptyList(), emptyList())),
                        listOf(Reading("ねこ", false, emptyList(), emptyList(), emptyList())),
                        listOf(
                            Sense(
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                emptyList(),
                                listOf(Translation("cat", "eng", null, null)),
                            )
                        ),
                    ),
                    listOf(
                        WordFormMatch("猫", listOf(WordMatch(ReadingMatch(0UL, 0UL), listOf(0UL))))
                    ),
                )
            frame.value =
                frame.value.copy(
                    ankiSetup = AnkiSetupPhase.Ready,
                    ankiEntries =
                        listOf(
                            AnkiEntryView(
                                selection,
                                AnkiEntryKey.Word(1, ReadingMatch(0UL, 0UL)),
                                "猫",
                                "ねこ",
                                AnkiEntryPhase.Add,
                            )
                        ),
                    effectiveReaderWord =
                        frame.value.effectiveReaderWord?.let {
                            it.copy(
                                details =
                                    it.details?.let { details ->
                                        details.copy(
                                            candidates =
                                                details.candidates.copy(words = listOf(candidate))
                                        )
                                    }
                            )
                        },
                )
        }
    }

    private fun assertMarker(index: Int?) {
        val annotations = text().getStringAnnotations(READER_MARKER, 0, text().length)
        if (index == null) assertTrue(annotations.isEmpty())
        else {
            val snapshot = frame.value.effectiveReaderTokens!!
            val token =
                effectiveDisplayTokens(frame.value.reader!!, snapshot.identity, snapshot)!![index]
            assertEquals(token.start, annotations.single().start)
            assertEquals(token.end, annotations.single().end)
        }
    }

    @Test
    fun crossingMarkerUnderlinesBothPagesAndPutsTheDotOnTheEndingPage() {
        val initial = fixture("猫𠮷")
        val page = initial.reader!!
        val marker = initial.effectiveReaderTokens!!.tokens.first().anchor
        val split = page.text.indexOf("𠮷")
        val firstText = page.text.substring(0, split)
        val boundary = page.anchor.range.start.value + firstText.toByteArray().size
        frame.value =
            initial.copy(
                reader =
                    page.copy(
                        text = firstText,
                        anchor = anchor(page.anchor.range.start.value, boundary),
                    ),
                effectiveReaderTokens = null,
                readerWordMarker = marker,
            )
        show()

        val first = text()
        val firstUnderline = first.getStringAnnotations(READER_MARKER, 0, first.length).single()
        assertEquals("猫", first.text.substring(firstUnderline.start, firstUnderline.end))
        assertTrue(first.getStringAnnotations(READER_MARKER_DOT, 0, first.length).isEmpty())

        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    reader =
                        page.copy(
                            text = page.text.substring(split),
                            anchor = anchor(boundary, page.anchor.range.end.value),
                            position = page.position.copy(offset = TextOffset(boundary)),
                            pageNumber = 2UL,
                        )
                )
        }
        val second = text()
        val secondUnderline = second.getStringAnnotations(READER_MARKER, 0, second.length).single()
        val dot = second.getStringAnnotations(READER_MARKER_DOT, 0, second.length).single()
        assertEquals("𠮷", second.text.substring(secondUnderline.start, secondUnderline.end))
        assertEquals(secondUnderline.start, dot.start)
        assertEquals(secondUnderline.end, dot.end)
    }

    @Test
    fun oneBookMarkerReplacesTheExactOccurrenceAcrossPages() {
        val first = frame.value
        val page = first.reader!!
        val snapshot = first.effectiveReaderTokens!!
        val secondAnchor = anchor(200, 200 + page.text.toByteArray().size.toLong())
        val second =
            first.copy(
                reader =
                    page.copy(
                        anchor = secondAnchor,
                        position = page.position.copy(offset = TextOffset(200)),
                        pageNumber = 2UL,
                    ),
                effectiveReaderTokens =
                    snapshot.copy(
                        identity =
                            snapshot.identity.copy(
                                revision = 2UL,
                                base = snapshot.identity.base.copy(page = secondAnchor),
                            ),
                        tokens =
                            snapshot.tokens.map { token ->
                                token.copy(
                                    anchor =
                                        anchor(
                                            token.anchor.range.start.value + 196,
                                            token.anchor.range.end.value + 196,
                                        )
                                )
                            },
                    ),
            )
        show()
        assertMarker(null)
        tap(1)
        assertMarker(1)
        click("word_status_NEW")
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertMarker(1)
        tap(2)
        assertMarker(2)
        click("word_status_KNOWN")
        assertTrue(events.last() is Event.SetEffectiveReaderStatus)
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens =
                        snapshot.copy(
                            tokens =
                                snapshot.tokens.map {
                                    it.copy(
                                        vocabulary =
                                            it.vocabulary?.copy(status = VocabularyStatus.KNOWN)
                                    )
                                }
                        )
                )
        }
        assertMarker(2)
        compose.runOnUiThread {
            frame.value = second.copy(readerWordMarker = frame.value.readerWordMarker)
        }
        assertMarker(null)
        tap(0)
        assertMarker(0)
        compose.runOnUiThread {
            frame.value =
                first.copy(
                    readerWordMarker = frame.value.readerWordMarker,
                    effectiveReaderTokens =
                        snapshot.copy(identity = snapshot.identity.copy(revision = 3UL)),
                )
        }
        assertMarker(null)
        compose.runOnUiThread {
            frame.value =
                second.copy(
                    readerWordMarker = frame.value.readerWordMarker,
                    effectiveReaderTokens =
                        second.effectiveReaderTokens!!.let {
                            it.copy(identity = it.identity.copy(revision = 4UL))
                        },
                )
        }
        assertMarker(0)
    }

    @Test
    fun typographyAndPageLengthChangesRetainTheMarker() {
        show()
        tap(1)
        click("word_status_NEW")
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    readerTypography =
                        ReaderTypography(ReaderFontSize.SP24, ReaderLineSpacing.PERCENT200)
                )
        }
        assertMarker(1)
        compose.runOnUiThread {
            frame.value = frame.value.copy(pageLength = PageLength.CHARACTERS750)
        }
        assertMarker(1)
    }

    @Test
    fun appRestoresTheMarkerAtPageTopWithoutOpeningTheSheet() {
        val first = frame.value
        compose.setContent { YomokuTheme { YomokuApp(frame.value, owner.value, ::dispatch) } }
        tap(1)
        click("word_status_NEW")
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = true) }
        compose.onNodeWithTag("reader_text").assertDoesNotExist()
        compose.runOnUiThread { frame.value = frame.value.copy(settingsOpen = false) }
        assertMarker(1)
        val saved = frame.value.readerWordMarker
        compose.runOnUiThread {
            frame.value = first.copy(reader = null, contentRoute = ContentRoute.MAIN)
        }
        compose.onNodeWithTag("reader_text").assertDoesNotExist()
        compose.runOnUiThread {
            owner.value = "replacement"
            frame.value = first.copy(readerWordMarker = saved, effectiveReaderTokens = null)
        }
        val text = text()
        val mark = text.getStringAnnotations(READER_MARKER, 0, text.length).single()
        assertEquals(text.text.indexOf("猫"), mark.start)
        assertEquals(mark.start + 1, mark.end)
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        val scroll = compose.onNodeWithTag("reader_scroll").fetchSemanticsNode()
        assertEquals(0f, scroll.config[SemanticsProperties.VerticalScrollAxisRange].value(), 0f)
    }

    @Test
    fun copyIconIsCenteredBesideTheWrappedReaderSurface() {
        val surface = "𠮷か\u3099".repeat(24)
        frame.value = fixture(surface)
        show()
        tap()
        assertWrappedWordHeader(compose)
        assertCopiedWord(compose, compose.activity, surface)
    }

    @Test
    fun copyUsesExactWholeOrSelectedSurfaceDuringLookupErrorsAndStatusWrites() {
        show()
        tap()
        val whole = frame.value.effectiveReaderWord!!.details!!.token.surface
        fun setLookup(lookup: DictionaryLookup) {
            compose.runOnUiThread {
                val word = frame.value.effectiveReaderWord!!
                frame.value =
                    frame.value.copy(
                        effectiveReaderWord =
                            word.copy(
                                details =
                                    word.details!!.copy(
                                        lookup =
                                            lookup.copy(whole = lookup.whole.copy(surface = whole))
                                    )
                            )
                    )
            }
        }
        setLookup(copyLookup())
        val before = events.size
        assertCopiedWord(compose, compose.activity, whole)
        assertEquals(before, events.size)
        click("dictionary_part_0")
        assertTrue(events.last() is Event.ChooseDictionaryPart)
        setLookup(copyLookup().copy(active = 0UL))
        assertCopiedWord(compose, compose.activity, "𠮷")
        val settledEvents = events.toList()
        for (busy in listOf(true, false, true)) {
            compose.runOnUiThread {
                frame.value =
                    frame.value.copy(
                        storageBusy = busy,
                        effectiveReaderStatus = EffectiveStatusState(true, busy),
                    )
            }
            compose.onNodeWithTag("dictionary_part_0").assertIsSelected().assertIsNotEnabled()
            assertCopiedWord(compose, compose.activity, "𠮷")
            assertEquals(settledEvents, events)
        }
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    storageBusy = false,
                    effectiveReaderStatus = EffectiveStatusState(false, false),
                )
        }
        setLookup(copyLookup().copy(active = 1UL, error = DocumentError.UNAVAILABLE))
        compose.onNodeWithTag("word_lookup_error").assertExists()
        assertCopiedWord(compose, compose.activity, "か\u3099")
        setLookup(copyLookup().copy(active = 1UL, loading = true))
        assertCopiedWord(compose, compose.activity, "か\u3099")
        setLookup(copyLookup().copy(active = 1UL))
        click("dictionary_whole")
        assertNull((events.last() as Event.ChooseDictionaryPart).part)
        setLookup(copyLookup())
        assertCopiedWord(compose, compose.activity, whole)
    }

    @Test
    fun dictionaryAddUsesLoadedCompactSelectionAndRespectsReaderBusyGates() {
        frame.value =
            frame.value.copy(
                ankiSettings = AnkiSettingsView(AnkiSettingsPhase.READY, Long.MAX_VALUE, null),
                ankiSetup = app.yomoku.shared.AnkiSetupPhase.Unchecked,
                ankiEntries = emptyList(),
            )
        show()
        tap(2)
        val selected = frame.value.effectiveReaderWord!!.selection!!
        exposeDictionaryAnki(AnkiSelection.Effective(selected, 1UL))
        click("anki_entry_word_1_0_0")
        assertEquals(
            Event.AddReaderAnkiEntry(
                AnkiSelection.Effective(selected, 1UL),
                AnkiEntryKey.Word(1, ReadingMatch(0UL, 0UL)),
            ),
            events.last(),
        )
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertIsNotEnabled()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
        val before = events.size
        compose.waitForIdle()
        assertEquals(before, events.size)
        compose.onNodeWithTag("word_anki_settings").assertDoesNotExist()
        assertTrue(events.none { it is Event.SetEffectiveReaderStatus })
    }

    @Test
    fun statusRefreshAndTransientDetailsDoNotRestartAnkiObservation() {
        show()
        tap(1)
        val selection = frame.value.effectiveReaderWord!!.selection!!
        exposeDictionaryAnki(AnkiSelection.Effective(selection, 1UL))
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertExists()
        fun queries() = events.filterIsInstance<Event.LoadReaderAnkiEntries>().size
        assertEquals(1, queries())
        val original = frame.value
        val snapshot = original.effectiveReaderTokens!!
        compose.runOnUiThread {
            frame.value =
                original.copy(
                    storageBusy = true,
                    effectiveReaderStatus = EffectiveStatusState(true, true),
                )
        }
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertIsNotEnabled()
        // Vocabulary refresh can be published before selected-word details catch up.
        compose.runOnUiThread {
            frame.value =
                original.copy(
                    effectiveReaderTokens =
                        snapshot.copy(
                            tokens =
                                snapshot.tokens.map {
                                    it.copy(
                                        vocabulary =
                                            it.vocabulary?.copy(status = VocabularyStatus.LEARNING)
                                    )
                                }
                        )
                )
        }
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertIsEnabled()
        assertEquals(1, queries())
        compose.runOnUiThread {
            frame.value =
                original.copy(
                    effectiveReaderWord = original.effectiveReaderWord!!.copy(details = null)
                )
        }
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertDoesNotExist()
        compose.runOnUiThread { frame.value = original }
        compose.onNodeWithTag("anki_entry_word_1_0_0").assertIsEnabled()
        assertEquals(1, queries())
    }

    @Test
    fun readerShowsSavedSeenDateWithProjectedOccurrenceCount() {
        show()
        tap()
        compose.runOnUiThread {
            val word = frame.value.effectiveReaderWord!!
            frame.value =
                frame.value.copy(
                    effectiveReaderWord =
                        word.copy(
                            details =
                                word.details!!.copy(
                                    encountered = 1UL,
                                    lastSeenUnixMs =
                                        System.currentTimeMillis() - 21L * 24 * 60 * 60 * 1000,
                                )
                        )
                )
        }
        compose
            .onNodeWithTag("word_encountered")
            .performScrollTo()
            .assertTextEquals("Seen 1 time · 3 weeks ago")
    }

    @Test
    fun manualUnicodeSelectionUsesCompactIdentityAndAllFiveStatusesWithoutGuessingMetadata() {
        show()
        compose.onNodeWithText("Edit word boundaries").assertDoesNotExist()
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
            .assertTextEquals("Seen ${ULong.MAX_VALUE} times")
        compose.onNodeWithTag("word_no_match").assertExists()
        val selectedStatus = frame.value.effectiveReaderTokens!!.tokens.first().vocabulary!!.status
        compose.onNodeWithText("Close").assertDoesNotExist()
        val beforeDismiss = events.size
        click("word_status_$selectedStatus")
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertEquals(listOf(Event.DismissReaderWord), events.drop(beforeDismiss))
        tap()
        for (status in VocabularyStatus.entries.filter { it != selectedStatus }) {
            click("word_status_$status")
            compose.onNodeWithTag("reader_word_panel").assertExists()
            val event = events.last() as Event.SetEffectiveReaderStatus
            assertEquals(selection, event.selection)
            assertEquals(VocabularyId(Long.MAX_VALUE), event.vocabularyId)
            assertEquals(status, event.status)
            assertTrue(event.occurredAtUnixMs > 0)
        }
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
        compose.runOnUiThread { frame.value = ready.copy(storageBusy = true) }
        tap(1)
        assertTrue(events.none { it is Event.LoadEffectiveReaderWord })
        compose.onNodeWithTag("word_status_KNOWN").assertIsNotEnabled()
        failLookup = true
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
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
    fun newRevisionAndSessionReplacementRetireSelectionsAndOldLinkCallbacks() {
        show()
        val old = text().getLinkAnnotations(0, text().length).first().item
        tap()
        compose.runOnUiThread {
            frame.value =
                frame.value.copy(
                    effectiveReaderTokens =
                        frame.value.effectiveReaderTokens!!.let {
                            it.copy(identity = it.identity.copy(revision = 2UL))
                        },
                    effectiveReaderWord = EffectiveReaderWordState(busy = false),
                )
        }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        compose.runOnUiThread { old.linkInteractionListener!!.onClick(old) }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        tap()
        compose.runOnUiThread { owner.value = "session-two" }
        compose.onNodeWithTag("reader_word_panel").assertDoesNotExist()
        assertEquals(2, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        tap()
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
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
            .sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        tap(2)
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = true) }
        compose.onNodeWithTag("effective_status_retry").assertIsNotEnabled()
        compose.onNodeWithTag("effective_status_cancel").assertIsNotEnabled()
        compose.runOnUiThread { frame.value = frame.value.copy(storageBusy = false) }
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
            for (cat in cats) {
                val highlights = text.getStringAnnotations(READER_HIGHLIGHT, cat.start, cat.end)
                if (status == VocabularyStatus.KNOWN) assertTrue(highlights.isEmpty())
                else assertEquals(status.name, highlights.single().item)
            }
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
        compose.onNodeWithTag("word_encountered").performScrollTo().assertTextEquals("Seen 0 times")
        assertEquals(1, events.filterIsInstance<Event.LoadEffectiveReaderWord>().size)
    }

    @Test
    fun bundledCandidatesAndCrossingTokenKeepFullSurfaceIdentity() {
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
                effectiveReaderTokens = EffectiveReaderPage(identity, listOf(token)),
            )
        show()
        assertEquals("か\u3099", text().text)
        tap()
        compose.onNodeWithTag("word_surface").assertTextEquals(full)
        exposeDictionaryAnki(
            AnkiSelection.Effective(EffectiveReaderSelection(identity, token.segmentIndex), 1UL)
        )
        assertDictionaryEntries(compose)
        assertEquals(
            EffectiveReaderSelection(identity, token.segmentIndex),
            events.filterIsInstance<Event.LoadEffectiveReaderWord>().single().value,
        )
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
