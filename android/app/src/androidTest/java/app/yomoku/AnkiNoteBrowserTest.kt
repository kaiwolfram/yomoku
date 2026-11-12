package app.yomoku

import android.content.ActivityNotFoundException
import android.content.Intent
import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.yomoku.shared.*
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AnkiNoteBrowserTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val frozen =
        AnkiExportView(
            ULong.MAX_VALUE,
            "猫𠮷",
            Long.MAX_VALUE,
            AnkiExportPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L)),
        )

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)

    private class Fixture(val export: AnkiExportView) {
        val executor = Executors.newSingleThreadExecutor()
        var current: AnkiExportView? = export
        var entries = emptyList<AnkiEntryView>()
        val entryReads = mutableListOf<List<Long>>()
        val reads = mutableListOf<Long>()
        val launches = mutableListOf<Intent>()
        var resolve: (AnkiProviderNoteId) -> AnkiReadResult<Intent?> = {
            AnkiReadResult.Value(Intent("test.${it.value}"))
        }
        var launch: (Intent) -> Unit = { launches.add(it) }
        val browser =
            AnkiNoteBrowser(
                executor,
                {
                    assertNotEquals(Looper.getMainLooper(), Looper.myLooper())
                    reads.add(it.value)
                    resolve(it)
                },
                currentEntries = { entries },
                resolveEntries = { ids ->
                    entryReads.add(ids.map { it.value })
                    AnkiReadResult.Value(Intent("test.entries"))
                },
            ) {
                current
            }

        fun attach() = browser.attach {
            assertEquals(Looper.getMainLooper(), Looper.myLooper())
            launch(it)
        }
    }

    private fun drain(f: Fixture) {
        f.executor.submit {}.get(30, TimeUnit.SECONDS)
        instrumentation.waitForIdleSync()
    }

    private fun fixture(block: (Fixture) -> Unit) {
        lateinit var f: Fixture
        main {
            f = Fixture(frozen)
            f.attach()
        }
        try {
            block(f)
        } finally {
            main { f.browser.close() }
            f.executor.shutdown()
            assertTrue(f.executor.awaitTermination(30, TimeUnit.SECONDS))
        }
    }

    @Test
    fun entryOpenUsesAllVerifiedIdsAndRejectsRemovedSelections() = fixture { f ->
        val key =
            ReaderTokenKey(
                DocumentId(1),
                TextAnchor(SourceId(2), ChapterId(3), TextRange(TextOffset(0), TextOffset(3))),
                ReaderAnalysis.Available(AnalysisId(4)),
            )
        val entry =
            AnkiEntryView(
                AnkiSelection.Effective(
                    EffectiveReaderSelection(EffectivePageIdentity(1UL, key), 0UL)
                ),
                AnkiEntryKey.Word(5, ReadingMatch(0UL, 0UL)),
                "猫",
                "ねこ",
                AnkiEntryPhase.Existing(listOf(Long.MAX_VALUE, 9007199254740993L)),
            )
        main {
            f.entries = listOf(entry)
            f.browser.openEntry(entry)
            f.browser.openEntry(entry)
        }
        drain(f)
        assertEquals(listOf(listOf(Long.MAX_VALUE, 9007199254740993L)), f.entryReads)
        assertEquals(1, f.launches.size)
        main {
            f.entries = emptyList()
            f.browser.refresh()
            f.browser.openEntry(entry)
        }
        drain(f)
        assertEquals(1, f.launches.size)
        assertEquals(1, f.entryReads.size)
        main {
            f.entries = listOf(entry)
            f.browser.openEntry(entry)
            f.entries = emptyList()
        }
        drain(f)
        assertEquals(1, f.launches.size)
        assertEquals(2, f.entryReads.size)
    }

    @Test
    fun explicitOpenCoalescesAndKeepsEveryExactFrozenIdWithoutReturnReplay() = fixture { f ->
        assertTrue(f.reads.isEmpty())
        main {
            f.browser.open(frozen, Long.MAX_VALUE)
            f.browser.open(frozen, 9007199254740993L)
            assertEquals(AnkiBrowserPhase.Resolving, f.browser.state?.phase)
        }
        drain(f)
        assertEquals(listOf(Long.MAX_VALUE), f.reads)
        assertEquals(listOf("test.${Long.MAX_VALUE}"), f.launches.map { it.action })
        main {
            assertNull(f.browser.state)
            f.browser.detach()
            f.attach()
            f.browser.refresh()
        }
        drain(f)
        assertEquals(1, f.launches.size)
        main { f.browser.open(frozen, 9007199254740993L) }
        drain(f)
        assertEquals(listOf(Long.MAX_VALUE, 9007199254740993L), f.reads)
        assertEquals(frozen, f.current)
        assertEquals(2, f.launches.size)
    }

    @Test
    fun staleGenerationsPhasesAndUnknownIdsNeverResolve() = fixture { f ->
        main {
            f.browser.open(frozen.copy(preparationId = 1UL), Long.MAX_VALUE)
            f.browser.open(frozen.copy(surface = "犬"), Long.MAX_VALUE)
            f.browser.open(frozen, 7)
            f.browser.open(frozen, 0)
            for (phase in
                listOf(
                    AnkiExportPhase.Prepared,
                    AnkiExportPhase.NoteUncertain(Long.MAX_VALUE),
                    AnkiExportPhase.ModelCreated(Long.MAX_VALUE),
                )) {
                f.current = frozen.copy(phase = phase)
                f.browser.open(f.current!!, Long.MAX_VALUE)
            }
            assertNull(f.browser.state)
        }
        drain(f)
        assertTrue(f.reads.isEmpty())
        main {
            f.current = frozen.copy(phase = AnkiExportPhase.NoteAdded(Long.MAX_VALUE))
            f.browser.open(f.current!!, Long.MAX_VALUE)
        }
        drain(f)
        assertEquals(listOf(Long.MAX_VALUE), f.reads)
    }

    @Test
    fun unavailableAndFailedReadsRequireExplicitOpenAndNeverChangeExport() = fixture { f ->
        val results =
            AnkiReadError.entries.map { AnkiReadResult.Failed(it) } + AnkiReadResult.Value(null)
        for (result in results) {
            f.resolve = { result }
            val before = f.reads.size
            main { f.browser.open(frozen, Long.MAX_VALUE) }
            drain(f)
            main {
                assertEquals(
                    if (result is AnkiReadResult.Failed) AnkiBrowserPhase.Failed(result.error)
                    else AnkiBrowserPhase.Unavailable,
                    f.browser.state?.phase,
                )
                f.browser.refresh()
            }
            drain(f)
            assertEquals(before + 1, f.reads.size)
            assertTrue(f.launches.isEmpty())
            assertEquals(frozen, f.current)
        }
    }

    @Test
    fun resolutionAndLaunchExceptionsAreVisibleAndExplicitlyRecoverable() = fixture { f ->
        val failures =
            listOf(
                ActivityNotFoundException() to AnkiBrowserPhase.Unavailable,
                SecurityException() to AnkiBrowserPhase.Failed(AnkiReadError.PermissionDenied),
                IllegalStateException() to AnkiBrowserPhase.Failed(AnkiReadError.ProviderFailure),
            )
        for ((error, expected) in failures) {
            f.launch = { throw error }
            main { f.browser.open(frozen, Long.MAX_VALUE) }
            drain(f)
            main { assertEquals(expected, f.browser.state?.phase) }
        }
        for ((error, expected) in failures.drop(1)) {
            f.resolve = { throw error }
            main { f.browser.open(frozen, Long.MAX_VALUE) }
            drain(f)
            main { assertEquals(expected, f.browser.state?.phase) }
        }
        f.resolve = { AnkiReadResult.Value(Intent("fresh")) }
        f.launch = { f.launches.add(it) }
        main { f.browser.open(frozen, 9007199254740993L) }
        drain(f)
        assertEquals("fresh", f.launches.single().action)
        main { assertNull(f.browser.state) }
    }

    @Test
    fun aCancelledResultCannotConsumeANewRequestForTheSameFrozenNote() = fixture { f ->
        val entered = CountDownLatch(1)
        val release = CountDownLatch(1)
        f.resolve = {
            if (f.reads.size == 1) {
                entered.countDown()
                assertTrue(release.await(30, TimeUnit.SECONDS))
            }
            AnkiReadResult.Value(Intent("attempt.${f.reads.size}"))
        }
        try {
            main { f.browser.open(frozen, Long.MAX_VALUE) }
            assertTrue(entered.await(30, TimeUnit.SECONDS))
            main {
                f.browser.detach()
                f.attach()
                f.browser.open(frozen, Long.MAX_VALUE)
            }
        } finally {
            release.countDown()
        }
        drain(f)
        assertEquals(listOf(Long.MAX_VALUE, Long.MAX_VALUE), f.reads)
        assertEquals("attempt.2", f.launches.single().action)
        main { assertNull(f.browser.state) }
    }

    @Test
    fun pauseCloseAndChangedExportCancelQueuedAndLateLaunches() {
        for (queued in listOf(true, false)) {
            for (cancel in listOf("pause", "close", "changed")) fixture { f ->
                val entered = CountDownLatch(1)
                val release = CountDownLatch(1)
                val wait = {
                    entered.countDown()
                    assertTrue(release.await(30, TimeUnit.SECONDS))
                }
                if (queued) f.executor.execute { wait() }
                else
                    f.resolve = {
                        wait()
                        AnkiReadResult.Value(Intent("late"))
                    }
                try {
                    main { f.browser.open(frozen, Long.MAX_VALUE) }
                    assertTrue(entered.await(30, TimeUnit.SECONDS))
                    main {
                        when (cancel) {
                            "pause" -> {
                                f.browser.detach()
                                f.attach()
                            }
                            "close" -> f.browser.close()
                            else -> {
                                f.current = null
                                f.browser.refresh()
                            }
                        }
                    }
                } finally {
                    release.countDown()
                }
                drain(f)
                assertEquals(if (queued) 0 else 1, f.reads.size)
                assertTrue(f.launches.isEmpty())
                main { assertNull(f.browser.state) }
            }
        }
    }
}
