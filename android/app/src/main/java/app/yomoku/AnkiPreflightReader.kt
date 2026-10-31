package app.yomoku

import app.yomoku.shared.AnkiExportError
import app.yomoku.shared.AnkiPreflightDeck
import app.yomoku.shared.AnkiPreflightModel
import app.yomoku.shared.AnkiPreflightObservation
import app.yomoku.shared.AnkiPreflightOperation
import app.yomoku.shared.AnkiPreflightResult

/** Complete read-only evidence for a native frozen export. Never returns partial success. */
internal class AnkiPreflightReader(private val provider: AnkiProviderReader) {
    fun read(operation: AnkiPreflightOperation): AnkiPreflightResult {
        return try {
            require(operation.deckId > 0)
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
            val decks =
                when (val result = provider.decks()) {
                    is AnkiReadResult.Failed -> return failure(result.error)
                    is AnkiReadResult.Value ->
                        result.value.map { AnkiPreflightDeck(it.id, it.filtered) }
                }
            val notes =
                when (val result = provider.duplicates(operation.surface)) {
                    is AnkiReadResult.Failed -> return failure(result.error)
                    is AnkiReadResult.Value -> result.value.map { it.value }
                }
            val discovered =
                when (val result = provider.models()) {
                    is AnkiReadResult.Failed -> return failure(result.error)
                    is AnkiReadResult.Value -> result.value
                }
            val models = mutableListOf<AnkiPreflightModel>()
            for (model in discovered) {
                val matches =
                    if (model.name == frozen.name) {
                        when (val result = provider.modelMatches(model.id, layout)) {
                            is AnkiReadResult.Failed -> return failure(result.error)
                            is AnkiReadResult.Value -> result.value
                        }
                    } else false
                models.add(AnkiPreflightModel(model.id.value, model.name, matches))
            }
            AnkiPreflightResult.Complete(AnkiPreflightObservation(decks, notes, models))
        } catch (_: IllegalArgumentException) {
            AnkiPreflightResult.Failed(AnkiExportError.INVALIDRESPONSE)
        }
    }

    private fun failure(error: AnkiReadError): AnkiPreflightResult =
        AnkiPreflightResult.Failed(error.exportError())
}
