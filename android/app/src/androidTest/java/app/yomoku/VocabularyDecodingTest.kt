package app.yomoku

import android.os.Bundle
import android.os.Debug
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.VocabularyId
import app.yomoku.shared.deserializeListOf
import app.yomoku.shared.serialize
import com.novi.bincode.BincodeDeserializer
import com.novi.bincode.BincodeSerializer
import com.novi.serde.DeserializationError
import com.novi.serde.Deserializer
import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Isolated ID-list experiments. Candidates are test-only, never production snapshot decoders. */
@RunWith(AndroidJUnit4::class)
class VocabularyDecodingTest {
    private enum class Decoder(val buffered: Boolean, val capacity: Boolean) {
        GENERATED(false, false),
        CAPACITY(false, true),
        BUFFERED(true, false),
        BUFFERED_CAPACITY(true, true),
    }

    private fun ids(size: Int): List<VocabularyId> =
        List(size) { index ->
            VocabularyId(
                when (index) {
                    size - 1 -> Long.MAX_VALUE
                    size - 2 -> 9_007_199_254_740_993L
                    else -> index * 7L + 1
                }
            )
        }

    private fun encode(ids: List<VocabularyId>): ByteArray =
        BincodeSerializer().apply { ids.serialize(this) { it.serialize(this) } }.get_bytes()

    // Match the generated higher-order helper, adding only validated initial capacity.
    private fun <T> Deserializer.deserializeListWithCapacity(
        payloadBytes: Int,
        deserializeElement: (Deserializer) -> T,
    ): List<T> {
        val length = deserialize_len()
        if (length > (payloadBytes - get_buffer_offset()) / Long.SIZE_BYTES) {
            throw DeserializationError("Truncated ID list")
        }
        val list = ArrayList<T>(length.toInt())
        repeat(length.toInt()) { list.add(deserializeElement(this)) }
        return list
    }

    private fun decode(bytes: ByteArray, decoder: Decoder): List<VocabularyId> {
        if (decoder.buffered) {
            val input = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
            if (input.remaining() < Long.SIZE_BYTES) throw DeserializationError("Missing length")
            val length = input.long
            // Validate the complete fixed-width payload before using its length for capacity.
            if (
                length < 0 ||
                    length > Int.MAX_VALUE ||
                    length != input.remaining() / 8L ||
                    input.remaining() % 8 != 0
            ) {
                throw DeserializationError("Incorrect ID list length")
            }
            val result =
                if (decoder.capacity) ArrayList<VocabularyId>(length.toInt())
                else ArrayList<VocabularyId>()
            repeat(length.toInt()) { result.add(VocabularyId(input.long)) }
            return result
        }
        val input = BincodeDeserializer(bytes)
        val result =
            if (decoder.capacity) {
                input.deserializeListWithCapacity(bytes.size) { VocabularyId.deserialize(it) }
            } else {
                // The exact generated helper and element decoder used for selection/confirmation.
                input.deserializeListOf { VocabularyId.deserialize(it) }
            }
        if (input.get_buffer_offset() != bytes.size) throw DeserializationError("Trailing bytes")
        return result
    }

    private fun verify(expected: List<VocabularyId>, bytes: ByteArray, actual: List<VocabularyId>) {
        assertEquals(expected, actual)
        assertArrayEquals(bytes, encode(actual))
    }

    @Test
    fun candidatesPreserveEveryIdAndGeneratedBytes() {
        for (size in listOf(0, 1, 2, 100, 10_000, 100_000)) {
            val expected = ids(size)
            val bytes = encode(expected)
            for (decoder in Decoder.entries) verify(expected, bytes, decode(bytes, decoder))
        }
        // Wire precision includes the signed range; domain validity remains Rust's responsibility.
        val signed = listOf(Long.MIN_VALUE, -1, 0, 1, Long.MAX_VALUE).map(::VocabularyId)
        val bytes = encode(signed)
        for (decoder in Decoder.entries) verify(signed, bytes, decode(bytes, decoder))
    }

    @Test
    fun candidatesRejectMalformedLengthsTruncationAndTrailingBytes() {
        val valid = encode(ids(3))
        val malformed = mutableListOf<ByteArray>()
        repeat(valid.size) { malformed.add(valid.copyOf(it)) }
        malformed.add(valid + byteArrayOf(0))
        for (length in listOf(-1L, Long.MIN_VALUE, Long.MAX_VALUE, Int.MAX_VALUE.toLong(), 4L)) {
            malformed.add(
                valid.copyOf().apply {
                    ByteBuffer.wrap(this).order(ByteOrder.LITTLE_ENDIAN).putLong(length)
                }
            )
        }
        for (decoder in Decoder.entries) {
            for (bytes in malformed) assertThrows(DeserializationError::class.java) {
                decode(bytes, decoder)
            }
        }
    }

    private fun stat(name: String): Long =
        checkNotNull(Debug.getRuntimeStat("art.gc.$name")) { "Missing ART statistic: $name" }
            .toLong()

    private data class Sample(
        val decoder: Decoder,
        val nanos: Long,
        val allocated: Long,
        val collections: Long,
        val gcMs: Long,
    )

    @Test
    fun measureIdenticalIdPayloads() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assumeTrue(
            "Opt in with -e yomokuMeasureDecode true",
            args.getString("yomokuMeasureDecode") == "true",
        )
        val size = (args.getString("yomokuVocabularyEntries") ?: "100000").toInt()
        val iterations = (args.getString("yomokuMeasureIterations") ?: "8").toInt()
        require(size in 1..100_000)
        require(iterations in 8..100 && iterations % 8 == 0)
        val expected = ids(size)
        val bytes = encode(expected)
        val original = bytes.copyOf()
        fun emit(line: String) {
            instrumentation.sendStatus(0, Bundle().apply { putString("stream", "$line\n") })
        }
        emit(
            "DECODE_HEADER,entries,iteration,order,decoder,payload_bytes,decode_ms,process_allocated_bytes,process_gc_count,process_gc_ms"
        )
        // Four full rounds warm every position. Timed rounds balance forward/reverse orders.
        for (iteration in -4 until iterations) {
            val rotation = Math.floorMod(iteration, Decoder.entries.size)
            val ordered =
                List(Decoder.entries.size) {
                        Decoder.entries[(rotation + it) % Decoder.entries.size]
                    }
                    .let { if (iteration >= 0 && iteration % 8 >= 4) it.reversed() else it }
            val samples = mutableListOf<Sample>()
            val results = mutableListOf<List<VocabularyId>>()
            // Assertions, re-encoding, status output and scheduling are outside timed intervals.
            // Each candidate consumes the same immutable bytes on the production render thread.
            for (decoder in ordered) {
                lateinit var actual: List<VocabularyId>
                instrumentation.runOnMainSync {
                    val gcBefore = stat("gc-count")
                    val gcMsBefore = stat("gc-time")
                    val allocatedBefore = stat("bytes-allocated")
                    val start = System.nanoTime()
                    actual = decode(bytes, decoder)
                    val elapsed = System.nanoTime() - start
                    val allocated = stat("bytes-allocated") - allocatedBefore
                    val gcMs = stat("gc-time") - gcMsBefore
                    val collections = stat("gc-count") - gcBefore
                    samples.add(Sample(decoder, elapsed, allocated, collections, gcMs))
                }
                results.add(actual)
            }
            // Retain the four results until the round ends; balance their live-heap effect by
            // rotating order. Re-encoding between candidates would contaminate the next sample.
            results.forEach { verify(expected, bytes, it) }
            assertArrayEquals(original, bytes)
            if (iteration >= 0)
                samples.forEachIndexed { order, sample ->
                    assertTrue(
                        sample.nanos >= 0 &&
                            sample.allocated >= 0 &&
                            sample.collections >= 0 &&
                            sample.gcMs >= 0
                    )
                    emit(
                        "DECODE,$size,$iteration,$order,${sample.decoder.name.lowercase()},${bytes.size},${sample.nanos / 1e6},${sample.allocated},${sample.collections},${sample.gcMs}"
                    )
                }
        }
        emit("DECODE_DONE,$size,$iterations,exact_ids_and_bytes_unchanged")
    }
}
