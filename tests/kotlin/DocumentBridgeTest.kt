package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.DocumentError
import app.yomoku.shared.DocumentId
import app.yomoku.shared.EffectiveReaderSelection
import app.yomoku.shared.Event
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.PageLength
import app.yomoku.shared.ReaderAnalysis
import app.yomoku.shared.ReaderFontSize
import app.yomoku.shared.ReaderLineSpacing
import app.yomoku.shared.ReaderTokenKey
import app.yomoku.shared.ReaderTypography
import app.yomoku.shared.Requests
import app.yomoku.shared.ShellEffect
import app.yomoku.shared.TextOffset
import app.yomoku.shared.ViewModel
import app.yomoku.shared.VocabularyStatus
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

private class DocumentListener : EffectsListener {
    val effects = LinkedBlockingQueue<Pair<Thread, ByteArray>>()
    val errors = LinkedBlockingQueue<FfiError>()

    override fun onEffects(requests: ByteArray) {
        effects.add(Thread.currentThread() to requests)
    }

    override fun onError(error: FfiError) {
        errors.add(error)
    }

    fun dispatch(bridge: NativeBridge, event: Event, callbackCount: Int = 1): ViewModel {
        val immediate = Requests.bincodeDeserialize(bridge.update(event.bincodeSerialize()))
        check(immediate.value.single().effect is ShellEffect.Render)
        repeat(callbackCount) {
            val (thread, bytes) = checkNotNull(effects.poll(60, TimeUnit.SECONDS))
            check(thread != Thread.currentThread())
            check(Requests.bincodeDeserialize(bytes).value.single().effect is ShellEffect.Render)
        }
        check(errors.isEmpty())
        return ViewModel.bincodeDeserialize(bridge.view()).also { check(!it.storageBusy) }
    }
}

internal fun verifyDocumentBridge() {
    val directory = Files.createTempDirectory("yomoku-jni-").toFile()
    try {
        val database = directory.resolve("db.sqlite3")
        val listener = DocumentListener()
        val dictionary = HostDictionary()
        val text = "  題𠮷📖  \r\nか\u3099👩‍👩‍👧‍👦\u0000終\n"
        val (document, analysis) =
            NativeBridge(database.path, listener, dictionary).use { bridge ->
                check(!database.exists())
                val initial = listener.dispatch(bridge, Event.RefreshLibrary)
                check(initial.documents.isEmpty())
                check(dictionary.calls.get() == 0)
                dictionary.fail = true
                val failed = listener.dispatch(bridge, Event.CreatePaste(text))
                check(failed.documentError == DocumentError.UNAVAILABLE)
                check(failed.documents.isEmpty() && failed.importSummary == null)
                dictionary.fail = false
                val created = listener.dispatch(bridge, Event.CreatePaste(text))
                check(dictionary.calls.get() == 2)
                check(dictionary.threads.all { it != Thread.currentThread() })
                check(created.documents.single().title == "  題𠮷📖  ")
                check(created.reader == null)
                val summary = checkNotNull(created.importSummary)
                check(summary.outcome == ImportOutcome.CREATED)
                check(summary.title == "  題𠮷📖  ")
                val id = created.documents.single().documentId
                val loaded = checkNotNull(listener.dispatch(bridge, Event.OpenReader(id)).reader)
                check(loaded.text == text)
                val opened = listener.dispatch(bridge, Event.OpenReader(id))
                check(opened.contentRoute == ContentRoute.READER)
                val analysis = checkNotNull(opened.readerAnalysis) as ReaderAnalysis.Available
                check(analysis.value.value > 0L)
                check(dictionary.calls.get() == 2)
                val page = checkNotNull(opened.reader)
                check(page.documentId == id && page.text == text)
                check(page.anchor.sourceId == loaded.anchor.sourceId)
                check(page.anchor.range.end.value == text.toByteArray(Charsets.UTF_8).size.toLong())
                check(page.position.offset.value == 0L)
                check(page.pageNumber == 1UL && page.totalPages == 1UL)

                val tokenKey = ReaderTokenKey(id, page.anchor, analysis)
                val tokensView = listener.dispatch(bridge, Event.LoadReaderTokens(tokenKey))
                val tokens = checkNotNull(tokensView.effectiveReaderTokens)
                check(tokens.identity.base == tokenKey)
                check(tokens.tokens.isNotEmpty())
                check(tokens.tokens.all { it.vocabulary?.status == VocabularyStatus.NEW })
                for (token in tokens.tokens) {
                    val visible =
                        page.text
                            .toByteArray(Charsets.UTF_8)
                            .copyOfRange(
                                token.displayRange.start.value.toInt(),
                                token.displayRange.end.value.toInt(),
                            )
                            .toString(Charsets.UTF_8)
                    check(visible == token.surface)
                    check(token.anchor.sourceId == page.anchor.sourceId)
                }

                check(tokensView.reader == opened.reader && tokensView.completionNotice == null)
                check(dictionary.calls.get() == 2)
                val selected = tokens.tokens.first()
                val vocabularyId = checkNotNull(selected.vocabulary).id
                for (status in
                    listOf(
                        VocabularyStatus.LEARNING,
                        VocabularyStatus.FAMILIAR,
                        VocabularyStatus.KNOWN,
                        VocabularyStatus.IGNORED,
                        VocabularyStatus.NEW,
                    )) {
                    val current =
                        checkNotNull(
                            ViewModel.bincodeDeserialize(bridge.view()).effectiveReaderTokens
                        )
                    val change =
                        Event.SetEffectiveReaderStatus(
                            EffectiveReaderSelection(current.identity, selected.segmentIndex),
                            vocabularyId,
                            status,
                            4321L,
                        )
                    val changed = listener.dispatch(bridge, change)
                    check(changed.documentError == null && changed.reader == page)

                    check(changed.reader == opened.reader)
                    val expected =
                        tokens.tokens.map { token ->
                            if (token.vocabulary?.id == vocabularyId) {
                                token.copy(
                                    vocabulary =
                                        checkNotNull(token.vocabulary).copy(status = status)
                                )
                            } else token
                        }
                    check(changed.effectiveReaderTokens?.tokens == expected)
                    val repeated = listener.dispatch(bridge, change)
                    check(repeated.effectiveReaderTokens == changed.effectiveReaderTokens)
                    val refreshed = listener.dispatch(bridge, Event.LoadReaderTokens(tokenKey))
                    check(
                        refreshed.effectiveReaderTokens?.tokens ==
                            changed.effectiveReaderTokens.tokens
                    )
                    check(
                        refreshed.effectiveReaderTokens.identity !=
                            changed.effectiveReaderTokens.identity
                    )
                    check(dictionary.calls.get() == 2)
                }
                var saved = loaded
                for (byte in listOf(text.toByteArray(Charsets.UTF_8).size.toLong(), 5L, 0L, 5L)) {
                    val position = page.position.copy(offset = TextOffset(byte))
                    val updated = listener.dispatch(bridge, Event.SaveReadingPosition(position))
                    check(updated.documentError == null)
                    check(updated.reader?.position == position)
                    check(updated.readerAnalysis == analysis)

                    saved = checkNotNull(updated.reader)
                    check(saved.position == position)
                    check(
                        updated.documents.single().progressPercent ==
                            (byte * 100 / text.toByteArray(Charsets.UTF_8).size).toULong()
                    )
                }
                val invalid = page.position.copy(offset = TextOffset(3L))
                val rejected = listener.dispatch(bridge, Event.SaveReadingPosition(invalid))
                check(rejected.documentError == DocumentError.INVALIDDOCUMENT)
                check(rejected.reader == saved)
                bridge.update(Event.OpenSettings.bincodeSerialize())
                for (length in PageLength.entries) {
                    if (ViewModel.bincodeDeserialize(bridge.view()).pageLength == length) continue
                    val changed = listener.dispatch(bridge, Event.SetPageLength(length))
                    check(changed.pageLength == length)
                    check(changed.reader?.position == saved.position)
                    check(changed.readerAnalysis == analysis)
                    check(changed.reader == saved)
                }
                bridge.update(Event.CloseSettings.bincodeSerialize())
                val typographyPage = ViewModel.bincodeDeserialize(bridge.view()).reader
                for (size in ReaderFontSize.entries) {
                    for (spacing in ReaderLineSpacing.entries) {
                        val typography = ReaderTypography(size, spacing)
                        val before = ViewModel.bincodeDeserialize(bridge.view())
                        if (before.readerTypography == typography) continue
                        check(!before.settingsOpen && before.contentRoute == ContentRoute.READER)
                        val event = Event.SetReaderTypography(typography)
                        check(Event.bincodeDeserialize(event.bincodeSerialize()) == event)
                        val changed = listener.dispatch(bridge, event)
                        check(changed.readerTypography == typography)
                        check(changed.reader == typographyPage)
                        check(changed.reader == saved && changed.readerAnalysis == analysis)
                    }
                }
                val readerTypography = ViewModel.bincodeDeserialize(bridge.view()).readerTypography
                bridge.update(Event.OpenSettings.bincodeSerialize())
                check(
                    ViewModel.bincodeDeserialize(bridge.view()).readerTypography == readerTypography
                )
                bridge.update(Event.CloseSettings.bincodeSerialize())
                bridge.update(Event.CloseDocument.bincodeSerialize())
                check(ViewModel.bincodeDeserialize(bridge.view()).contentRoute == ContentRoute.MAIN)
                check(ViewModel.bincodeDeserialize(bridge.view()).readerAnalysis == null)

                val duplicate = listener.dispatch(bridge, Event.CreatePaste(text))
                check(duplicate.documents.single().documentId == id)
                check(duplicate.documents.single().progressPercent > 0UL)
                check(duplicate.importSummary?.outcome == ImportOutcome.EXISTING)
                check(duplicate.contentRoute == ContentRoute.MAIN && duplicate.reader == null)
                check(dictionary.calls.get() == 2) // foreground cache reused
                bridge.setForeground(false)
                listener.dispatch(bridge, Event.RefreshLibrary) // FIFO release barrier
                bridge.setForeground(true)
                check(dictionary.calls.get() == 2) // no eager reopen
                check(
                    listener.dispatch(bridge, Event.CreatePaste(text)).importSummary?.outcome ==
                        ImportOutcome.EXISTING
                )
                check(dictionary.calls.get() == 2) // duplicate skips asset loading
                check(
                    listener.dispatch(bridge, Event.CreatePaste(" \n")).documentError ==
                        DocumentError.EMPTYPASTE
                )
                check(listener.dispatch(bridge, Event.RefreshLibrary).documents.size == 1)
                check(
                    listener
                        .dispatch(bridge, Event.OpenReader(DocumentId(Long.MAX_VALUE)))
                        .documentError == DocumentError.NOTFOUND
                )
                saved to analysis
            }
        val reopened = DocumentListener()
        val unavailableDictionary = HostDictionary().also { it.fail = true }
        NativeBridge(database.path, reopened, unavailableDictionary).use { bridge ->
            val initial = reopened.dispatch(bridge, Event.RefreshLibrary)
            check(initial.importSummary == null)
            check(initial.documents.single().documentId == document.documentId)
            check(initial.pageLength == PageLength.CHARACTERS1500)
            check(
                initial.readerTypography ==
                    ReaderTypography(ReaderFontSize.SP24, ReaderLineSpacing.PERCENT200)
            )
            check(
                reopened.dispatch(bridge, Event.OpenReader(document.documentId)).reader == document
            )
            val opened = reopened.dispatch(bridge, Event.OpenReader(document.documentId))
            check(opened.reader?.text == text)
            check(opened.readerTypography == initial.readerTypography)
            check(opened.readerAnalysis == analysis)

            check(unavailableDictionary.calls.get() == 0)
            check(ViewModel.bincodeDeserialize(bridge.view()).reader?.position == document.position)
            val page = checkNotNull(opened.reader)
            val current =
                reopened.dispatch(
                    bridge,
                    Event.LoadReaderTokens(ReaderTokenKey(page.documentId, page.anchor, analysis)),
                )
            val request =
                Event.CompleteEffectiveReaderPage(
                    checkNotNull(current.effectiveReaderTokens).identity,
                    page.position,
                    1234L,
                )
            val completed = reopened.dispatch(bridge, request, callbackCount = 2)
            check(completed.documentError == null && !completed.completionSavePending)

            check(completed.contentRoute == ContentRoute.MAIN && completed.reader == null)
            check(completed.effectiveReaderTokens == null)
            check(completed.bookCompletionNotice == completed.completionNotice?.commandId)
            val notice = checkNotNull(completed.completionNotice)
            check(notice.newlyKnown > 0UL && notice.commandId.value > 0L)
            check(
                completed.documents.single().progressPercent == 100UL &&
                    completed.documents.single().finishedAtLeastOnce
            )
            check(completed.documents.single().finishedAtLeastOnce)
            check(completed.documents.single().progressPercent == 100UL)
            val committedView = bridge.view()
            bridge.update(request.bincodeSerialize()) // Late duplicate retains the committed view.
            bridge.update(Event.RetryReaderCompletion.bincodeSerialize())
            check(bridge.view().contentEquals(committedView))
            bridge.update(Event.DismissCompletionNotice.bincodeSerialize())
            check(ViewModel.bincodeDeserialize(bridge.view()).completionNotice == null)
            val reopenedBook = reopened.dispatch(bridge, Event.OpenReader(document.documentId))
            check(reopenedBook.bookCompletionNotice == null)
            check(reopenedBook.reader?.finishedAtLeastOnce == true)
            val rereadPage = checkNotNull(reopenedBook.reader)
            val knownTokens =
                checkNotNull(
                    reopened
                        .dispatch(
                            bridge,
                            Event.LoadReaderTokens(
                                ReaderTokenKey(document.documentId, rereadPage.anchor, analysis)
                            ),
                        )
                        .effectiveReaderTokens
                )
            check(knownTokens.tokens.all { it.vocabulary?.status == VocabularyStatus.KNOWN })
            check(knownTokens.tokens.map { it.surface }.isNotEmpty())
            check(unavailableDictionary.calls.get() == 0)
            val reread =
                reopened.dispatch(
                    bridge,
                    Event.CompleteEffectiveReaderPage(
                        knownTokens.identity,
                        rereadPage.position,
                        5678L,
                    ),
                    callbackCount = 2,
                )
            check(reread.documentError == null && reread.completionNotice == null)
            check(reread.reader == null && reread.contentRoute == ContentRoute.MAIN)
            check(
                reread.bookCompletionNotice != null &&
                    reread.bookCompletionNotice != completed.bookCompletionNotice
            )
            check(
                unavailableDictionary.calls.get() == 0
            ) // Stored analysis, no Sudachi reopen/rescan.
        }
        val finishedListener = DocumentListener()
        NativeBridge(database.path, finishedListener, unavailableDictionary).use { bridge ->
            val summary = finishedListener.dispatch(bridge, Event.RefreshLibrary).documents.single()
            check(summary.finishedAtLeastOnce && summary.progressPercent == 100UL)
            val opened = finishedListener.dispatch(bridge, Event.OpenReader(document.documentId))
            check(opened.reader?.position?.offset?.value == 0L)
            check(opened.reader.finishedAtLeastOnce && opened.reader.pageNumber == 1UL)
            check(
                opened.documents.single().finishedAtLeastOnce &&
                    opened.documents.single().progressPercent == 0UL
            )
            check(opened.readerAnalysis == analysis)
            check(
                finishedListener.dispatch(bridge, Event.OpenReader(document.documentId)).reader ==
                    opened.reader
            )
            check(unavailableDictionary.calls.get() == 0)
        }
        val failure = DocumentListener()
        NativeBridge(directory.resolve("missing/db.sqlite3").path, failure, HostDictionary()).use {
            bridge ->
            val unavailable = failure.dispatch(bridge, Event.RefreshLibrary)
            check(unavailable.documentError == DocumentError.UNAVAILABLE)
            check(unavailable.pageLength == null)
            check(directory.resolve("missing").mkdir())
            check(failure.dispatch(bridge, Event.RefreshLibrary).documentError == null)
        }
        // Closing during an operation must safely release the JNI callback after
        // the worker completes; the registry assertions in main wait for this.
        repeat(20) {
            NativeBridge(":memory:", DocumentListener(), HostDictionary()).use { bridge ->
                bridge.update(Event.RefreshLibrary.bincodeSerialize())
            }
        }
        println(
            "PASS: async JNI storage callbacks, Unicode paste/list/load/reopen, typed failures and close during I/O"
        )
    } finally {
        check(directory.deleteRecursively())
    }
}
