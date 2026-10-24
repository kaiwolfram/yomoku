use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode,
    },
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    vocabulary::{Surface, VocabularyStatus},
};

use super::*;

fn analysis(chapter: &ChapterText) -> ChapterAnalysis {
    // Synthetic adapter output, deliberately not a substitute for Sudachi fixtures.
    let tokens = chapter
        .text
        .char_indices()
        .map(|(start, scalar)| {
            let end = start + scalar.len_utf8();
            AnalyzedToken {
                anchor: anchor(chapter, start, end),
                surface: scalar.to_string(),
                is_japanese: false,
                reading: None,
                dictionary_form: None,
            }
        })
        .collect();
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture-engine@1".into(),
            dictionary_revision: "fixture-full@1".into(),
        },
        mode: SplitMode::B,
        tokens,
    }
}

fn anchor(chapter: &ChapterText, start: usize, end: usize) -> TextAnchor {
    TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::at(&chapter.text, start).unwrap(),
            end: TextOffset::at(&chapter.text, end).unwrap(),
        },
    }
}

fn edit(chapter: &ChapterText, scope: OverrideScope, parts: &[&str]) -> SegmentationOverride {
    SegmentationOverride {
        scope,
        anchor: anchor(chapter, 0, chapter.text.len()),
        surface: Surface::new(&chapter.text).unwrap(),
        parts: parts.iter().map(|s| Surface::new(*s).unwrap()).collect(),
    }
}

fn counts(storage: &Storage) -> (i64, i64, i64, i64) {
    storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM chapter_analyses), (SELECT COUNT(*) FROM analyzed_tokens),
         (SELECT COUNT(*) FROM segmentation_overrides), (SELECT COUNT(*) FROM segmentation_override_parts)",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap()
}

#[test]
fn stores_versioned_modes_exact_unicode_and_reopens_without_mutating_snapshots() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("𠮷が食べた。\r\nABC\0e\u{301}📖".into()).unwrap())
        .unwrap();
    let mut expected = analysis(&document.chapter);
    expected.tokens[0].is_japanese = true;
    expected.tokens[0].reading = Some("ヨシ".into());
    expected.tokens[0].dictionary_form = Some(Surface::new("𠮷").unwrap());
    let mut saved = Vec::new();
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        expected.mode = mode;
        let id = storage
            .store_chapter_analysis(document.chapter.chapter_id, &expected)
            .unwrap();
        assert_eq!(
            storage
                .store_chapter_analysis(document.chapter.chapter_id, &expected)
                .unwrap(),
            id
        );
        saved.push((id, expected.clone()));
    }
    let before = counts(&storage);
    let mut conflict = expected.clone();
    conflict.tokens[0].reading = None;
    assert!(matches!(
        storage.store_chapter_analysis(document.chapter.chapter_id, &conflict),
        Err(StorageError::InvalidAnalysis)
    ));
    assert_eq!(counts(&storage), before);
    // A different exact dictionary revision is a new immutable snapshot.
    conflict.profile.dictionary_revision = "fixture-full@2".into();
    let new_id = storage
        .store_chapter_analysis(document.chapter.chapter_id, &conflict)
        .unwrap();
    assert!(i64::from(new_id) > i64::from(saved.last().unwrap().0));
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    for (id, expected) in saved {
        assert_eq!(storage.chapter_analysis(id).unwrap(), Some(expected));
    }
    assert!(storage
        .chapter_analysis(i64::MAX.try_into().unwrap())
        .unwrap()
        .is_none());
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(document)
    );
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM vocabulary", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn invalid_analysis_and_override_inputs_leave_database_unchanged() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("が𠮷食べた".into()).unwrap())
        .unwrap();
    let valid = analysis(&document.chapter);
    for variant in 0..5 {
        let mut bad = valid.clone();
        match variant {
            0 => bad.tokens[0].anchor.source_id = i64::MAX.try_into().unwrap(),
            1 => bad.tokens[0].surface = "か\u{3099}".into(),
            2 => bad.tokens[0].anchor.range.end = 1.try_into().unwrap(),
            3 => bad.tokens.clear(),
            _ => bad.tokens.reverse(),
        }
        assert!(matches!(
            storage.store_chapter_analysis(document.chapter.chapter_id, &bad),
            Err(StorageError::InvalidAnalysis)
        ));
    }
    let valid = edit(
        &document.chapter,
        OverrideScope::ThisOccurrence,
        &["が", "𠮷", "食べた"],
    );
    for variant in 0..4 {
        let mut bad = valid.clone();
        match variant {
            0 => bad.parts[0] = Surface::new("か\u{3099}").unwrap(),
            1 => bad.anchor.source_id = i64::MAX.try_into().unwrap(),
            2 => bad.anchor.range.end = 1.try_into().unwrap(),
            _ => bad.parts.clear(),
        }
        assert!(matches!(
            storage.save_segmentation_override(&bad),
            Err(StorageError::InvalidAnalysis)
        ));
    }
    assert_eq!(counts(&storage), (0, 0, 0, 0));
}

#[test]
fn analysis_statement_and_commit_failures_roll_back_and_retry() {
    for trigger in [
        "CREATE TEMP TRIGGER fail_analysis BEFORE INSERT ON analyzed_tokens WHEN NEW.token_index = 1
         BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail_analysis AFTER INSERT ON analyzed_tokens
         BEGIN INSERT INTO deferred_child VALUES (99); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage.create_paste(&PasteText::new("日本語".into()).unwrap()).unwrap();
        let expected = analysis(&document.chapter);
        storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
            CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.store_chapter_analysis(document.chapter.chapter_id, &expected), Err(StorageError::Sqlite(_))));
        assert_eq!(counts(&storage), (0, 0, 0, 0));
        storage.connection.execute_batch("DROP TRIGGER fail_analysis;").unwrap();
        let id = storage.store_chapter_analysis(document.chapter.chapter_id, &expected).unwrap();
        assert_eq!(storage.chapter_analysis(id).unwrap(), Some(expected));
    }
}

#[test]
fn split_merge_replace_and_reverse_preserve_base_vocabulary_history_and_text() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("𠮷野家で食べた".into()).unwrap())
        .unwrap();
    let base = analysis(&document.chapter);
    let analysis_id = storage
        .store_chapter_analysis(document.chapter.chapter_id, &base)
        .unwrap();
    let word = Surface::new("食べた").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&word), 10)
        .unwrap();
    let entry = storage.vocabulary_entry(&word).unwrap().unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Learning, 20)
        .unwrap();
    let split = edit(
        &document.chapter,
        OverrideScope::AllOccurrences,
        &["𠮷野家", "で", "食べた"],
    );
    let first = storage.save_segmentation_override(&split).unwrap();
    assert_eq!(storage.save_segmentation_override(&split).unwrap(), first);
    let local = edit(
        &document.chapter,
        OverrideScope::ThisOccurrence,
        &["𠮷野家で食べた"],
    );
    let local_id = storage.save_segmentation_override(&local).unwrap();
    assert_ne!(local_id, first);
    let merged = edit(
        &document.chapter,
        OverrideScope::AllOccurrences,
        &["𠮷野家で食べた"],
    );
    let second = storage.save_segmentation_override(&merged).unwrap();
    assert!(
        !storage
            .segmentation_override(first)
            .unwrap()
            .unwrap()
            .active
    );
    assert!(
        storage
            .segmentation_override(second)
            .unwrap()
            .unwrap()
            .active
    );
    // A stale reversal cannot disable the replacement or another scope.
    assert!(!storage.reverse_segmentation_override(first).unwrap());
    assert!(
        storage
            .segmentation_override(second)
            .unwrap()
            .unwrap()
            .active
    );
    assert!(
        storage
            .segmentation_override(local_id)
            .unwrap()
            .unwrap()
            .active
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.segmentation_override(second).unwrap().unwrap().edit,
        merged
    );
    assert!(storage.reverse_segmentation_override(second).unwrap());
    assert!(!storage.reverse_segmentation_override(second).unwrap());
    assert!(!storage
        .reverse_segmentation_override(i64::MAX.try_into().unwrap())
        .unwrap());
    assert!(
        !storage
            .segmentation_override(first)
            .unwrap()
            .unwrap()
            .active
    );
    assert!(storage.reverse_segmentation_override(local_id).unwrap());
    let third = storage.save_segmentation_override(&split).unwrap();
    assert!(i64::from(third) > i64::from(second));
    assert_eq!(storage.chapter_analysis(analysis_id).unwrap(), Some(base));
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(document)
    );
    assert_eq!(
        storage.vocabulary_entry(&word).unwrap().unwrap().status,
        VocabularyStatus::Learning
    );
    let totals: (i64, i64) = storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM vocabulary_status_events), (SELECT COUNT(*) FROM reading_completions)",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(totals, (2, 0));
}

#[test]
fn global_rules_deduplicate_across_documents_and_local_rules_keep_source_identity() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = PasteText::new("食べた".into()).unwrap();
    let first = storage.create_paste(&text).unwrap();
    let second = storage.create_paste(&text).unwrap();
    let global = storage
        .save_segmentation_override(&edit(
            &first.chapter,
            OverrideScope::AllOccurrences,
            &["食べ", "た"],
        ))
        .unwrap();
    assert_eq!(
        storage
            .save_segmentation_override(&edit(
                &second.chapter,
                OverrideScope::AllOccurrences,
                &["食べ", "た"]
            ))
            .unwrap(),
        global
    );
    assert_eq!(
        storage
            .segmentation_override(global)
            .unwrap()
            .unwrap()
            .edit
            .anchor
            .source_id,
        first.chapter.source_id
    );
    let local1 = storage
        .save_segmentation_override(&edit(
            &first.chapter,
            OverrideScope::ThisOccurrence,
            &["食べ", "た"],
        ))
        .unwrap();
    let local2 = storage
        .save_segmentation_override(&edit(
            &second.chapter,
            OverrideScope::ThisOccurrence,
            &["食べ", "た"],
        ))
        .unwrap();
    assert_ne!(local1, local2);
    assert_eq!(counts(&storage).2, 3);
}

#[test]
fn concurrent_analysis_and_override_retries_allocate_one_active_identity() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("食べた".into()).unwrap())
        .unwrap();
    drop(storage);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            let chapter = document.chapter.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                let id = storage
                    .store_chapter_analysis(chapter.chapter_id, &analysis(&chapter))
                    .unwrap();
                let edit_id = storage
                    .save_segmentation_override(&edit(
                        &chapter,
                        OverrideScope::AllOccurrences,
                        &["食べ", "た"],
                    ))
                    .unwrap();
                (id, edit_id)
            })
        })
        .collect();
    barrier.wait();
    let ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(ids[0], ids[1]);
    assert_eq!(
        counts(&Storage::open(database.path()).unwrap()),
        (1, 3, 1, 2)
    );
}

#[test]
fn replacement_and_reversal_failures_preserve_previous_active_edit() {
    for trigger in [
        "CREATE TEMP TRIGGER fail_override BEFORE INSERT ON segmentation_override_parts WHEN NEW.part_index = 1
         BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail_override AFTER INSERT ON segmentation_override_parts
         BEGIN INSERT INTO deferred_child VALUES (99); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage.create_paste(&PasteText::new("食べた".into()).unwrap()).unwrap();
        let previous = storage.save_segmentation_override(&edit(&document.chapter, OverrideScope::AllOccurrences, &["食べた"])).unwrap();
        let split = edit(&document.chapter, OverrideScope::AllOccurrences, &["食べ", "た"]);
        storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
            CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.save_segmentation_override(&split), Err(StorageError::Sqlite(_))));
        assert_eq!(counts(&storage), (0, 0, 1, 1));
        assert!(storage.segmentation_override(previous).unwrap().unwrap().active);
        storage.connection.execute_batch("DROP TRIGGER fail_override;").unwrap();
        let current = storage.save_segmentation_override(&split).unwrap();
        storage.connection.execute_batch("CREATE TEMP TRIGGER fail_reverse AFTER UPDATE ON segmentation_overrides
            BEGIN INSERT INTO deferred_child VALUES (99); END;").unwrap();
        assert!(matches!(storage.reverse_segmentation_override(current), Err(StorageError::Sqlite(_))));
        assert!(storage.segmentation_override(current).unwrap().unwrap().active);
        storage.connection.execute_batch("DROP TRIGGER fail_reverse;").unwrap();
        assert!(storage.reverse_segmentation_override(current).unwrap());
    }
}

#[test]
fn deleting_source_content_clears_analysis_but_keeps_global_rule_reversible() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("食べた".into()).unwrap())
        .unwrap();
    let base_id = storage
        .store_chapter_analysis(document.chapter.chapter_id, &analysis(&document.chapter))
        .unwrap();
    let rule = edit(
        &document.chapter,
        OverrideScope::AllOccurrences,
        &["食べ", "た"],
    );
    let id = storage.save_segmentation_override(&rule).unwrap();
    // Test-only deletion, following the already documented document semantics.
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(document.chapter.chapter_id)],
        )
        .unwrap();
    assert!(storage.chapter_analysis(base_id).unwrap().is_none());
    assert_eq!(counts(&storage), (0, 0, 1, 2));
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.segmentation_override(id).unwrap().unwrap().edit,
        rule
    );
    assert!(storage.reverse_segmentation_override(id).unwrap());
}

#[test]
fn corrupt_stored_anchors_or_parts_error_without_repair() {
    for sql in [
        "UPDATE analyzed_tokens SET surface = '別' WHERE token_index = 0",
        "UPDATE analyzed_tokens SET start_byte = 1 WHERE token_index = 0",
        "DELETE FROM analyzed_tokens WHERE token_index = 1",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage
            .create_paste(&PasteText::new("食べた".into()).unwrap())
            .unwrap();
        let id = storage
            .store_chapter_analysis(document.chapter.chapter_id, &analysis(&document.chapter))
            .unwrap();
        storage.connection.execute(sql, []).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.chapter_analysis(id),
            Err(StorageError::InvalidAnalysis)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("食べた".into()).unwrap())
        .unwrap();
    let id = storage
        .save_segmentation_override(&edit(
            &document.chapter,
            OverrideScope::ThisOccurrence,
            &["食べ", "た"],
        ))
        .unwrap();
    storage
        .connection
        .execute(
            "UPDATE segmentation_override_parts SET surface = '別' WHERE part_index = 0",
            [],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.segmentation_override(id),
        Err(StorageError::InvalidAnalysis)
    ));
    assert!(matches!(
        storage.reverse_segmentation_override(id),
        Err(StorageError::InvalidAnalysis)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn upgrades_version_six_without_rescanning_and_failed_migration_preserves_data() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..6]).unwrap();
    let mut storage = Storage { connection };
    let document = storage
        .create_paste(&PasteText::new("𠮷食べた".into()).unwrap())
        .unwrap();
    let word = Surface::new("食べた").unwrap();
    storage.register_vocabulary(&[word], 10).unwrap();
    let before = history(&database.path());
    // Force a real version-7 SQL collision after the first table was created.
    storage
        .connection
        .execute_batch("CREATE TABLE analyzed_tokens (fixture TEXT);")
        .unwrap();
    drop(storage);
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(history(&database.path()), before);
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection.table_exists(None, "chapter_analyses").unwrap());
    connection
        .execute_batch("DROP TABLE analyzed_tokens;")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 10);
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(document)
    );
    assert_eq!(counts(&storage), (0, 0, 0, 0));
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM vocabulary_status_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        1
    );
}
