package app.yomoku

import android.app.Application
import androidx.lifecycle.AndroidViewModel

/** Retains the Rust session across Activity recreation without copying domain state. */
internal class CoreSessionOwner(application: Application) : AndroidViewModel(application) {
    val session =
        CoreSession(
            java.io.File(application.filesDir, "yomoku.sqlite3").absolutePath,
            sudachiDictionaryProvider(application),
            readAnkiRecovery = AnkiRecoveryReader(AnkiProviderReader(application))::read,
            readAnkiPreflight = AnkiPreflightReader(AnkiProviderReader(application))::read,
        )
    val fileImports: FileImports =
        FileImports(
            application.contentResolver,
            session,
            ImportNotifications(application) { fileImports.cancel() },
        )

    override fun onCleared() {
        fileImports.close()
        session.close()
    }
}
