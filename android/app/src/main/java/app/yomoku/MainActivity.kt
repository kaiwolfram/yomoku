package app.yomoku

import android.os.Bundle
import android.view.MotionEvent
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
    private lateinit var txtImports: TxtImports
    private val txtPicker =
        registerForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
            txtImports.start(uris)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val owner = ViewModelProvider(this)[CoreSessionOwner::class.java]
        core = owner.session
        txtImports = owner.txtImports
        setContent {
            YomokuTheme {
                BackHandler(
                    enabled = core.view.settingsOpen || core.view.contentRoute != ContentRoute.MAIN
                ) {
                    core.dispatch(
                        if (core.view.settingsOpen) Event.CloseSettings else Event.CloseDocument
                    )
                }
                YomokuApp(core.view, core::dispatch, core::onReaderInteraction) {
                    txtPicker.launch(arrayOf("text/plain"))
                }
                TxtImportDialogs(txtImports)
            }
        }
    }

    override fun onStart() {
        super.onStart()
        core.setForeground(true)
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
