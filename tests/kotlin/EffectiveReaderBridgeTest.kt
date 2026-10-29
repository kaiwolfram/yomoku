package app.yomoku.tests

import app.yomoku.nativebridge.EffectsListener
import app.yomoku.nativebridge.FfiError
import app.yomoku.nativebridge.NativeBridge
import app.yomoku.shared.*
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/** Real Full + production JNI: edits must affect the same reader, Library and completion. */
internal fun verifyEffectiveReaderBridge() {
    val directory = Files.createTempDirectory("yomoku-effective-jni-").toFile()
    val callbacks = LinkedBlockingQueue<Thread>()
    val listener =
        object : EffectsListener {
            override fun onEffects(requests: ByteArray) {
                check(
                    Requests.bincodeDeserialize(requests).value.single().effect
                        is ShellEffect.Render
                )
                callbacks.add(Thread.currentThread())
            }

            override fun onError(error: FfiError) {
                throw error
            }
        }
    fun view(bridge: NativeBridge) = ViewModel.bincodeDeserialize(bridge.view())
    fun dispatch(bridge: NativeBridge, event: Event, count: Int = 1): ViewModel {
        bridge.update(event.bincodeSerialize())
        repeat(count) {
            check(checkNotNull(callbacks.poll(60, TimeUnit.SECONDS)) != Thread.currentThread())
        }
        return view(bridge).also { check(!it.storageBusy && it.documentError == null) }
    }
    fun reload(bridge: NativeBridge): ViewModel {
        val current = view(bridge)
        val page = checkNotNull(current.reader)
        return dispatch(
            bridge,
            Event.LoadReaderTokens(
                ReaderTokenKey(page.documentId, page.anchor, checkNotNull(current.readerAnalysis))
            ),
        )
    }
    fun edit(bridge: NativeBridge): EffectiveOverridePrompt {
        val current = view(bridge)
        val snapshot = checkNotNull(current.effectiveReaderTokens)
        val boundaries = snapshot.originalBoundaries.take(2)
        check(boundaries.map { it.surface } == listOf("国家", "公務員"))
        val anchor =
            boundaries[0]
                .anchor
                .copy(
                    range =
                        TextRange(boundaries[0].anchor.range.start, boundaries[1].anchor.range.end)
                )
        return checkNotNull(
            dispatch(
                    bridge,
                    Event.EffectiveReaderOverride(
                        EffectiveOverrideAction.Open(snapshot.identity, anchor)
                    ),
                )
                .effectiveReaderOverride
                ?.prompt
        )
    }
    fun surfaces(current: ViewModel) =
        checkNotNull(current.effectiveReaderTokens).tokens.map { it.surface }
    try {
        val database = directory.resolve("reader.sqlite3")
        val provider = HostDictionary()
        val id =
            NativeBridge(database.path, listener, provider).use { bridge ->
                val id =
                    checkNotNull(
                            dispatch(bridge, Event.CreatePaste("国家公務員。国家公務員。ヌヌヌヌヌヌ")).importSummary
                        )
                        .documentId
                dispatch(bridge, Event.OpenReader(id))
                val original = reload(bridge)
                check(original.readerTokens == null && original.effectiveReaderWord != null)
                check(surfaces(original) == listOf("国家", "公務員", "国家", "公務員", "ヌヌヌヌヌヌ"))
                for (mode in listOf(SplitMode.A, SplitMode.C)) {
                    val prompt = edit(bridge)
                    check(prompt.defaultScope == OverrideScope.ALLOCCURRENCES)
                    check(prompt.choices.none { it.mode == mode })
                    val generated =
                        dispatch(
                            bridge,
                            Event.EffectiveReaderOverride(
                                EffectiveOverrideAction.Generate(prompt.identity, mode)
                            ),
                        )
                    check(generated.effectiveReaderOverride?.reloadRequired == true)
                    check(generated.effectiveReaderTokens == null)
                    reload(bridge)
                }
                check(
                    provider.calls.get() == 1
                ) // whole-chapter alternatives reuse the import worker
                val prompt = edit(bridge)
                check(
                    prompt.choices.map { it.mode } == listOf(SplitMode.A, SplitMode.B, SplitMode.C)
                )
                check(
                    prompt.choices.first { it.mode == SplitMode.A }.parts == listOf("国家", "公務", "員")
                )
                check(prompt.choices.first { it.mode == SplitMode.C }.parts == listOf("国家公務員"))
                dispatch(
                    bridge,
                    Event.EffectiveReaderOverride(
                        EffectiveOverrideAction.Choose(
                            prompt.identity,
                            SplitMode.C,
                            OverrideScope.THISOCCURRENCE,
                            100,
                        )
                    ),
                )
                val local = reload(bridge)
                check(surfaces(local).take(3) == listOf("国家公務員", "国家", "公務員"))
                val current = edit(bridge)
                check(current.winningRules.single().scope == OverrideScope.THISOCCURRENCE)
                dispatch(
                    bridge,
                    Event.EffectiveReaderOverride(
                        EffectiveOverrideAction.Reverse(
                            current.identity,
                            current.winningRules.single().id,
                        )
                    ),
                )
                reload(bridge)
                val manual = edit(bridge)
                dispatch(
                    bridge,
                    Event.EffectiveReaderOverride(
                        EffectiveOverrideAction.Save(
                            manual.identity,
                            manual.defaultScope,
                            listOf("国", "家公務員"),
                            200,
                        )
                    ),
                )
                val split = reload(bridge)
                check(surfaces(split).take(4) == listOf("国", "家公務員", "国", "家公務員"))
                check(
                    split.effectiveReaderTokens!!.tokens.take(4).all {
                        it.reading == null && it.dictionaryForm == null
                    }
                )
                dispatch(bridge, Event.OpenSettings, 0)
                dispatch(
                    bridge,
                    Event.SetReaderTypography(
                        checkNotNull(split.readerTypography).copy(furigana = true)
                    ),
                )
                dispatch(bridge, Event.CloseSettings, 0)
                check(view(bridge).readerFurigana.none { it.range.start.value < 30 })
                dispatch(bridge, Event.CloseDocument, 0)
                val counts =
                    checkNotNull(
                            dispatch(bridge, Event.LoadLibraryWordDistributions)
                                .libraryWordDistributions
                        )
                        .single()
                        .counts
                check(counts == WordStatusCounts(3UL, 0UL, 0UL, 0UL, 0UL))
                id
            }
        val unavailable = HostDictionary().also { it.fail = true }
        NativeBridge(database.path, listener, unavailable).use { bridge ->
            dispatch(bridge, Event.RefreshLibrary)
            dispatch(bridge, Event.OpenReader(id))
            val split = reload(bridge)
            check(surfaces(split).take(4) == listOf("国", "家公務員", "国", "家公務員"))
            val prompt = edit(bridge)
            check(
                prompt.choices.size == 3 &&
                    prompt.winningRules.single().scope == OverrideScope.ALLOCCURRENCES
            )
            dispatch(
                bridge,
                Event.EffectiveReaderOverride(
                    EffectiveOverrideAction.Reverse(
                        prompt.identity,
                        prompt.winningRules.single().id,
                    )
                ),
            )
            val restored = reload(bridge)
            check(surfaces(restored).take(4) == listOf("国家", "公務員", "国家", "公務員"))
            check(restored.readerFurigana.size >= 4)
            val page = checkNotNull(restored.reader)
            val tokens = checkNotNull(restored.effectiveReaderTokens)
            val request = Event.CompleteEffectiveReaderPage(tokens.identity, page.position, 300)
            val completed = dispatch(bridge, request, 2)
            check(
                completed.contentRoute == ContentRoute.MAIN &&
                    completed.documents.single().finishedAtLeastOnce
            )
            check(completed.completionNotice?.newlyKnown == 3UL)
            val before = database.readBytes()
            dispatch(bridge, request, 0)
            dispatch(bridge, Event.RetryReaderCompletion, 0)
            check(database.readBytes().contentEquals(before))
            val reopened = dispatch(bridge, Event.OpenReader(id))
            check(reopened.effectiveReaderOverride?.reloadRequired == false)
            val reread = reload(bridge)
            check(reread.reader?.position?.offset?.value == 0L)
            val selected =
                checkNotNull(reread.effectiveReaderTokens).let {
                    EffectiveReaderSelection(it.identity, it.tokens.first().segmentIndex)
                }
            val word =
                checkNotNull(
                    dispatch(bridge, Event.LoadEffectiveReaderWord(selected))
                        .effectiveReaderWord
                        ?.details
                )
            check(
                word.encountered == 2UL && word.token.vocabulary?.status == VocabularyStatus.KNOWN
            )
            check(unavailable.calls.get() == 0)
            check(callbacks.isEmpty())
        }
        println(
            "PASS: production effective JNI real A/B/C, All/local/manual, reversal/reopen, Library/furigana/completion"
        )
    } finally {
        directory.deleteRecursively()
    }
}
