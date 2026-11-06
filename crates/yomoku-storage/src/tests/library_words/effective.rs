use yomoku_core::document::DocumentId;

use super::*;

fn counts(storage: &mut Storage, document: DocumentId) -> Option<WordStatusCounts> {
    storage
        .effective_library_word_distributions()
        .unwrap()
        .into_iter()
        .find(|row| row.document_id == document)
        .unwrap()
        .counts
}

#[test]
fn effective_library_words_distinguish_missing_b_from_zero_and_empty_library() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(storage
        .effective_library_word_distributions()
        .unwrap()
        .is_empty());
    let legacy = storage
        .unanalyzed_fixture(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    let mut alternate = fixture(&legacy.chapter).unwrap();
    for mode in [SplitMode::A, SplitMode::B] {
        alternate.mode = mode;
        storage
            .store_chapter_analysis(legacy.chapter.chapter_id, &alternate)
            .unwrap();
    }
    let empty = import(&mut storage, "English 。 📖 \0");
    assert_eq!(counts(&mut storage, legacy.document_id), None);
    assert_eq!(
        counts(&mut storage, empty.document.document_id),
        Some(WordStatusCounts::default())
    );
    let EpubImportOutcome::Created(epub) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>猫</p>"),
            "本.epub",
            100,
            fixture,
        )
        .unwrap()
    else {
        panic!("fresh EPUB");
    };
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE chapter_id = ?1",
            [i64::from(epub.document.chapters[1].chapter.chapter_id)],
        )
        .unwrap();
    assert_eq!(counts(&mut storage, epub.document.document_id), None);
}

#[test]
fn effective_library_shared_spellings_refresh_after_external_changes_and_failed_scan() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut ids = Vec::new();
    for index in 0..24 {
        // Repeated identical spellings across books still count once PER book.
        ids.push(
            import(
                &mut storage,
                &format!("猫 猫 𠮷 が か\u{3099} 食べた{}", " ".repeat(index)),
            )
            .document
            .document_id,
        );
    }
    let surfaces = ["猫", "𠮷", "が", "か\u{3099}", "食べた"].map(|s| Surface::new(s).unwrap());
    let statuses = [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ];
    let mut writer = Storage::open(database.path()).unwrap();
    for (surface, status) in surfaces.iter().zip(statuses) {
        let word = writer.vocabulary_entry(surface).unwrap().unwrap();
        writer.set_vocabulary_status(word.id, status, 200).unwrap();
    }
    let expected = WordStatusCounts {
        new: 1,
        learning: 1,
        familiar: 1,
        known: 1,
        ignored: 1,
    };
    let assert_all = |storage: &mut Storage, expected| {
        let before = fs::read(database.path()).unwrap();
        let rows = storage.effective_library_word_distributions().unwrap();
        assert_eq!(rows.len(), ids.len());
        for (row, id) in rows.iter().zip(&ids) {
            assert_eq!(row.document_id, *id);
            assert_eq!(row.counts, Some(expected));
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    };
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_all(&mut storage, expected);
    let cat = writer.vocabulary_entry(&surfaces[0]).unwrap().unwrap();
    writer
        .set_vocabulary_status(cat.id, VocabularyStatus::Known, 210)
        .unwrap();
    let expected = WordStatusCounts {
        new: 0,
        known: 2,
        ..expected
    };
    assert_all(&mut storage, expected);
    writer
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    let expected = WordStatusCounts {
        known: 1,
        ..expected
    };
    assert_all(&mut storage, expected);
    writer.register_vocabulary(&surfaces[..1], 220).unwrap();
    let replacement = writer.vocabulary_entry(&surfaces[0]).unwrap().unwrap();
    assert_ne!(replacement.id, cat.id);
    writer
        .set_vocabulary_status(replacement.id, VocabularyStatus::Ignored, 230)
        .unwrap();
    let expected = WordStatusCounts {
        ignored: 2,
        ..expected
    };
    assert_all(&mut storage, expected);

    // A scan that has already resolved earlier documents must fail if a later
    // analysis is corrupt; its observations cannot leak into the next scan.
    let last = i64::from(*ids.last().unwrap());
    writer.connection.execute(
        "UPDATE chapter_analyses SET engine_revision = ' ' WHERE chapter_id =
         (SELECT c.id FROM chapters c JOIN documents d ON d.source_id = c.source_id WHERE d.id = ?1)",
        [last],
    ).unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.effective_library_word_distributions(),
        Err(StorageError::InvalidAnalysis)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    writer.connection.execute(
        "UPDATE chapter_analyses SET engine_revision = 'import-fixture@1' WHERE chapter_id =
         (SELECT c.id FROM chapters c JOIN documents d ON d.source_id = c.source_id WHERE d.id = ?1)",
        [last],
    ).unwrap();
    writer
        .set_vocabulary_status(replacement.id, VocabularyStatus::Learning, 240)
        .unwrap();
    assert_all(
        &mut storage,
        WordStatusCounts {
            learning: 2,
            ignored: 1,
            ..expected
        },
    );
}
