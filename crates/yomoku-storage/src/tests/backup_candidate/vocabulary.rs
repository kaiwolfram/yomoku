use yomoku_core::vocabulary::{Surface, VocabularyStatus};

use super::*;

fn prefix(version: usize) -> (TestDatabase, Connection) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
    (database, connection)
}

#[test]
fn backup_candidate_vocabulary_rejects_blank_surfaces_before_and_after_migration() {
    for version in [6, 9, 16, 17, 20, 23, 24] {
        for blank in [" ", "\t\r\n", "\u{85}\u{a0}", "\u{2003}\u{3000}"] {
            let (database, connection) = prefix(version);
            connection
                .execute(
                    "INSERT INTO vocabulary (surface, status, registered_at_unix_ms)
                     VALUES (?1, 'new', 0)",
                    [blank],
                )
                .unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_vocabulary_checks_every_custom_field_without_registration() {
    for version in [17, 20, 23, 24] {
        for field in 0..3 {
            for blank in [" ", "\n\t", "\u{a0}\u{3000}"] {
                let (database, connection) = prefix(version);
                let mut values = ["未登録", "meaning", "よみ"];
                values[field] = blank;
                connection
                    .execute(
                        "INSERT INTO custom_dictionary_entries
                         (surface, semantics_version, english_meaning, reading)
                         VALUES (?1, 1, ?2, ?3)",
                        values,
                    )
                    .unwrap();
                drop(connection);
                check_rejected(&database, version as u32, CandidateError::InvalidData);
            }
        }
    }
}

#[test]
fn backup_candidate_vocabulary_rejects_invalid_utf8_text_in_every_entry_field() {
    // SQLite TEXT can contain invalid UTF-8 even with STRICT/nonempty CHECKs.
    // Bundled literal SQL only; no candidate field becomes an SQL identifier.
    for sql in [
        "INSERT INTO vocabulary (surface, status, registered_at_unix_ms)
         VALUES (CAST(X'ff' AS TEXT), 'new', 0)",
        "INSERT INTO custom_dictionary_entries VALUES (CAST(X'ff' AS TEXT), 1, 'meaning', 'よみ')",
        "INSERT INTO custom_dictionary_entries VALUES ('未登録', 1, CAST(X'ff' AS TEXT), 'よみ')",
        "INSERT INTO custom_dictionary_entries VALUES ('未登録', 1, 'meaning', CAST(X'ff' AS TEXT))",
    ] {
        for version in [17, 24] {
            let (database, connection) = prefix(version);
            connection.execute_batch(sql).unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_vocabulary_preserves_literal_entries_all_statuses_and_deleted_registration() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    // No normalization, Japanese detection or NUL restriction belongs to these
    // domain types. Even whitespace is preserved when the value is not blank.
    for (spelling, status) in [
        (" 読む\n", VocabularyStatus::New),
        ("読む", VocabularyStatus::Learning),
        ("ｶﾅ", VocabularyStatus::Familiar),
        ("か\u{3099}", VocabularyStatus::Known),
        ("Latin\0𠮷", VocabularyStatus::Ignored),
    ] {
        let surface = Surface::new(spelling).unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&surface), i64::MAX)
            .unwrap();
        let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
        // Wall-clock rollback is accepted by the existing writer.
        storage.set_vocabulary_status(entry.id, status, 0).unwrap();
    }
    let erased = Surface::new("消す").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&erased), 0)
        .unwrap();
    let erased_id = storage.vocabulary_entry(&erased).unwrap().unwrap().id;
    for surface in ["消す", "未登録", " 読む\n", "か\u{3099}", "Latin\0𠮷"] {
        storage
            .connection
            .execute(
                "INSERT INTO custom_dictionary_entries VALUES (?1, 1, ?2, ?3)",
                (surface, " literal\0<& meaning \n", " よみ𠮷\u{3099}\t"),
            )
            .unwrap();
    }
    storage.delete_vocabulary_entries(&[erased_id]).unwrap();
    assert!(storage.vocabulary_entry(&erased).unwrap().is_none());
    accept_unchanged(&database, 24);
}

#[test]
fn backup_candidate_vocabulary_migrates_populated_historical_entries_without_rewriting() {
    for version in [6, 9, 16, 17, 20, 23] {
        let (database, connection) = prefix(version);
        connection
            .execute_batch(
                "INSERT INTO vocabulary (id, surface, status, registered_at_unix_ms)
             VALUES (9007199254740993, ' が𠮷 ', 'ignored', 9223372036854775807);
             INSERT INTO vocabulary_status_events
             (semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms)
             VALUES (1, 9007199254740993, NULL, 'new', 'registration', 9223372036854775807),
                    (1, 9007199254740993, 'new', 'ignored', 'manual', 0);",
            )
            .unwrap();
        if version >= 17 {
            connection.execute_batch(
                "INSERT INTO custom_dictionary_entries VALUES (' 未登録 ', 1, ' literal meaning ', ' よみ ')",
            ).unwrap();
        }
        let vocabulary = connection
            .query_row(
                "SELECT id, surface, status, registered_at_unix_ms FROM vocabulary",
                [],
                crate::vocabulary::entry_columns,
            )
            .unwrap();
        drop(connection);
        let before = fs::read(database.path()).unwrap();
        let directory = database.0.join("migrated");
        let checked = candidate(&directory, &before)
            .check_schema(version as u32, before.len() as u64, || false)
            .unwrap();
        let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
        assert_eq!(
            copy.query_row(
                "SELECT id, surface, status, registered_at_unix_ms FROM vocabulary",
                [],
                crate::vocabulary::entry_columns,
            )
            .unwrap(),
            vocabulary,
        );
        let custom: Vec<(String, i64, String, String)> = copy
            .prepare("SELECT * FROM custom_dictionary_entries")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            custom,
            if version >= 17 {
                vec![(
                    " 未登録 ".into(),
                    1,
                    " literal meaning ".into(),
                    " よみ ".into(),
                )]
            } else {
                vec![]
            }
        );
        drop(copy);
        checked.discard().unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}
