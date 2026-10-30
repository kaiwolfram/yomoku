use std::convert::Infallible;

use yomoku_core::{
    analysis::{ChapterAnalysis, OverrideScope, SegmentationOverride, SplitMode},
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    pagination::ReadingPosition,
    vocabulary::{Surface, VocabularyStatus},
};

use super::*;

fn analyze(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut analysis = imports::fixture(chapter).unwrap();
    for token in &mut analysis.tokens {
        token.dictionary_form = match token.surface.as_str() {
            "未解決" => None,
            "食べた" | "食べます" => Some(Surface::new("食べる").unwrap()),
            surface => Some(Surface::new(surface).unwrap()),
        };
    }
    Ok(analysis)
}

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, analyze)
        .unwrap()
}

fn links(storage: &Storage) -> Vec<(String, String)> {
    storage
        .connection
        .prepare(
            "SELECT v.surface, f.dictionary_form FROM vocabulary_dictionary_forms f
         JOIN vocabulary v ON v.id = f.vocabulary_id ORDER BY v.surface, f.dictionary_form",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn entry(storage: &Storage, surface: &str) -> yomoku_core::vocabulary::VocabularyEntry {
    storage
        .vocabulary_entry(&Surface::new(surface).unwrap())
        .unwrap()
        .unwrap()
}

fn intent(storage: &mut Storage, imported: &AnalyzedPaste) -> PageCompletionIntent {
    let chapter = &imported.document.chapter;
    PageCompletionIntent {
        document_id: imported.document.document_id,
        page: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: TextOffset::START,
                end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
            },
        },
        page_length: storage.page_length().unwrap(),
        departing_position: ReadingPosition {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            offset: TextOffset::START,
        },
        completed_at_unix_ms: 500,
        analysis_id: imported.analysis_id,
    }
}

#[test]
fn vocabulary_links_keep_unread_exact_spelling_evidence_after_document_deletion() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(
        &mut storage,
        "食べた 食べた 食べます 未解決 が か\u{3099} 𠮷 English 。",
    );
    let expected = [
        ("か\u{3099}", "か\u{3099}"),
        ("が", "が"),
        ("食べた", "食べる"),
        ("食べます", "食べる"),
        ("𠮷", "𠮷"),
    ]
    .map(|(surface, form)| (surface.to_string(), form.to_string()))
    .to_vec();
    assert_eq!(links(&storage), expected);
    assert_eq!(entry(&storage, "食べた").status, VocabularyStatus::New);
    assert_eq!(
        storage
            .lifetime_reading_totals()
            .unwrap()
            .japanese_occurrences,
        0
    );
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(links(&storage), expected);
    let word = entry(&storage, "食べた");
    storage
        .set_vocabulary_status(word.id, VocabularyStatus::Known, 200)
        .unwrap();
    assert_eq!(links(&storage), expected);
    assert_eq!(
        storage
            .lifetime_reading_totals()
            .unwrap()
            .japanese_occurrences,
        0
    );
}

#[test]
fn vocabulary_links_erasure_and_fresh_completion_never_recover_an_old_generation() {
    for effective in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた 未解決");
        let old = entry(&storage, "食べた");
        storage
            .connection
            .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(old.id)])
            .unwrap();
        assert!(links(&storage).is_empty());
        // Reading projections, raw registration and an identical snapshot retry
        // must not scan old snapshots to restore erased associations.
        storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&old.surface), 200)
            .unwrap();
        let fresh = entry(&storage, "食べた");
        assert_ne!(fresh.id, old.id);
        assert_eq!(fresh.status, VocabularyStatus::New);
        let analysis = analyze(&imported.document.chapter).unwrap();
        storage
            .store_chapter_analysis(imported.document.chapter.chapter_id, &analysis)
            .unwrap();
        assert!(links(&storage).is_empty());
        let intent = intent(&mut storage, &imported);
        if effective {
            let key = storage
                .load_effective_reader_tokens(intent.document_id, intent.page)
                .unwrap()
                .unwrap()
                .key()
                .unwrap();
            let frozen = storage
                .prepare_effective_page_completion(intent, &key)
                .unwrap();
            assert!(links(&storage).is_empty());
            assert_eq!(
                storage.record_effective_page_completion(&frozen).unwrap(),
                AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
            );
            let before = fs::read(database.path()).unwrap();
            assert_eq!(
                storage.record_effective_page_completion(&frozen).unwrap(),
                AnalyzedCompletionOutcome::AlreadyRecorded
            );
            assert_eq!(fs::read(database.path()).unwrap(), before);
        } else {
            let command = storage.prepare_page_completion(intent).unwrap();
            storage
                .record_analyzed_page_completion(command.completion, command.analysis_id)
                .unwrap();
        }
        assert_eq!(links(&storage), vec![("食べた".into(), "食べる".into())]);
        storage
            .delete_document(imported.document.document_id)
            .unwrap();
        assert_eq!(links(&storage).len(), 1);
        storage
            .connection
            .execute(
                "DELETE FROM vocabulary WHERE id = ?1",
                [i64::from(fresh.id)],
            )
            .unwrap();
        drop(storage);
        let storage = Storage::open(database.path()).unwrap();
        assert!(links(&storage).is_empty());
    }
}

#[test]
fn vocabulary_links_preserve_distinct_resolutions_without_reading_identity_or_null_erasure() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 未解決");
    let mut analysis = analyze(&imported.document.chapter).unwrap();
    analysis.mode = SplitMode::A;
    analysis.tokens[0].reading = Some("タベタ".into());
    analysis.tokens[0].dictionary_form = Some(Surface::new("喰べる").unwrap());
    storage
        .store_chapter_analysis(imported.document.chapter.chapter_id, &analysis)
        .unwrap();
    analysis.mode = SplitMode::C;
    analysis.tokens[0].reading = Some("べつのよみ".into());
    analysis.tokens[0].dictionary_form = None;
    storage
        .store_chapter_analysis(imported.document.chapter.chapter_id, &analysis)
        .unwrap();
    assert_eq!(
        links(&storage),
        vec![
            ("食べた".into(), "喰べる".into()),
            ("食べた".into(), "食べる".into())
        ]
    );
    assert_eq!(entry(&storage, "食べた").status, VocabularyStatus::New);
}

#[test]
fn vocabulary_links_registered_edits_use_only_exact_analyzer_metadata() {
    for with_metadata in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた");
        let chapter = &imported.document.chapter;
        let mut analysis = analyze(chapter).unwrap();
        analysis.mode = SplitMode::A;
        let mut first = analysis.tokens[0].clone();
        first.surface = "食べ".into();
        first.anchor.range.end = TextOffset::at(&chapter.text, "食べ".len()).unwrap();
        first.dictionary_form = Some(Surface::new("食べる").unwrap());
        let mut second = first.clone();
        second.surface = "た".into();
        second.anchor.range.start = first.anchor.range.end;
        second.anchor.range.end = TextOffset::at(&chapter.text, chapter.text.len()).unwrap();
        second.dictionary_form = None;
        analysis.tokens = vec![first, second];
        if with_metadata {
            storage
                .store_chapter_analysis(chapter.chapter_id, &analysis)
                .unwrap();
        }
        assert_eq!(links(&storage).len(), 1); // analysis never registers new surfaces
        let rule = SegmentationOverride {
            scope: OverrideScope::AllOccurrences,
            anchor: analyze(chapter).unwrap().tokens[0].anchor,
            surface: Surface::new("食べた").unwrap(),
            parts: ["食べ", "た"].map(|s| Surface::new(s).unwrap()).to_vec(),
        };
        let (_, registered) = storage
            .save_registered_segmentation_override(&rule, 200)
            .unwrap();
        assert_eq!(registered, 2);
        assert_eq!(links(&storage).len(), if with_metadata { 2 } else { 1 });
        assert!(links(&storage).iter().all(|(surface, _)| surface != "た"));
        storage
            .delete_document(imported.document.document_id)
            .unwrap();
        assert_eq!(links(&storage).len(), if with_metadata { 2 } else { 1 });
    }
}

#[test]
fn vocabulary_links_import_statement_and_deferred_commit_failures_are_atomic() {
    for fail in [
        "SELECT RAISE(ABORT, 'link failure');",
        "INSERT INTO deferred_child VALUES (99);",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        import(&mut storage, "猫");
        let before = fs::read(database.path()).unwrap();
        storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
            CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        storage.connection.execute_batch(&format!(
            "CREATE TEMP TRIGGER fail BEFORE INSERT ON vocabulary_dictionary_forms BEGIN {fail} END;"
        )).unwrap();
        assert!(storage
            .create_analyzed_paste(&PasteText::new("食べた".into()).unwrap(), 200, analyze)
            .is_err());
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert_eq!(links(&storage), vec![("猫".into(), "猫".into())]);
        assert!(storage
            .vocabulary_entry(&Surface::new("食べた").unwrap())
            .unwrap()
            .is_none());
        storage
            .connection
            .execute_batch("DROP TRIGGER fail")
            .unwrap();
        import(&mut storage, "食べた");
        assert_eq!(links(&storage).len(), 2);
    }
}

#[test]
fn vocabulary_links_completion_failures_roll_back_links_status_and_history_before_exact_retry() {
    for effective in [false, true] {
        for fail in [
            "SELECT RAISE(ABORT, 'link failure');",
            "INSERT INTO deferred_child VALUES (99);",
        ] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let imported = import(&mut storage, "食べた 未解決");
            storage
                .connection
                .execute("DELETE FROM vocabulary WHERE surface = '食べた'", [])
                .unwrap();
            let intent = intent(&mut storage, &imported);
            let key = storage
                .load_effective_reader_tokens(intent.document_id, intent.page)
                .unwrap()
                .unwrap()
                .key()
                .unwrap();
            let frozen = if effective {
                Some(
                    storage
                        .prepare_effective_page_completion(intent, &key)
                        .unwrap(),
                )
            } else {
                None
            };
            let original = if effective {
                None
            } else {
                Some(storage.prepare_page_completion(intent).unwrap())
            };
            let before = fs::read(database.path()).unwrap();
            storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
                CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
            storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fail BEFORE INSERT ON vocabulary_dictionary_forms BEGIN {fail} END;")).unwrap();
            let result = if let Some(frozen) = &frozen {
                storage.record_effective_page_completion(frozen)
            } else {
                let original = original.as_ref().unwrap();
                storage.record_analyzed_page_completion(original.completion, original.analysis_id)
            };
            assert!(result.is_err());
            assert_eq!(fs::read(database.path()).unwrap(), before);
            assert!(links(&storage).is_empty());
            assert_eq!(
                storage
                    .lifetime_reading_totals()
                    .unwrap()
                    .japanese_occurrences,
                0
            );
            assert_eq!(entry(&storage, "未解決").status, VocabularyStatus::New);
            storage
                .connection
                .execute_batch("DROP TRIGGER fail")
                .unwrap();
            if let Some(frozen) = &frozen {
                storage.record_effective_page_completion(frozen).unwrap();
            } else {
                let original = original.as_ref().unwrap();
                storage
                    .record_analyzed_page_completion(original.completion, original.analysis_id)
                    .unwrap();
            }
            assert_eq!(links(&storage), vec![("食べた".into(), "食べる".into())]);
            assert_eq!(
                storage
                    .lifetime_reading_totals()
                    .unwrap()
                    .japanese_occurrences,
                2
            );
        }
    }
}

#[test]
fn vocabulary_links_upgrade_recovers_unread_snapshots_and_deleted_document_occurrences() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let read = import(&mut storage, "食べた");
    let intent = intent(&mut storage, &read);
    let command = storage.prepare_page_completion(intent).unwrap();
    storage
        .record_analyzed_page_completion(command.completion, command.analysis_id)
        .unwrap();
    storage.delete_document(read.document.document_id).unwrap();
    let unread = import(&mut storage, "食べます 未解決 English");
    let expected = links(&storage);
    let totals = storage.lifetime_reading_totals().unwrap();
    let timeline = storage.vocabulary_status_timeline().unwrap();
    storage.connection.execute_batch("DROP TABLE vocabulary_dictionary_forms; DELETE FROM schema_migrations WHERE version = 20;").unwrap();
    drop(storage);
    // Inject failure after table creation/backfill, then prove byte-exact rollback.
    let before = fs::read(database.path()).unwrap();
    let mut connection = Connection::open(database.path()).unwrap();
    let sql = concat!(
        include_str!("../../migrations/0020_vocabulary_dictionary_forms.sql"),
        " INSERT INTO nonexistent VALUES (1);"
    );
    let mut failing = MIGRATIONS[..19]
        .iter()
        .map(|migration| Migration { ..*migration })
        .collect::<Vec<_>>();
    failing.push(Migration {
        version: 20,
        name: MIGRATIONS[19].name,
        sql,
    });
    assert!(migrate(&mut connection, &failing).is_err());
    drop(connection);
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(links(&storage), expected);
    assert_eq!(storage.lifetime_reading_totals().unwrap(), totals);
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), timeline);
    storage
        .delete_document(unread.document.document_id)
        .unwrap();
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(links(&storage), expected);
}
