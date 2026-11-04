package app.yomoku

import android.content.ContentResolver
import android.content.Context
import app.yomoku.shared.AnkiLiveOperation
import app.yomoku.shared.AnkiLiveResult
import app.yomoku.shared.AnkiPreflightOperation
import app.yomoku.shared.AnkiPreflightResult

/** Only called by the retained session for an issued, durably committed native invocation. */
internal class AnkiLiveDispatcher(resolver: ContentResolver, environment: AnkiProviderEnvironment) {
    constructor(
        context: Context
    ) : this(context.applicationContext.contentResolver, AndroidAnkiProviderEnvironment(context))

    private val provider = AnkiProviderReader(resolver, environment)
    private val reader = AnkiPreflightReader(provider)
    private val notes = AnkiProviderNoteInserter(resolver, environment)
    private val models = AnkiProviderModelCreator(resolver, environment)

    fun invoke(operation: AnkiLiveOperation): AnkiLiveResult {
        // Each adapter classifies its own failures before/after the first mutating call.
        // Unexpected exceptions escape to CoreSession and remain uncertain.
        return when (operation) {
            is AnkiLiveOperation.InsertNote -> {
                val frozen = operation.layout
                val layout =
                    AnkiProviderModelLayout(
                        frozen.name,
                        frozen.fieldNames,
                        frozen.css,
                        frozen.cardName,
                        frozen.questionFormat,
                        frozen.answerFormat,
                    )
                val request =
                    AnkiProviderNoteRequest(
                        operation.surface,
                        operation.fields,
                        AnkiProviderModelId(operation.modelId),
                        operation.deckId,
                        layout,
                    )
                when (val result = notes.insert(request)) {
                    is AnkiNoteInsertionResult.Inserted -> AnkiLiveResult.NoteAdded(result.id.value)
                    is AnkiNoteInsertionResult.Existing ->
                        AnkiLiveResult.NoteExisting(result.ids.map { it.value })
                    is AnkiNoteInsertionResult.Failed -> AnkiLiveResult.NoSubmission
                    is AnkiNoteInsertionResult.Uncertain ->
                        AnkiLiveResult.NoteUncertain(result.id?.value)
                }
            }
            is AnkiLiveOperation.CreateModel -> {
                // Refresh the frozen deck and exact duplicates as well as model conflicts.
                val observation =
                    reader.read(
                        AnkiPreflightOperation(
                            operation.surface,
                            operation.deckId,
                            operation.layout,
                        )
                    )
                if (
                    observation !is AnkiPreflightResult.Complete ||
                        observation.value.decks.none {
                            it.id == operation.deckId && !it.filtered
                        } ||
                        observation.value.duplicateNoteIds.isNotEmpty() ||
                        observation.value.models.any { it.name == operation.layout.name }
                ) {
                    return AnkiLiveResult.NoSubmission
                }
                val frozen = operation.layout
                val layout =
                    AnkiProviderModelLayout(
                        frozen.name,
                        frozen.fieldNames,
                        frozen.css,
                        frozen.cardName,
                        frozen.questionFormat,
                        frozen.answerFormat,
                    )
                when (
                    val result =
                        models.create(layout) {
                            val decks = provider.decks()
                            val duplicates =
                                if (operation.surface.isEmpty()) AnkiReadResult.Value(emptyList())
                                else provider.duplicates(operation.surface)
                            decks is AnkiReadResult.Value &&
                                decks.value.any { it.id == operation.deckId && !it.filtered } &&
                                duplicates is AnkiReadResult.Value &&
                                duplicates.value.isEmpty()
                        }
                ) {
                    is AnkiModelCreationResult.Created ->
                        AnkiLiveResult.ModelCreated(result.id.value)
                    is AnkiModelCreationResult.Failed -> AnkiLiveResult.NoSubmission
                    is AnkiModelCreationResult.Uncertain ->
                        AnkiLiveResult.ModelUncertain(result.id?.value)
                }
            }
        }
    }
}
