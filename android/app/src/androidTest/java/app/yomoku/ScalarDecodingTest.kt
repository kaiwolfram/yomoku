package app.yomoku

import android.os.Bundle
import android.os.Debug
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.security.MessageDigest
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ScalarDecodingTest {
    @Test fun scalarValuesErrorsAndOffsetsMatchLegacy() = verifyScalarDecoding()

    @Test fun generatedFrozenSnapshotsMatchLegacy() = verifyScalarSnapshots()

    @Test
    fun measureLegacyAndGenerated() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assumeTrue(
            "Opt in with -e yomokuMeasureScalar true",
            args.getString("yomokuMeasureScalar") == "true",
        )
        val size = (args.getString("yomokuVocabularyEntries") ?: "100000").toInt()
        require(size in 1..100_000)
        val ids = scalarIds(size)
        val view = frozenScalarView(ids)
        fun emit(line: String) =
            instrumentation.sendStatus(0, Bundle().apply { putString("stream", "$line\n") })
        fun allocated() = checkNotNull(Debug.getRuntimeStat("art.gc.bytes-allocated")).toLong()
        emit(
            "SCALAR_HEADER,entries,shape,iteration,order,decoder,payload_bytes,decode_ms,process_allocated_bytes"
        )
        for (snapshot in listOf(false, true)) {
            val bytes = if (snapshot) view.bincodeSerialize() else scalarListBytes(ids)
            val original = bytes.copyOf()
            val digest =
                MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") {
                    "%02x".format(it)
                }
            emit("SCALAR_PAYLOAD,${if (snapshot) "frozen" else "list"},${bytes.size},$digest")
            for (iteration in -4 until 8) {
                val results = mutableListOf<Any>()
                val samples = mutableListOf<String>()
                for (order in 0..1) {
                    val legacy = Math.floorMod(iteration + order, 2) == 0
                    lateinit var actual: Any
                    instrumentation.runOnMainSync {
                        val before = allocated()
                        val start = System.nanoTime()
                        actual =
                            if (snapshot) decodeScalarView(bytes, legacy)
                            else decodeScalarList(bytes, legacy)
                        val nanos = System.nanoTime() - start
                        val allocation = allocated() - before
                        check(nanos >= 0 && allocation >= 0)
                        samples.add(
                            "SCALAR,$size,${if (snapshot) "frozen" else "list"},$iteration,$order,${if (legacy) "legacy" else "generated"},${bytes.size},${nanos / 1e6},$allocation"
                        )
                    }
                    results.add(actual)
                }
                results.forEach { actual ->
                    if (snapshot) {
                        check(actual == view)
                        check(
                            (actual as app.yomoku.shared.ViewModel)
                                .bincodeSerialize()
                                .contentEquals(bytes)
                        )
                    } else {
                        check(actual == ids)
                        @Suppress("UNCHECKED_CAST")
                        check(
                            scalarListBytes(actual as List<app.yomoku.shared.VocabularyId>)
                                .contentEquals(bytes)
                        )
                    }
                }
                check(bytes.contentEquals(original))
                if (iteration >= 0) samples.forEach(::emit)
            }
        }
        emit("SCALAR_DONE,$size,exact_ids_and_bytes_unchanged")
    }
}
