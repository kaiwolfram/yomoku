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

fn split_effective(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
) -> yomoku_core::document::OverrideId {
    use yomoku_core::analysis::{OverrideScope, SegmentationOverride};
    let chapter = &imported.document.chapter;
    storage
        .save_segmentation_override(&SegmentationOverride {
            scope: OverrideScope::AllOccurrences,
            anchor: TextAnchor {
                range: TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(&chapter.text, "試験語だった".len()).unwrap(),
                },
                ..key(imported).page
            },
            surface: Surface::new("試験語だった").unwrap(),
            parts: ["試験語", "だった"]
                .map(|s| Surface::new(s).unwrap())
                .into(),
        })
        .unwrap()
}

#[test]
fn effective_reader_words_keep_manual_metadata_unresolved_and_read_only_custom_and_builtin_views() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語だった 試験語だった English 。 📖 \0");
    split_effective(&mut storage, &imported);
    // Existing global custom entries may predate an override or a dictionary update.
    for surface in ["試験語", "だった"] {
        storage
            .connection
            .execute(
                "INSERT INTO custom_dictionary_entries VALUES (?1, 1, ?2, ?3)",
                (surface, " literal meaning\n𠮷\0 ", " よみ\u{3099} "),
            )
            .unwrap();
    }
    let snapshot = effective_snapshot(&mut storage, &imported);
    let frozen = snapshot.key().unwrap();
    assert_eq!(snapshot.tokens.len(), 4);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let index_before = fs::read(index.path()).unwrap();
    for token in &snapshot.tokens {
        let details = storage
            .load_effective_reader_word(&dictionary, &frozen, token.segment_index)
            .unwrap()
            .unwrap();
        assert_eq!(details.key, frozen);
        assert_eq!(details.token, *token);
        assert!(details.token.metadata.is_none());
        assert!(details.token.vocabulary.is_none());
        assert_eq!(details.encountered, 0);
        assert!(details.candidates.dictionary_form.is_none());
        if token.surface == "試験語" {
            assert_eq!(details.candidates.words.len(), 1);
            assert_eq!(details.candidates.names.len(), 1);
            assert_eq!(details.candidates.words[0].forms[0].matches.len(), 2);
            assert_eq!(details.candidates.words[0].entry.senses.len(), 3);
            assert!(details.custom_entry.is_none());
        } else {
            assert!(details.candidates.words.is_empty() && details.candidates.names.is_empty());
            let custom = details.custom_entry.unwrap();
            assert_eq!(custom.surface, "だった");
            assert_eq!(custom.english_meaning, " literal meaning\n𠮷\0 ");
            assert_eq!(custom.reading, " よみ\u{3099} ");
        }
    }
    assert_eq!(effective_snapshot(&mut storage, &imported), snapshot);
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(fs::read(index.path()).unwrap(), index_before);
}

#[test]
fn effective_reader_words_require_current_resolution_including_override_and_metadata_provenance() {
    use yomoku_core::analysis::{AnalyzedToken, ChapterAnalysis, SplitMode};
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語だった 試験語だった");
    let rule = split_effective(&mut storage, &imported);
    let snapshot = effective_snapshot(&mut storage, &imported);
    let frozen = snapshot.key().unwrap();
    storage.reverse_segmentation_override(rule).unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &frozen, 0),
        Err(ReaderWordError::InvalidSelection)
    ));
    let replacement = split_effective(&mut storage, &imported);
    assert_ne!(replacement, rule);
    // Same spelling/boundaries, new rule identity: old selections stay stale.
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &frozen, 0),
        Err(ReaderWordError::InvalidSelection)
    ));
    let fresh = effective_snapshot(&mut storage, &imported).key().unwrap();
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    let alternate = ChapterAnalysis {
        mode: SplitMode::A,
        profile: original.profile,
        tokens: snapshot
            .tokens
            .iter()
            .map(|token| AnalyzedToken {
                anchor: token.anchor,
                surface: token.surface.clone(),
                is_japanese: token.segment_index != 1,
                reading: Some(format!("context-{}", token.segment_index)),
                dictionary_form: Some(Surface::new("試験語").unwrap()),
            })
            .collect(),
    };
    let alternate_id = storage
        .store_chapter_analysis(imported.document.chapter.chapter_id, &alternate)
        .unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &fresh, 0),
        Err(ReaderWordError::InvalidSelection)
    ));
    let current = effective_snapshot(&mut storage, &imported);
    let current_key = current.key().unwrap();
    assert_eq!(
        current
            .tokens
            .iter()
            .map(|t| t.segment_index)
            .collect::<Vec<_>>(),
        [0, 2, 3]
    );
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &current_key, 1),
        Err(ReaderWordError::InvalidSelection)
    ));
    for token in &current.tokens {
        let details = storage
            .load_effective_reader_word(&dictionary, &current_key, token.segment_index)
            .unwrap()
            .unwrap();
        let metadata = details.token.metadata.unwrap();
        assert_eq!(metadata.analysis_id, alternate_id);
        assert_eq!(
            metadata.reading,
            Some(format!("context-{}", token.segment_index))
        );
        assert_eq!(
            details.candidates.dictionary_form.as_deref(),
            Some("試験語")
        );
        assert_eq!(details.candidates.words.len(), 1);
        assert_eq!(details.candidates.names.len(), 1);
    }
    let mut forged = current_key.clone();
    forged.segmentation.segments[0]
        .metadata
        .as_mut()
        .unwrap()
        .dictionary_form = Some(Surface::new("別語").unwrap());
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &forged, 0),
        Err(ReaderWordError::InvalidSelection)
    ));
}

#[test]
fn effective_reader_words_use_live_exact_surface_counts_and_status_without_registration() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let history = import(&mut storage, "試験語 試験語");
    complete(&mut storage, &history);
    let target = import(&mut storage, "試験語だった 試験語だった");
    split_effective(&mut storage, &target);
    let snapshot = effective_snapshot(&mut storage, &target);
    let frozen = snapshot.key().unwrap();
    let entry = storage
        .vocabulary_entry(&Surface::new("試験語").unwrap())
        .unwrap()
        .unwrap();
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        storage
            .set_vocabulary_status(entry.id, status, 600)
            .unwrap();
        for segment_index in [0, 2] {
            let details = storage
                .load_effective_reader_word(&dictionary, &frozen, segment_index)
                .unwrap()
                .unwrap();
            assert_eq!(details.token.vocabulary.unwrap().status, status);
            assert_eq!(details.encountered, 2);
        }
        assert_eq!(
            effective_snapshot(&mut storage, &target).key().unwrap(),
            frozen
        );
    }
    // Deleting originating content preserves immutable history for this spelling.
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(history.document.document_id)],
        )
        .unwrap();
    assert_eq!(
        storage
            .load_effective_reader_word(&dictionary, &frozen, 0)
            .unwrap()
            .unwrap()
            .encountered,
        2
    );
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(entry.id)],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let details = storage
        .load_effective_reader_word(&dictionary, &frozen, 0)
        .unwrap()
        .unwrap();
    assert!(details.token.vocabulary.is_none());
    assert_eq!(details.encountered, 0);
    assert_eq!(fs::read(database.path()).unwrap(), before);
    storage
        .register_vocabulary(&[Surface::new("試験語").unwrap()], 700)
        .unwrap();
    let details = storage
        .load_effective_reader_word(&dictionary, &frozen, 0)
        .unwrap()
        .unwrap();
    assert_ne!(details.token.vocabulary.unwrap().id, entry.id);
    assert_eq!(
        details.token.vocabulary.unwrap().status,
        VocabularyStatus::New
    );
    assert_eq!(details.encountered, 0);
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
fn effective_reader_words_keep_full_unicode_identity_across_every_page_preset() {
    use yomoku_core::analysis::{OverrideScope, SegmentationOverride};
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let tail = format!("{}終", "𠮷か\u{3099}。\0\n".repeat(600));
    let text = format!("試験語{tail}");
    let imported = storage
        .create_analyzed_paste(&PasteText::new(text).unwrap(), 100, |chapter| {
            let mut analysis = fixture(chapter).unwrap();
            analysis.tokens.truncate(1);
            analysis.tokens[0].surface = chapter.text.clone();
            analysis.tokens[0].anchor.range.end =
                TextOffset::at(&chapter.text, chapter.text.len()).unwrap();
            Ok::<_, Infallible>(analysis)
        })
        .unwrap();
    let chapter = &imported.document.chapter;
    storage
        .save_segmentation_override(&SegmentationOverride {
            anchor: key(&imported).page,
            scope: OverrideScope::ThisOccurrence,
            surface: Surface::new(&chapter.text).unwrap(),
            parts: [
                Surface::new("試験語").unwrap(),
                Surface::new(&tail).unwrap(),
            ]
            .into(),
        })
        .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for preset in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(chapter), preset).unwrap();
        assert!(pages.pages().len() > 1);
        for page in pages.pages() {
            let snapshot = storage
                .load_effective_reader_tokens(imported.document.document_id, *page)
                .unwrap()
                .unwrap();
            let frozen = snapshot.key().unwrap();
            let details = storage
                .load_effective_reader_word(&dictionary, &frozen, 1)
                .unwrap()
                .unwrap();
            assert_eq!(details.token.surface, tail);
            assert_eq!(
                details.token.anchor.range.start,
                TextOffset::at(&chapter.text, "試験語".len()).unwrap()
            );
            assert!(details.token.metadata.is_none());
            assert!(details.candidates.dictionary_form.is_none());
            assert!(details.token.vocabulary.is_none());
            assert!(details
                .token
                .display_range
                .end
                .byte_index(page.slice(chapter).unwrap())
                .is_ok());
            if page.range.start != TextOffset::START {
                assert!(matches!(
                    storage.load_effective_reader_word(&dictionary, &frozen, 0),
                    Err(ReaderWordError::InvalidSelection)
                ));
            }
        }
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn effective_reader_words_propagate_dictionary_and_custom_corruption_then_retry_read_only() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語だった");
    split_effective(&mut storage, &imported);
    let frozen = effective_snapshot(&mut storage, &imported).key().unwrap();
    let index_writer = Connection::open(index.path()).unwrap();
    index_writer
        .execute(
            "UPDATE entries SET payload = 'bad' WHERE source = 'JMdict'",
            [],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &frozen, 0),
        Err(ReaderWordError::Dictionary(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    // Retry does not cache a failed/empty candidate response.
    let clean_index = self::dictionary();
    let repaired = Dictionary::open(clean_index.path()).unwrap();
    assert_eq!(
        storage
            .load_effective_reader_word(&repaired, &frozen, 0)
            .unwrap()
            .unwrap()
            .candidates
            .words
            .len(),
        1
    );
    storage
        .connection
        .execute(
            "INSERT INTO custom_dictionary_entries VALUES ('だった', 1, ' ', 'よみ')",
            [],
        )
        .unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&repaired, &frozen, 1),
        Err(ReaderWordError::InvalidCustomEntry)
    ));
    storage
        .connection
        .execute(
            "UPDATE custom_dictionary_entries SET english_meaning = 'was'",
            [],
        )
        .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage
            .load_effective_reader_word(&repaired, &frozen, 1)
            .unwrap()
            .unwrap()
            .custom_entry
            .unwrap()
            .english_meaning,
        "was"
    );
}
