package app.yomoku

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ImportBookRequestTest {
    @Test
    fun duplicateAndStaleResultsCannotLaunchWorkOrRestoreIntentAfterProcessDeath() {
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            val request = ImportBookRequest()
            assertFalse(request.permissionResult())
            assertFalse(request.pickerResult())
            assertEquals(ImportBookRequest.Step.PERMISSION, request.begin(true))
            assertNull(request.begin(true))
            assertFalse(request.pickerResult())
            assertTrue(request.permissionResult())
            assertFalse(request.permissionResult())
            assertNull(request.begin(false))
            assertTrue(request.pickerResult())
            assertFalse(request.pickerResult())
            // Granted permission and pre-33 devices go straight to the picker.
            assertEquals(ImportBookRequest.Step.PICKER, request.begin(false))
            assertFalse(request.permissionResult())
            // A fresh process receives no retained intent from saved state.
            val freshProcess = ImportBookRequest()
            assertFalse(freshProcess.permissionResult())
            assertFalse(freshProcess.pickerResult())
            assertNull(freshProcess.step)
        }
    }
}
