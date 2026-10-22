package app.yomoku

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.ViewModelProvider
import app.yomoku.shared.Event
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val core = ViewModelProvider(this)[CoreSessionOwner::class.java].session
        setContent {
            YomokuTheme {
                BackHandler(enabled = core.view.settingsOpen) {
                    core.dispatch(Event.CloseSettings)
                }
                YomokuApp(core.view, core::dispatch)
            }
        }
    }
}
