package app.yomoku;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.Looper;
import android.os.ParcelFileDescriptor;
import android.provider.OpenableColumns;
import android.util.Base64;
import java.io.FileNotFoundException;
import java.io.IOException;

/** Test APK providers run without the target APK's Kotlin runtime. Keep this fixture standalone. */
public final class TxtFixtureProvider extends ContentProvider {
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
        MatrixCursor cursor = new MatrixCursor(new String[] {OpenableColumns.DISPLAY_NAME});
        cursor.addRow(new Object[] {uri.getLastPathSegment()});
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
                try (ParcelFileDescriptor.AutoCloseOutputStream output =
                        new ParcelFileDescriptor.AutoCloseOutputStream(pipe[1])) {
                    output.write(bytes);
                } catch (IOException error) {
                    throw new IllegalStateException(error);
                }
            }).start();
            return pipe[0];
        } catch (IOException error) {
            throw new FileNotFoundException(error.toString());
        }
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
