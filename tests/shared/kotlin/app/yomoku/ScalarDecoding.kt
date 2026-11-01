package app.yomoku

import app.yomoku.shared.*
import com.novi.bincode.BincodeDeserializer
import com.novi.bincode.BincodeSerializer
import com.novi.serde.*
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.random.Random

/** Test oracle: the pinned runtime's scalar loops; all other decoding stays generated. */
internal class LegacyScalarDeserializer(input: ByteArray) :
    BinaryDeserializer(input, Long.MAX_VALUE) {
    override fun deserialize_u64(): ULong {
        var value = 0uL
        for (shift in 0 until 64 step 8) {
            val byteValue = getByte().toULong() and 0xffuL
            value = value or (byteValue shl shift)
        }
        return value
    }

    private fun legacyLong(): Long {
        var value = 0L
        for (shift in 0 until 64 step 8) {
            val byteValue = getByte().toLong() and 0xffL
            value = value or (byteValue shl shift)
        }
        return value
    }

    override fun deserialize_f32(): Float = Float.fromBits(getInt())

    override fun deserialize_f64(): Double = Double.fromBits(legacyLong())

    override fun deserialize_len(): Long {
        val value = legacyLong()
        if (value < 0 || value > Int.MAX_VALUE.toLong()) {
            throw DeserializationError("Incorrect length value")
        }
        return value
    }

    override fun deserialize_variant_index(): Int = getInt()

    override fun check_that_key_slices_are_increasing(key1: Slice, key2: Slice) = Unit
}

internal fun scalarDecoder(bytes: ByteArray, legacy: Boolean): Deserializer =
    if (legacy) LegacyScalarDeserializer(bytes) else BincodeDeserializer(bytes)

private data class ScalarOutcome(val value: Any?, val error: String?, val offset: Int)

internal fun verifyScalarDecoding() {
    fun compare(bytes: ByteArray, prefix: Int, read: (Deserializer) -> Any?): ScalarOutcome {
        fun outcome(legacy: Boolean): ScalarOutcome {
            val input = scalarDecoder(bytes, legacy)
            repeat(prefix) { input.deserialize_u8() }
            return try {
                ScalarOutcome(read(input), null, input.get_buffer_offset())
            } catch (error: DeserializationError) {
                ScalarOutcome(null, error.message, input.get_buffer_offset())
            }
        }
        val actual = outcome(false)
        check(actual == outcome(true)) { "Scalar mismatch: $actual" }
        return actual
    }

    val random = Random(42)
    val bits =
        listOf(
            0L,
            -1L,
            Long.MIN_VALUE,
            Long.MAX_VALUE,
            2_147_483_647L,
            2_147_483_648L,
            9_007_199_254_740_993L,
        ) +
            List(64) { 1L shl it } +
            List(1_000) { random.nextLong() } +
            // +/- zero, infinities, quiet/signalling NaNs and payloads.
            listOf(
                0x7ff0000000000000L,
                -0x10000000000000L,
                0x7ff0000000000001L,
                0x7ff8000000000042L,
                -0x7ffffffffffbeL,
            )
    for (value in bits) {
        val bytes = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putLong(value).array()
        for (prefix in listOf(0, 3)) {
            val framed = ByteArray(prefix) { 0x55 } + bytes + byteArrayOf(0x33)
            check(compare(framed, prefix) { it.deserialize_u64() }.value == value.toULong())
            check(compare(framed, prefix) { it.deserialize_i64() }.value == value)
            check(compare(framed, prefix) { it.deserialize_f64().toRawBits() }.value == value)
            val length = compare(framed, prefix) { it.deserialize_len() }
            check(length.offset == prefix + 8)
            if (value in 0..Int.MAX_VALUE.toLong()) check(length.value == value)
            else check(length.error == "Incorrect length value")
        }
    }
    val reads: List<Pair<Int, (Deserializer) -> Any?>> =
        listOf(
            8 to { it.deserialize_u64() },
            8 to { it.deserialize_i64() },
            8 to { it.deserialize_f64().toRawBits() },
            8 to { it.deserialize_len() },
            16 to { it.deserialize_u128() },
            16 to { it.deserialize_i128() },
        )
    for ((width, read) in reads) {
        for (prefix in listOf(0, 3)) {
            for (available in 0 until width) {
                val result = compare(ByteArray(prefix + available), prefix, read)
                check(result.error == "Input is not large enough")
                check(result.offset == prefix + available)
            }
        }
    }
    for (high in bits) {
        val low = high.inv()
        val bytes =
            ByteBuffer.allocate(16)
                .order(ByteOrder.LITTLE_ENDIAN)
                .putLong(low)
                .putLong(high)
                .array()
        check(
            compare(bytes, 0) { it.deserialize_u128() }.value ==
                UInt128(high.toULong(), low.toULong())
        )
        check(compare(bytes, 0) { it.deserialize_i128() }.value == Int128(high, low.toULong()))
    }
}

internal fun scalarIds(size: Int): List<VocabularyId> =
    List(size) { index ->
        VocabularyId(
            when (index) {
                size - 1 -> Long.MAX_VALUE
                size - 2 -> 9_007_199_254_740_993L
                else -> index * 7L + 1
            }
        )
    }

internal fun frozenScalarView(ids: List<VocabularyId>): ViewModel =
    ViewModel(
        ankiSettings = AnkiSettingsView(AnkiSettingsPhase.UNLOADED, null, null),
        vocabulary =
            VocabularyState(
                query =
                    VocabularyQuery(
                        "𠮷e\u0301",
                        VocabularyStatus.entries.toList(),
                        VocabularySort.NEWEST,
                    ),
                matchingCount = ids.size.toULong(),
                rows =
                    ids.take(100).map {
                        VocabularyRow(
                            it,
                            "語𠮷${it.value}",
                            VocabularyStatus.NEW,
                            1L,
                            ULong.MAX_VALUE,
                        )
                    },
                nextOffset = if (ids.size > 100) 100UL else null,
                loading = false,
                selection = ids,
                selectingAll = false,
                confirmation =
                    VocabularyBulkCommand(
                        ids,
                        VocabularyBulkAction.SetStatus(
                            VocabularyStatus.LEARNING,
                            9_007_199_254_740_993L,
                        ),
                    ),
                bulkError = DocumentError.UNAVAILABLE,
            ),
        statistics = StatisticsState(StatisticsTimeframe.LASTYEAR, loading = false),
        destination = Destination.VOCABULARY,
        settingsOpen = false,
        documents = emptyList(),
        libraryTitleQuery = "",
        librarySort = LibrarySort.LASTREAD,
        libraryDocumentCount = 0UL,
        storageBusy = false,
        contentRoute = ContentRoute.MAIN,
        readingTimeSavePending = false,
        completionSavePending = false,
        readerFurigana = emptyList(),
    )

internal fun scalarListBytes(ids: List<VocabularyId>): ByteArray =
    BincodeSerializer().apply { ids.serialize(this) { it.serialize(this) } }.get_bytes()

internal fun decodeScalarList(bytes: ByteArray, legacy: Boolean): List<VocabularyId> {
    val input = scalarDecoder(bytes, legacy)
    val result = input.deserializeListOf { VocabularyId.deserialize(it) }
    if (input.get_buffer_offset() != bytes.size) throw DeserializationError("Trailing bytes")
    return result
}

internal fun decodeScalarView(bytes: ByteArray, legacy: Boolean): ViewModel {
    val input = scalarDecoder(bytes, legacy)
    val result = ViewModel.deserialize(input)
    if (input.get_buffer_offset() != bytes.size) throw DeserializationError("Trailing bytes")
    return result
}

internal fun verifyScalarSnapshots() {
    for (size in listOf(0, 1, 2, 100, 10_000, 100_000)) {
        val ids = scalarIds(size)
        val view = frozenScalarView(ids)
        val bytes = view.bincodeSerialize()
        for (legacy in listOf(false, true)) {
            check(decodeScalarList(scalarListBytes(ids), legacy) == ids)
            val actual = decodeScalarView(bytes, legacy)
            check(actual == view)
            check(actual.bincodeSerialize().contentEquals(bytes))
        }
    }
    // Nested malformed data: compare exact generated failures and partial-read offsets.
    val valid = frozenScalarView(scalarIds(2)).bincodeSerialize()
    for (bytes in List(valid.size) { valid.copyOf(it) } + listOf(valid + byteArrayOf(0))) {
        data class Failure(val message: String?, val offset: Int)
        fun failure(legacy: Boolean): Failure {
            val input = scalarDecoder(bytes, legacy)
            try {
                ViewModel.deserialize(input)
                if (input.get_buffer_offset() != bytes.size)
                    throw DeserializationError("Trailing bytes")
            } catch (error: DeserializationError) {
                return Failure(error.message, input.get_buffer_offset())
            }
            error("Malformed snapshot accepted")
        }
        check(failure(false) == failure(true))
    }
}
