package app.yomoku

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * Retained in the Activity ViewModel across recreation, discarded on process death. ActivityResult
 * callbacks never recreate a request.
 */
internal class ImportBookRequest {
    enum class Step {
        PERMISSION,
        PICKER,
    }

    var step by mutableStateOf<Step?>(null)
        private set

    fun begin(needsPermission: Boolean): Step? {
        if (step != null) return null
        return (if (needsPermission) Step.PERMISSION else Step.PICKER).also { step = it }
    }

    fun permissionResult(): Boolean {
        if (step != Step.PERMISSION) return false
        step = Step.PICKER
        return true
    }

    fun pickerResult(): Boolean {
        if (step != Step.PICKER) return false
        step = null
        return true
    }
}
