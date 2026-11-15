package app.yomoku

import androidx.compose.ui.test.junit4.v2.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import app.yomoku.shared.DocumentError
import app.yomoku.shared.ImportOutcome
import app.yomoku.shared.TextEncoding
import java.util.concurrent.atomic.AtomicLong
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FileImportAdmissionTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test
    fun rejectionIsTerminalAndReleasesAttemptBeforeNextImportAndEncodingRetry() {
        val core = ViewModelProvider(compose.activity)[CoreSessionOwner::class.java].session
        compose.waitUntil(30_000) { !core.view.storageBusy }
        val memory = AtomicLong(8192) // A deterministic 1 KiB admission budget.
        lateinit var imports: FileImports
        var previousObserver: (() -> Unit)? = null
        val results = mutableListOf<FileImportResult?>()
        compose.runOnIdle {
            previousObserver = core.onViewChanged
            imports =
                FileImports(
                    compose.activity.contentResolver,
                    core,
                    observer =
                        object : FileImportObserver {
                            override fun update(progress: FileImportProgress) {}

                            override fun finished(result: FileImportResult?) {
                                results.add(result)
                            }

                            override fun close() {}
                        },
                    availableImportMemory = memory::get,
                )
        }
        val initialCount = core.view.documents.size
        val prefix = "memory-${System.nanoTime()}"
        try {
            // All metadata variants must reject from actual bytes and leave native storage alone.
            for (reportedSize in listOf(null, "1", "1024", "999999999")) {
                val uri =
                    FileImportFixtures.uri("$prefix.epub", ByteArray(1025))
                        .buildUpon()
                        .apply {
                            if (reportedSize != null)
                                appendQueryParameter("reportedSize", reportedSize)
                        }
                        .build()
                compose.runOnIdle { imports.start(uri) }
                compose.waitUntil(30_000) { !imports.active && !core.view.storageBusy }
                assertEquals(R.string.import_file_too_large, imports.result?.message)
                assertEquals(initialCount, core.view.documents.size)
                assertNull(core.view.documentError)
            }
            assertEquals(4, results.size)
            // Acquisition fits, but transport headroom may disappear before main-thread submit.
            var memoryReads = 0
            compose.runOnIdle {
                imports.close()
                imports =
                    FileImports(
                        compose.activity.contentResolver,
                        core,
                        availableImportMemory = { if (++memoryReads == 1) 8192 else 0 },
                    )
                imports.start(FileImportFixtures.uri("$prefix.txt", "small".toByteArray()))
            }
            compose.waitUntil(30_000) { !imports.active && !core.view.storageBusy }
            assertEquals(R.string.import_file_too_large, imports.result?.message)
            assertEquals(initialCount, core.view.documents.size)
            compose.runOnIdle {
                imports.close()
                imports =
                    FileImports(
                        compose.activity.contentResolver,
                        core,
                        availableImportMemory = memory::get,
                    )
                imports.start(
                    FileImportFixtures.uri(
                        "$prefix.txt",
                        "$prefix ".toByteArray() + byteArrayOf(0xc2.toByte(), 0xa9.toByte()),
                    )
                )
            }
            compose.waitUntil(30_000) { imports.choosingEncoding }
            assertEquals(DocumentError.TXTENCODINGREQUIRED, imports.encodingError)
            compose.runOnIdle {
                memory.set(0)
                imports.selectEncoding(TextEncoding.UTF8)
            }
            assertFalse(imports.active)
            assertEquals(R.string.import_file_too_large, imports.result?.message)
            assertEquals(initialCount, core.view.documents.size)
            compose.waitUntil(30_000) { !core.view.storageBusy }
            compose.runOnIdle {
                memory.set(8192)
                imports.start(
                    FileImportFixtures.uri("$prefix-last.txt", "\uFEFF$prefix 猫。".toByteArray())
                )
            }
            compose.waitUntil(30_000) { !imports.active && !core.view.storageBusy }
            assertEquals(ImportOutcome.CREATED, imports.result?.outcome)
            assertEquals(initialCount + 1, core.view.documents.size)
        } finally {
            compose.runOnIdle {
                imports.close()
                core.onViewChanged = previousObserver
            }
        }
    }
}
