package app.yomoku.tests

import app.yomoku.shared.*
import java.io.File

internal fun verifyVocabularyCodecs(directory: String) {
    val dir = File(directory)
    VocabularySort.entries.forEachIndexed { index, sort ->
        val bytes = File(dir, "vocabulary-view-$index.bin").readBytes()
        val view = ViewModel.bincodeDeserialize(bytes)
        val state = view.vocabulary
        check(state.query.sort == sort && state.query.prefix == " 𠮷e\u0301\u0000_%")
        check(state.query.statuses == listOf(VocabularyStatus.NEW, VocabularyStatus.IGNORED))
        check(state.matchingCount == ULong.MAX_VALUE)
        check(state.nextOffset == 9_007_199_254_740_993UL)
        check(state.rows.first().id.value == Long.MAX_VALUE)
        check(state.rows.first().encounteredCount == ULong.MAX_VALUE)
        check(state.rows.first().registeredAtUnixMs == Long.MAX_VALUE)
        check(state.rows.first().lastSeenUnixMs == 9_007_199_254_740_993L)
        check(state.rows.last().lastSeenUnixMs == null)
        check(view.bincodeSerialize().contentEquals(bytes))
        File(dir, "vocabulary-view-$index-return.bin").writeBytes(view.bincodeSerialize())
        val eventBytes = File(dir, "vocabulary-event-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(eventBytes)
        check(event == Event.SetVocabularyQuery(state.query))
        check(event.bincodeSerialize().contentEquals(eventBytes))
        File(dir, "vocabulary-event-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
    repeat(3) { index ->
        val bytes = File(dir, "vocabulary-state-$index.bin").readBytes()
        val view = ViewModel.bincodeDeserialize(bytes)
        check(view.vocabulary.matchingCount == null && view.vocabulary.rows.isEmpty())
        check(view.vocabulary.query.sort == VocabularySort.NEWEST)
        check(view.vocabulary.query.statuses.size == 5)
        check(view.vocabulary.loading == (index == 1))
        check(view.vocabulary.error == if (index == 2) DocumentError.INVALIDDOCUMENT else null)
        check(view.bincodeSerialize().contentEquals(bytes))
        File(dir, "vocabulary-state-$index-return.bin").writeBytes(view.bincodeSerialize())
        val actionBytes = File(dir, "vocabulary-action-$index.bin").readBytes()
        val event = Event.bincodeDeserialize(actionBytes)
        check(
            event ==
                listOf(Event.LoadVocabulary, Event.LoadMoreVocabulary, Event.RetryVocabulary)[index]
        )
        check(event.bincodeSerialize().contentEquals(actionBytes))
        File(dir, "vocabulary-action-$index-return.bin").writeBytes(event.bincodeSerialize())
    }
}
