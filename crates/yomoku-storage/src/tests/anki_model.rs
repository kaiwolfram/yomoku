use super::*;
use crate::{
    anki::{AnkiJournalError, AnkiSubmissionOutcome},
    anki_model::{AnkiModelCreationOutcome, AnkiModelReconciliation},
};
use yomoku_core::{
    anki::{AnkiModelCreation, AnkiModelId, AnkiModelLayout},
    document::PasteText,
    vocabulary::Surface,
};

fn creation() -> AnkiModelCreation {
    let mut layout = AnkiModelLayout::yomoku();
    layout.name.push_str(" 語𠮷e\u{301}");
    layout
        .css
        .push_str("\n/* frozen layout from an older app */");
    AnkiModelCreation::new(layout).unwrap()
}

fn model(id: i64) -> AnkiModelId {
    AnkiModelId::try_from(id).unwrap()
}

fn observed(id: i64) -> AnkiModelReconciliation {
    AnkiModelReconciliation::Observed {
        id: model(id),
        layout: Box::new(creation().layout().clone()),
    }
}

#[test]
fn anki_model_pending_recovery_preserves_exact_old_layout_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.anki_model_recovery().unwrap(), None);
    let live = storage.begin_anki_model_creation(&creation()).unwrap();
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    for _ in 0..3 {
        let storage = Storage::open(database.path()).unwrap();
        let recovery = storage.anki_model_recovery().unwrap().unwrap();
        assert_eq!(recovery.id, live.id());
        assert_eq!(recovery.creation, creation());
        assert_eq!(recovery.known_model_id, None);
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn anki_model_interrupted_creation_cannot_be_replayed_or_replaced_by_partial_reads() {
    for known_model_id in [None, Some(model(i64::MAX))] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = storage.begin_anki_model_creation(&creation()).unwrap();
        if known_model_id.is_some() {
            storage
                .finish_anki_model_creation(
                    &live,
                    AnkiModelCreationOutcome::Uncertain { known_model_id },
                )
                .unwrap();
        }
        drop(storage);
        let mut mismatched = creation().layout().clone();
        mismatched.answer_format.push_str("partial");
        let mismatch = AnkiModelReconciliation::Observed {
            id: model(i64::MAX),
            layout: Box::new(mismatched),
        };
        let mut observations = vec![AnkiModelReconciliation::Unresolved, mismatch];
        if known_model_id.is_some() {
            observations.push(observed(9007199254740993));
        }
        for observation in observations {
            let mut storage = Storage::open(database.path()).unwrap();
            storage
                .reconcile_anki_model_creation(live.id(), observation)
                .unwrap();
            let recovery = storage.anki_model_recovery().unwrap().unwrap();
            assert_eq!(recovery.known_model_id, known_model_id);
            assert_eq!(recovery.creation, creation());
            for candidate in [
                creation(),
                AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
            ] {
                assert!(matches!(
                    storage.begin_anki_model_creation(&candidate),
                    Err(AnkiJournalError::UnresolvedModelCreation)
                ));
            }
            assert!(matches!(
                storage.begin_anki_submission(&anki::payload(1)),
                Err(AnkiJournalError::UnresolvedModelCreation)
            ));
            assert!(matches!(
                storage.finish_anki_model_creation(&live, AnkiModelCreationOutcome::NotCreated),
                Err(AnkiJournalError::StaleCallback)
            ));
        }
    }
}

#[test]
fn anki_model_outcomes_release_gate_once_and_retain_payload_and_result() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut previous = 0;
    for outcome in [
        AnkiModelCreationOutcome::NotCreated,
        AnkiModelCreationOutcome::Created(model(i64::MAX)),
    ] {
        let live = storage.begin_anki_model_creation(&creation()).unwrap();
        assert!(live.id().value() > previous);
        previous = live.id().value();
        storage
            .finish_anki_model_creation(&live, outcome.clone())
            .unwrap();
        assert_eq!(storage.anki_model_recovery().unwrap(), None);
        let (payload, state, id): (String, String, Option<i64>) = storage
            .connection
            .query_row(
                "SELECT payload, state, known_model_id FROM anki_model_creations WHERE id = ?1",
                [previous],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<AnkiModelCreation>(&payload).unwrap(),
            creation()
        );
        match outcome {
            AnkiModelCreationOutcome::Created(_) => {
                assert_eq!(state, "created");
                assert_eq!(id, Some(i64::MAX));
            }
            _ => {
                assert_eq!(state, "not_created");
                assert_eq!(id, None);
            }
        }
        let newer = storage.begin_anki_model_creation(&creation()).unwrap();
        assert!(newer.id().value() > previous);
        assert!(matches!(
            storage.finish_anki_model_creation(&live, outcome),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert!(matches!(
            storage.reconcile_anki_model_creation(live.id(), observed(1)),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert_eq!(
            storage.anki_model_recovery().unwrap().unwrap().id,
            newer.id()
        );
        storage
            .finish_anki_model_creation(&newer, AnkiModelCreationOutcome::NotCreated)
            .unwrap();
    }
}

#[test]
fn anki_model_full_readback_resolves_usability_and_preserves_known_id() {
    for known_model_id in [None, Some(model(9007199254740993))] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = storage.begin_anki_model_creation(&creation()).unwrap();
        if known_model_id.is_some() {
            storage
                .finish_anki_model_creation(
                    &live,
                    AnkiModelCreationOutcome::Uncertain { known_model_id },
                )
                .unwrap();
        }
        storage
            .reconcile_anki_model_creation(live.id(), observed(9007199254740993))
            .unwrap();
        assert_eq!(storage.anki_model_recovery().unwrap(), None);
        let (state, known, existing): (String, Option<i64>, i64) = storage.connection.query_row(
            "SELECT state, known_model_id, existing_model_id FROM anki_model_creations WHERE id = ?1", [live.id().value()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(state, "existing");
        assert_eq!(known, known_model_id.map(i64::from));
        assert_eq!(existing, 9007199254740993);
        assert!(matches!(
            storage.finish_anki_model_creation(&live, AnkiModelCreationOutcome::NotCreated),
            Err(AnkiJournalError::StaleCallback)
        ));
        let note = storage.begin_anki_submission(&anki::payload(1)).unwrap();
        assert!(matches!(
            storage.begin_anki_model_creation(&creation()),
            Err(AnkiJournalError::UnresolvedSubmission)
        ));
        storage
            .finish_anki_submission(&note, AnkiSubmissionOutcome::NotInserted)
            .unwrap();
        assert!(storage.begin_anki_model_creation(&creation()).is_ok());
    }
}

#[test]
fn anki_model_and_note_concurrent_preparations_share_one_mutation_gate() {
    for both_models in [false, true] {
        let database = TestDatabase::new();
        drop(Storage::open(database.path()).unwrap());
        let barrier = Arc::new(Barrier::new(2));
        let workers = (0..2)
            .map(|index| {
                let path = database.path();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    if index == 0 || both_models {
                        storage.begin_anki_model_creation(&creation()).map(|_| ())
                    } else {
                        storage.begin_anki_submission(&anki::payload(1)).map(|_| ())
                    }
                })
            })
            .collect::<Vec<_>>();
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert!(results.iter().any(|result| matches!(
            result,
            Err(AnkiJournalError::UnresolvedModelCreation | AnkiJournalError::UnresolvedSubmission)
        )));
        let storage = Storage::open(database.path()).unwrap();
        assert_ne!(
            storage.anki_recovery().unwrap().is_some(),
            storage.anki_model_recovery().unwrap().is_some()
        );
    }
}

#[test]
fn anki_model_statement_and_commit_failures_preserve_obligation() {
    for operation in ["INSERT", "UPDATE"] {
        for commit_failure in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            storage.connection.execute_batch("CREATE TABLE deferred_model (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
            let live = if operation == "UPDATE" {
                Some(storage.begin_anki_model_creation(&creation()).unwrap())
            } else {
                None
            };
            let action = if commit_failure {
                "INSERT INTO deferred_model VALUES (-1);"
            } else {
                "SELECT RAISE(ABORT, 'fixture');"
            };
            storage.connection.execute_batch(&format!("CREATE TRIGGER fail_model AFTER {operation} ON anki_model_creations BEGIN {action} END;")).unwrap();
            if let Some(live) = &live {
                assert!(storage
                    .finish_anki_model_creation(live, AnkiModelCreationOutcome::Created(model(1)))
                    .is_err());
                assert!(storage
                    .reconcile_anki_model_creation(live.id(), observed(1))
                    .is_err());
                assert_eq!(
                    storage.anki_model_recovery().unwrap().unwrap().id,
                    live.id()
                );
            } else {
                assert!(storage.begin_anki_model_creation(&creation()).is_err());
                assert_eq!(storage.anki_model_recovery().unwrap(), None);
            }
            assert_eq!(
                Storage::open(database.path())
                    .unwrap()
                    .anki_model_recovery()
                    .unwrap(),
                storage.anki_model_recovery().unwrap()
            );
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_model")
                .unwrap();
            if let Some(live) = live {
                storage
                    .reconcile_anki_model_creation(live.id(), observed(1))
                    .unwrap();
            }
            assert!(storage.begin_anki_model_creation(&creation()).is_ok());
        }
    }
}

#[test]
fn anki_model_corrupt_payloads_fail_closed_without_rewriting_data() {
    let valid = serde_json::to_value(creation()).unwrap();
    let mut invalid = vec![
        serde_json::json!({}),
        serde_json::json!({"version": 2, "layout": valid["layout"]}),
    ];
    for (path, value) in [
        ("name", ""),
        ("name", "bad\0"),
        ("css", "bad\0"),
        ("card_name", ""),
        ("question_format", "bad\0"),
        ("answer_format", "bad\0"),
    ] {
        let mut payload = valid.clone();
        payload["layout"][path] = value.into();
        invalid.push(payload);
    }
    for value in ["", "Surface", "bad\u{1f}", "bad\0"] {
        let mut payload = valid.clone();
        payload["layout"]["field_names"][1] = value.into();
        invalid.push(payload);
    }
    let mut unknown = valid.clone();
    unknown["unknown"] = true.into();
    invalid.push(unknown);
    let mut unknown_layout = valid.clone();
    unknown_layout["layout"]["unknown"] = true.into();
    invalid.push(unknown_layout);
    for payload in invalid {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.begin_anki_model_creation(&creation()).unwrap();
        storage
            .connection
            .execute(
                "UPDATE anki_model_creations SET payload = ?1",
                [payload.to_string()],
            )
            .unwrap();
        drop(storage);
        let before = fs::read(database.path()).unwrap();
        let mut storage = Storage::open(database.path()).unwrap();
        assert!(matches!(
            storage.anki_model_recovery(),
            Err(AnkiJournalError::InvalidRecord)
        ));
        assert!(matches!(
            storage.begin_anki_model_creation(&creation()),
            Err(AnkiJournalError::InvalidRecord)
        ));
        assert!(matches!(
            storage.begin_anki_submission(&anki::payload(1)),
            Err(AnkiJournalError::InvalidRecord)
        ));
        drop(storage);
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn anki_model_missing_table_or_corrupt_state_never_opens_either_gate() {
    for sql in [
        "DROP TABLE anki_model_creations",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_model_creations SET state = 'unknown'",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_model_creations SET known_model_id = 1",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_model_creations SET state = 'uncertain', known_model_id = -1",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_model_creations SET existing_model_id = 1",
        "UPDATE anki_model_creations SET id = -1",
        "DROP INDEX anki_one_unresolved_model; INSERT INTO anki_model_creations (payload, state) SELECT payload, state FROM anki_model_creations",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.begin_anki_model_creation(&creation()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        assert!(storage.anki_model_recovery().is_err());
        assert!(storage.begin_anki_model_creation(&creation()).is_err());
        assert!(storage.begin_anki_submission(&anki::payload(1)).is_err());
    }
}

#[test]
fn anki_model_journal_preserves_reader_history_and_survives_reader_deletion() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("猫。".into()).unwrap())
        .unwrap();
    let surface = Surface::new("猫").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&surface), 100)
        .unwrap();
    let reader = storage
        .load_document_reader_snapshot(document.document_id)
        .unwrap();
    let entry = storage.vocabulary_entry(&surface).unwrap();
    let timeline = storage.vocabulary_status_timeline().unwrap();
    let totals = storage.lifetime_reading_totals().unwrap();
    for outcome in [
        AnkiModelCreationOutcome::NotCreated,
        AnkiModelCreationOutcome::Created(model(1)),
        AnkiModelCreationOutcome::Uncertain {
            known_model_id: Some(model(1)),
        },
    ] {
        let live = storage.begin_anki_model_creation(&creation()).unwrap();
        storage.finish_anki_model_creation(&live, outcome).unwrap();
        if storage.anki_model_recovery().unwrap().is_some() {
            storage
                .reconcile_anki_model_creation(live.id(), observed(1))
                .unwrap();
        }
        assert_eq!(
            storage
                .load_document_reader_snapshot(document.document_id)
                .unwrap(),
            reader
        );
        assert_eq!(storage.vocabulary_entry(&surface).unwrap(), entry);
        assert_eq!(storage.vocabulary_status_timeline().unwrap(), timeline);
        assert_eq!(storage.lifetime_reading_totals().unwrap(), totals);
    }
    let live = storage.begin_anki_model_creation(&creation()).unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute("DELETE FROM vocabulary", [])
        .unwrap();
    assert_eq!(
        storage.anki_model_recovery().unwrap().unwrap().id,
        live.id()
    );
}

#[test]
fn anki_model_exhausted_ids_never_return_a_live_token() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage.begin_anki_model_creation(&creation()).unwrap();
    storage
        .finish_anki_model_creation(&live, AnkiModelCreationOutcome::NotCreated)
        .unwrap();
    storage
        .connection
        .execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'anki_model_creations'",
            [i64::MAX],
        )
        .unwrap();
    assert!(storage.begin_anki_model_creation(&creation()).is_err());
    assert_eq!(storage.anki_model_recovery().unwrap(), None);
}

#[test]
fn anki_model_migration_preserves_pending_note_and_rolls_back_on_conflict() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..21]).unwrap();
    let note = anki::payload(9007199254740993);
    connection
        .execute(
            "INSERT INTO anki_submissions (payload, state) VALUES (?1, 'pending')",
            [serde_json::to_string(&note).unwrap()],
        )
        .unwrap();
    connection
        .execute_batch("CREATE TABLE anki_model_creations (conflict TEXT)")
        .unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    assert!(Storage::open(database.path()).is_err());
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch("DROP TABLE anki_model_creations")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 22);
    assert_eq!(storage.anki_model_recovery().unwrap(), None);
    assert_eq!(storage.anki_recovery().unwrap().unwrap().submission, note);
    assert!(matches!(
        storage.begin_anki_model_creation(&creation()),
        Err(AnkiJournalError::UnresolvedSubmission)
    ));
}
