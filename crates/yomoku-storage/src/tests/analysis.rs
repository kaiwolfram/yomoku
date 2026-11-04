use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    pagination::{PageLength, ReadingPosition},
    reader::{ReaderAnalysis, ReaderSnapshot},
    vocabulary::Surface,
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
        mode: SplitMode::C,
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

fn counts(storage: &Storage) -> (i64, i64, i64, i64) {
    storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM chapter_analyses), (SELECT COUNT(*) FROM analyzed_tokens),
         0, 0",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap()
}

#[test]
fn reader_snapshot_pins_original_mode_b_with_exact_position_and_preset_after_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut document = storage
        .create_paste(&PasteText::new("𠮷が食べた。\r\nABC\0e\u{301}📖".into()).unwrap())
        .unwrap();
    let chapter_id = document.chapter.chapter_id;
    let mut snapshot = analysis(&document.chapter);
    snapshot.mode = SplitMode::A;
    storage
        .store_chapter_analysis(chapter_id, &snapshot)
        .unwrap();
    snapshot.mode = SplitMode::C;
    let original = storage
        .store_chapter_analysis(chapter_id, &snapshot)
        .unwrap();
    snapshot.profile.engine_revision = "later-engine@2".into();
    let later = storage
        .store_chapter_analysis(chapter_id, &snapshot)
        .unwrap();
    assert_ne!(original, later);
    snapshot.mode = SplitMode::B;
    storage
        .store_chapter_analysis(chapter_id, &snapshot)
        .unwrap();
    let position = ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id,
        offset: TextOffset::try_from(4).unwrap(), // after supplementary Unicode
    };
    storage
        .save_reading_position(document.document_id, position)
        .unwrap();
    document.reading_position = Some(position);
    storage.save_page_length(PageLength::Characters750).unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert_eq!(
        storage.load_reader_snapshot(document.document_id).unwrap(),
        Some(ReaderSnapshot {
            document,
            page_length: PageLength::Characters750,
            analysis: ReaderAnalysis::Available(original),
        })
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn reader_snapshot_rejects_corrupt_identity_or_settings_without_fallback_or_repair() {
    for sql in [
        "UPDATE chapter_analyses SET engine_revision = ' '",
        "UPDATE chapter_analyses SET dictionary_revision = ' '",
        "PRAGMA ignore_check_constraints = ON; UPDATE chapter_analyses SET semantics_version = 2",
        "DELETE FROM reader_settings",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage
            .create_paste(&PasteText::new("猫".into()).unwrap())
            .unwrap();
        let mut snapshot = analysis(&document.chapter);
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &snapshot)
            .unwrap();
        storage.connection.execute_batch(sql).unwrap();
        // A healthy later snapshot must not mask a broken original identity.
        snapshot.profile.engine_revision = "later-engine@2".into();
        snapshot.profile.dictionary_revision = "later-full@2".into();
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &snapshot)
            .unwrap();
        let before = fs::read(database.path()).unwrap();
        let result = storage.load_reader_snapshot(document.document_id);
        if sql == "DELETE FROM reader_settings" {
            assert!(matches!(result, Err(StorageError::InvalidSettings)));
        } else {
            assert!(matches!(result, Err(StorageError::InvalidAnalysis)));
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
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
    for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
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
