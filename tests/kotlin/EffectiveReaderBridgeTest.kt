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
    try {
        NativeBridge(directory.resolve("reader.sqlite3").path, listener, HostDictionary()).use {
            bridge ->
            val id =
                checkNotNull(dispatch(bridge, Event.CreatePaste("国家公務員。国家公務員。")).importSummary)
                    .documentId
            dispatch(bridge, Event.OpenReader(id))
            val current = reload(bridge)
            val snapshot = checkNotNull(current.effectiveReaderTokens)
            check(snapshot.tokens.map { it.surface } == listOf("国家公務員", "国家公務員"))
            val selection =
                EffectiveReaderSelection(snapshot.identity, snapshot.tokens.first().segmentIndex)
            val word =
                checkNotNull(
                    dispatch(bridge, Event.LoadEffectiveReaderWord(selection))
                        .effectiveReaderWord
                        ?.details
                )
            check(word.token.surface == "国家公務員")
            val forms = word.candidates.words.flatMap { it.forms }.map { it.form }.toSet()
            check(forms.containsAll(listOf("国家公務員", "国家", "公務員", "公務", "員")))
            check(word.encountered == 2UL)
        }
        println("PASS: Mode C JNI selection and A/B component dictionary lookup")
    } finally {
        directory.deleteRecursively()
    }
}
