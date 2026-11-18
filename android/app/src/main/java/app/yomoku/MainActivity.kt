package app.yomoku

import android.Manifest
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.os.Build
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
    private lateinit var owner: CoreSessionOwner
    private lateinit var core: CoreSession
    private lateinit var fileImports: FileImports
    private lateinit var importBookRequest: ImportBookRequest
    private val filePicker =
        registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (importBookRequest.pickerResult() && uri != null) fileImports.start(uri)
        }
    private val notificationPermissionRequest =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {
            // Grant, denial and an unavailable prompt all continue to the picker.
            if (importBookRequest.permissionResult()) openBookPicker()
        }
    private val ankiPermissionRequest =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            core.ankiPermissionResult(granted)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val isolated =
            (applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0 &&
                intent.getBooleanExtra(CoreSessionOwner.ISOLATED_DATABASE, false)
        owner =
            ViewModelProvider(this, CoreSessionOwner.factory(application, isolated))[
                CoreSessionOwner::class.java]
        core = owner.session
        fileImports = owner.fileImports
        importBookRequest = owner.importBookRequest
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
                    libraryActionsEnabled = !fileImports.active && importBookRequest.step == null,
                    importActive = fileImports.active,
                    ankiDecks = core.ankiDecks.state,
                    ankiBrowser = core.ankiBrowser,
                    checkAnkiProvider = {
                        core.configureAnki {
                            ankiPermissionRequest.launch(AnkiProviderReader.PERMISSION)
                        }
                    },
                ) {
                    when (importBookRequest.begin(needsNotificationPermission())) {
                        ImportBookRequest.Step.PERMISSION -> {
                            if (Build.VERSION.SDK_INT >= 33) {
                                notificationPermissionRequest.launch(
                                    Manifest.permission.POST_NOTIFICATIONS
                                )
                            }
                        }
                        ImportBookRequest.Step.PICKER -> openBookPicker()
                        null -> Unit
                    }
                }
                FileImportDialogs(fileImports)
            }
        }
    }

    private fun needsNotificationPermission(): Boolean =
        Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) !=
                PackageManager.PERMISSION_GRANTED

    private fun openBookPicker() {
        fileImports.refreshNotifications()
        // Providers frequently label SRT as text/plain or application/octet-stream.
        // Validate the literal display-name extension in FileImports.
        filePicker.launch(arrayOf("*/*"))
    }

    override fun onStart() {
        super.onStart()
        owner.setForeground(true)
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
        if (!isChangingConfigurations) owner.setForeground(false)
        super.onStop()
    }
}
