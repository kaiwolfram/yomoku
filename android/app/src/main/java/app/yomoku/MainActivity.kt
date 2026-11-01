package app.yomoku

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.MotionEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModelProvider
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    private lateinit var core: CoreSession
    private lateinit var fileImports: FileImports
    private var notificationPermission by mutableStateOf(true)
    private val notificationRequest =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            notificationPermission = granted
            fileImports.refreshNotifications()
        }
    private val filePicker =
        registerForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            fileImports.start(uris)
        }
    private val ankiPermissionRequest =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            if (granted) core.checkAnkiDecks()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val owner = ViewModelProvider(this)[CoreSessionOwner::class.java]
        core = owner.session
        fileImports = owner.fileImports
        setContent {
            YomokuTheme {
                BackHandler(
                    enabled =
                        core.view.settingsOpen ||
                            core.view.contentRoute != ContentRoute.MAIN ||
                            core.view.librarySelection != null
                ) {
                    core.dispatch(
                        when {
                            core.view.settingsOpen -> Event.CloseSettings
                            core.view.contentRoute != ContentRoute.MAIN -> Event.CloseDocument
                            else -> Event.CancelLibrarySelection
                        }
                    )
                }
                YomokuApp(
                    core.view,
                    core.sessionKey,
                    core::dispatch,
                    core::onReaderInteraction,
                    core.images,
                    core.covers,
                    libraryActionsEnabled = !fileImports.active,
                    ankiDecks = core.ankiDecks.state,
                    checkAnkiProvider = core::checkAnkiDecks,
                    requestAnkiPermission = {
                        if (
                            core.view.settingsOpen &&
                                core.ankiDecks.state ==
                                    AnkiDeckDiscoveryState.Failed(AnkiReadError.PermissionDenied)
                        ) {
                            ankiPermissionRequest.launch(AnkiProviderReader.PERMISSION)
                        }
                    },
                ) {
                    // Providers frequently label SRT as text/plain or application/octet-stream.
                    // Validate the literal display-name extension in FileImports.
                    filePicker.launch(arrayOf("*/*"))
                }
                FileImportDialogs(fileImports, !notificationPermission) {
                    if (Build.VERSION.SDK_INT >= 33)
                        notificationRequest.launch(Manifest.permission.POST_NOTIFICATIONS)
                }
            }
        }
    }

    override fun onStart() {
        super.onStart()
        core.setForeground(true)
        notificationPermission =
            Build.VERSION.SDK_INT < 33 ||
                checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) ==
                    PackageManager.PERMISSION_GRANTED
        fileImports.refreshNotifications()
    }

    override fun onResume() {
        super.onResume()
        core.setReaderActive(true)
    }

    override fun onPause() {
        core.setReaderActive(false)
        super.onPause()
    }

    override fun onUserInteraction() {
        core.onReaderInteraction()
        super.onUserInteraction()
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        // Activity.onUserInteraction covers down/key/trackball; include sustained scrolling.
        if (event.actionMasked == MotionEvent.ACTION_MOVE) core.onReaderInteraction()
        return super.dispatchTouchEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_SCROLL) core.onReaderInteraction()
        return super.dispatchGenericMotionEvent(event)
    }

    override fun onStop() {
        // Configuration changes retain the worker and its warm analyzer.
        if (!isChangingConfigurations) core.setForeground(false)
        super.onStop()
    }
}
