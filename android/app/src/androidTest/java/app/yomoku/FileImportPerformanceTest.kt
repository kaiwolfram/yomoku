package app.yomoku

import android.content.ContentResolver
import android.net.Uri
import android.os.Bundle
import android.os.ParcelFileDescriptor
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.DocumentError
import app.yomoku.shared.EpubImport
import app.yomoku.shared.Event
import java.io.File
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FileImportPerformanceTest {
    @Test
    fun measureEncodingAndNativeSubmissionSeparately() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val mode = InstrumentationRegistry.getArguments().getString("yomokuMeasureImport")
        assumeTrue(mode == "main" || mode == "worker")
        val context = instrumentation.targetContext
        val directory = File(context.cacheDir, "import-transport-${System.nanoTime()}")
        check(directory.mkdir())
        val fixture = File(directory, "transport.epub")
        // Intentionally invalid EPUB. The same actual bytes exercise serialization
        // and JNI on both paths without extracting or loading analysis assets.
        val bytes = ByteArray(8 * 1024 * 1024) { it.toByte() }
        fixture.writeBytes(bytes)
        val uri = Uri.parse("content://app.yomoku.test.txt-fixtures/transport.epub")
        val resolver =
            ContentResolver.wrap(
                object : TxtFixtureProvider() {
                    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor =
                        ParcelFileDescriptor.open(fixture, ParcelFileDescriptor.MODE_READ_ONLY)
                }
            )
        lateinit var core: CoreSession
        val ready = CompletableFuture<Unit>()
        var sample: CoreRequestsMeasurement? = null
        var finished = CompletableFuture<DocumentError?>()
        var imports: FileImports? = null
        instrumentation.runOnMainSync {
            core =
                CoreSession(
                    File(directory, "storage.sqlite3").path,
                    sudachiDictionaryProvider(context),
                )
            core.onViewChanged = { if (!core.view.storageBusy) ready.complete(Unit) }
            if (!core.view.storageBusy) ready.complete(Unit)
        }
        try {
            ready.get(60, TimeUnit.SECONDS)
            instrumentation.runOnMainSync {
                core.onRequestsMeasured = {
                    if (
                        it.source == CoreRenderSource.DISPATCH &&
                            core.view.storageBusy &&
                            it.eventEncodeNanos != null
                    ) {
                        sample = it
                    }
                }
                if (mode == "worker") {
                    imports =
                        FileImports(
                            resolver,
                            core,
                            observer =
                                object : FileImportObserver {
                                    override fun update(progress: FileImportProgress) {}

                                    override fun finished(result: FileImportResult?) {
                                        finished.complete(result?.error)
                                    }

                                    override fun close() {}
                                },
                        )
                } else {
                    core.onViewChanged = {
                        if (!core.view.storageBusy && core.view.documentError != null)
                            finished.complete(core.view.documentError)
                    }
                }
            }
            val event = Event.ImportEpub(EpubImport(FileImportBytes(bytes), fixture.name))
            repeat(6) { iteration ->
                instrumentation.runOnMainSync {
                    finished = CompletableFuture()
                    sample = null
                    if (mode == "worker") checkNotNull(imports).start(uri) else core.dispatch(event)
                }
                assertEquals(DocumentError.INVALIDEPUB, finished.get(60, TimeUnit.SECONDS))
                val measured = checkNotNull(sample)
                if (iteration != 0) {
                    instrumentation.sendStatus(
                        0,
                        Bundle().apply {
                            putString(
                                "stream",
                                "IMPORT_TRANSPORT,$mode,${bytes.size},${measured.eventEncodeNanos},${measured.nativeUpdateNanos}\n",
                            )
                        },
                    )
                }
            }
        } finally {
            instrumentation.runOnMainSync {
                imports?.close()
                core.close()
                check(core.isClosed)
                directory.deleteRecursively()
            }
        }
    }
}
