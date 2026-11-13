package app.yomoku

import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.onNodeWithText
import org.junit.Assert.assertTrue

internal fun assertDictionaryEntries(compose: ComposeContentTestRule) {
    compose.onNodeWithText("JMdict · English definitions").assertDoesNotExist()
    assertTrue(
        compose
            .onAllNodes(
                SemanticsMatcher("dictionary sense") {
                    it.config.getOrNull(SemanticsProperties.TestTag)?.startsWith("word_sense_") ==
                        true
                }
            )
            .fetchSemanticsNodes()
            .isNotEmpty()
    )
}
