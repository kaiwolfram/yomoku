package app.yomoku

import app.yomoku.shared.AnkiExportError
import app.yomoku.shared.AnkiRecoveryReadOperation
import app.yomoku.shared.AnkiRecoveryReadResult

/** Worker-only translation of the core's frozen comparison data to provider reads. */
internal class AnkiRecoveryReader(private val provider: AnkiProviderReader) {
    fun read(operation: AnkiRecoveryReadOperation): AnkiRecoveryReadResult =
        try {
            when (operation) {
                is AnkiRecoveryReadOperation.Note ->
                    when (val result = provider.duplicates(operation.surface)) {
                        is AnkiReadResult.Value ->
                            AnkiRecoveryReadResult.Notes(result.value.map { it.value })
                        is AnkiReadResult.Failed -> failure(result.error)
                    }
                is AnkiRecoveryReadOperation.Model -> model(operation)
            }
        } catch (_: IllegalArgumentException) {
            AnkiRecoveryReadResult.Failed(AnkiExportError.INVALIDRESPONSE)
        }

    private fun model(operation: AnkiRecoveryReadOperation.Model): AnkiRecoveryReadResult {
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
        val candidates =
            operation.knownId?.let { listOf(AnkiProviderModelId(it)) }
                ?: when (val result = provider.models()) {
                    is AnkiReadResult.Failed -> return failure(result.error)
                    is AnkiReadResult.Value ->
                        result.value.filter { it.name == frozen.name }.map { it.id }
                }
        val matches = mutableListOf<Long>()
        for (id in candidates) {
            when (val result = provider.modelMatches(id, layout)) {
                // A failed later read must discard an earlier match, never return partial evidence.
                is AnkiReadResult.Failed -> return failure(result.error)
                is AnkiReadResult.Value -> if (result.value) matches.add(id.value)
            }
        }
        return AnkiRecoveryReadResult.Model(matches.singleOrNull())
    }

    private fun failure(error: AnkiReadError): AnkiRecoveryReadResult =
        AnkiRecoveryReadResult.Failed(
            when (error) {
                AnkiReadError.ProviderUnavailable -> AnkiExportError.PROVIDERUNAVAILABLE
                AnkiReadError.UnsupportedProvider -> AnkiExportError.UNSUPPORTEDPROVIDER
                AnkiReadError.PermissionDenied -> AnkiExportError.PERMISSIONDENIED
                AnkiReadError.CollectionUnavailable -> AnkiExportError.COLLECTIONUNAVAILABLE
                AnkiReadError.DuplicateLookupFailed -> AnkiExportError.DUPLICATELOOKUPFAILED
                AnkiReadError.InvalidResponse -> AnkiExportError.INVALIDRESPONSE
                AnkiReadError.ProviderFailure -> AnkiExportError.PROVIDERFAILURE
            }
        )
}
