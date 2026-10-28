package app.yomoku.tests

import app.yomoku.shared.DocumentError
import app.yomoku.shared.EffectiveOverrideAction
import app.yomoku.shared.EffectiveOverrideFailure
import app.yomoku.shared.EffectiveReaderOverrideState
import app.yomoku.shared.OverrideScope
import app.yomoku.shared.SplitMode
import java.io.File

fun verifyEffectiveOverrideCodecs(dir: File) {
    val states =
        (0..4).map { index ->
            val bytes = File(dir, "effective-override-state-$index.bin").readBytes()
            val state = EffectiveReaderOverrideState.bincodeDeserialize(bytes)
            check(state.bincodeSerialize().contentEquals(bytes))
            File(dir, "effective-override-state-$index-return.bin")
                .writeBytes(state.bincodeSerialize())
            state
        }
    val ready = states[0]
    val prompt = checkNotNull(ready.prompt)
    val identity = checkNotNull(ready.identity)
    check(prompt.identity == identity)
    check(identity.revision == ULong.MAX_VALUE && identity.page.revision == ULong.MAX_VALUE)
    check(identity.bincodeSerialize().size == 68)
    check(prompt.anchor.range.start.value == 4L && prompt.anchor.range.end.value == 15L)
    check(prompt.surface == "𠮷か\u3099\u0000")
    val parts = listOf("𠮷", "か\u3099\u0000")
    check(prompt.defaultScope == OverrideScope.ALLOCCURRENCES)
    check(prompt.choices.map { it.mode } == SplitMode.entries)
    check(prompt.choices[0].parts == parts && prompt.choices[2].parts == parts)
    check(prompt.choices[1].parts == listOf(prompt.surface))
    check(prompt.winningRules.map { it.id.value } == listOf(Long.MAX_VALUE, Long.MAX_VALUE - 1L))
    check(prompt.winningRules.map { it.scope } == OverrideScope.entries)
    check(prompt.winningRules.all { it.surface == prompt.surface && it.parts == parts })
    check(!ready.pending && !ready.busy && ready.error == null && !ready.reloadRequired)
    check(states[1].prompt == null && states[1].pending && states[1].busy)
    check(
        states[2].identity == identity &&
            states[2].prompt == null &&
            states[2].pending &&
            !states[2].busy
    )
    check(states[2].error == EffectiveOverrideFailure.Storage(DocumentError.UNAVAILABLE))
    check(states[3].error == EffectiveOverrideFailure.InvalidSelection)
    check(states[4].error == EffectiveOverrideFailure.StaleContext)
    check(
        states.drop(3).all {
            it.identity == null && it.prompt == null && !it.pending && it.reloadRequired
        }
    )

    val expectedActions = buildList {
        add(EffectiveOverrideAction.Open(identity.page, prompt.anchor))
        for (scope in OverrideScope.entries) {
            for (mode in SplitMode.entries) {
                add(EffectiveOverrideAction.Choose(identity, mode, scope, Long.MAX_VALUE))
            }
            add(EffectiveOverrideAction.Save(identity, scope, parts, Long.MAX_VALUE))
        }
        for (mode in listOf(SplitMode.A, SplitMode.C)) {
            add(EffectiveOverrideAction.Generate(identity, mode))
        }
        add(EffectiveOverrideAction.Reverse(identity, prompt.winningRules[0].id))
        add(EffectiveOverrideAction.Retry(identity))
        add(EffectiveOverrideAction.Cancel(identity))
        add(EffectiveOverrideAction.Dismiss(identity))
    }
    expectedActions.forEachIndexed { index, expected ->
        val bytes = File(dir, "effective-override-action-$index.bin").readBytes()
        val action = EffectiveOverrideAction.bincodeDeserialize(bytes)
        check(action == expected)
        check(action.bincodeSerialize().contentEquals(bytes))
        File(dir, "effective-override-action-$index-return.bin")
            .writeBytes(action.bincodeSerialize())
    }
}
