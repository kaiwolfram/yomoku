use yomoku_core::{
    analysis::SplitMode,
    document::PasteText,
    library::WordStatusCounts,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

mod effective;

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

#[test]
fn library_words_count_distinct_exact_surfaces_with_all_live_global_statuses() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(
        &mut storage,
        "猫 猫 𠮷 食べた が か\u{3099} 食べる English 。 📖 \0",
    );
    let second = import(&mut storage, "猫 猫 食べた");
    for (surface, status) in [
        ("𠮷", VocabularyStatus::Learning),
        ("食べた", VocabularyStatus::Familiar),
        ("が", VocabularyStatus::Known),
        ("か\u{3099}", VocabularyStatus::Ignored),
    ] {
        let entry = storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(entry.id, status, 200)
            .unwrap();
    }
    let result = storage.library_word_distributions().unwrap();
    assert_eq!(result[0].document_id, first.document.document_id);
    assert_eq!(
        result[0].counts,
        Some(WordStatusCounts {
            new: 2,
            learning: 1,
            familiar: 1,
            known: 1,
            ignored: 1,
        })
    );
    assert_eq!(result[1].document_id, second.document.document_id);
    assert_eq!(
        result[1].counts,
        Some(WordStatusCounts {
            new: 1,
            familiar: 1,
            ..Default::default()
        })
    );
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(cat.id, VocabularyStatus::Known, 201)
        .unwrap();
    let result = storage.library_word_distributions().unwrap();
    assert_eq!(result[0].counts.unwrap().new, 1);
    assert_eq!(result[0].counts.unwrap().known, 2);
    assert_eq!(result[1].counts.unwrap().new, 0);
    assert_eq!(result[1].counts.unwrap().known, 1);
}

#[test]
fn library_words_pin_original_b_and_deduplicate_across_epub_chapters() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let epub = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>猫 猫</p>"),
            "本.epub",
            100,
            fixture,
        )
        .unwrap();
    let EpubImportOutcome::Created(epub) = epub else {
        panic!("fresh import")
    };
    // The first fixture chapter has Japanese surfaces too; compare against exact
    // spellings across all original chapters, not the sum of chapter counts.
    let mut surfaces = std::collections::BTreeSet::new();
    for stored in &epub.document.chapters {
        let chapter = &stored.chapter;
        let original = fixture(chapter).unwrap();
        surfaces.extend(
            original
                .tokens
                .iter()
                .filter(|t| t.is_japanese)
                .map(|t| t.surface.clone()),
        );
        let mut later = original;
        later.profile.engine_revision = "later@2".into();
        for token in &mut later.tokens {
            token.is_japanese = false;
        }
        for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
            later.mode = mode;
            storage
                .store_chapter_analysis(chapter.chapter_id, &later)
                .unwrap();
        }
    }
    let result = storage.library_word_distributions().unwrap();
    assert_eq!(
        result[0].counts,
        Some(WordStatusCounts {
            new: surfaces.len() as u64,
            ..Default::default()
        })
    );
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage.library_word_distributions().unwrap()[0]
            .counts
            .unwrap()
            .new,
        surfaces.len() as u64 - 1
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.library_word_distributions().unwrap()[0]
            .counts
            .unwrap()
            .new,
        surfaces.len() as u64 - 1
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn library_words_distinguish_missing_analysis_zero_words_and_deleted_documents() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let legacy = storage
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    import(&mut storage, "English 。 📖");
    let deleted = import(&mut storage, "犬");
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(deleted.document.document_id)],
        )
        .unwrap();
    let result = storage.library_word_distributions().unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].document_id, legacy.document_id);
    assert_eq!(result[0].counts, None);
    assert_eq!(result[1].counts, Some(WordStatusCounts::default()));
    assert!(storage
        .vocabulary_entry(&Surface::new("犬").unwrap())
        .unwrap()
        .is_some());
}

#[test]
fn library_words_fail_closed_on_invalid_original_header_or_missing_chapters() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "猫");
    storage
        .connection
        .execute(
            "UPDATE chapter_analyses SET engine_revision = ' ' WHERE id = ?1",
            [i64::from(first.analysis_id)],
        )
        .unwrap();
    assert!(matches!(
        storage.library_word_distributions(),
        Err(StorageError::InvalidAnalysis)
    ));
    storage
        .connection
        .execute(
            "UPDATE chapter_analyses SET engine_revision = 'fixed' WHERE id = ?1",
            [i64::from(first.analysis_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(first.document.chapter.chapter_id)],
        )
        .unwrap();
    assert_eq!(
        storage.library_word_distributions().unwrap()[0].counts,
        None
    );
}
