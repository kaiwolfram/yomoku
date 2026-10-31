use super::*;
use crate::{anki::AnkiSubmissionOutcome, anki_model::AnkiModelCreationOutcome};
use yomoku_core::anki::{AnkiModelCreation, AnkiModelId, AnkiModelLayout, AnkiNoteId};

#[test]
fn anki_combined_restore_preserves_pending_uncertain_and_terminal_gates_on_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.anki_recovery_snapshot().unwrap(),
        Default::default()
    );
    let payload = super::anki::payload(9007199254740993);
    let live = storage.begin_anki_submission(&payload).unwrap();
    let pending = storage.anki_recovery_snapshot().unwrap();
    assert_eq!(pending.note.as_ref().unwrap().submission, payload);
    assert!(pending.model.is_none());
    let known = AnkiNoteId::try_from(i64::MAX).unwrap();
    storage
        .finish_anki_submission(
            &live,
            AnkiSubmissionOutcome::Uncertain {
                known_note_id: Some(known),
            },
        )
        .unwrap();
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let uncertain = storage.anki_recovery_snapshot().unwrap();
    assert_eq!(uncertain.note.as_ref().unwrap().known_note_id, Some(known));
    assert_eq!(uncertain.note.as_ref().unwrap().submission, payload);
    assert_eq!(fs::read(database.path()).unwrap(), before);

    storage
        .reconcile_anki_submission(
            live.id(),
            yomoku_core::anki::AnkiReconciliation::Existing(
                yomoku_core::anki::AnkiExistingNotes::new(vec![known]).unwrap(),
            ),
        )
        .unwrap();
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* frozen */");
    let creation = AnkiModelCreation::new(layout).unwrap();
    let live = storage.begin_anki_model_creation(&creation).unwrap();
    let pending = storage.anki_recovery_snapshot().unwrap();
    assert!(pending.note.is_none());
    assert_eq!(pending.model.as_ref().unwrap().creation, creation);
    let known = AnkiModelId::try_from(i64::MAX).unwrap();
    storage
        .finish_anki_model_creation(
            &live,
            AnkiModelCreationOutcome::Uncertain {
                known_model_id: Some(known),
            },
        )
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let restored = storage.anki_recovery_snapshot().unwrap();
    assert_eq!(restored.model.as_ref().unwrap().creation, creation);
    assert_eq!(restored.model.as_ref().unwrap().known_model_id, Some(known));
}

#[test]
fn anki_combined_restore_never_returns_partial_or_empty_on_either_journal_failure() {
    for failed_table in ["anki_submissions", "anki_model_creations"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage
            .begin_anki_submission(&super::anki::payload(1))
            .unwrap();
        storage
            .connection
            .execute(
                &format!("ALTER TABLE {failed_table} RENAME TO saved_journal"),
                [],
            )
            .unwrap();
        assert!(storage.anki_recovery_snapshot().is_err());
        storage
            .connection
            .execute(
                &format!("ALTER TABLE saved_journal RENAME TO {failed_table}"),
                [],
            )
            .unwrap();
        // The failed transaction is closed and can be explicitly retried.
        assert!(storage.anki_recovery_snapshot().unwrap().note.is_some());
    }
}

#[test]
fn anki_combined_restore_rejects_corruption_in_second_journal_after_first_read() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
    storage.begin_anki_model_creation(&creation).unwrap();
    storage
        .connection
        .execute("UPDATE anki_model_creations SET payload = '{}'", [])
        .unwrap();
    assert!(storage.anki_recovery_snapshot().is_err());
    let payload = serde_json::to_string(&creation).unwrap();
    storage
        .connection
        .execute("UPDATE anki_model_creations SET payload = ?1", [payload])
        .unwrap();
    assert!(storage.anki_recovery_snapshot().unwrap().model.is_some());
}

#[test]
fn anki_combined_restore_never_mixes_two_concurrent_journal_snapshots() {
    let database = TestDatabase::new();
    let mut reader = Storage::open(database.path()).unwrap();
    reader
        .connection
        .execute_batch("PRAGMA journal_mode = WAL")
        .unwrap();
    let note = super::anki::payload(1);
    reader.begin_anki_submission(&note).unwrap();
    let model = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let ready = barrier.clone();
    let path = database.path();
    let note_payload = serde_json::to_string(&note).unwrap();
    let model_payload = serde_json::to_string(&model).unwrap();
    let writer = std::thread::spawn(move || {
        let mut storage = Storage::open(path).unwrap();
        ready.wait();
        for index in 0..500 {
            // Atomically switch the sole obligation to the other journal.
            // Independent reads could falsely see zero or two obligations.
            let tx = storage.connection.transaction().unwrap();
            tx.execute("UPDATE anki_submissions SET state = 'not_inserted'", [])
                .unwrap();
            tx.execute("UPDATE anki_model_creations SET state = 'not_created'", [])
                .unwrap();
            if index % 2 == 0 {
                tx.execute(
                    "INSERT INTO anki_model_creations (payload, state) VALUES (?1, 'pending')",
                    [&model_payload],
                )
                .unwrap();
            } else {
                tx.execute(
                    "INSERT INTO anki_submissions (payload, state) VALUES (?1, 'pending')",
                    [&note_payload],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
    });
    barrier.wait();
    for _ in 0..500 {
        let snapshot = reader.anki_recovery_snapshot().unwrap();
        assert_ne!(snapshot.note.is_some(), snapshot.model.is_some());
        if let Some(recovery) = snapshot.note {
            assert_eq!(recovery.submission, note);
        }
        if let Some(recovery) = snapshot.model {
            assert_eq!(recovery.creation, model);
        }
    }
    writer.join().unwrap();
}
