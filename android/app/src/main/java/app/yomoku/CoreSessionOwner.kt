package app.yomoku

import android.app.Application
import android.app.NotificationManager
import androidx.lifecycle.AndroidViewModel

/** Retains the Rust session across Activity recreation without copying domain state. */
internal class CoreSessionOwner(application: Application) : AndroidViewModel(application) {
    var activityVisible = false
        private set

    fun setForeground(visible: Boolean) {
        activityVisible = visible
        session.setForeground(visible)
    }

    val importBookRequest = ImportBookRequest()
    val session =
        CoreSession(
            java.io.File(application.filesDir, "yomoku.sqlite3").absolutePath,
            sudachiDictionaryProvider(application),
            readAnkiPreflight = AnkiPreflightReader(AnkiProviderReader(application))::read,
            invokeAnkiLive = AnkiLiveDispatcher(application)::invoke,
            needsAnkiPermission = {
                val environment = AndroidAnkiProviderEnvironment(application)
                environment.provider() != null && !environment.hasPermission()
            },
            readAnkiDecks = AnkiProviderReader(application)::decks,
            resolveAnkiNotes = AnkiProviderReader(application)::existingNotesIntent,
            resolveAnkiNote = AnkiProviderReader(application)::existingNoteIntent,
        )
    val fileImports: FileImports =
        FileImports(
            application.contentResolver,
            session,
            ImportNotifications(application, isActivityVisible = { activityVisible }) {
                fileImports.cancel()
            },
            notificationsAvailable = {
                val manager = application.getSystemService(NotificationManager::class.java)
                manager.areNotificationsEnabled() &&
                    manager.getNotificationChannel(ImportNotifications.CHANNEL)?.importance !=
                        NotificationManager.IMPORTANCE_NONE
            },
        )

    override fun onCleared() {
        fileImports.close()
        session.close()
    }
}
