use super::*;

fn effective_snapshot(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
) -> yomoku_core::effective_reader::EffectiveReaderTokenSnapshot {
    storage
        .load_effective_reader_tokens(imported.document.document_id, key(imported).page)
        .unwrap()
        .unwrap()
}

#[test]
fn effective_reader_words_validate_pages_indices_missing_deleted_and_corrupt_analysis() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語だった English 。");
    let snapshot = effective_snapshot(&mut storage, &imported);
    let frozen = snapshot.key().unwrap();
    for index in [1, 2, 3, u64::MAX] {
        assert!(matches!(
            storage.load_effective_reader_word(&dictionary, &frozen, index),
            Err(ReaderWordError::InvalidSelection)
        ));
    }
    for range in [
        TextRange {
            start: 0.try_into().unwrap(),
            end: 0.try_into().unwrap(),
        },
        TextRange {
            start: 1.try_into().unwrap(),
            end: 3.try_into().unwrap(),
        },
    ] {
        let mut invalid = frozen.clone();
        invalid.page.range = range;
        assert!(matches!(
            storage.load_effective_reader_word(&dictionary, &invalid, 0),
            Err(ReaderWordError::Storage(StorageError::InvalidAnalysis))
        ));
    }
    let other = import(&mut storage, "別語");
    let mut foreign = frozen.clone();
    foreign.document_id = other.document.document_id;
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &foreign, 0),
        Err(ReaderWordError::Storage(StorageError::InvalidAnalysis))
    ));
    // Even off-page corruption fails before resolving the selected word.
    let mut first_token_page = frozen.clone();
    first_token_page.page.range.end = snapshot.tokens[0].anchor.range.end;
    storage.connection.execute("UPDATE analyzed_tokens SET surface = 'corrupt' WHERE analysis_id = ?1 AND token_index = 2", [i64::from(imported.analysis_id)]).unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &first_token_page, 0),
        Err(ReaderWordError::Storage(StorageError::InvalidAnalysis))
    ));
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    assert!(effective_snapshot(&mut storage, &imported).key().is_none());
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &frozen, 0),
        Err(ReaderWordError::InvalidSelection)
    ));
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    assert!(storage
        .load_effective_reader_word(&dictionary, &frozen, 0)
        .unwrap()
        .is_none());
}

#[test]
fn longest_token_lookup_suggests_parts_without_registering_or_counting_them() {
    use yomoku_core::analysis::{AnalyzedToken, SplitMode};
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語別語");
    let chapter = &imported.document.chapter;
    let before = storage
        .connection
        .query_row("SELECT COUNT(*) FROM vocabulary", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    let snapshot = effective_snapshot(&mut storage, &imported);
    assert_eq!(
        snapshot
            .tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["試験語別語"]
    );
    let frozen = snapshot.key().unwrap();
    for mode in [SplitMode::A, SplitMode::B] {
        let mut analysis = storage
            .chapter_analysis(imported.analysis_id)
            .unwrap()
            .unwrap();
        analysis.mode = mode;
        analysis.tokens = [
            ("試験語", 0, "試験語".len()),
            ("別語", "試験語".len(), chapter.text.len()),
        ]
        .into_iter()
        .map(|(surface, start, end)| AnalyzedToken {
            anchor: TextAnchor {
                range: TextRange {
                    start: TextOffset::at(&chapter.text, start).unwrap(),
                    end: TextOffset::at(&chapter.text, end).unwrap(),
                },
                ..key(&imported).page
            },
            surface: surface.into(),
            is_japanese: true,
            reading: None,
            dictionary_form: Some(Surface::new("試験語").unwrap()),
        })
        .collect();
        storage
            .store_chapter_analysis(chapter.chapter_id, &analysis)
            .unwrap();
    }
    let word = storage
        .load_effective_reader_word(&dictionary, &frozen, 0)
        .unwrap()
        .unwrap();
    assert_eq!(word.token.surface, "試験語別語");
    assert!(word.candidates.words.is_empty() && word.candidates.names.is_empty());
    assert_eq!(
        word.parts
            .iter()
            .map(|part| part.surface.as_str())
            .collect::<Vec<_>>(),
        ["試験語", "別語"]
    );
    assert_eq!(word.parts[1].dictionary_form.as_deref(), Some("試験語"));
    let chosen = dictionary
        .lookup(
            &word.parts[1].surface,
            word.parts[1].dictionary_form.as_deref(),
        )
        .unwrap();
    assert_eq!(chosen.words.len(), 1);
    assert_eq!(chosen.names.len(), 1);
    assert_eq!(word.encountered, 0);
    assert_eq!(
        before,
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM vocabulary", [], |r| r
                .get::<_, i64>(0))
            .unwrap()
    );
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        effective_snapshot(&mut storage, &imported).key().unwrap(),
        frozen
    );
}
