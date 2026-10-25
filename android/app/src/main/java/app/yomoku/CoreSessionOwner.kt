package app.yomoku

import android.app.Application
import androidx.lifecycle.AndroidViewModel

/** Retains the Rust session across Activity recreation without copying domain state. */
internal class CoreSessionOwner(application: Application) : AndroidViewModel(application) {
    val session =
        CoreSession(
            java.io.File(application.filesDir, "yomoku.sqlite3").absolutePath,
            sudachiDictionaryProvider(application),
        )
    val txtImports: TxtImports =
        TxtImports(
            application.contentResolver,
            session,
            ImportNotifications(application) { txtImports.cancel() },
        )

    override fun onCleared() {
        txtImports.close()
        session.close()
    }
}
