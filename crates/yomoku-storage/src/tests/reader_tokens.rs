use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride, SplitMode},
    document::{ChapterText, DocumentId, PasteText, TextAnchor, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength},
    reader::{ReaderAnalysis, ReaderVocabulary},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

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

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

#[test]
fn read_only_lookup_reopens_original_b_metadata_and_all_five_global_statuses() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new("English 📖 \0 猫 猫 𠮷 食べた が か\u{3099}".into()).unwrap(),
            100,
            |chapter| {
                let mut analysis = fixture(chapter)?;
                for token in &mut analysis.tokens {
                    if token.surface == "食べた" {
                        token.reading = Some("タベタ".into());
                        token.dictionary_form = Some(Surface::new("食べる").unwrap());
                    }
                }
                Ok::<_, std::convert::Infallible>(analysis)
            },
        )
        .unwrap();
    let document = &imported.document;
    let mut entries = Vec::new();
    for (surface, status) in [
        ("猫", VocabularyStatus::New),
        ("𠮷", VocabularyStatus::Learning),
        ("食べた", VocabularyStatus::Familiar),
        ("が", VocabularyStatus::Known),
        ("か\u{3099}", VocabularyStatus::Ignored),
    ] {
        let id = storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .unwrap()
            .id;
        storage.set_vocabulary_status(id, status, 200).unwrap();
        entries.push((surface, ReaderVocabulary { id, status }));
    }
    // Alternate modes and a newer B must not substitute for the import's B.
    let mut later = fixture(&document.chapter).unwrap();
    later.profile.engine_revision = "later-engine@2".into();
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        later.mode = mode;
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &later)
            .unwrap();
    }
    // Lookup also leaves an accepted boundary edit unapplied in this base-only slice.
    let start = document.chapter.text.find("食べた").unwrap();
    storage
        .save_segmentation_override(&SegmentationOverride {
            scope: OverrideScope::ThisOccurrence,
            anchor: anchor(&document.chapter, start, start + "食べた".len()),
            surface: Surface::new("食べた").unwrap(),
            parts: ["食べ", "た"].map(|s| Surface::new(s).unwrap()).to_vec(),
        })
        .unwrap();
    let position = super::positions::position(document, start);
    storage
        .save_reading_position(document.document_id, position)
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let page = anchor(&document.chapter, 0, document.chapter.text.len());
    let result = storage
        .load_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(result.document_id, document.document_id);
    assert_eq!(result.page, page);
    assert_eq!(
        result.analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    assert_eq!(result.tokens.len(), 6);
    assert_eq!(result.tokens[0].token_index, 3);
    assert_eq!(result.tokens[0].vocabulary, result.tokens[1].vocabulary);
    for token in &result.tokens {
        assert_eq!(
            token.vocabulary,
            Some(entries.iter().find(|(s, _)| *s == token.surface).unwrap().1)
        );
        assert_eq!(
            token.anchor.slice(&document.chapter).unwrap(),
            token.surface
        );
        assert_eq!(
            token.display_range.slice(&document.chapter.text).unwrap(),
            token.surface
        );
    }
    let verb = &result.tokens[3];
    assert_eq!(verb.surface, "食べた");
    assert_eq!(verb.reading.as_deref(), Some("タベタ"));
    assert_eq!(verb.dictionary_form.as_deref(), Some("食べる"));
    assert_eq!(
        storage
            .load_paste(document.document_id)
            .unwrap()
            .unwrap()
            .reading_position,
        Some(position)
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn repeated_and_cross_document_surfaces_refresh_globally_without_registration_or_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "猫 猫 食べた");
    let second = import(&mut storage, "猫 食べる");
    let mut writer = Storage::open(database.path()).unwrap();
    let before_change = storage
        .load_reader_tokens(
            first.document.document_id,
            anchor(
                &first.document.chapter,
                0,
                first.document.chapter.text.len(),
            ),
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        before_change.tokens[0].vocabulary.unwrap().status,
        VocabularyStatus::New
    );
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    writer
        .set_vocabulary_status(cat.id, VocabularyStatus::Ignored, 200)
        .unwrap();
    for imported in [first, second] {
        let document = imported.document;
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        let before = fs::read(database.path()).unwrap();
        let result = storage
            .load_reader_tokens(
                document.document_id,
                anchor(&document.chapter, 0, document.chapter.text.len()),
            )
            .unwrap()
            .unwrap();
        for token in result.tokens {
            if token.surface == "猫" {
                assert_eq!(
                    token.vocabulary,
                    Some(ReaderVocabulary {
                        id: cat.id,
                        status: VocabularyStatus::Ignored
                    })
                );
            } else {
                assert_eq!(token.vocabulary.unwrap().status, VocabularyStatus::New);
            }
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn deleted_or_never_registered_vocabulary_stays_absent_while_base_tokens_survive() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    let legacy = storage
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    storage
        .store_chapter_analysis(
            legacy.chapter.chapter_id,
            &fixture(&legacy.chapter).unwrap(),
        )
        .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for document in [&imported.document, &legacy] {
        let result = storage
            .load_reader_tokens(
                document.document_id,
                anchor(&document.chapter, 0, document.chapter.text.len()),
            )
            .unwrap()
            .unwrap();
        assert!(result
            .tokens
            .iter()
            .filter(|t| t.surface == "猫")
            .all(|t| t.vocabulary.is_none()));
        assert!(!result.tokens.is_empty());
        assert!(storage
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .is_none());
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn legacy_absence_never_substitutes_another_mode_or_document_and_missing_document_returns_none() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    import(&mut storage, "犬");
    let mut alternate = fixture(&document.chapter).unwrap();
    for mode in [SplitMode::A, SplitMode::C] {
        alternate.mode = mode;
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &alternate)
            .unwrap();
    }
    let page = anchor(&document.chapter, 0, document.chapter.text.len());
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let result = storage
        .load_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap();
    assert_eq!(result.analysis, ReaderAnalysis::Missing);
    assert!(result.tokens.is_empty());
    assert!(storage
        .load_reader_tokens(DocumentId::try_from(i64::MAX).unwrap(), page)
        .unwrap()
        .is_none());
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn page_fragments_reflow_with_full_token_identity_and_fresh_status() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(
        &mut storage,
        &format!("{}。{}。猫", "𠮷".repeat(600), "犬".repeat(1000)),
    );
    let document = &imported.document;
    let entry = storage
        .vocabulary_entry(&Surface::new(&document.chapter.text).unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Learning, 200)
        .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for length in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), length).unwrap();
        let mut visible = String::new();
        for page in pages.pages() {
            let result = storage
                .load_reader_tokens(document.document_id, *page)
                .unwrap()
                .unwrap();
            assert_eq!(
                result.analysis,
                ReaderAnalysis::Available(imported.analysis_id)
            );
            assert_eq!(result.tokens.len(), 1);
            let token = &result.tokens[0];
            assert_eq!(
                token.anchor,
                anchor(&document.chapter, 0, document.chapter.text.len())
            );
            assert_eq!(
                token.vocabulary,
                Some(ReaderVocabulary {
                    id: entry.id,
                    status: VocabularyStatus::Learning
                })
            );
            visible.push_str(
                token
                    .display_range
                    .slice(page.slice(&document.chapter).unwrap())
                    .unwrap(),
            );
        }
        assert_eq!(visible, document.chapter.text);
    }
}

#[test]
fn invalid_page_or_corrupt_original_analysis_and_status_fail_without_repair_or_fallback() {
    for sql in [
        "UPDATE analyzed_tokens SET surface = '犬'",
        "UPDATE analyzed_tokens SET start_byte = 1",
        "UPDATE analyzed_tokens SET token_index = 10",
        "UPDATE chapter_analyses SET engine_revision = ' '",
        "PRAGMA ignore_check_constraints = ON; UPDATE chapter_analyses SET semantics_version = 2",
        "PRAGMA ignore_check_constraints = ON; UPDATE vocabulary SET status = 'broken'",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫𠮷");
        let document = &imported.document;
        let page = anchor(&document.chapter, 0, document.chapter.text.len());
        for invalid in [
            TextAnchor {
                source_id: 99.try_into().unwrap(),
                ..page
            },
            TextAnchor {
                chapter_id: 99.try_into().unwrap(),
                ..page
            },
            TextAnchor {
                range: TextRange {
                    start: 1.try_into().unwrap(),
                    ..page.range
                },
                ..page
            },
            TextAnchor {
                range: TextRange {
                    start: page.range.end,
                    end: page.range.start,
                },
                ..page
            },
            anchor(&document.chapter, 0, 0),
        ] {
            assert!(matches!(
                storage.load_reader_tokens(document.document_id, invalid),
                Err(StorageError::InvalidAnalysis)
            ));
        }
        storage.connection.execute_batch(sql).unwrap();
        let mut later = fixture(&document.chapter).unwrap();
        later.profile.engine_revision = "healthy-later@2".into();
        storage
            .store_chapter_analysis(document.chapter.chapter_id, &later)
            .unwrap();
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        let before = fs::read(database.path()).unwrap();
        let result = storage.load_reader_tokens(document.document_id, page);
        if sql.contains("vocabulary SET status") {
            assert!(matches!(result, Err(StorageError::InvalidVocabulary)));
        } else {
            assert!(matches!(result, Err(StorageError::InvalidAnalysis)));
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}
