use super::*;
use crate::anki::{AnkiJournalError, AnkiSubmissionOutcome};
use yomoku_core::{
    anki::{
        AnkiDeckId, AnkiExistingNotes, AnkiExportError, AnkiModelId, AnkiModelLayout, AnkiNote,
        AnkiNoteId, AnkiReconciliation, AnkiSubmission,
    },
    document::{PasteText, TextOffset, TextRange},
    vocabulary::Surface,
};

pub(super) fn payload(deck: i64) -> AnkiSubmission {
    let surface = Surface::new("𠮷<&[猫]e\u{301}\u{1f}").unwrap();
    let sentence = format!("前。{surface}と{surface}。", surface = surface.as_str());
    let start = "前。".len() as i64;
    let note = AnkiNote::new(
        &surface,
        "ねこ\nかな",
        &["cat & feline", "<literal>"],
        &sentence,
        TextRange {
            start: TextOffset::try_from(start).unwrap(),
            end: TextOffset::try_from(start + surface.as_str().len() as i64).unwrap(),
        },
    )
    .unwrap();
    AnkiSubmission::new(
        note,
        AnkiDeckId::try_from(deck).unwrap(),
        AnkiModelId::try_from(i64::MAX).unwrap(),
        AnkiModelLayout::yomoku(),
    )
    .unwrap()
}

fn note(value: i64) -> AnkiNoteId {
    AnkiNoteId::try_from(value).unwrap()
}
fn existing() -> AnkiExistingNotes {
    AnkiExistingNotes::new(vec![note(i64::MAX), note(9007199254740993)]).unwrap()
}

#[test]
fn anki_pending_reopens_exactly_and_recovery_is_read_only() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.anki_recovery().unwrap(), None);
    let mut original = payload(9007199254740993);
    // Model layout must survive a later change in the app's canonical constants.
    let mut layout = original.layout().clone();
    layout.css.push_str("\n/* frozen old version */");
    original = AnkiSubmission::new(
        original.note().clone(),
        original.deck(),
        original.model(),
        layout,
    )
    .unwrap();
    let id = storage.begin_anki_submission(&original).unwrap();
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    for _ in 0..3 {
        let storage = Storage::open(database.path()).unwrap();
        let restored = storage.anki_recovery().unwrap().unwrap();
        assert_eq!(restored.id, id.id());
        assert_eq!(restored.submission, original);
        assert_eq!(restored.known_note_id, None);
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn anki_pending_and_uncertain_block_identical_or_replacement_payloads() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let original = payload(1);
    let id = storage.begin_anki_submission(&original).unwrap();
    for known_note_id in [None, Some(note(i64::MAX))] {
        if known_note_id.is_some() {
            storage
                .finish_anki_submission(&id, AnkiSubmissionOutcome::Uncertain { known_note_id })
                .unwrap();
        }
        for candidate in [&original, &payload(2)] {
            assert!(matches!(
                storage.begin_anki_submission(candidate),
                Err(AnkiJournalError::UnresolvedSubmission)
            ));
        }
        assert_eq!(
            storage.anki_recovery().unwrap().unwrap().submission,
            original
        );
    }
}

#[test]
fn anki_interrupted_insert_and_placement_remain_uncertain_after_empty_or_failed_reads() {
    for known_note_id in [None, Some(note(i64::MAX))] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let original = payload(9007199254740993);
        let id = storage.begin_anki_submission(&original).unwrap();
        if known_note_id.is_some() {
            storage
                .finish_anki_submission(&id, AnkiSubmissionOutcome::Uncertain { known_note_id })
                .unwrap();
        }
        drop(storage);
        for observation in [
            AnkiReconciliation::Unresolved,
            AnkiReconciliation::Failed(AnkiExportError::PermissionDenied),
            AnkiReconciliation::Failed(AnkiExportError::DuplicateLookupFailed),
            AnkiReconciliation::Unresolved,
        ] {
            let mut restored = Storage::open(database.path()).unwrap();
            restored
                .reconcile_anki_submission(id.id(), observation)
                .unwrap();
            let recovery = restored.anki_recovery().unwrap().unwrap();
            assert_eq!(recovery.submission, original);
            assert_eq!(recovery.known_note_id, known_note_id);
            assert!(matches!(
                restored.begin_anki_submission(&payload(2)),
                Err(AnkiJournalError::UnresolvedSubmission)
            ));
            assert!(matches!(
                restored.finish_anki_submission(&id, AnkiSubmissionOutcome::NotInserted),
                Err(AnkiJournalError::StaleCallback)
            ));
        }
    }
}

#[test]
fn anki_terminal_callbacks_release_gate_without_reusing_ids_or_accepting_old_callbacks() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut prior = None;
    for outcome in [
        AnkiSubmissionOutcome::NotInserted,
        AnkiSubmissionOutcome::Added(note(i64::MAX)),
        AnkiSubmissionOutcome::Existing(existing()),
    ] {
        let id = storage.begin_anki_submission(&payload(1)).unwrap();
        if let Some(old) = prior {
            assert!(id.id().value() > old);
        }
        storage
            .finish_anki_submission(&id, outcome.clone())
            .unwrap();
        assert!(matches!(
            storage.finish_anki_submission(&id, outcome),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert_eq!(storage.anki_recovery().unwrap(), None);
        prior = Some(id.id().value());
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        let newer = storage.begin_anki_submission(&payload(2)).unwrap();
        assert!(newer.id().value() > id.id().value());
        assert!(matches!(
            storage.finish_anki_submission(&id, AnkiSubmissionOutcome::NotInserted),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert!(matches!(
            storage.reconcile_anki_submission(id.id(), AnkiReconciliation::Existing(existing())),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert_eq!(storage.anki_recovery().unwrap().unwrap().id, newer.id());
        storage
            .finish_anki_submission(&newer, AnkiSubmissionOutcome::NotInserted)
            .unwrap();
    }
}

#[test]
fn anki_existing_reconciliation_preserves_known_id_without_claiming_placement() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let id = storage.begin_anki_submission(&payload(1)).unwrap();
    storage
        .finish_anki_submission(
            &id,
            AnkiSubmissionOutcome::Uncertain {
                known_note_id: Some(note(123)),
            },
        )
        .unwrap();
    storage
        .reconcile_anki_submission(id.id(), AnkiReconciliation::Existing(existing()))
        .unwrap();
    assert_eq!(storage.anki_recovery().unwrap(), None);
    let result: (String, i64, String) = storage
        .connection
        .query_row(
            "SELECT state, known_note_id, existing_ids FROM anki_submissions WHERE id = ?1",
            [id.id().value()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        result,
        (
            "existing".into(),
            123,
            "[9223372036854775807,9007199254740993]".into()
        )
    );
    assert!(matches!(
        storage.reconcile_anki_submission(id.id(), AnkiReconciliation::Existing(existing())),
        Err(AnkiJournalError::StaleCallback)
    ));
}

#[test]
fn anki_concurrent_preparation_has_only_one_committed_live_attempt() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (1..=2)
        .map(|deck| {
            let path = database.path();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage.begin_anki_submission(&payload(deck))
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(AnkiJournalError::UnresolvedSubmission)))
            .count(),
        1
    );
    assert!(Storage::open(database.path())
        .unwrap()
        .anki_recovery()
        .unwrap()
        .is_some());
}

#[test]
fn anki_statement_and_commit_failures_cannot_lose_or_release_an_obligation() {
    for operation in ["INSERT", "UPDATE"] {
        for commit_failure in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            if commit_failure {
                storage.connection.execute_batch("CREATE TABLE deferred_anki (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
            }
            let pending = if operation == "UPDATE" {
                Some(storage.begin_anki_submission(&payload(1)).unwrap())
            } else {
                None
            };
            let action = if commit_failure {
                "INSERT INTO deferred_anki VALUES (-1);"
            } else {
                "SELECT RAISE(ABORT, 'fixture');"
            };
            storage.connection.execute_batch(&format!("CREATE TRIGGER fail_anki AFTER {operation} ON anki_submissions BEGIN {action} END;")).unwrap();
            if let Some(id) = &pending {
                assert!(storage
                    .finish_anki_submission(id, AnkiSubmissionOutcome::Added(note(123)))
                    .is_err());
                assert!(storage
                    .reconcile_anki_submission(id.id(), AnkiReconciliation::Existing(existing()))
                    .is_err());
                assert_eq!(storage.anki_recovery().unwrap().unwrap().id, id.id());
                assert_eq!(
                    Storage::open(database.path())
                        .unwrap()
                        .anki_recovery()
                        .unwrap()
                        .unwrap()
                        .id,
                    id.id()
                );
            } else {
                assert!(storage.begin_anki_submission(&payload(1)).is_err());
                assert_eq!(storage.anki_recovery().unwrap(), None);
                assert_eq!(
                    Storage::open(database.path())
                        .unwrap()
                        .anki_recovery()
                        .unwrap(),
                    None
                );
            }
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_anki")
                .unwrap();
            if let Some(id) = &pending {
                storage
                    .reconcile_anki_submission(id.id(), AnkiReconciliation::Existing(existing()))
                    .unwrap();
            } else {
                assert!(storage.begin_anki_submission(&payload(1)).is_ok());
            }
        }
    }
}

#[test]
fn anki_invalid_restored_payloads_fail_closed_and_do_not_rewrite_data() {
    let original = serde_json::to_value(payload(1)).unwrap();
    let mut invalid = vec![serde_json::Value::Null];
    for (pointer, replacement) in [
        ("/version", serde_json::json!(2)),
        ("/surface", serde_json::json!("")),
        ("/surface", serde_json::json!("猫\0")),
        ("/deck", serde_json::json!(0)),
        ("/model", serde_json::json!(-1)),
        ("/fields/0", serde_json::json!("wrong exact identity")),
        ("/fields/1", serde_json::json!("\u{1f}")),
        ("/fields/2", serde_json::json!("\0")),
        ("/layout/field_names/1", serde_json::json!("Surface")),
        ("/layout/name", serde_json::json!("")),
        ("/layout/css", serde_json::json!("\0")),
    ] {
        let mut candidate = original.clone();
        *candidate.pointer_mut(pointer).unwrap() = replacement;
        invalid.push(candidate);
    }
    let mut extra = original.clone();
    extra["unknown"] = serde_json::json!(true);
    invalid.push(extra);
    for value in invalid {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.begin_anki_submission(&payload(1)).unwrap();
        storage
            .connection
            .execute(
                "UPDATE anki_submissions SET payload = ?1",
                [value.to_string()],
            )
            .unwrap();
        drop(storage);
        let before = fs::read(database.path()).unwrap();
        let mut storage = Storage::open(database.path()).unwrap();
        assert!(matches!(
            storage.anki_recovery(),
            Err(AnkiJournalError::InvalidRecord)
        ));
        assert!(matches!(
            storage.begin_anki_submission(&payload(2)),
            Err(AnkiJournalError::InvalidRecord)
        ));
        drop(storage);
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn anki_missing_table_unknown_state_and_corrupt_ids_cannot_be_treated_as_empty() {
    for corrupt in [
        "DROP TABLE anki_submissions",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_submissions SET state = 'unknown'",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_submissions SET state = 'uncertain', known_note_id = -1",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_submissions SET known_note_id = 1",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_submissions SET existing_ids = '[]'",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.begin_anki_submission(&payload(1)).unwrap();
        storage.connection.execute_batch(corrupt).unwrap();
        assert!(storage.anki_recovery().is_err());
        assert!(storage.begin_anki_submission(&payload(2)).is_err());
    }
}

#[test]
fn anki_reader_data_deletion_does_not_erase_external_obligation() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("猫。".into()).unwrap())
        .unwrap();
    let id = storage.begin_anki_submission(&payload(1)).unwrap();
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
    assert_eq!(storage.anki_recovery().unwrap().unwrap().id, id.id());
    assert!(storage.list_documents().unwrap().is_empty());
}

#[test]
fn anki_export_journal_preserves_reader_vocabulary_and_statistics() {
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
        AnkiSubmissionOutcome::NotInserted,
        AnkiSubmissionOutcome::Added(note(123)),
        AnkiSubmissionOutcome::Existing(existing()),
        AnkiSubmissionOutcome::Uncertain {
            known_note_id: Some(note(456)),
        },
    ] {
        let live = storage.begin_anki_submission(&payload(1)).unwrap();
        storage.finish_anki_submission(&live, outcome).unwrap();
        if storage.anki_recovery().unwrap().is_some() {
            storage
                .reconcile_anki_submission(live.id(), AnkiReconciliation::Existing(existing()))
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
}

#[test]
fn anki_exhausted_submission_ids_never_return_a_live_token() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage.begin_anki_submission(&payload(1)).unwrap();
    storage
        .finish_anki_submission(&live, AnkiSubmissionOutcome::NotInserted)
        .unwrap();
    storage
        .connection
        .execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'anki_submissions'",
            [i64::MAX],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(storage.begin_anki_submission(&payload(2)).is_err());
    assert_eq!(storage.anki_recovery().unwrap(), None);
    drop(storage);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn anki_migration_upgrade_is_empty_and_failure_preserves_prior_schema_and_data() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..20]).unwrap();
    connection.execute_batch("CREATE TABLE saved_fixture (value TEXT); INSERT INTO saved_fixture VALUES ('保持𠮷'); CREATE TABLE anki_submissions (conflict TEXT);").unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    assert!(Storage::open(database.path()).is_err());
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch("DROP TABLE anki_submissions")
        .unwrap();
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 22);
    assert_eq!(storage.anki_recovery().unwrap(), None);
    assert_eq!(
        storage
            .connection
            .query_row("SELECT value FROM saved_fixture", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "保持𠮷"
    );
}

#[test]
fn anki_submission_rejects_nul_and_layout_ambiguity_before_journaling() {
    let original = payload(1);
    let surface = Surface::new("猫\0").unwrap();
    let note = AnkiNote::new(
        &surface,
        "",
        &[],
        surface.as_str(),
        TextRange {
            start: TextOffset::try_from(0).unwrap(),
            end: TextOffset::try_from(4).unwrap(),
        },
    )
    .unwrap();
    assert!(AnkiSubmission::new(
        note,
        original.deck(),
        original.model(),
        original.layout().clone()
    )
    .is_err());
    for field in ["", "Surface", "bad\u{1f}name", "bad\0name"] {
        let mut layout = original.layout().clone();
        layout.field_names[1] = field.into();
        assert!(AnkiSubmission::new(
            original.note().clone(),
            original.deck(),
            original.model(),
            layout
        )
        .is_err());
    }
}
