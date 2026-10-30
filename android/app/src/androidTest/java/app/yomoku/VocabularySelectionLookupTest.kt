package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.VocabularyId
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class VocabularySelectionLookupTest {
    @Test
    fun membershipUsesExactIdsAcrossGapsAndSignedIdentityRange() {
        val ids = listOf(1L, 7L, 9_007_199_254_740_993L, Long.MAX_VALUE).map(::VocabularyId)
        // Browser rows may arrive in any sort order; selection remains ordered by ID.
        for (id in ids.reversed()) assertTrue(ids.containsSortedVocabularyId(id))
        for (id in listOf(2L, 6L, 8L, 9_007_199_254_740_992L, Long.MAX_VALUE - 1)) assertFalse(
            ids.containsSortedVocabularyId(VocabularyId(id))
        )
        assertFalse(emptyList<VocabularyId>().containsSortedVocabularyId(VocabularyId(1)))
        assertTrue(listOf(VocabularyId(1)).containsSortedVocabularyId(VocabularyId(1)))
        assertFalse(listOf(VocabularyId(1)).containsSortedVocabularyId(VocabularyId(2)))
    }

    @Test
    fun hundredThousandSelectionNeedsOnlyLogarithmicReadsPerComposedRow() {
        var reads = 0
        val ids =
            object : AbstractList<VocabularyId>() {
                override val size = 100_000

                override fun get(index: Int): VocabularyId {
                    check(index in indices)
                    reads++
                    return VocabularyId(index * 2L + 1)
                }
            }
        for (id in listOf(1L, 99_999L, 199_999L, 2L, 100_000L, 200_000L)) {
            reads = 0
            assertEquals(id % 2L == 1L, ids.containsSortedVocabularyId(VocabularyId(id)))
            assertTrue("ID $id read $reads entries", reads in 1..17)
        }
    }
}
