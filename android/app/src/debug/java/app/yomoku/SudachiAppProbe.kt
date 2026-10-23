package app.yomoku

/** Measurement-only JNI primitives. Nothing in the app opens this harness at startup. */
internal class SudachiAppProbe {
    /** Caller holds a verified descriptor open and preserves its immutable inode until close. */
    external fun nativeOpen(fd: Int): Long

    /** Positive token count, or negative error code; handle is specific to its worker thread. */
    external fun nativeValidate(generation: Long, fixture: Int): Long

    external fun nativeClose(generation: Long): Long

    companion object {
        init {
            System.loadLibrary("android_app_probe")
        }
    }
}
