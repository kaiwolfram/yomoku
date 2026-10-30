package app.yomoku

import app.yomoku.shared.VocabularyId

/** Core retains unique IDs in ascending order, independently of the browser's row sort. */
internal fun List<VocabularyId>.containsSortedVocabularyId(id: VocabularyId): Boolean =
    binarySearch { it.value.compareTo(id.value) } >= 0
