use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride, SplitMode},
    completion::PageCompletion,
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    pagination::{LogicalPages, ReadingPosition},
    segmentation::ChapterSegmentation,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

fn edit(
    chapter: &ChapterText,
    start: usize,
    surface: &str,
    parts: &[&str],
    scope: OverrideScope,
) -> SegmentationOverride {
    SegmentationOverride {
        anchor: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: TextOffset::at(&chapter.text, start).unwrap(),
                end: TextOffset::at(&chapter.text, start + surface.len()).unwrap(),
            },
        },
        surface: Surface::new(surface).unwrap(),
        scope,
        parts: parts
            .iter()
            .map(|part| Surface::new(*part).unwrap())
            .collect(),
    }
}

fn surfaces(result: &ChapterSegmentation) -> Vec<&str> {
    result.segments.iter().map(|s| s.surface.as_str()).collect()
}

#[test]
fn effective_reader_tokens_use_live_exact_surface_statuses_without_registration() {
    use yomoku_core::effective_reader::EffectiveTokenEligibility;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた English 。 📖");
    // Register a spelling in a different document, then split both occurrences.
    import(&mut storage, "食べ");
    let entry = storage
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Familiar, 110)
        .unwrap();
    let chapter = &imported.document.chapter;
    let id = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "食べた",
            &["食べ", "た"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    let page = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::START,
            end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
        },
    };
    let original = storage
        .load_reader_tokens(imported.document.document_id, page)
        .unwrap();
    let before = preserved_rows(&storage);
    let file_before = std::fs::read(database.path()).unwrap();
    storage
        .connection
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    let snapshot = storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    let expected_boundaries = original
        .as_ref()
        .unwrap()
        .tokens
        .iter()
        .map(
            |token| yomoku_core::effective_reader::OriginalBoundaryToken {
                anchor: token.anchor,
                display_range: token.display_range,
                surface: token.surface.clone(),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(snapshot.original_boundaries, expected_boundaries);
    assert_eq!(
        snapshot
            .tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た", "食べ", "た"]
    );
    assert!(snapshot
        .tokens
        .iter()
        .all(|t| t.metadata.is_none()
            && t.eligibility == EffectiveTokenEligibility::ManualJapaneseSpan));
    for token in &snapshot.tokens {
        assert_eq!(
            token.vocabulary.map(|v| (v.id, v.status)),
            (token.surface == "食べ").then_some((entry.id, VocabularyStatus::Familiar))
        );
    }
    assert_eq!(
        snapshot.segmentation.as_ref().unwrap().original_analysis_id,
        imported.analysis_id
    );
    assert_eq!(preserved_rows(&storage), before);
    assert_eq!(std::fs::read(database.path()).unwrap(), file_before);
    assert_eq!(
        storage
            .load_reader_tokens(imported.document.document_id, page)
            .unwrap(),
        original
    );
    storage
        .connection
        .execute_batch("PRAGMA query_only = OFF")
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .load_effective_reader_tokens(imported.document.document_id, page)
            .unwrap()
            .unwrap(),
        snapshot
    );
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Ignored, 120)
        .unwrap();
    let updated = storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        updated.tokens[0].vocabulary.unwrap().status,
        VocabularyStatus::Ignored
    );
    assert_eq!(updated.segmentation, snapshot.segmentation);
    assert_eq!(updated.original_boundaries, snapshot.original_boundaries);
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(entry.id)],
        )
        .unwrap();
    let before = preserved_rows(&storage);
    assert!(storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap()
        .tokens
        .iter()
        .all(|t| t.vocabulary.is_none()));
    assert_eq!(preserved_rows(&storage), before);
    storage.reverse_segmentation_override(id).unwrap();
    let reversed = storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        reversed
            .tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べた", "食べた"]
    );
    assert_ne!(reversed.segmentation, snapshot.segmentation);
    assert_eq!(reversed.original_boundaries, snapshot.original_boundaries);
    // A previously returned result remains the original frozen projection.
    assert_eq!(snapshot.tokens[0].surface, "食べ");
}

#[test]
fn effective_reader_tokens_keep_missing_deleted_and_invalid_states_distinct() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬");
    let chapter = &imported.document.chapter;
    let page = edit(chapter, 0, "猫犬", &["猫犬"], OverrideScope::AllOccurrences).anchor;
    let mut foreign = page;
    foreign.source_id = 999.try_into().unwrap();
    assert!(matches!(
        storage.load_effective_reader_tokens(imported.document.document_id, foreign),
        Err(StorageError::InvalidAnalysis)
    ));
    let mut empty = page;
    empty.range.end = empty.range.start;
    assert!(matches!(
        storage.load_effective_reader_tokens(imported.document.document_id, empty),
        Err(StorageError::InvalidAnalysis)
    ));
    let mut broken_utf8 = page;
    broken_utf8.range.end = 1.try_into().unwrap();
    assert!(matches!(
        storage.load_effective_reader_tokens(imported.document.document_id, broken_utf8),
        Err(StorageError::InvalidAnalysis)
    ));
    storage
        .connection
        .execute("DELETE FROM chapter_analyses", [])
        .unwrap();
    let before = preserved_rows(&storage);
    let snapshot = storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.segmentation, None);
    assert!(snapshot.original_boundaries.is_empty());
    assert!(snapshot.tokens.is_empty());
    assert_eq!(preserved_rows(&storage), before);
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    assert_eq!(
        storage
            .load_effective_reader_tokens(imported.document.document_id, page)
            .unwrap(),
        None
    );
}

#[test]
fn effective_reader_tokens_load_exact_alternate_metadata_and_validate_off_page_data() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬 鳥");
    let chapter = &imported.document.chapter;
    let rule = edit(
        chapter,
        0,
        "猫犬",
        &["猫", "犬"],
        OverrideScope::AllOccurrences,
    );
    storage.save_segmentation_override(&rule).unwrap();
    let mut a = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    a.mode = SplitMode::A;
    let mut cat = a.tokens[0].clone();
    cat.surface = "猫".into();
    cat.anchor.range.end = 3.try_into().unwrap();
    cat.reading = Some("ネコ".into());
    cat.dictionary_form = Some(Surface::new("猫").unwrap());
    let mut dog = cat.clone();
    dog.surface = "犬".into();
    dog.anchor.range.start = cat.anchor.range.end;
    dog.anchor.range.end = 6.try_into().unwrap();
    dog.reading = Some("イヌ".into());
    dog.dictionary_form = Some(Surface::new("犬").unwrap());
    a.tokens.splice(0..1, [cat, dog]);
    let alternate_id = storage
        .store_chapter_analysis(chapter.chapter_id, &a)
        .unwrap();
    // A crossing selection retains the entire token and exact source metadata.
    let mut page = rule.anchor;
    page.range.start = 3.try_into().unwrap();
    let before = preserved_rows(&storage);
    let snapshot = storage
        .load_effective_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.tokens.len(), 1);
    assert_eq!(snapshot.tokens[0].surface, "犬");
    let metadata = snapshot.tokens[0].metadata.as_ref().unwrap();
    assert_eq!(
        (metadata.analysis_id, metadata.mode, metadata.token_index),
        (alternate_id, SplitMode::A, 1)
    );
    assert_eq!(metadata.reading.as_deref(), Some("イヌ"));
    assert_eq!(metadata.dictionary_form.as_ref().unwrap().as_str(), "犬");
    assert_eq!(preserved_rows(&storage), before);
    storage
        .connection
        .execute(
            "UPDATE analyzed_tokens SET surface = '別' WHERE analysis_id = ?1 AND token_index = 2",
            [i64::from(alternate_id)],
        )
        .unwrap();
    assert!(matches!(
        storage.load_effective_reader_tokens(imported.document.document_id, page),
        Err(StorageError::InvalidAnalysis)
    ));
}

fn preserved_rows(storage: &Storage) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "reading_positions",
        "reader_settings",
        "documents",
        "pending_reading_time",
    ]
    .iter()
    .map(|table| {
        let mut statement = storage
            .connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |row| {
                (0..columns)
                    .map(|index| row.get(index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    })
    .collect()
}

#[test]
fn segmentation_reopens_global_local_reversal_and_pinned_b_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "食べた 食べた");
    let second = import(&mut storage, "食べた");
    let chapter = &first.document.chapter;
    let global = edit(
        chapter,
        0,
        "食べた",
        &["食べ", "た"],
        OverrideScope::AllOccurrences,
    );
    let global_id = storage.save_segmentation_override(&global).unwrap();
    let local_id = storage
        .save_segmentation_override(&edit(
            chapter,
            "食べた ".len(),
            "食べた",
            &["食べた"],
            OverrideScope::ThisOccurrence,
        ))
        .unwrap();
    let mut later = fixture(chapter).unwrap();
    later.profile.engine_revision = "later@2".into();
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        later.mode = mode;
        storage
            .store_chapter_analysis(chapter.chapter_id, &later)
            .unwrap();
    }
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    let first_resolved = storage
        .load_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(first_resolved.original_analysis_id, first.analysis_id);
    assert_eq!(surfaces(&first_resolved), ["食べ", "た", "食べた"]);
    assert_eq!(first_resolved.segments[2].override_id, Some(local_id));
    assert_eq!(first_resolved.segments[2].original_token_index, Some(1));
    let second_resolved = storage
        .load_chapter_segmentation(second.document.chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(surfaces(&second_resolved), ["食べ", "た"]);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    storage.reverse_segmentation_override(local_id).unwrap();
    assert_eq!(
        surfaces(
            &storage
                .load_chapter_segmentation(chapter.chapter_id)
                .unwrap()
                .unwrap()
        ),
        ["食べ", "た", "食べ", "た"]
    );
    storage.reverse_segmentation_override(global_id).unwrap();
    assert_eq!(
        surfaces(
            &storage
                .load_chapter_segmentation(chapter.chapter_id)
                .unwrap()
                .unwrap()
        ),
        ["食べた", "食べた"]
    );
}

#[test]
fn edits_resolution_and_reversal_preserve_original_analyses_and_every_history_value() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた 𠮷");
    let document = &imported.document;
    let chapter = &document.chapter;
    let page_length = storage.page_length().unwrap();
    let page = LogicalPages::new(std::slice::from_ref(chapter), page_length)
        .unwrap()
        .pages()[0];
    let command = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page,
        page_length,
        departing_position: ReadingPosition {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            offset: TextOffset::START,
        },
        completed_at_unix_ms: 200,
        pending_time: Default::default(),
    };
    storage
        .record_analyzed_page_completion(command, imported.analysis_id)
        .unwrap();
    let before = preserved_rows(&storage);
    let split = edit(
        chapter,
        0,
        "食べた",
        &["食べ", "た"],
        OverrideScope::AllOccurrences,
    );
    let old = storage.save_segmentation_override(&split).unwrap();
    assert_eq!(
        surfaces(
            &storage
                .load_chapter_segmentation(chapter.chapter_id)
                .unwrap()
                .unwrap()
        ),
        ["食べ", "た", "食べ", "た", "𠮷"]
    );
    assert_eq!(preserved_rows(&storage), before);
    let replacement = SegmentationOverride {
        parts: vec![split.surface.clone()],
        ..split
    };
    let current = storage.save_segmentation_override(&replacement).unwrap();
    assert_ne!(old, current);
    storage.reverse_segmentation_override(current).unwrap();
    let effective = storage
        .load_effective_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(effective
        .segments
        .iter()
        .all(|s| s.metadata.as_ref().unwrap().analysis_id == imported.analysis_id));
    assert!(!storage.segmentation_override(old).unwrap().unwrap().active);
    assert_eq!(
        surfaces(
            &storage
                .load_chapter_segmentation(chapter.chapter_id)
                .unwrap()
                .unwrap()
        ),
        ["食べた", "食べた", "𠮷"]
    );
    assert_eq!(preserved_rows(&storage), before);
    assert!(storage
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn global_rule_survives_origin_content_deletion_and_reimport_gets_no_local_edits() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "猫犬");
    let global_id = storage
        .save_segmentation_override(&edit(
            &first.document.chapter,
            0,
            "猫犬",
            &["猫", "犬"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    storage
        .save_segmentation_override(&edit(
            &first.document.chapter,
            0,
            "猫犬",
            &["猫犬"],
            OverrideScope::ThisOccurrence,
        ))
        .unwrap();
    // Same test-only content deletion used by the existing retained-anchor tests.
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(first.document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(first.document.chapter.chapter_id)],
        )
        .unwrap();
    let fresh = import(&mut storage, "猫犬");
    assert_ne!(
        fresh.document.chapter.source_id,
        first.document.chapter.source_id
    );
    let resolved = storage
        .load_chapter_segmentation(fresh.document.chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(surfaces(&resolved), ["猫", "犬"]);
    assert!(resolved
        .segments
        .iter()
        .all(|s| s.override_id == Some(global_id)));
    let effective = storage
        .load_effective_chapter_segmentation(fresh.document.chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(effective
        .segments
        .iter()
        .all(|s| s.boundary.override_id == Some(global_id) && s.metadata.is_none()));
    assert!(storage.reverse_segmentation_override(global_id).unwrap());
    assert_eq!(
        surfaces(
            &storage
                .load_chapter_segmentation(fresh.document.chapter.chapter_id)
                .unwrap()
                .unwrap()
        ),
        ["猫犬"]
    );
}

#[test]
fn missing_b_never_uses_other_modes_and_missing_chapters_fail() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    let mut alternate = fixture(&document.chapter).unwrap();
    for mode in [SplitMode::A, SplitMode::C] {
        alternate.mode = mode;
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &alternate)
            .unwrap();
    }
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    assert_eq!(
        storage
            .load_chapter_segmentation(document.chapter.chapter_id)
            .unwrap(),
        None
    );
    assert!(matches!(
        storage.load_chapter_segmentation(i64::MAX.try_into().unwrap()),
        Err(StorageError::InvalidAnalysis)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn corrupt_base_or_rules_fail_read_only_without_repair_or_partial_results() {
    for sql in [
        "UPDATE analyzed_tokens SET surface = '別'",
        "UPDATE segmentation_override_parts SET surface = '別' WHERE part_index = 0",
        "UPDATE segmentation_override_parts SET part_index = 7 WHERE part_index = 0",
        "UPDATE segmentation_overrides SET source_id = 999",
        "PRAGMA ignore_check_constraints = ON; UPDATE segmentation_overrides SET semantics_version = 2",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫犬");
        storage.save_segmentation_override(&edit(&imported.document.chapter, 0, "猫犬", &["猫", "犬"], OverrideScope::AllOccurrences)).unwrap();
        if sql.contains("source_id = 999") {
            // Retain a valid FK to a different source, then corrupt ownership.
            let other = import(&mut storage, "鳥");
            storage.connection.execute("UPDATE segmentation_overrides SET source_id = ?1", [i64::from(other.document.chapter.source_id)]).unwrap();
        } else {
            storage.connection.execute_batch(sql).unwrap();
        }
        storage.connection.pragma_update(None, "query_only", true).unwrap();
        let bytes = fs::read(database.path()).unwrap();
        assert!(matches!(storage.load_chapter_segmentation(imported.document.chapter.chapter_id), Err(StorageError::InvalidAnalysis)), "{sql}");
        assert!(matches!(storage.load_effective_chapter_segmentation(imported.document.chapter.chapter_id), Err(StorageError::InvalidAnalysis)), "{sql}");
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}

#[test]
fn effective_metadata_reopens_with_exact_provenance_without_changing_reader_or_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた");
    let chapter = &imported.document.chapter;
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    let before = preserved_rows(&storage);
    let global = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "食べた",
            &["食べ", "た"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    let unresolved = storage
        .load_effective_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(unresolved.segments.iter().all(|s| s.metadata.is_none()));
    assert_eq!(preserved_rows(&storage), before);

    let mut a = original.clone();
    a.mode = SplitMode::A;
    a.tokens = original
        .tokens
        .iter()
        .flat_map(|token| {
            let middle = TextOffset::try_from(
                i64::from(token.anchor.range.start) + i64::try_from("食べ".len()).unwrap(),
            )
            .unwrap();
            let mut first = token.clone();
            first.anchor.range.end = middle;
            first.surface = "食べ".into();
            first.dictionary_form = Some(Surface::new("食べる").unwrap());
            first.reading = Some("タベ".into());
            let mut second = token.clone();
            second.anchor.range.start = middle;
            second.surface = "た".into();
            second.dictionary_form = Some(Surface::new("た").unwrap());
            second.reading = Some("タ".into());
            [first, second]
        })
        .collect();
    let a_id = storage
        .store_chapter_analysis(chapter.chapter_id, &a)
        .unwrap();
    let mut c = a.clone();
    c.mode = SplitMode::C;
    for token in &mut c.tokens {
        token.reading = Some("Cヨミ".into());
    }
    storage
        .store_chapter_analysis(chapter.chapter_id, &c)
        .unwrap();
    // Later profiles and B output cannot replace the pinned import metadata.
    let mut foreign = a.clone();
    foreign.profile.dictionary_revision.push_str("-later");
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        foreign.mode = mode;
        let foreign_id = storage
            .store_chapter_analysis(chapter.chapter_id, &foreign)
            .unwrap();
        // Unrelated versions are not loaded or used to repair missing metadata.
        storage
            .connection
            .execute(
                "UPDATE analyzed_tokens SET surface = '別' WHERE analysis_id = ?1",
                [i64::from(foreign_id)],
            )
            .unwrap();
    }
    let local = storage
        .save_segmentation_override(&edit(
            chapter,
            "食べた ".len(),
            "食べた",
            &["食べた"],
            OverrideScope::ThisOccurrence,
        ))
        .unwrap();
    let before = preserved_rows(&storage);
    let expected = storage
        .load_effective_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(expected.original_analysis_id, imported.analysis_id);
    assert_eq!(expected.segments.len(), 3);
    for (index, segment) in expected.segments[..2].iter().enumerate() {
        assert_eq!(segment.boundary.override_id, Some(global));
        let metadata = segment.metadata.as_ref().unwrap();
        assert_eq!(
            (metadata.analysis_id, metadata.token_index, metadata.mode),
            (a_id, index, SplitMode::A)
        );
        assert_eq!(metadata.reading, a.tokens[index].reading);
        assert_eq!(metadata.dictionary_form, a.tokens[index].dictionary_form);
    }
    let last = &expected.segments[2];
    assert_eq!(last.boundary.override_id, Some(local));
    assert_eq!(
        last.metadata.as_ref().unwrap().analysis_id,
        imported.analysis_id
    );
    assert_eq!(last.metadata.as_ref().unwrap().token_index, 1);
    let page = LogicalPages::new(
        std::slice::from_ref(chapter),
        storage.page_length().unwrap(),
    )
    .unwrap()
    .pages()[0];
    let live = storage
        .load_reader_tokens(imported.document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(
        live.tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べた", "食べた"]
    );
    assert_eq!(preserved_rows(&storage), before);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    assert_eq!(
        storage
            .load_effective_chapter_segmentation(chapter.chapter_id)
            .unwrap()
            .unwrap(),
        expected
    );
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    storage.reverse_segmentation_override(local).unwrap();
    assert_eq!(
        storage
            .load_effective_chapter_segmentation(chapter.chapter_id)
            .unwrap()
            .unwrap()
            .segments
            .len(),
        4
    );
    storage.reverse_segmentation_override(global).unwrap();
    let restored = storage
        .load_effective_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(restored
        .segments
        .iter()
        .all(|s| s.boundary.override_id.is_none()
            && s.metadata.as_ref().unwrap().analysis_id == imported.analysis_id));
    assert_eq!(preserved_rows(&storage), before);
}

#[test]
fn effective_metadata_missing_b_and_corrupt_matching_snapshots_fail_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("猫犬".into()).unwrap())
        .unwrap();
    let mut alternate = fixture(&document.chapter).unwrap();
    alternate.mode = SplitMode::A;
    storage
        .store_chapter_analysis(document.chapter.chapter_id, &alternate)
        .unwrap();
    assert_eq!(
        storage
            .load_effective_chapter_segmentation(document.chapter.chapter_id)
            .unwrap(),
        None
    );
    assert!(matches!(
        storage.load_effective_chapter_segmentation(i64::MAX.try_into().unwrap()),
        Err(StorageError::InvalidAnalysis)
    ));

    for sql in [
        "UPDATE analyzed_tokens SET reading = ' ' WHERE analysis_id = ?1",
        "UPDATE analyzed_tokens SET surface = '別' WHERE analysis_id = ?1",
        "PRAGMA ignore_check_constraints = ON; UPDATE chapter_analyses SET semantics_version = 2 WHERE id = ?1",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫犬");
        let mut alternate = fixture(&imported.document.chapter).unwrap();
        alternate.mode = SplitMode::A;
        let alternate_id = storage.store_chapter_analysis(imported.document.chapter.chapter_id, &alternate).unwrap();
        if sql.starts_with("PRAGMA") {
            storage.connection.pragma_update(None, "ignore_check_constraints", true).unwrap();
            storage.connection.execute("UPDATE chapter_analyses SET semantics_version = 2 WHERE id = ?1", [i64::from(alternate_id)]).unwrap();
        } else {
            storage.connection.execute(sql, [i64::from(alternate_id)]).unwrap();
        }
        storage.connection.pragma_update(None, "query_only", true).unwrap();
        let bytes = fs::read(database.path()).unwrap();
        assert!(matches!(storage.load_effective_chapter_segmentation(imported.document.chapter.chapter_id), Err(StorageError::InvalidAnalysis)), "{sql}");
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}
