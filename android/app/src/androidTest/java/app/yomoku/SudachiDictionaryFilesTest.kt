package app.yomoku

import android.content.Context
import android.content.ContextWrapper
import android.os.SystemClock
import android.system.Os
import android.system.OsConstants
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.ByteArrayInputStream
import java.io.File
import java.io.FileInputStream
import java.io.IOException
import java.io.InputStream
import java.nio.channels.FileChannel
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.Callable
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SudachiDictionaryFilesTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val bytes = "猫と犬。東京都。𠮷野家。".toByteArray()
    private val checksum = sha256(bytes)

    @Test
    fun verifiedPathNeverReturnsCorruptBytesAndRepairsWithoutChangingHeldReader() =
        withDirectory { directory ->
            val store =
                VerifiedDictionaryFiles(
                    directory,
                    bytes.size.toLong(),
                    checksum,
                    publishedName = "index-$checksum.sqlite3",
                ) {
                    ByteArrayInputStream(bytes)
                }
            assertTrue(directory.mkdirs())
            val published = File(directory, "index-$checksum.sqlite3")
            published.writeBytes(ByteArray(bytes.size))
            Os.chmod(published.path, 0b100000000)
            FileInputStream(published).use { old ->
                val broken =
                    VerifiedDictionaryFiles(
                        directory,
                        bytes.size.toLong(),
                        checksum,
                        publishedName = published.name,
                    ) {
                        ByteArrayInputStream(ByteArray(bytes.size))
                    }
                assertThrows(IOException::class.java) { broken.verifiedPath() }
                assertEquals(Os.fstat(old.fd).st_ino, Os.stat(published.path).st_ino)
                assertEquals(published, store.verifiedPath())
                assertTrue(Os.fstat(old.fd).st_ino != Os.stat(published.path).st_ino)
                assertArrayEquals(bytes, published.readBytes())
                assertArrayEquals(ByteArray(bytes.size), old.readBytes())
                assertEquals(published, store.verifiedPath())
            }
        }

    @Test
    fun constructionIsLazyAndMainThreadOpenIsRejected() = withDirectory { directory ->
        val store = store(directory) { error("Must not read the asset") }
        assertFalse(directory.exists())
        instrumentation.runOnMainSync {
            assertThrows(IllegalStateException::class.java) { store.open() }
        }
        assertFalse(directory.exists())
    }

    @Test
    fun verifiedCacheReopensWithoutAssetAndRejectsDescriptorWrites() = withDirectory { directory ->
        val calls = AtomicInteger()
        store(directory) {
                calls.incrementAndGet()
                ByteArrayInputStream(bytes)
            }
            .open()
            .use { input ->
                assertEquals(0L, input.channel.position())
                assertArrayEquals(bytes, input.readBytes())
                assertEquals(0b100000000, Os.fstat(input.fd).st_mode and 0b111111111)
                assertThrows(android.system.ErrnoException::class.java) {
                    Os.write(input.fd, byteArrayOf(1), 0, 1)
                }
            }
        store(directory) { error("Verified cache must not decompress the APK again") }
            .open()
            .use {
                assertArrayEquals(bytes, it.readBytes())
            }
        assertEquals(1, calls.get())
        assertNoStaging(directory)
    }

    @Test
    fun interruptedShortOversizedAndCorruptAssetsNeverPublishAndCanRetry() =
        withDirectory { directory ->
            val invalid =
                listOf<() -> InputStream>(
                    { throw IOException("Simulated missing asset") },
                    {
                        object : InputStream() {
                            private var count = 0

                            override fun read(): Int {
                                if (count++ >= 4) throw IOException("Simulated interrupted input")
                                return 1
                            }
                        }
                    },
                    { ByteArrayInputStream(bytes.copyOf(bytes.size - 1)) },
                    { ByteArrayInputStream(bytes + byteArrayOf(1)) },
                    { ByteArrayInputStream(ByteArray(bytes.size)) },
                )
            for (openAsset in invalid) {
                assertThrows(IOException::class.java) { store(directory, openAsset).open() }
                assertFalse(published(directory).exists())
                assertNoStaging(directory)
            }
            store(directory).open().use { assertArrayEquals(bytes, it.readBytes()) }
        }

    @Test
    fun abandonedCopyAndCorruptCacheRecoverWithoutMutatingOldInode() = withDirectory { directory ->
        assertTrue(directory.mkdirs())
        File(directory, "extract-killed.partial").writeText("abandoned")
        val published = published(directory)
        published.writeBytes(ByteArray(bytes.size))
        Os.chmod(published.path, 0b100000000)
        FileInputStream(published).use { old ->
            val inode = Os.fstat(old.fd).st_ino
            val mapping = old.channel.map(FileChannel.MapMode.READ_ONLY, 0, bytes.size.toLong())
            assertThrows(IOException::class.java) {
                store(directory) { throw IOException("Simulated extraction failure") }.open()
            }
            assertEquals(inode, Os.stat(published.path).st_ino)
            store(directory).open().use { repaired ->
                assertTrue(inode != Os.fstat(repaired.fd).st_ino)
                assertArrayEquals(bytes, repaired.readBytes())
                assertArrayEquals(ByteArray(bytes.size), old.readBytes())
                val mappedBytes = ByteArray(bytes.size)
                mapping.get(mappedBytes)
                assertArrayEquals(ByteArray(bytes.size), mappedBytes)
            }
        }
        assertNoStaging(directory)
    }

    @Test
    fun failedRenamePreservesPublishedBytesAndRetryCleansAbandonedStaging() =
        withDirectory { directory ->
            assertTrue(directory.mkdirs())
            val published = published(directory)
            published.writeBytes(ByteArray(bytes.size))
            Os.chmod(published.path, 0b100000000)
            val inode = Os.stat(published.path).st_ino
            try {
                assertThrows(IOException::class.java) {
                    store(directory) {
                            // The writable staging descriptor already exists. Deny
                            // publication/cleanup.
                            Os.chmod(directory.path, 0b101000000)
                            ByteArrayInputStream(bytes)
                        }
                        .open()
                }
                assertEquals(inode, Os.stat(published.path).st_ino)
                assertArrayEquals(ByteArray(bytes.size), published.readBytes())
                assertEquals(1, directory.listFiles()!!.count { it.name.endsWith(".partial") })
            } finally {
                Os.chmod(directory.path, 0b111000000)
            }
            store(directory).open().use { assertArrayEquals(bytes, it.readBytes()) }
            assertNoStaging(directory)
        }

    @Test
    fun writableCacheIsReplacedWhileExistingValidReaderStaysIntact() = withDirectory { directory ->
        store(directory).open().use { old ->
            val inode = Os.fstat(old.fd).st_ino
            // Simulate a legacy cache with unsafe permissions; never modify the held bytes.
            Os.chmod(published(directory).path, 0b110000000)
            store(directory).open().use { repaired ->
                assertTrue(inode != Os.fstat(repaired.fd).st_ino)
                assertArrayEquals(bytes, repaired.readBytes())
                assertArrayEquals(bytes, old.readBytes())
            }
        }
    }

    @Test
    fun concurrentStoresPublishOnceAndReturnIndependentVerifiedReaders() =
        withDirectory { directory ->
            val calls = AtomicInteger()
            val start = CountDownLatch(1)
            val executor = Executors.newFixedThreadPool(4)
            try {
                val requests =
                    (1..4).map {
                        executor.submit(
                            Callable {
                                assertTrue(start.await(10, TimeUnit.SECONDS))
                                store(directory) {
                                        calls.incrementAndGet()
                                        ByteArrayInputStream(bytes)
                                    }
                                    .open()
                                    .use { input ->
                                        assertArrayEquals(bytes, input.readBytes())
                                        Os.fstat(input.fd).st_ino
                                    }
                            }
                        )
                    }
                start.countDown()
                val inodes = requests.map { it.get(20, TimeUnit.SECONDS) }
                assertEquals(1, calls.get())
                assertEquals(1, inodes.distinct().size)
                assertNoStaging(directory)
            } finally {
                executor.shutdownNow()
                assertTrue(executor.awaitTermination(10, TimeUnit.SECONDS))
            }
        }

    @Test
    fun realCompressedFullExtractsAndReopensWithBoundedBuffer() = withDirectory { directory ->
        val privateContext =
            object : ContextWrapper(context) {
                override fun getApplicationContext(): Context = this

                override fun getNoBackupFilesDir(): File = directory
            }
        val store = sudachiDictionaryFiles(privateContext)
        assertFalse(File(directory, "sudachi").exists())
        val start = SystemClock.elapsedRealtimeNanos()
        var firstInode = 0L
        store.open().use { input ->
            val stat = Os.fstat(input.fd)
            firstInode = stat.st_ino
            assertEquals(330_506_339L, stat.st_size)
            assertEquals(0L, input.channel.position())
            assertEquals(
                OsConstants.O_RDONLY,
                Os.fcntlInt(input.fd, OsConstants.F_GETFL, 0) and OsConstants.O_ACCMODE,
            )
        }
        val extracted = SystemClock.elapsedRealtimeNanos()
        // A new store simulates a new owner; its cache verification reads the entire pinned file.
        sudachiDictionaryFiles(privateContext).open().use { input ->
            assertEquals(firstInode, Os.fstat(input.fd).st_ino)
            assertEquals(0L, input.channel.position())
        }
        val reopened = SystemClock.elapsedRealtimeNanos()
        Log.i(
            "SudachiExtraction",
            "size=330506339 extract_ms=${(extracted - start) / 1_000_000.0} verified_reopen_ms=${(reopened - extracted) / 1_000_000.0}",
        )
        assertNoStaging(File(directory, "sudachi"))
    }

    private fun store(
        directory: File,
        openAsset: () -> InputStream = { ByteArrayInputStream(bytes) },
    ) = VerifiedDictionaryFiles(directory, bytes.size.toLong(), checksum, openAsset = openAsset)

    private fun published(directory: File) = File(directory, "full-$checksum.dic")

    private fun assertNoStaging(directory: File) {
        assertFalse(directory.listFiles()!!.any { it.name.endsWith(".partial") })
    }

    private fun withDirectory(test: (File) -> Unit) {
        val directory = File(context.noBackupFilesDir, "dictionary-test-${UUID.randomUUID()}")
        try {
            test(directory)
        } finally {
            assertTrue(!directory.exists() || directory.deleteRecursively())
        }
    }

    private fun sha256(value: ByteArray) =
        MessageDigest.getInstance("SHA-256").digest(value).joinToString("") { "%02x".format(it) }
}
