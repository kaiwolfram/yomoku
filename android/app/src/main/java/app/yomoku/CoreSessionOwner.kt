package app.yomoku

import android.app.Application
import android.app.NotificationManager
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import java.io.File
import java.nio.file.Files

/** Retains the Rust session across Activity recreation without copying domain state. */
internal class CoreSessionOwner
private constructor(
    application: Application,
    isolatedDirectory: File?,
) : AndroidViewModel(application) {
    companion object {
        const val ISOLATED_DATABASE = "app.yomoku.test.ISOLATED_DATABASE"

        fun factory(application: Application, isolated: Boolean) = viewModelFactory {
            initializer {
                val directory =
                    if (isolated)
                        Files.createTempDirectory(application.cacheDir.toPath(), "yomoku-test-")
                            .toFile()
                    else null
                CoreSessionOwner(application, directory)
            }
        }
    }

    var activityVisible = false
        private set

    fun setForeground(visible: Boolean) {
        activityVisible = visible
        session.setForeground(visible)
    }

    val importBookRequest = ImportBookRequest()
    val session =
        CoreSession(
            File(isolatedDirectory ?: application.filesDir, "yomoku.sqlite3").absolutePath,
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
            onClosed = { isolatedDirectory?.deleteRecursively() },
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
