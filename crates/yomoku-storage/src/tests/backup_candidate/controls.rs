use yomoku_core::document::PasteText;

use super::*;

#[test]
fn backup_candidate_controls_require_singletons_before_private_migration() {
    for table in [
        "reader_settings",
        "library_read_clock",
        "anki_settings",
        "statistics_settings",
    ] {
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        connection
            .execute(&format!("DELETE FROM {table}"), [])
            .unwrap();
        drop(connection);
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_controls_reject_malformed_or_missing_allocation_counters() {
    for sql in [
        "DELETE FROM sqlite_sequence WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = 0 WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = -1 WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = NULL WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = '99999999999999999999999999' WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = 9.5 WHERE name = 'sources'",
        "UPDATE sqlite_sequence SET seq = X'3130' WHERE name = 'sources'",
        "INSERT INTO sqlite_sequence SELECT name, seq FROM sqlite_sequence WHERE name = 'sources'",
        "INSERT INTO sqlite_sequence VALUES ('unknown', 1)",
        "INSERT INTO sqlite_sequence VALUES ('reader_settings', 1)",
        "INSERT INTO sqlite_sequence VALUES (NULL, 1)",
        "INSERT INTO sqlite_sequence VALUES (X'ff', 1)",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage
            .unanalyzed_fixture(&PasteText::new("本文".into()).unwrap())
            .unwrap();
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_controls_reject_library_clock_regression_and_tied_ordinals() {
    for duplicate in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        for title in ["一", "二"] {
            storage
                .unanalyzed_fixture(&PasteText::new(title.into()).unwrap())
                .unwrap();
        }
        storage.connection.execute_batch(if duplicate {
            "UPDATE documents SET last_read_sequence = 1; UPDATE library_read_clock SET sequence = 5"
        } else {
            "UPDATE documents SET last_read_sequence = id; UPDATE library_read_clock SET sequence = 1"
        }).unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute("UPDATE library_read_clock SET sequence = ?1", [i64::MAX])
        .unwrap();
    accept_unchanged(&database, 1);
}
