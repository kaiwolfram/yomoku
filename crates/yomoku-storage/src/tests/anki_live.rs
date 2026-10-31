use super::*;
use crate::{
    anki::{AnkiJournalError, AnkiSubmissionOutcome},
    anki_live::{AnkiLiveIntent, AnkiLiveOutcome},
    anki_model::AnkiModelCreationOutcome,
};
use yomoku_core::anki::{
    AnkiExistingNotes, AnkiModelCreation, AnkiModelLayout, AnkiModelReconciliation,
    AnkiReconciliation, AnkiRecoverySnapshot,
};

fn intent(model: bool) -> AnkiLiveIntent {
    if model {
        let mut layout = AnkiModelLayout::yomoku();
        layout.css.push_str("\n/* frozen 𠮷e\u{301} */");
        AnkiLiveIntent::Model(Box::new(AnkiModelCreation::new(layout).unwrap()))
    } else {
        AnkiLiveIntent::Note(Box::new(super::anki::payload(9007199254740993)))
    }
}

fn terminal(model: bool) -> AnkiLiveOutcome {
    if model {
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Created(
            i64::MAX.try_into().unwrap(),
        ))
    } else {
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Added(i64::MAX.try_into().unwrap()))
    }
}

fn uncertain(model: bool, known: Option<i64>) -> AnkiLiveOutcome {
    if model {
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Uncertain {
            known_model_id: known.map(|id| id.try_into().unwrap()),
        })
    } else {
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Uncertain {
            known_note_id: known.map(|id| id.try_into().unwrap()),
        })
    }
}

fn table(model: bool) -> &'static str {
    if model {
        "anki_model_creations"
    } else {
        "anki_submissions"
    }
}

fn reconcile(storage: &mut Storage, snapshot: &AnkiRecoverySnapshot) {
    if let Some(note) = &snapshot.note {
        storage
            .reconcile_anki_submission(
                note.id,
                AnkiReconciliation::Existing(
                    AnkiExistingNotes::new(vec![i64::MAX.try_into().unwrap()]).unwrap(),
                ),
            )
            .unwrap();
    }
    if let Some(model) = &snapshot.model {
        storage
            .reconcile_anki_model_creation(
                model.id,
                AnkiModelReconciliation::Observed {
                    id: i64::MAX.try_into().unwrap(),
                    layout: Box::new(model.creation.layout().clone()),
                },
            )
            .unwrap();
    }
}

#[test]
fn anki_live_begin_returns_exact_committed_pair_and_blocks_both_kinds() {
    for model in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let frozen = intent(model);
        let live = storage.begin_anki_live(&frozen).unwrap();
        assert_eq!(&storage.anki_recovery_snapshot().unwrap(), live.snapshot());
        match &frozen {
            AnkiLiveIntent::Note(payload) => {
                assert_eq!(
                    &live.snapshot().note.as_ref().unwrap().submission,
                    payload.as_ref()
                );
                assert!(live.snapshot().model.is_none());
            }
            AnkiLiveIntent::Model(payload) => {
                assert_eq!(
                    &live.snapshot().model.as_ref().unwrap().creation,
                    payload.as_ref()
                );
                assert!(live.snapshot().note.is_none());
            }
        }
        let mut other = Storage::open(database.path()).unwrap();
        assert_eq!(&other.anki_recovery_snapshot().unwrap(), live.snapshot());
        for candidate in [intent(false), intent(true)] {
            assert!(other.begin_anki_live(&candidate).is_err());
        }
    }
}

#[test]
fn anki_live_all_terminal_outcomes_return_empty_only_after_commit() {
    for (model, outcome, state) in [
        (false, terminal(false), "added"),
        (true, terminal(true), "created"),
        (
            false,
            AnkiLiveOutcome::Note(AnkiSubmissionOutcome::NotInserted),
            "not_inserted",
        ),
        (
            true,
            AnkiLiveOutcome::Model(AnkiModelCreationOutcome::NotCreated),
            "not_created",
        ),
        (
            false,
            AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Existing(
                AnkiExistingNotes::new(vec![
                    i64::MAX.try_into().unwrap(),
                    9007199254740993.try_into().unwrap(),
                ])
                .unwrap(),
            )),
            "existing",
        ),
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = storage.begin_anki_live(&intent(model)).unwrap();
        assert_eq!(
            storage.finish_anki_live(&live, outcome.clone()).unwrap(),
            Box::default()
        );
        let stored: String = storage
            .connection
            .query_row(&format!("SELECT state FROM {}", table(model)), [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(stored, state);
        let mut other = Storage::open(database.path()).unwrap();
        assert_eq!(other.anki_recovery_snapshot().unwrap(), Default::default());
        assert!(matches!(
            storage.finish_anki_live(&live, outcome),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert!(other.begin_anki_live(&intent(!model)).is_ok());
    }
}

#[test]
fn anki_live_uncertain_outcomes_preserve_frozen_payload_and_known_ids_across_restart() {
    for model in [false, true] {
        for known in [None, Some(9007199254740993), Some(i64::MAX)] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let live = storage.begin_anki_live(&intent(model)).unwrap();
            let mut expected = live.snapshot().clone();
            if model {
                expected.model.as_mut().unwrap().known_model_id =
                    known.map(|id| id.try_into().unwrap());
            } else {
                expected.note.as_mut().unwrap().known_note_id =
                    known.map(|id| id.try_into().unwrap());
            }
            assert_eq!(
                *storage
                    .finish_anki_live(&live, uncertain(model, known))
                    .unwrap(),
                expected
            );
            assert!(matches!(
                storage.finish_anki_live(&live, terminal(model)),
                Err(AnkiJournalError::StaleCallback)
            ));
            drop(storage);
            let mut reopened = Storage::open(database.path()).unwrap();
            assert_eq!(reopened.anki_recovery_snapshot().unwrap(), expected);
            assert!(reopened.begin_anki_live(&intent(false)).is_err());
            assert!(reopened.begin_anki_live(&intent(true)).is_err());
        }
    }
}

#[test]
fn anki_live_wrong_kind_stale_and_changed_payload_callbacks_cannot_release_gate() {
    for model in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = storage.begin_anki_live(&intent(model)).unwrap();
        assert!(matches!(
            storage.finish_anki_live(&live, terminal(!model)),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert_eq!(&storage.anki_recovery_snapshot().unwrap(), live.snapshot());
        // Same ID, valid but different frozen content, changed by another writer.
        let replacement = match intent(!model) {
            AnkiLiveIntent::Note(_) => {
                serde_json::to_string(&AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap())
                    .unwrap()
            }
            AnkiLiveIntent::Model(_) => serde_json::to_string(&super::anki::payload(1)).unwrap(),
        };
        let other = Storage::open(database.path()).unwrap();
        other
            .connection
            .execute(
                &format!("UPDATE {} SET payload = ?1", table(model)),
                [replacement],
            )
            .unwrap();
        assert!(matches!(
            storage.finish_anki_live(&live, terminal(model)),
            Err(AnkiJournalError::StaleCallback)
        ));
        let changed = storage.anki_recovery_snapshot().unwrap();
        reconcile(&mut storage, &changed);
        let newer = storage.begin_anki_live(&intent(model)).unwrap();
        assert!(matches!(
            storage.finish_anki_live(&live, terminal(model)),
            Err(AnkiJournalError::StaleCallback)
        ));
        assert_eq!(&storage.anki_recovery_snapshot().unwrap(), newer.snapshot());
    }
}

#[test]
fn anki_live_outcome_rechecks_other_journal_and_its_integrity() {
    for model in [false, true] {
        for corrupt in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let live = storage.begin_anki_live(&intent(model)).unwrap();
            let payload = if corrupt {
                "{}".into()
            } else {
                match intent(!model) {
                    AnkiLiveIntent::Note(value) => serde_json::to_string(&value).unwrap(),
                    AnkiLiveIntent::Model(value) => serde_json::to_string(&value).unwrap(),
                }
            };
            storage
                .connection
                .execute(
                    &format!(
                        "INSERT INTO {} (payload,state) VALUES (?1,'pending')",
                        table(!model)
                    ),
                    [payload],
                )
                .unwrap();
            assert!(storage.finish_anki_live(&live, terminal(model)).is_err());
            let state: String = storage
                .connection
                .query_row(&format!("SELECT state FROM {}", table(model)), [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(state, "pending");
        }
    }
}

#[test]
fn anki_live_statement_and_deferred_commit_failures_never_return_a_grant_or_release() {
    for model in [false, true] {
        for finish in [false, true] {
            for deferred in [false, true] {
                let database = TestDatabase::new();
                let mut storage = Storage::open(database.path()).unwrap();
                let live = finish.then(|| storage.begin_anki_live(&intent(model)).unwrap());
                let before = storage.anki_recovery_snapshot().unwrap();
                storage.connection.execute_batch("CREATE TABLE deferred_live (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
                let action = if deferred {
                    "INSERT INTO deferred_live VALUES (-1);"
                } else {
                    "SELECT RAISE(ABORT, 'fixture');"
                };
                let operation = if finish { "UPDATE" } else { "INSERT" };
                storage
                    .connection
                    .execute_batch(&format!(
                        "CREATE TRIGGER fail_live AFTER {operation} ON {} BEGIN {action} END;",
                        table(model)
                    ))
                    .unwrap();
                if let Some(live) = &live {
                    assert!(storage.finish_anki_live(live, terminal(model)).is_err());
                } else {
                    assert!(storage.begin_anki_live(&intent(model)).is_err());
                }
                assert_eq!(storage.anki_recovery_snapshot().unwrap(), before);
                let mut reopened = Storage::open(database.path()).unwrap();
                assert_eq!(reopened.anki_recovery_snapshot().unwrap(), before);
                // Continue through read-only recovery after an outcome error.
                storage
                    .connection
                    .execute_batch("DROP TRIGGER fail_live")
                    .unwrap();
                if live.is_some() {
                    reconcile(&mut reopened, &before);
                }
                assert!(reopened.begin_anki_live(&intent(!model)).is_ok());
            }
        }
    }
}

#[test]
fn anki_live_snapshot_validation_rolls_back_trigger_changes_before_commit() {
    for model in [false, true] {
        for finish in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let live = finish.then(|| storage.begin_anki_live(&intent(model)).unwrap());
            let before = storage.anki_recovery_snapshot().unwrap();
            let operation = if finish { "UPDATE" } else { "INSERT" };
            // Valid JSON alone is insufficient: require the exact newly written pair.
            let replacement = if model {
                serde_json::to_string(&AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap())
                    .unwrap()
            } else {
                serde_json::to_string(&super::anki::payload(1)).unwrap()
            };
            storage
                .connection
                .execute_batch("CREATE TABLE replacement_live (payload TEXT)")
                .unwrap();
            storage
                .connection
                .execute("INSERT INTO replacement_live VALUES (?1)", [replacement])
                .unwrap();
            storage.connection.execute_batch(&format!("CREATE TRIGGER change_live AFTER {operation} ON {table} BEGIN UPDATE {table} SET payload = (SELECT payload FROM replacement_live) WHERE id = NEW.id; END;", table = table(model))).unwrap();
            let error = if let Some(live) = &live {
                storage
                    .finish_anki_live(live, uncertain(model, None))
                    .unwrap_err()
            } else {
                storage.begin_anki_live(&intent(model)).unwrap_err()
            };
            assert!(matches!(error, AnkiJournalError::InvalidRecord));
            assert_eq!(storage.anki_recovery_snapshot().unwrap(), before);
        }
    }
}

#[test]
fn anki_live_snapshot_validation_rolls_back_new_obligations_in_the_other_journal() {
    for model in [false, true] {
        for finish in [false, true] {
            for corrupt in [false, true] {
                let database = TestDatabase::new();
                let mut storage = Storage::open(database.path()).unwrap();
                let live = finish.then(|| storage.begin_anki_live(&intent(model)).unwrap());
                let before = storage.anki_recovery_snapshot().unwrap();
                let payload = if corrupt {
                    "{}".into()
                } else {
                    match intent(!model) {
                        AnkiLiveIntent::Note(value) => serde_json::to_string(&value).unwrap(),
                        AnkiLiveIntent::Model(value) => serde_json::to_string(&value).unwrap(),
                    }
                };
                storage
                    .connection
                    .execute_batch("CREATE TABLE injected_live (payload TEXT)")
                    .unwrap();
                storage
                    .connection
                    .execute("INSERT INTO injected_live VALUES (?1)", [payload])
                    .unwrap();
                let operation = if finish { "UPDATE" } else { "INSERT" };
                storage
                    .connection
                    .execute_batch(&format!(
                        "CREATE TRIGGER inject_live AFTER {operation} ON {} BEGIN
                     INSERT INTO {} (payload,state) SELECT payload,'pending' FROM injected_live;
                     END;",
                        table(model),
                        table(!model)
                    ))
                    .unwrap();
                let error = if let Some(live) = &live {
                    storage.finish_anki_live(live, terminal(model)).unwrap_err()
                } else {
                    storage.begin_anki_live(&intent(model)).unwrap_err()
                };
                assert!(matches!(error, AnkiJournalError::InvalidRecord));
                assert_eq!(storage.anki_recovery_snapshot().unwrap(), before);
                let mut reopened = Storage::open(database.path()).unwrap();
                assert_eq!(reopened.anki_recovery_snapshot().unwrap(), before);
            }
        }
    }
}

#[test]
fn anki_live_lost_begin_and_outcome_responses_restore_without_replay() {
    for model in [false, true] {
        for finish in [false, true] {
            for uncertain_outcome in [false, true] {
                let database = TestDatabase::new();
                let mut storage = Storage::open(database.path()).unwrap();
                let live = storage.begin_anki_live(&intent(model)).unwrap();
                let mut expected = live.snapshot().clone();
                if finish {
                    expected = *storage
                        .finish_anki_live(
                            &live,
                            if uncertain_outcome {
                                uncertain(model, Some(i64::MAX))
                            } else {
                                terminal(model)
                            },
                        )
                        .unwrap();
                }
                drop(live); // discard the only live token, as if the callback was lost
                drop(storage);
                let before = fs::read(database.path()).unwrap();
                let mut reopened = Storage::open(database.path()).unwrap();
                assert_eq!(reopened.anki_recovery_snapshot().unwrap(), expected);
                drop(reopened);
                assert_eq!(fs::read(database.path()).unwrap(), before);
                if expected != AnkiRecoverySnapshot::default() {
                    let mut reopened = Storage::open(database.path()).unwrap();
                    assert!(reopened.begin_anki_live(&intent(false)).is_err());
                    assert!(reopened.begin_anki_live(&intent(true)).is_err());
                    reconcile(&mut reopened, &expected);
                    assert_eq!(
                        reopened.anki_recovery_snapshot().unwrap(),
                        Default::default()
                    );
                }
            }
        }
    }
}

#[test]
fn anki_live_concurrent_begins_and_duplicate_outcomes_commit_only_once() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    let ready = Arc::new(Barrier::new(2));
    let handles: Vec<_> = [false, true]
        .into_iter()
        .map(|model| {
            let path = database.path();
            let ready = ready.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                ready.wait();
                storage
                    .begin_anki_live(&intent(model))
                    .map(|live| (live, model))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let (live, model) = results.into_iter().find_map(Result::ok).unwrap();
    let live = Arc::new(live);
    let ready = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let path = database.path();
            let ready = ready.clone();
            let live = live.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                ready.wait();
                storage.finish_anki_live(&live, terminal(model))
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .any(|r| matches!(r, Err(AnkiJournalError::StaleCallback))));
}
