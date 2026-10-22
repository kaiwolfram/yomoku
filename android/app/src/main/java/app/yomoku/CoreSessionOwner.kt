package app.yomoku

import androidx.lifecycle.ViewModel

/** Retains the Rust session across Activity recreation without copying domain state. */
internal class CoreSessionOwner : ViewModel() {
    val session = CoreSession()

    override fun onCleared() {
        session.close()
    }
}
