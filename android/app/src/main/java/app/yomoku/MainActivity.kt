package app.yomoku

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.lifecycle.ViewModelProvider
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    private lateinit var core: CoreSession
    private lateinit var fileImports: FileImports
    private val filePicker =
        registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri != null) fileImports.start(uri)
        }
    private val ankiPermissionRequest =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            core.ankiPermissionResult(granted)
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
                    enabled = core.view.settingsOpen || core.view.contentRoute != ContentRoute.MAIN
                ) {
                    core.dispatch(
                        if (core.view.settingsOpen) Event.CloseSettings else Event.CloseDocument
                    )
                }
                YomokuApp(
                    core.view,
                    core.sessionKey,
                    core::dispatch,
                    core.images,
                    core.covers,
                    libraryActionsEnabled = !fileImports.active,
                    importActive = fileImports.active,
                    ankiDecks = core.ankiDecks.state,
                    ankiBrowser = core.ankiBrowser,
                    checkAnkiProvider = {
                        core.configureAnki {
                            ankiPermissionRequest.launch(AnkiProviderReader.PERMISSION)
                        }
                    },
                ) {
                    // Providers frequently label SRT as text/plain or application/octet-stream.
                    // Validate the literal display-name extension in FileImports.
                    filePicker.launch(arrayOf("*/*"))
                }
                FileImportDialogs(fileImports)
            }
        }
    }

    override fun onStart() {
        super.onStart()
        core.setForeground(true)
        fileImports.refreshNotifications()
    }

    override fun onResume() {
        super.onResume()
        core.ankiBrowser.attach { startActivity(it) }
        core.refreshAnkiSetup()
    }

    override fun onPause() {
        core.ankiBrowser.detach()
        super.onPause()
    }

    override fun onStop() {
        // Configuration changes retain the worker and its warm analyzer.
        if (!isChangingConfigurations) core.setForeground(false)
        super.onStop()
    }
}
