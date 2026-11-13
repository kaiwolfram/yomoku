package app.yomoku;

import android.database.sqlite.SQLiteDatabase;

/** Holds a real cross-process SQLite writer lock until the instrumentation closes stdin. */
public final class SqliteWriteLock {
    public static void main(String[] args) throws Exception {
        try (SQLiteDatabase database =
                SQLiteDatabase.openDatabase(args[0], null, SQLiteDatabase.OPEN_READWRITE)) {
            database.beginTransaction();
            try {
                System.out.println("locked");
                System.out.flush();
                System.in.read();
            } finally {
                database.endTransaction();
            }
        }
    }
}
