package app.yomoku;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.Looper;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.util.Base64;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

/** Test APK providers run without the target APK's Kotlin runtime. Keep this fixture standalone. */
public class TxtFixtureProvider extends ContentProvider {
    private final ConcurrentHashMap<String, CountDownLatch> gates = new ConcurrentHashMap<>();
    private final ConcurrentHashMap<String, Boolean> finished = new ConcurrentHashMap<>();

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public String getType(Uri uri) {
        return "text/plain";
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] args, String sort) {
        requireWorker();
        MatrixCursor cursor = new MatrixCursor(new String[] {OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE});
        String name = uri.getQueryParameter("name");
        String size = uri.getQueryParameter("reportedSize");
        cursor.addRow(new Object[] {name == null ? uri.getLastPathSegment() : name,
                size == null ? null : Long.valueOf(size)});
        return cursor;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        requireWorker();
        String payload = uri.getQueryParameter("bytes");
        if (!"r".equals(mode) || payload == null) {
            throw new FileNotFoundException("Test file missing");
        }
        byte[] bytes = Base64.decode(payload, Base64.NO_WRAP);
        try {
            ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createPipe();
            new Thread(() -> {
                String gate = uri.getQueryParameter("gate");
                try (ParcelFileDescriptor.AutoCloseOutputStream output =
                        new ParcelFileDescriptor.AutoCloseOutputStream(pipe[1])) {
                    if (gate != null && !gates.computeIfAbsent(gate, key -> new CountDownLatch(1))
                            .await(90, TimeUnit.SECONDS)) {
                        throw new IOException("Test read gate timed out");
                    }
                    output.write(bytes);
                } catch (IOException | InterruptedException error) {
                    throw new IllegalStateException(error);
                } finally {
                    if (gate != null) finished.put(gate, true);
                }
            }).start();
            return pipe[0];
        } catch (IOException error) {
            throw new FileNotFoundException(error.toString());
        }
    }

    @Override
    public Bundle call(String method, String arg, Bundle extras) {
        if ("finished".equals(method) && arg != null) {
            Bundle result = new Bundle();
            result.putBoolean("finished", finished.containsKey(arg));
            return result;
        }
        if (!"release".equals(method) || arg == null) {
            throw new UnsupportedOperationException("Unknown fixture operation");
        }
        gates.computeIfAbsent(arg, key -> new CountDownLatch(1)).countDown();
        return Bundle.EMPTY;
    }

    private static void requireWorker() {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            throw new IllegalStateException("Document read on main thread");
        }
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException("Read only");
    }

    @Override
    public int delete(Uri uri, String selection, String[] args) {
        throw new UnsupportedOperationException("Read only");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] args) {
        throw new UnsupportedOperationException("Read only");
    }
}
