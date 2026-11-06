package app.yomoku.tests

import app.yomoku.nativebridge.DictionaryProvider
import java.io.FileDescriptor
import java.io.FileInputStream
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

/** Linux host test only: equivalent to Android's dup/detachFd ownership transfer. */
internal class HostDictionary : DictionaryProvider {
    val calls = AtomicInteger()
    val threads = CopyOnWriteArrayList<Thread>()
    @Volatile var fail = false

    val wordCalls = AtomicInteger()
    val wordThreads = CopyOnWriteArrayList<Thread>()
    @Volatile var failWords = false

    override fun verifiedWordDictionaryPath(): String {
        wordCalls.incrementAndGet()
        wordThreads.add(Thread.currentThread())
        return if (failWords) "" else checkNotNull(System.getenv("YOMOKU_WORD_DICTIONARY"))
    }

    override fun openVerifiedDictionary(): Long {
        calls.incrementAndGet()
        threads.add(Thread.currentThread())
        if (fail) return 0L
        return try {
            // Full fixtures are explicitly prepared and never modified during tests.
            FileInputStream(checkNotNull(System.getenv("YOMOKU_SUDACHI_DICTIONARY"))).use { input ->
                val field = FileDescriptor::class.java.getDeclaredField("fd")
                field.isAccessible = true
                val dup =
                    Class.forName("sun.nio.fs.UnixNativeDispatcher")
                        .getDeclaredMethod("dup", Int::class.javaPrimitiveType)
                dup.isAccessible = true
                (dup.invoke(null, field.getInt(input.fd)) as Int).toLong() + 1L
            }
        } catch (_: Exception) {
            0L
        }
    }
}
