package app.yomoku

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.ViewModelProvider
import app.yomoku.shared.ContentRoute
import app.yomoku.shared.Event
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    private lateinit var core: CoreSession

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        core = ViewModelProvider(this)[CoreSessionOwner::class.java].session
        setContent {
            YomokuTheme {
                BackHandler(
                    enabled = core.view.settingsOpen || core.view.contentRoute != ContentRoute.MAIN
                ) {
                    core.dispatch(
                        if (core.view.settingsOpen) Event.CloseSettings else Event.CloseDocument
                    )
                }
                YomokuApp(core.view, core::dispatch)
            }
        }
    }

    override fun onStart() {
        super.onStart()
        core.setForeground(true)
    }

    override fun onStop() {
        // Configuration changes retain the worker and its warm analyzer.
        if (!isChangingConfigurations) core.setForeground(false)
        super.onStop()
    }
}
