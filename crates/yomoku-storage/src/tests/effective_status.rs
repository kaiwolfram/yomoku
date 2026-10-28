use super::{imports::fixture, *};
use crate::reader_words::ReaderWordError;
use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride},
    document::{OverrideId, PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveReaderTokenSnapshot,
    vocabulary::{Surface, VocabularyStatus as Status},
};

fn setup(storage: &mut Storage) -> (EffectiveReaderTokenSnapshot, OverrideId) {
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new("𠮷か\u{3099}\0猫 𠮷か\u{3099}\0猫 English 。".into()).unwrap(),
            100,
            fixture,
        )
        .unwrap();
    let chapter = &imported.document.chapter;
    let anchor = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::START,
            end: TextOffset::at(&chapter.text, "𠮷か\u{3099}\0猫".len()).unwrap(),
        },
    };
    let (rule, _) = storage
        .save_registered_segmentation_override(
            &SegmentationOverride {
                scope: OverrideScope::AllOccurrences,
                anchor,
                surface: Surface::new("𠮷か\u{3099}\0猫").unwrap(),
                parts: ["𠮷か\u{3099}\0", "猫"]
                    .map(|s| Surface::new(s).unwrap())
                    .into(),
            },
            110,
        )
        .unwrap();
    let page = TextAnchor {
        range: TextRange {
            start: TextOffset::START,
            end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
        },
        ..anchor
    };
    (
        storage
            .load_effective_reader_tokens(imported.document.document_id, page)
            .unwrap()
            .unwrap(),
        rule,
    )
}

fn history(storage: &Storage) -> i64 {
    storage
        .connection
        .query_row("SELECT COUNT(*) FROM vocabulary_status_events", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn effective_status_choices_share_literal_surface_identity_and_atomic_history_without_reading() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (snapshot, _) = setup(&mut storage);
    let key = snapshot.key().unwrap();
    let token = &snapshot.tokens[0];
    assert!(token.metadata.is_none());
    let id = token.vocabulary.unwrap().id;
    let original = storage
        .chapter_analysis(key.segmentation.original_analysis_id)
        .unwrap();
    let before = storage
        .load_document_reader_snapshot(key.document_id)
        .unwrap();
    let initial_events = history(&storage);
    for (index, status) in [
        Status::Learning,
        Status::Familiar,
        Status::Known,
        Status::Ignored,
        Status::New,
    ]
    .into_iter()
    .enumerate()
    {
        let at = 200 + index as i64;
        let entry = storage
            .set_effective_reader_status(&key, token.segment_index, id, status, at)
            .unwrap()
            .unwrap();
        assert_eq!(entry.status, status);
        assert_eq!(history(&storage), initial_events + index as i64 + 1);
        assert_eq!(
            storage
                .set_effective_reader_status(&key, token.segment_index, id, status, at)
                .unwrap(),
            Some(entry)
        );
        assert_eq!(history(&storage), initial_events + index as i64 + 1);
        let fresh = storage
            .load_effective_reader_tokens(key.document_id, key.page)
            .unwrap()
            .unwrap();
        for repeat in fresh.tokens.iter().filter(|t| t.surface == token.surface) {
            assert_eq!(repeat.vocabulary, Some(entry));
        }
        let event: (String, i64) = storage.connection.query_row("SELECT cause, occurred_at_unix_ms FROM vocabulary_status_events WHERE vocabulary_id = ?1 ORDER BY id DESC LIMIT 1", [i64::from(id)], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(event, ("manual".into(), at));
    }
    assert_eq!(
        storage
            .chapter_analysis(key.segmentation.original_analysis_id)
            .unwrap(),
        original
    );
    assert_eq!(
        storage
            .load_document_reader_snapshot(key.document_id)
            .unwrap(),
        before
    );
    for table in [
        "reading_occurrences",
        "reading_completions",
        "pending_reading_time",
        "completion_commands",
        "effective_completion_commands",
    ] {
        assert_eq!(
            storage
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .set_effective_reader_status(&key, token.segment_index, id, Status::New, 204)
            .unwrap()
            .unwrap()
            .status,
        Status::New
    );
    assert_eq!(history(&storage), initial_events + 5);
}

#[test]
fn effective_status_rejects_wrong_spelling_off_page_foreign_and_reversed_keys() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (snapshot, rule) = setup(&mut storage);
    let key = snapshot.key().unwrap();
    let token = &snapshot.tokens[0];
    let id = token.vocabulary.unwrap().id;
    let before = history(&storage);
    let wrong = snapshot.tokens[1].vocabulary.unwrap().id;
    for (index, id) in [(token.segment_index, wrong), (u64::MAX, id), (5, id)] {
        assert!(matches!(
            storage.set_effective_reader_status(&key, index, id, Status::Known, 200),
            Err(ReaderWordError::InvalidSelection)
        ));
    }
    let mut clipped = key.clone();
    clipped.page.range.start = token.anchor.range.end;
    assert!(matches!(
        storage.set_effective_reader_status(&clipped, token.segment_index, id, Status::Known, 200),
        Err(ReaderWordError::InvalidSelection)
    ));
    let mut foreign = key.clone();
    foreign.document_id = i64::MAX.try_into().unwrap();
    assert!(matches!(
        storage.set_effective_reader_status(&foreign, token.segment_index, id, Status::Known, 200),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert!(storage
        .set_effective_reader_status(&key, token.segment_index, id, Status::Known, -1)
        .is_err());
    storage.reverse_segmentation_override(rule).unwrap();
    assert!(matches!(
        storage.set_effective_reader_status(&key, token.segment_index, id, Status::Known, 200),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert_eq!(history(&storage), before);
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new(&token.surface).unwrap())
            .unwrap()
            .unwrap()
            .status,
        Status::New
    );
}

#[test]
fn effective_status_deleted_entry_stays_absent_and_old_id_cannot_change_replacement() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (snapshot, _) = setup(&mut storage);
    let key = snapshot.key().unwrap();
    let token = &snapshot.tokens[0];
    let id = token.vocabulary.unwrap().id;
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(id)])
        .unwrap();
    let before = history(&storage);
    assert_eq!(
        storage
            .set_effective_reader_status(&key, token.segment_index, id, Status::Known, 200)
            .unwrap(),
        None
    );
    assert_eq!(history(&storage), before);
    let surface = Surface::new(&token.surface).unwrap();
    assert!(storage.vocabulary_entry(&surface).unwrap().is_none());
    storage
        .register_vocabulary(std::slice::from_ref(&surface), 210)
        .unwrap();
    let replacement = storage.vocabulary_entry(&surface).unwrap().unwrap();
    assert_ne!(replacement.id, id);
    assert!(matches!(
        storage.set_effective_reader_status(&key, token.segment_index, id, Status::Known, 220),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert_eq!(
        storage.vocabulary_entry(&surface).unwrap(),
        Some(replacement)
    );
}

#[test]
fn effective_status_statement_and_commit_failures_roll_back_and_exact_retry_is_noop() {
    for trigger in [
        "CREATE TEMP TRIGGER fail AFTER UPDATE ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON vocabulary_status_events BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON vocabulary_status_events BEGIN INSERT INTO deferred_child VALUES (99); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let (snapshot, _) = setup(&mut storage);
        let key = snapshot.key().unwrap();
        let token = &snapshot.tokens[0];
        let id = token.vocabulary.unwrap().id;
        let before = history(&storage);
        storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(storage.set_effective_reader_status(&key, token.segment_index, id, Status::Known, 200).is_err());
        assert_eq!(history(&storage), before);
        assert_eq!(storage.load_effective_reader_tokens(key.document_id, key.page).unwrap(), Some(snapshot.clone()));
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        for _ in 0..2 { storage.set_effective_reader_status(&key, token.segment_index, id, Status::Known, 200).unwrap(); }
        assert_eq!(history(&storage), before + 1);
    }
}
