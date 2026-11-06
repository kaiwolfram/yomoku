use yomoku_core::{
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
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
        .analyzed_fixture(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

#[test]
fn repeated_and_cross_document_surfaces_refresh_globally_without_registration_or_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "猫 猫 食べた");
    let second = import(&mut storage, "猫 食べる");
    let mut writer = Storage::open(database.path()).unwrap();
    let before_change = storage
        .load_effective_reader_tokens(
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
            .load_effective_reader_tokens(
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
        .unanalyzed_fixture(&PasteText::new("猫".into()).unwrap())
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
            .load_effective_reader_tokens(
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
                .load_effective_reader_tokens(document.document_id, *page)
                .unwrap()
                .unwrap();
            assert_eq!(
                result
                    .segmentation
                    .as_ref()
                    .map(|s| s.original_analysis_id)
                    .map_or(ReaderAnalysis::Missing, ReaderAnalysis::Available),
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
                storage.load_effective_reader_tokens(document.document_id, invalid),
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
        let result = storage.load_effective_reader_tokens(document.document_id, page);
        if sql.contains("vocabulary SET status") {
            assert!(matches!(result, Err(StorageError::InvalidVocabulary)));
        } else {
            assert!(matches!(result, Err(StorageError::InvalidAnalysis)));
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}
