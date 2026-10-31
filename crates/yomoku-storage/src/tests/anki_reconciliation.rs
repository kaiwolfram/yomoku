use super::*;
use crate::anki::{AnkiJournalError, AnkiSubmissionOutcome};
use crate::anki_model::AnkiModelCreationOutcome;
use yomoku_core::anki::{
    AnkiExistingNotes, AnkiExportError, AnkiModelCreation, AnkiModelId, AnkiModelLayout,
    AnkiModelReconciliation, AnkiNoteId, AnkiReconciliation, AnkiRecoveryCommit,
    AnkiRecoveryObservation,
};

fn note_observation() -> AnkiRecoveryObservation {
    AnkiRecoveryObservation::Note(AnkiReconciliation::Existing(
        AnkiExistingNotes::new(vec![
            AnkiNoteId::try_from(i64::MAX).unwrap(),
            AnkiNoteId::try_from(9007199254740993).unwrap(),
        ])
        .unwrap(),
    ))
}

fn model_observation(creation: &AnkiModelCreation, id: i64) -> AnkiRecoveryObservation {
    AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
        id: AnkiModelId::try_from(id).unwrap(),
        layout: Box::new(creation.layout().clone()),
    })
}

#[test]
fn anki_reconciliation_commits_exact_note_evidence_and_retains_known_id_payload() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let payload = super::anki::payload(9007199254740993);
    let live = storage.begin_anki_submission(&payload).unwrap();
    storage
        .finish_anki_submission(
            &live,
            AnkiSubmissionOutcome::Uncertain {
                known_note_id: Some(7.try_into().unwrap()),
            },
        )
        .unwrap();
    let command = AnkiRecoveryCommit {
        expected: Box::new(storage.anki_recovery_snapshot().unwrap()),
        observation: note_observation(),
    };
    assert_eq!(
        storage.commit_anki_recovery(&command).unwrap(),
        Box::default()
    );
    assert!(matches!(
        storage.commit_anki_recovery(&command),
        Err(AnkiJournalError::StaleCallback)
    ));
    let row: (String, i64, String, String) = storage
        .connection
        .query_row(
            "SELECT state, known_note_id, existing_ids, payload FROM anki_submissions",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        (row.0.as_str(), row.1, row.2.as_str()),
        ("existing", 7, "[9223372036854775807,9007199254740993]")
    );
    assert_eq!(
        serde_json::from_str::<yomoku_core::anki::AnkiSubmission>(&row.3).unwrap(),
        payload
    );
    drop(storage); // a lost commit response is recovered by reads alone
    let mut reopened = Storage::open(database.path()).unwrap();
    assert_eq!(
        reopened.anki_recovery_snapshot().unwrap(),
        Default::default()
    );
    let newer = reopened.begin_anki_submission(&payload).unwrap();
    assert!(matches!(
        reopened.commit_anki_recovery(&command),
        Err(AnkiJournalError::StaleCallback)
    ));
    assert_eq!(reopened.anki_recovery().unwrap().unwrap().id, newer.id());
}

#[test]
fn anki_reconciliation_failed_empty_note_reads_keep_exact_obligation() {
    for observation in [
        AnkiReconciliation::Unresolved,
        AnkiReconciliation::Failed(AnkiExportError::PermissionDenied),
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = storage
            .begin_anki_submission(&super::anki::payload(1))
            .unwrap();
        storage
            .finish_anki_submission(
                &live,
                AnkiSubmissionOutcome::Uncertain {
                    known_note_id: Some(i64::MAX.try_into().unwrap()),
                },
            )
            .unwrap();
        let expected = Box::new(storage.anki_recovery_snapshot().unwrap());
        let command = AnkiRecoveryCommit {
            expected: expected.clone(),
            observation: AnkiRecoveryObservation::Note(observation),
        };
        assert_eq!(storage.commit_anki_recovery(&command).unwrap(), expected);
        assert!(storage
            .begin_anki_model_creation(&AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap())
            .is_err());
        let mut reopened = Storage::open(database.path()).unwrap();
        assert_eq!(reopened.anki_recovery_snapshot().unwrap(), *expected);
    }
}

#[test]
fn anki_reconciliation_model_requires_frozen_layout_and_original_known_id() {
    for known in [None, Some(9007199254740993)] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let mut layout = AnkiModelLayout::yomoku();
        layout.css.push_str("/* old 𠮷 layout */");
        let creation = AnkiModelCreation::new(layout).unwrap();
        let live = storage.begin_anki_model_creation(&creation).unwrap();
        if let Some(id) = known {
            storage
                .finish_anki_model_creation(
                    &live,
                    AnkiModelCreationOutcome::Uncertain {
                        known_model_id: Some(id.try_into().unwrap()),
                    },
                )
                .unwrap();
        }
        let expected = Box::new(storage.anki_recovery_snapshot().unwrap());
        for observation in [
            AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved),
            model_observation(
                &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                known.unwrap_or(1),
            ),
        ] {
            let command = AnkiRecoveryCommit {
                expected: expected.clone(),
                observation,
            };
            assert_eq!(storage.commit_anki_recovery(&command).unwrap(), expected);
        }
        if known.is_some() {
            let wrong_id = AnkiRecoveryCommit {
                expected: expected.clone(),
                observation: model_observation(&creation, 1),
            };
            assert_eq!(storage.commit_anki_recovery(&wrong_id).unwrap(), expected);
        }
        let command = AnkiRecoveryCommit {
            expected,
            observation: model_observation(&creation, known.unwrap_or(i64::MAX)),
        };
        assert_eq!(
            storage.commit_anki_recovery(&command).unwrap(),
            Box::default()
        );
        let mut reopened = Storage::open(database.path()).unwrap();
        assert_eq!(
            reopened.anki_recovery_snapshot().unwrap(),
            Default::default()
        );
    }
}

#[test]
fn anki_reconciliation_rechecks_known_ids_payload_and_other_journal_under_transaction() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage
        .begin_anki_submission(&super::anki::payload(1))
        .unwrap();
    let pending = Box::new(storage.anki_recovery_snapshot().unwrap());
    let command = AnkiRecoveryCommit {
        expected: pending.clone(),
        observation: note_observation(),
    };
    storage
        .finish_anki_submission(
            &live,
            AnkiSubmissionOutcome::Uncertain {
                known_note_id: Some(42.try_into().unwrap()),
            },
        )
        .unwrap();
    assert!(matches!(
        storage.commit_anki_recovery(&command),
        Err(AnkiJournalError::StaleCallback)
    ));
    let actual = Box::new(storage.anki_recovery_snapshot().unwrap());
    let mut changed = actual.clone();
    changed.note.as_mut().unwrap().submission = super::anki::payload(2);
    assert!(matches!(
        storage.commit_anki_recovery(&AnkiRecoveryCommit {
            expected: changed,
            observation: note_observation()
        }),
        Err(AnkiJournalError::StaleCallback)
    ));
    // Simulate a second obligation created externally after read dispatch.
    let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
    storage
        .connection
        .execute(
            "INSERT INTO anki_model_creations (payload,state) VALUES (?1,'pending')",
            [serde_json::to_string(&creation).unwrap()],
        )
        .unwrap();
    assert!(matches!(
        storage.commit_anki_recovery(&AnkiRecoveryCommit {
            expected: actual,
            observation: note_observation()
        }),
        Err(AnkiJournalError::StaleCallback)
    ));
    let pair = Box::new(storage.anki_recovery_snapshot().unwrap());
    let note_commit = AnkiRecoveryCommit {
        expected: pair,
        observation: note_observation(),
    };
    let remaining = storage.commit_anki_recovery(&note_commit).unwrap();
    assert!(remaining.note.is_none());
    assert!(remaining.model.is_some());
    assert!(storage
        .begin_anki_submission(&super::anki::payload(1))
        .is_err());
}

#[test]
fn anki_reconciliation_statement_and_deferred_commit_failures_roll_back_both_gates() {
    for model in [false, true] {
        for deferred in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
            if model {
                storage.begin_anki_model_creation(&creation).unwrap();
            } else {
                storage
                    .begin_anki_submission(&super::anki::payload(1))
                    .unwrap();
            }
            let expected = Box::new(storage.anki_recovery_snapshot().unwrap());
            let command = AnkiRecoveryCommit {
                expected: expected.clone(),
                observation: if model {
                    model_observation(&creation, 1)
                } else {
                    note_observation()
                },
            };
            storage.connection.execute_batch("CREATE TABLE deferred_recovery (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
            let action = if deferred {
                "INSERT INTO deferred_recovery VALUES (-1);"
            } else {
                "SELECT RAISE(ABORT, 'fixture');"
            };
            let table = if model {
                "anki_model_creations"
            } else {
                "anki_submissions"
            };
            storage
                .connection
                .execute_batch(&format!(
                    "CREATE TRIGGER fail_recovery AFTER UPDATE ON {table} BEGIN {action} END;"
                ))
                .unwrap();
            assert!(storage.commit_anki_recovery(&command).is_err());
            assert_eq!(storage.anki_recovery_snapshot().unwrap(), *expected);
            let mut reopened = Storage::open(database.path()).unwrap();
            assert_eq!(reopened.anki_recovery_snapshot().unwrap(), *expected);
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_recovery")
                .unwrap();
            assert_eq!(
                storage.commit_anki_recovery(&command).unwrap(),
                Box::default()
            );
        }
    }
}

#[test]
fn anki_reconciliation_concurrent_duplicate_observations_release_only_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .begin_anki_submission(&super::anki::payload(1))
        .unwrap();
    let command = AnkiRecoveryCommit {
        expected: Box::new(storage.anki_recovery_snapshot().unwrap()),
        observation: note_observation(),
    };
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = database.path();
            let ready = barrier.clone();
            let command = command.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                ready.wait();
                storage.commit_anki_recovery(&command)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .any(|r| matches!(r, Err(AnkiJournalError::StaleCallback))));
}

#[test]
fn anki_reconciliation_missing_target_or_corrupt_second_journal_cannot_commit_partial_state() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .begin_anki_submission(&super::anki::payload(1))
        .unwrap();
    let expected = Box::new(storage.anki_recovery_snapshot().unwrap());
    assert!(storage
        .commit_anki_recovery(&AnkiRecoveryCommit {
            expected: expected.clone(),
            observation: AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved)
        })
        .is_err());
    let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
    storage
        .connection
        .execute(
            "INSERT INTO anki_model_creations (payload,state) VALUES (?1,'pending')",
            [serde_json::to_string(&creation).unwrap()],
        )
        .unwrap();
    let expected = Box::new(storage.anki_recovery_snapshot().unwrap());
    storage
        .connection
        .execute("UPDATE anki_model_creations SET payload = '{}'", [])
        .unwrap();
    assert!(storage
        .commit_anki_recovery(&AnkiRecoveryCommit {
            expected,
            observation: note_observation()
        })
        .is_err());
    assert!(storage.anki_recovery().unwrap().is_some());
}
