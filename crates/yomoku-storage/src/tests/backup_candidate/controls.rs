use rusqlite::params;
use serde_json::json;
use yomoku_core::anki::{
    AnkiExistingNotes, AnkiModelCreation, AnkiModelCreationOutcome, AnkiModelLayout,
    AnkiModelReconciliation, AnkiReconciliation, AnkiSubmission, AnkiSubmissionOutcome,
};
use yomoku_core::document::PasteText;

use super::*;

#[test]
fn backup_candidate_controls_require_singletons_before_private_migration() {
    for (version, table) in [
        (4, "reader_settings"),
        (15, "reader_settings"),
        (19, "library_read_clock"),
        (23, "reader_settings"),
        (23, "library_read_clock"),
        (24, "anki_settings"),
    ] {
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
        connection
            .execute(&format!("DELETE FROM {table}"), [])
            .unwrap();
        drop(connection);
        check_rejected(&database, version as u32, CandidateError::InvalidData);
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
            .create_paste(&PasteText::new("本文".into()).unwrap())
            .unwrap();
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
    // Pre-migration validation must catch an invalid sources counter before
    // migration 12 rebuilds its table and could incidentally raise the counter.
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..11]).unwrap();
    connection.execute_batch("INSERT INTO sources VALUES (5, 'paste', 'old'); UPDATE sqlite_sequence SET seq = 1 WHERE name = 'sources'").unwrap();
    drop(connection);
    check_rejected(&database, 11, CandidateError::InvalidData);
}

#[test]
fn backup_candidate_controls_preserve_empty_deleted_and_exhausted_counters() {
    for maximum in [9007199254740993_i64, i64::MAX] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage
            .create_paste(&PasteText::new("削除本文".into()).unwrap())
            .unwrap();
        storage.delete_document(document.document_id).unwrap();
        storage
            .connection
            .execute("UPDATE sqlite_sequence SET seq = ?1", [maximum])
            .unwrap();
        // A retained counter on an empty table is meaningful deleted-ID history.
        storage
            .connection
            .execute(
                "INSERT INTO sqlite_sequence VALUES ('anki_submissions', ?1)",
                [maximum],
            )
            .unwrap();
        accept_unchanged(&database, 24);
    }
    let database = TestDatabase::new();
    let _storage = Storage::open(database.path()).unwrap();
    accept_unchanged(&database, 24); // Never-written tables have no counters.
}

#[test]
fn backup_candidate_controls_reject_library_clock_regression_and_tied_ordinals() {
    for duplicate in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        for title in ["一", "二"] {
            storage
                .create_paste(&PasteText::new(title.into()).unwrap())
                .unwrap();
        }
        storage.connection.execute_batch(if duplicate {
            "UPDATE documents SET last_read_sequence = 1; UPDATE library_read_clock SET sequence = 5"
        } else {
            "UPDATE documents SET last_read_sequence = id; UPDATE library_read_clock SET sequence = 1"
        }).unwrap();
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute("UPDATE library_read_clock SET sequence = ?1", [i64::MAX])
        .unwrap();
    accept_unchanged(&database, 24);
}

#[test]
fn backup_candidate_controls_decode_note_payloads_in_every_state() {
    let valid = serde_json::to_value(super::super::anki::payload(1)).unwrap();
    let mut invalid = vec![json!(null), json!({}), json!([])];
    for (key, value) in [
        ("version", json!(2)),
        ("deck", json!(0)),
        ("model", json!(-1)),
        ("surface", json!(" ")),
        ("fields", json!(["wrong", "", "", ""])),
        ("extra", json!(true)),
    ] {
        let mut payload = valid.clone();
        payload[key] = value;
        invalid.push(payload);
    }
    let mut payload = valid.clone();
    payload["layout"]["field_names"] = json!(["same", "same", "c", "d"]);
    invalid.push(payload);
    for state in ["pending", "uncertain", "not_inserted", "added", "existing"] {
        for payload in invalid.iter().map(|value| value.to_string()).chain([
            "malformed secret payload".into(),
            format!("{valid} trailing"),
            valid
                .to_string()
                .replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        ]) {
            let database = TestDatabase::new();
            let storage = Storage::open(database.path()).unwrap();
            storage
                .connection
                .execute(
                    "INSERT INTO anki_submissions (payload, state, known_note_id, existing_ids)
                 VALUES (?1, ?2, ?3, ?4)",
                    params![
                        payload,
                        state,
                        (state == "added").then_some(1),
                        (state == "existing").then_some("[1]")
                    ],
                )
                .unwrap();
            check_rejected(&database, 24, CandidateError::InvalidData);
        }
    }
    assert_eq!(
        CandidateError::InvalidData.to_string(),
        "backup candidate application data is invalid"
    );
}

#[test]
fn backup_candidate_controls_decode_model_payloads_in_every_state() {
    let valid =
        serde_json::to_value(AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap()).unwrap();
    let mut invalid = vec![json!(null), json!({})];
    for (key, value) in [("version", json!(2)), ("extra", json!(true))] {
        let mut payload = valid.clone();
        payload[key] = value;
        invalid.push(payload);
    }
    for (key, value) in [
        ("name", json!("")),
        ("css", json!("\0")),
        ("field_names", json!(["a", "a", "c", "d"])),
    ] {
        let mut payload = valid.clone();
        payload["layout"][key] = value;
        invalid.push(payload);
    }
    for state in ["pending", "uncertain", "not_created", "created", "existing"] {
        for payload in &invalid {
            let database = TestDatabase::new();
            let storage = Storage::open(database.path()).unwrap();
            storage.connection.execute(
                "INSERT INTO anki_model_creations (payload, state, known_model_id, existing_model_id) VALUES (?1, ?2, ?3, ?4)",
                params![payload.to_string(), state, (state == "created").then_some(1), (state == "existing").then_some(1)],
            ).unwrap();
            check_rejected(&database, 24, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_controls_reject_invalid_terminal_ids_and_model_identity() {
    for ids in [
        "[]",
        "[0]",
        "[-1]",
        "[1,1]",
        "[1.0]",
        "[9223372036854775808]",
        "null",
        "{}",
        "secret",
        "[1] trailing",
    ] {
        let database = TestDatabase::new();
        let storage = Storage::open(database.path()).unwrap();
        storage.connection.execute(
            "INSERT INTO anki_submissions (payload, state, existing_ids) VALUES (?1, 'existing', ?2)",
            params![serde_json::to_string(&super::super::anki::payload(1)).unwrap(), ids],
        ).unwrap();
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
    for model in [false, true] {
        for id in [0, -1] {
            let database = TestDatabase::new();
            let storage = Storage::open(database.path()).unwrap();
            let (table, payload) = if model {
                (
                    "anki_model_creations",
                    serde_json::to_string(
                        &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                    )
                    .unwrap(),
                )
            } else {
                (
                    "anki_submissions",
                    serde_json::to_string(&super::super::anki::payload(1)).unwrap(),
                )
            };
            storage
                .connection
                .execute(
                    &format!(
                        "INSERT INTO {table} (id, payload, state) VALUES (?1, ?2, 'uncertain')"
                    ),
                    params![id, payload],
                )
                .unwrap();
            check_rejected(&database, 24, CandidateError::InvalidData);
        }
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage
        .begin_anki_model_creation(&AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap())
        .unwrap();
    storage
        .finish_anki_model_creation(
            &live,
            AnkiModelCreationOutcome::Created(1.try_into().unwrap()),
        )
        .unwrap();
    storage
        .connection
        .execute_batch("UPDATE anki_model_creations SET state = 'existing', existing_model_id = 2")
        .unwrap();
    check_rejected(&database, 24, CandidateError::InvalidData);
}

#[test]
fn backup_candidate_controls_preserve_all_valid_journal_states_and_historical_layouts() {
    let mut layout = AnkiModelLayout::yomoku();
    layout
        .css
        .push_str("\n/* historical layout must remain frozen */");
    let original = super::super::anki::payload(i64::MAX);
    let payload = AnkiSubmission::new(
        original.note().clone(),
        original.deck(),
        original.model(),
        layout.clone(),
    )
    .unwrap();
    for model in [false, true] {
        for outcome in 0..5 {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            if model {
                let live = storage
                    .begin_anki_model_creation(&AnkiModelCreation::new(layout.clone()).unwrap())
                    .unwrap();
                match outcome {
                    0 => {} // pending
                    1 => storage
                        .finish_anki_model_creation(&live, AnkiModelCreationOutcome::NotCreated)
                        .unwrap(),
                    2 => storage
                        .finish_anki_model_creation(
                            &live,
                            AnkiModelCreationOutcome::Created(i64::MAX.try_into().unwrap()),
                        )
                        .unwrap(),
                    _ => {
                        storage
                            .finish_anki_model_creation(
                                &live,
                                AnkiModelCreationOutcome::Uncertain {
                                    known_model_id: Some(i64::MAX.try_into().unwrap()),
                                },
                            )
                            .unwrap();
                        if outcome == 4 {
                            storage
                                .reconcile_anki_model_creation(
                                    live.id(),
                                    AnkiModelReconciliation::Observed {
                                        id: i64::MAX.try_into().unwrap(),
                                        layout: Box::new(layout.clone()),
                                    },
                                )
                                .unwrap();
                        }
                    }
                }
            } else {
                let live = storage.begin_anki_submission(&payload).unwrap();
                match outcome {
                    0 => {}
                    1 => storage
                        .finish_anki_submission(&live, AnkiSubmissionOutcome::NotInserted)
                        .unwrap(),
                    2 => storage
                        .finish_anki_submission(
                            &live,
                            AnkiSubmissionOutcome::Added(i64::MAX.try_into().unwrap()),
                        )
                        .unwrap(),
                    _ => {
                        storage
                            .finish_anki_submission(
                                &live,
                                AnkiSubmissionOutcome::Uncertain {
                                    known_note_id: Some(i64::MAX.try_into().unwrap()),
                                },
                            )
                            .unwrap();
                        if outcome == 4 {
                            // Existing evidence may name another note while the
                            // originally inserted/uncertain ID must be retained.
                            storage
                                .reconcile_anki_submission(
                                    live.id(),
                                    AnkiReconciliation::Existing(
                                        AnkiExistingNotes::new(vec![
                                            1.try_into().unwrap(),
                                            9007199254740993_i64.try_into().unwrap(),
                                        ])
                                        .unwrap(),
                                    ),
                                )
                                .unwrap();
                        }
                    }
                }
            }
            accept_unchanged(&database, 24);
        }
    }
}

#[test]
fn backup_candidate_controls_cancel_every_populated_row_boundary_without_mutating_source() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    super::super::backup_snapshot::populate(&mut storage);
    let live = storage
        .begin_anki_submission(&super::super::anki::payload(1))
        .unwrap();
    storage
        .finish_anki_submission(
            &live,
            AnkiSubmissionOutcome::Existing(
                AnkiExistingNotes::new(vec![1.try_into().unwrap(), i64::MAX.try_into().unwrap()])
                    .unwrap(),
            ),
        )
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    let mut polls = 0;
    candidate(&database.0.join("count"), &bytes)
        .check_schema(24, bytes.len() as u64, || {
            polls += 1;
            false
        })
        .unwrap()
        .discard()
        .unwrap();
    for stop in 1..=polls {
        let directory = database.0.join("cancel");
        let mut calls = 0;
        assert!(matches!(
            candidate(&directory, &bytes).check_schema(24, bytes.len() as u64, || {
                calls += 1;
                calls == stop
            }),
            Err(CandidateError::Cancelled)
        ));
        assert!(!directory.exists());
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}
