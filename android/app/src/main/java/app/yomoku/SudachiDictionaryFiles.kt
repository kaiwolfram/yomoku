package app.yomoku

import android.content.Context
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import app.yomoku.nativebridge.DictionaryProvider
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.RandomAccessFile
import java.security.MessageDigest

/** Platform I/O only. The Rust FIFO worker opens this lazily on the first import. */
internal fun sudachiDictionaryProvider(context: Context): DictionaryProvider {
    return sudachiDictionaryProvider(sudachiDictionaryFiles(context))
}

internal fun sudachiDictionaryProvider(files: VerifiedDictionaryFiles): DictionaryProvider {
    return object : DictionaryProvider {
        override fun openVerifiedDictionary(): Long =
            try {
                files.open().use { input ->
                    // Transfer an owned FD; +1 reserves zero for callback failure.
                    ParcelFileDescriptor.dup(input.fd).use { it.detachFd().toLong() + 1L }
                }
            } catch (_: Exception) {
                // Typed import failure in Rust; no exception crosses a foreign callback.
                0L
            }
    }
}

internal fun sudachiDictionaryFiles(context: Context): VerifiedDictionaryFiles {
    val application = context.applicationContext
    return VerifiedDictionaryFiles(
        File(application.noBackupFilesDir, "sudachi"),
        330_506_339L,
        "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1",
    ) {
        application.assets.open("sudachi/system_full.dic")
    }
}

/**
 * Owns its private directory. Published inodes are never opened for writing or truncated, including
 * during repair. Atomic replacement leaves existing readers/mappings on their original inode.
 * Permissions are defense in depth; all future maintenance must preserve this lifetime rule.
 */
internal class VerifiedDictionaryFiles(
    private val directory: File,
    private val expectedSize: Long,
    private val expectedSha256: String,
    private val publishedName: String = "full-$expectedSha256.dic",
    private val openAsset: () -> InputStream,
) {
    init {
        require(expectedSize > 0)
        require(expectedSha256.matches(Regex("[0-9a-f]{64}")))
        require(publishedName.matches(Regex("[A-Za-z0-9][A-Za-z0-9._-]*")))
    }

    /**
     * For read-only SQLite consumers. Fully verifies before returning; every publisher must
     * preserve immutable inodes. Replacement at this content-addressed path always has the same
     * pinned bytes.
     */
    fun verifiedPath(): File {
        open().use { /* Verify on this worker, including a previously published cache. */ }
        return File(directory, publishedName)
    }

    /** Returns the verified read-only descriptor, positioned at zero. The caller must close it. */
    fun open(): FileInputStream {
        check(Looper.myLooper() != Looper.getMainLooper()) { "Dictionary I/O requires a worker" }
        // FileChannel locks are process-wide and throw on overlapping locks in the same JVM.
        synchronized(publicationLock) {
            try {
                if (!directory.mkdirs() && !directory.isDirectory) {
                    throw IOException("Cannot create private dictionary directory")
                }
                Os.chmod(directory.path, 0b111000000)
                RandomAccessFile(File(directory, "publication.lock"), "rw").use { lockFile ->
                    lockFile.channel.lock().use {
                        // A killed process may leave staging bytes. They were never published.
                        val files =
                            directory.listFiles()
                                ?: throw IOException("Cannot list dictionary directory")
                        for (file in files) {
                            if (
                                file.name.startsWith("extract-") && file.name.endsWith(".partial")
                            ) {
                                if (!file.delete())
                                    throw IOException(
                                        "Cannot remove abandoned dictionary extraction"
                                    )
                            }
                        }
                        val published = File(directory, publishedName)
                        verified(published, requireReadOnly = true)?.let {
                            return it
                        }
                        return extract(published)
                    }
                }
            } catch (error: ErrnoException) {
                throw IOException("Private dictionary publication failed", error)
            }
        }
    }

    private fun extract(published: File): FileInputStream {
        val staging = File.createTempFile("extract-", ".partial", directory)
        var reader: FileInputStream? = null
        try {
            FileOutputStream(staging).use { output ->
                openAsset().use { input ->
                    val buffer = ByteArray(64 * 1024)
                    var size = 0L
                    while (true) {
                        val count = input.read(buffer)
                        if (count == -1) break
                        size += count
                        if (size > expectedSize)
                            throw IOException("Dictionary asset exceeds pinned size")
                        output.write(buffer, 0, count)
                    }
                    if (size != expectedSize)
                        throw IOException("Dictionary asset has incorrect size")
                }
                output.fd.sync()
            }
            reader =
                verified(staging, requireReadOnly = false)
                    ?: throw IOException("Dictionary asset checksum mismatch")
            Os.fchmod(reader.fd, 0b100000000)
            Os.fsync(reader.fd)
            Os.rename(staging.path, published.path)
            val directoryFd = Os.open(directory.path, OsConstants.O_RDONLY, 0)
            try {
                Os.fsync(directoryFd)
            } finally {
                Os.close(directoryFd)
            }
            return reader
        } catch (error: Throwable) {
            try {
                reader?.close()
            } catch (closeError: Throwable) {
                error.addSuppressed(closeError)
            }
            throw error
        } finally {
            // After rename this name no longer exists; never delete the published file on failure.
            // If cleanup itself fails (e.g. storage I/O failure), the next open retries it.
            staging.delete()
        }
    }

    private fun verified(file: File, requireReadOnly: Boolean): FileInputStream? {
        if (!file.exists()) return null
        val input = FileInputStream(file)
        try {
            val stat = Os.fstat(input.fd)
            if (
                stat.st_size != expectedSize ||
                    (requireReadOnly && stat.st_mode and 0b010010010 != 0)
            ) {
                input.close()
                return null
            }
            val digest = MessageDigest.getInstance("SHA-256")
            val buffer = ByteArray(64 * 1024)
            var size = 0L
            while (true) {
                val count = input.read(buffer)
                if (count == -1) break
                size += count
                if (size > expectedSize) break
                digest.update(buffer, 0, count)
            }
            val checksum = digest.digest().joinToString("") { "%02x".format(it) }
            if (size != expectedSize || checksum != expectedSha256) {
                input.close()
                return null
            }
            input.channel.position(0)
            return input
        } catch (error: Throwable) {
            try {
                input.close()
            } catch (closeError: Throwable) {
                error.addSuppressed(closeError)
            }
            throw error
        }
    }

    private companion object {
        val publicationLock = Any()
    }
}
