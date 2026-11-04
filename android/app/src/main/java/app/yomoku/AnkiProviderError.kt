package app.yomoku

import app.yomoku.shared.AnkiExportError

internal fun AnkiReadError.exportError(): AnkiExportError =
    when (this) {
        AnkiReadError.ProviderUnavailable -> AnkiExportError.PROVIDERUNAVAILABLE
        AnkiReadError.UnsupportedProvider -> AnkiExportError.UNSUPPORTEDPROVIDER
        AnkiReadError.PermissionDenied -> AnkiExportError.PERMISSIONDENIED
        AnkiReadError.CollectionUnavailable -> AnkiExportError.COLLECTIONUNAVAILABLE
        AnkiReadError.DuplicateLookupFailed -> AnkiExportError.DUPLICATELOOKUPFAILED
        AnkiReadError.InvalidResponse -> AnkiExportError.INVALIDRESPONSE
        AnkiReadError.ProviderFailure -> AnkiExportError.PROVIDERFAILURE
    }

internal fun AnkiExportError.messageResource(): Int =
    when (this) {
        AnkiExportError.PROVIDERUNAVAILABLE -> R.string.anki_error_unavailable
        AnkiExportError.UNSUPPORTEDPROVIDER -> R.string.anki_error_unsupported
        AnkiExportError.PERMISSIONDENIED -> R.string.anki_error_permission
        AnkiExportError.COLLECTIONUNAVAILABLE -> R.string.anki_error_collection
        AnkiExportError.DECKUNAVAILABLE -> R.string.anki_error_deck
        AnkiExportError.INCOMPATIBLEMODEL -> R.string.anki_error_model
        AnkiExportError.DUPLICATELOOKUPFAILED -> R.string.anki_error_duplicates
        AnkiExportError.PROVIDERFAILURE -> R.string.anki_error_provider
        AnkiExportError.INVALIDRESPONSE -> R.string.anki_error_response
    }
