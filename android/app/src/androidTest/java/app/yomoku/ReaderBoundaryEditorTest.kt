package app.yomoku

import androidx.activity.ComponentActivity
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.unit.Density
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.*
import app.yomoku.shared.TextRange
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ReaderBoundaryEditorTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    private val actions = mutableListOf<EffectiveOverrideAction>()
    private val session = mutableStateOf("retained-session-one")
    private val enabled = mutableStateOf(true)
    private val dark = mutableStateOf(false)
    private val large = mutableStateOf(false)
    private var reloads = 0
    private var closes = 0
    private var interactions = 0

    private fun prompt(surface: String = "日本語猫", revision: ULong = 1uL): EffectiveOverridePrompt {
        val anchor =
            TextAnchor(
                SourceId(Long.MAX_VALUE),
                ChapterId(Long.MAX_VALUE),
                TextRange(
                    TextOffset(4),
                    TextOffset(4 + surface.toByteArray(Charsets.UTF_8).size.toLong()),
                ),
            )
        return EffectiveOverridePrompt(
            EffectiveOverrideIdentity(
                EffectivePageIdentity(
                    ULong.MAX_VALUE,
                    ReaderTokenKey(
                        DocumentId(Long.MAX_VALUE),
                        anchor,
                        ReaderAnalysis.Available(AnalysisId(Long.MAX_VALUE)),
                    ),
                ),
                revision,
            ),
            anchor,
            surface,
            listOf(
                EffectiveBoundaryChoice(SplitMode.A, listOf(surface)),
                EffectiveBoundaryChoice(SplitMode.B, listOf(surface)),
            ),
            emptyList(),
            OverrideScope.ALLOCCURRENCES,
        )
    }

    private fun ready(prompt: EffectiveOverridePrompt = prompt()) =
        EffectiveReaderOverrideState(prompt.identity, prompt, false, false, null, false)

    private fun show(
        state: MutableState<EffectiveReaderOverrideState>,
        restoration: StateRestorationTester? = null,
        sessionKey: () -> String = { session.value },
    ) {
        val content: @androidx.compose.runtime.Composable () -> Unit = {
            val density = LocalDensity.current
            CompositionLocalProvider(
                LocalDensity provides Density(density.density, if (large.value) 1.6f else 1f)
            ) {
                MaterialTheme(
                    colorScheme = if (dark.value) darkColorScheme() else lightColorScheme()
                ) {
                    ReaderBoundaryEditor(
                        state.value,
                        sessionKey(),
                        enabled.value,
                        actions::add,
                        onReload = { reloads++ },
                        onClose = { closes++ },
                        onInteraction = { interactions++ },
                        now = { Long.MAX_VALUE },
                    )
                }
            }
        }
        if (restoration == null) compose.setContent(content) else restoration.setContent(content)
    }

    private fun click(tag: String) = compose.onNodeWithTag(tag).performScrollTo().performClick()

    @Test
    fun modesScopesAndWinningRuleReversalDispatchExactCompactActions() {
        val base = prompt()
        val prompt =
            base.copy(
                choices =
                    listOf(
                        EffectiveBoundaryChoice(SplitMode.A, listOf("日本", "語", "猫")),
                        EffectiveBoundaryChoice(SplitMode.B, listOf("日本語", "猫")),
                    ),
                winningRules =
                    listOf(
                        EffectiveWinningRule(
                            OverrideId(Long.MAX_VALUE),
                            OverrideScope.ALLOCCURRENCES,
                            base.surface,
                            listOf(base.surface),
                        ),
                        EffectiveWinningRule(
                            OverrideId(7),
                            OverrideScope.THISOCCURRENCE,
                            "日本語",
                            listOf("日本", "語"),
                        ),
                    ),
            )
        show(mutableStateOf(ready(prompt)))
        compose.onNodeWithTag("boundary_scope_ALLOCCURRENCES").assertIsSelected()
        compose.runOnIdle { assertTrue(actions.isEmpty()) }
        click("boundary_mode_A")
        compose.runOnIdle {
            assertEquals(
                EffectiveOverrideAction.Choose(
                    prompt.identity,
                    SplitMode.A,
                    OverrideScope.ALLOCCURRENCES,
                    Long.MAX_VALUE,
                ),
                actions.last(),
            )
        }
        click("boundary_scope_THISOCCURRENCE")
        click("boundary_mode_B")
        compose.runOnIdle {
            assertEquals(
                EffectiveOverrideAction.Choose(
                    prompt.identity,
                    SplitMode.B,
                    OverrideScope.THISOCCURRENCE,
                    Long.MAX_VALUE,
                ),
                actions.last(),
            )
        }
        click("boundary_mode_C")
        compose.runOnIdle {
            assertEquals(
                EffectiveOverrideAction.Generate(prompt.identity, SplitMode.C),
                actions.last(),
            )
        }
        for (rule in prompt.winningRules) {
            click("boundary_reverse_${rule.id.value}")
            compose.runOnIdle {
                assertEquals(
                    EffectiveOverrideAction.Reverse(prompt.identity, rule.id),
                    actions.last(),
                )
            }
        }
        compose.runOnIdle {
            assertEquals(5, actions.size)
            assertTrue(interactions > 0)
        }
    }

    @Test
    fun manualSplitAndJoinPreserveLiteralUnicodeWithoutSurrogateCuts() {
        val base = prompt("𠮷か\u3099\u0000猫")
        val prompt =
            base.copy(
                choices =
                    listOf(EffectiveBoundaryChoice(SplitMode.B, listOf("𠮷か\u3099", "\u0000猫")))
            )
        show(mutableStateOf(ready(prompt)))
        compose.onNodeWithTag("boundary_cut_1").assertDoesNotExist()
        compose.onNodeWithTag("boundary_cut_4").assertIsSelected()
        click("boundary_merge_all")
        click("boundary_save")
        compose.runOnIdle {
            assertEquals(
                EffectiveOverrideAction.Save(
                    prompt.identity,
                    OverrideScope.ALLOCCURRENCES,
                    listOf(prompt.surface),
                    Long.MAX_VALUE,
                ),
                actions.last(),
            )
        }
        click("boundary_cut_2")
        click("boundary_cut_3")
        click("boundary_scope_THISOCCURRENCE")
        click("boundary_save")
        compose.runOnIdle {
            assertEquals(
                EffectiveOverrideAction.Save(
                    prompt.identity,
                    OverrideScope.THISOCCURRENCE,
                    listOf("𠮷", "か", "\u3099\u0000猫"),
                    Long.MAX_VALUE,
                ),
                actions.last(),
            )
        }
        click("boundary_cut_3")
        click("boundary_save")
        compose.runOnIdle {
            assertEquals(
                listOf("𠮷", "か\u3099\u0000猫"),
                (actions.last() as EffectiveOverrideAction.Save).parts,
            )
        }
    }

    @Test
    fun pendingWriteGatesControlsAndRetainsOnlyNativeRetryCancelIdentity() {
        val prompt = prompt()
        val state = mutableStateOf(ready(prompt))
        show(state)
        compose.runOnIdle { enabled.value = false }
        compose.onNodeWithTag("boundary_mode_A").assertIsNotEnabled()
        compose.onNodeWithTag("boundary_save").assertIsNotEnabled()
        compose.runOnIdle {
            enabled.value = true
            state.value =
                EffectiveReaderOverrideState(prompt.identity, null, true, true, null, false)
        }
        compose.onNodeWithTag("boundary_busy").assertExists()
        compose.onNodeWithTag("boundary_close").assertIsNotEnabled()
        compose.onNodeWithTag("boundary_retry").assertDoesNotExist()
        compose.onNodeWithTag("boundary_save").assertDoesNotExist()
        compose.runOnIdle {
            state.value =
                state.value.copy(
                    busy = false,
                    error = EffectiveOverrideFailure.Storage(DocumentError.UNAVAILABLE),
                )
        }
        compose.onNodeWithTag("boundary_close").assertIsNotEnabled()
        click("boundary_retry")
        click("boundary_cancel")
        compose.runOnIdle {
            assertEquals(
                listOf(
                    EffectiveOverrideAction.Retry(prompt.identity),
                    EffectiveOverrideAction.Cancel(prompt.identity),
                ),
                actions,
            )
            assertEquals(0, closes)
            enabled.value = false
        }
        compose.onNodeWithTag("boundary_retry").assertIsNotEnabled()
        compose.onNodeWithTag("boundary_cancel").assertIsNotEnabled()
        compose.runOnIdle {
            enabled.value = true
            state.value = EffectiveReaderOverrideState(null, null, false, false, null, true)
        }
        click("boundary_reload")
        compose.runOnIdle {
            assertEquals(1, reloads)
            assertEquals(2, actions.size)
        }
        click("boundary_close")
        compose.runOnIdle {
            assertEquals(1, closes)
            assertEquals(2, actions.size)
        }
    }

    @Test
    fun staleMismatchedAndMalformedPromptsRequireReloadWithoutEditing() {
        val initial = ready()
        val state = mutableStateOf(initial)
        show(state)
        val invalid =
            listOf(
                initial.copy(error = EffectiveOverrideFailure.StaleContext),
                initial.copy(error = EffectiveOverrideFailure.InvalidSelection),
                initial.copy(reloadRequired = true),
                initial.copy(identity = initial.identity!!.copy(revision = 2uL)),
                initial.copy(prompt = initial.prompt!!.copy(surface = "猫\uD800")),
            )
        for (value in invalid) {
            compose.runOnIdle { state.value = value }
            compose.onNodeWithTag("boundary_mode_A").assertDoesNotExist()
            compose.onNodeWithTag("boundary_save").assertDoesNotExist()
            click("boundary_reload")
        }
        compose.runOnIdle {
            assertEquals(invalid.size, reloads)
            assertTrue(actions.isEmpty())
        }
    }

    @Test
    fun draftRestoresForSameSessionAndPromptThenResetsForReplacements() {
        val restoration = StateRestorationTester(compose)
        val state = mutableStateOf(ready())
        var owner = "original-session"
        show(state, restoration) { owner }
        click("boundary_cut_2")
        click("boundary_scope_THISOCCURRENCE")
        restoration.emulateSavedInstanceStateRestore()
        compose.onNodeWithTag("boundary_cut_2").assertIsSelected()
        compose.onNodeWithTag("boundary_scope_THISOCCURRENCE").assertIsSelected()
        // This changes only at restoration: exercise rejecting the saved OLD owner's draft.
        compose.runOnIdle { owner = "replacement-session" }
        restoration.emulateSavedInstanceStateRestore()
        compose.onNodeWithTag("boundary_cut_2").assertIsNotSelected()
        compose.onNodeWithTag("boundary_scope_ALLOCCURRENCES").assertIsSelected()
        click("boundary_cut_2")
        click("boundary_scope_THISOCCURRENCE")
        compose.runOnIdle { state.value = ready(prompt(revision = 2uL)) }
        compose.onNodeWithTag("boundary_cut_2").assertIsNotSelected()
        compose.onNodeWithTag("boundary_scope_ALLOCCURRENCES").assertIsSelected()
        compose.runOnIdle { assertTrue(actions.isEmpty()) }
        click("boundary_save")
        compose.runOnIdle {
            assertEquals(
                state.value.identity,
                (actions.single() as EffectiveOverrideAction.Save).identity,
            )
        }
    }

    @Test
    fun failedReadsReloadExplicitlyAndCloseOnlyDismissesCurrentPrompt() {
        val prompt = prompt()
        val state =
            mutableStateOf(
                EffectiveReaderOverrideState(prompt.identity, null, true, true, null, false)
            )
        show(state)
        compose.runOnIdle {
            state.value =
                state.value.copy(
                    pending = false,
                    busy = false,
                    error = EffectiveOverrideFailure.Storage(DocumentError.UNAVAILABLE),
                )
        }
        compose.onNodeWithTag("boundary_retry").assertDoesNotExist()
        click("boundary_reload")
        compose.runOnIdle {
            assertEquals(1, reloads)
            assertTrue(actions.isEmpty())
        }
        click("boundary_close")
        compose.runOnIdle {
            assertEquals(listOf(EffectiveOverrideAction.Dismiss(prompt.identity)), actions)
            assertEquals(1, closes)
        }
    }

    @Test
    fun whitespaceOnlyPartsAndCorruptStoredPartitionsCannotBeSaved() {
        val base = prompt("猫\u0085犬")
        val prompt =
            base.copy(choices = listOf(EffectiveBoundaryChoice(SplitMode.B, listOf("猫", "wrong"))))
        show(mutableStateOf(ready(prompt)))
        compose.onNodeWithTag("boundary_mode_B").assertIsNotEnabled()
        click("boundary_cut_1")
        click("boundary_cut_2")
        compose.onNodeWithTag("boundary_invalid").assertExists()
        compose.onNodeWithTag("boundary_save").assertIsNotEnabled()
        click("boundary_cut_1")
        click("boundary_save")
        compose.runOnIdle {
            assertEquals(
                listOf("猫\u0085", "犬"),
                (actions.single() as EffectiveOverrideAction.Save).parts,
            )
        }
    }

    @Test
    fun longSpansRemainScrollableAtLargeFontScaleInBothThemes() {
        val prompt = prompt("日本語".repeat(18))
        show(mutableStateOf(ready(prompt)))
        for (night in listOf(false, true)) {
            compose.runOnIdle {
                large.value = true
                dark.value = night
            }
            click("boundary_cut_27")
            click("boundary_save")
            compose.runOnIdle {
                val action = actions.last() as EffectiveOverrideAction.Save
                assertEquals(prompt.surface, action.parts.joinToString(""))
                assertEquals(if (night) 1 else 2, action.parts.size)
            }
            compose.onNodeWithTag("boundary_close").performScrollTo().assertIsDisplayed()
        }
    }
}
