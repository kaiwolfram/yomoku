package app.yomoku

import android.content.Intent
import androidx.lifecycle.ViewModelProvider
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import org.junit.rules.ExternalResource

internal class IsolatedMainActivity : ExternalResource() {
    private lateinit var scenario: ActivityScenario<MainActivity>
    lateinit var activity: MainActivity
        private set

    lateinit var core: CoreSession
        private set

    override fun before() {
        scenario =
            ActivityScenario.launch(
                Intent(ApplicationProvider.getApplicationContext(), MainActivity::class.java)
                    .putExtra(CoreSessionOwner.ISOLATED_DATABASE, true)
            )
        updateActivity()
    }

    fun recreate() {
        scenario.recreate()
        updateActivity()
    }

    private fun updateActivity() {
        scenario.onActivity {
            activity = it
            core = ViewModelProvider(it)[CoreSessionOwner::class.java].session
        }
    }

    override fun after() = scenario.close()
}
