package app.yomoku

import android.content.Context
import java.io.File

/** Pinned independently of the asset manifest. No extraction until a worker requests access. */
internal fun wordDictionaryFiles(context: Context): VerifiedDictionaryFiles {
    val application = context.applicationContext
    return VerifiedDictionaryFiles(
        File(application.noBackupFilesDir, "dictionaries"),
        WORD_DICTIONARY_SIZE,
        WORD_DICTIONARY_SHA256,
        publishedName = "index-v1-$WORD_DICTIONARY_SHA256.sqlite3",
    ) {
        application.assets.open("dictionaries/dictionary.sqlite3")
    }
}

internal const val WORD_DICTIONARY_SIZE = 279_109_632L
internal const val WORD_DICTIONARY_SHA256 =
    "0be9d4493aa3fa4474dbb27291820a63a27748dbc63e5517f67be39ffea8e152"
