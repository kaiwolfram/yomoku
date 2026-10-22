package app.yomoku

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import app.yomoku.shared.Destination
import app.yomoku.shared.Event
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    private lateinit var core: CoreSession

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        core = CoreSession()
        core.dispatch(Event.SelectDestination(core.view.destination))
        setContent { YomokuTheme { YomokuApp(core.view.destination) } }
    }

    override fun onDestroy() {
        core.close()
        super.onDestroy()
    }
}

@Composable
private fun YomokuApp(destination: Destination) {
    Scaffold(modifier = Modifier.fillMaxSize()) { insets ->
        Text(
            text = stringResource(destination.titleResource()),
            modifier = Modifier.padding(insets).padding(16.dp),
        )
    }
}

private fun Destination.titleResource(): Int =
    when (this) {
        Destination.LIBRARY -> R.string.library_title
        Destination.NOSTR -> R.string.nostr_title
        Destination.STATISTICS -> R.string.statistics_title
        Destination.VOCABULARY -> R.string.vocabulary_title
    }

@Preview
@Composable
private fun YomokuPreview() {
    YomokuTheme { YomokuApp(Destination.LIBRARY) }
}
