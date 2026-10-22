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
import app.yomoku.theme.YomokuTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { YomokuTheme { YomokuApp() } }
    }
}

// Initial shell only. Destination state will come from the Rust bridge.
@Composable
private fun YomokuApp() {
    Scaffold(modifier = Modifier.fillMaxSize()) { insets ->
        Text(
            text = stringResource(R.string.library_title),
            modifier = Modifier.padding(insets).padding(16.dp),
        )
    }
}

@Preview
@Composable
private fun YomokuPreview() {
    YomokuTheme { YomokuApp() }
}
