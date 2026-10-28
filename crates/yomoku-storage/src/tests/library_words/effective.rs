use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride},
    document::{ChapterText, DocumentId, TextAnchor, TextOffset, TextRange},
};

use super::*;

fn edit(
    chapter: &ChapterText,
    start: usize,
    surface: &str,
    parts: &[&str],
    scope: OverrideScope,
) -> SegmentationOverride {
    SegmentationOverride {
        scope,
        anchor: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: TextOffset::at(&chapter.text, start).unwrap(),
                end: TextOffset::at(&chapter.text, start + surface.len()).unwrap(),
            },
        },
        surface: Surface::new(surface).unwrap(),
        parts: parts.iter().map(|s| Surface::new(*s).unwrap()).collect(),
    }
}

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
fn effective_library_words_follow_global_local_newer_rules_and_reversal() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "𠮷が 𠮷が 食べた 食べる English 。 📖 \0");
    let second = import(&mut storage, "𠮷 が");
    for (surface, status) in [
        ("𠮷", VocabularyStatus::Learning),
        ("が", VocabularyStatus::Familiar),
        ("食べた", VocabularyStatus::Known),
        ("食べる", VocabularyStatus::Ignored),
    ] {
        let entry = storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(entry.id, status, 200)
            .unwrap();
    }
    let chapter = &first.document.chapter;
    let global = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "𠮷が",
            &["𠮷", "が"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    let local = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "𠮷が",
            &["𠮷が"],
            OverrideScope::ThisOccurrence,
        ))
        .unwrap();
    let expected = WordStatusCounts {
        new: 1,
        learning: 1,
        familiar: 1,
        known: 1,
        ignored: 1,
    };
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(expected)
    );
    // Replacing the global removes its split-only surfaces from this book.
    let newer = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "𠮷が",
            &["𠮷が"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            learning: 0,
            familiar: 0,
            ..expected
        })
    );
    // A still newer global split cannot displace the local unsplit occurrence.
    let newest = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "𠮷が",
            &["𠮷", "が"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(expected)
    );
    storage.reverse_segmentation_override(newest).unwrap();
    assert!(!storage.reverse_segmentation_override(newer).unwrap());
    // Reversal does not reactivate the superseded global rule.
    assert!(
        !storage
            .segmentation_override(global)
            .unwrap()
            .unwrap()
            .active
    );
    storage.reverse_segmentation_override(local).unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            new: 1,
            known: 1,
            ignored: 1,
            ..Default::default()
        })
    );
    assert_eq!(
        counts(&mut storage, second.document.document_id),
        Some(WordStatusCounts {
            learning: 1,
            familiar: 1,
            ..Default::default()
        })
    );
    // The original-B API continues matching the live reader until its switch.
    assert_eq!(
        storage.library_word_distributions().unwrap()[0].counts,
        counts(&mut storage, first.document.document_id)
    );
}

#[test]
fn effective_library_words_merge_only_complete_contiguous_original_b_tokens() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = storage
        .create_analyzed_paste(
            &PasteText::new("猫猫 猫猫".into()).unwrap(),
            100,
            |chapter| {
                let mut analysis = fixture(chapter).unwrap();
                analysis.tokens = analysis
                    .tokens
                    .iter()
                    .flat_map(|token| {
                        let mut left = token.clone();
                        left.surface = "猫".into();
                        left.anchor.range.end =
                            TextOffset::try_from(i64::from(left.anchor.range.start) + 3).unwrap();
                        let mut right = token.clone();
                        right.surface = "猫".into();
                        right.anchor.range.start = left.anchor.range.end;
                        [left, right]
                    })
                    .collect();
                Ok::<_, std::convert::Infallible>(analysis)
            },
        )
        .unwrap();
    let second = import(&mut storage, "猫猫 猫猫猫");
    let merged = storage
        .vocabulary_entry(&Surface::new("猫猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(merged.id, VocabularyStatus::Known, 200)
        .unwrap();
    storage
        .save_segmentation_override(&edit(
            &first.document.chapter,
            0,
            "猫猫",
            &["猫猫"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    assert_eq!(
        storage.library_word_distributions().unwrap()[0]
            .counts
            .unwrap()
            .new,
        1
    );
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            known: 1,
            ..Default::default()
        })
    );
    // The same substring inside 猫猫猫 is not an original-B-aligned match.
    assert_eq!(
        counts(&mut storage, second.document.document_id),
        Some(WordStatusCounts {
            new: 1,
            known: 1,
            ..Default::default()
        })
    );
}

#[test]
fn effective_library_words_pin_b_and_use_exact_same_profile_metadata_classification() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "日本語 日本語");
    import(&mut storage, "日本 語");
    let chapter = &first.document.chapter;
    let mut alternate = fixture(chapter).unwrap();
    alternate.mode = SplitMode::A;
    alternate.tokens = alternate
        .tokens
        .iter()
        .flat_map(|token| {
            let mut left = token.clone();
            left.surface = "日本".into();
            left.anchor.range.end =
                TextOffset::try_from(i64::from(left.anchor.range.start) + 6).unwrap();
            left.is_japanese = false;
            let mut right = token.clone();
            right.surface = "語".into();
            right.anchor.range.start = left.anchor.range.end;
            [left, right]
        })
        .collect();
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    // A later profile cannot change either original B or exact A metadata.
    let mut foreign = alternate.clone();
    foreign.profile.engine_revision = "later@2".into();
    for token in &mut foreign.tokens {
        token.is_japanese = true;
    }
    for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
        foreign.mode = mode;
        storage
            .store_chapter_analysis(chapter.chapter_id, &foreign)
            .unwrap();
    }
    assert_eq!(
        counts(&mut storage, first.document.document_id)
            .unwrap()
            .new,
        1
    );
    let rule = storage
        .save_segmentation_override(&edit(
            chapter,
            0,
            "日本語",
            &["日本", "語"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    // Registered 日本 is excluded by exact non-Japanese analyzer metadata;
    // original 日本語 no longer belongs, and repeated 語 counts once.
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            new: 1,
            ..Default::default()
        })
    );
    let japanese = storage
        .vocabulary_entry(&Surface::new("語").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(japanese.id, VocabularyStatus::Ignored, 210)
        .unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            ignored: 1,
            ..Default::default()
        })
    );
    storage.reverse_segmentation_override(rule).unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts {
            new: 1,
            ..Default::default()
        })
    );
}

#[test]
fn effective_library_words_missing_deleted_and_unregistered_entries_stay_absent() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "食べた 食べた");
    let second = import(&mut storage, "食べ 食べる が か\u{3099}");
    let rule = storage
        .save_segmentation_override(&edit(
            &first.document.chapter,
            0,
            "食べた",
            &["食べ", "た"],
            OverrideScope::AllOccurrences,
        ))
        .unwrap();
    let original = storage.library_word_distributions().unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id)
            .unwrap()
            .new,
        1
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("た").unwrap())
        .unwrap()
        .is_none());
    let mut writer = Storage::open(database.path()).unwrap();
    let word = writer
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .unwrap();
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        writer.set_vocabulary_status(word.id, status, 220).unwrap();
        let expected = match status {
            VocabularyStatus::New => WordStatusCounts {
                new: 1,
                ..Default::default()
            },
            VocabularyStatus::Learning => WordStatusCounts {
                learning: 1,
                ..Default::default()
            },
            VocabularyStatus::Familiar => WordStatusCounts {
                familiar: 1,
                ..Default::default()
            },
            VocabularyStatus::Known => WordStatusCounts {
                known: 1,
                ..Default::default()
            },
            VocabularyStatus::Ignored => WordStatusCounts {
                ignored: 1,
                ..Default::default()
            },
        };
        assert_eq!(
            counts(&mut storage, first.document.document_id),
            Some(expected)
        );
        let other = counts(&mut storage, second.document.document_id).unwrap();
        assert_eq!(other.new, expected.new + 3); // literal inflection/combining spellings
        assert_eq!(other.ignored, expected.ignored);
    }
    writer
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(word.id)])
        .unwrap();
    drop(writer);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    let rows_before = super::super::registered_overrides::state(&storage);
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        counts(&mut storage, first.document.document_id),
        Some(WordStatusCounts::default())
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(
        super::super::registered_overrides::state(&storage),
        rows_before
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .is_none());
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    storage.reverse_segmentation_override(rule).unwrap();
    assert_eq!(
        storage.library_word_distributions().unwrap()[0],
        original[0]
    );
    assert_eq!(
        counts(&mut storage, first.document.document_id)
            .unwrap()
            .new,
        1
    );
}

#[test]
fn effective_library_words_deduplicate_chapters_and_keep_deleted_origin_global_rules() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let origin = import(&mut storage, "猫猫");
    storage
        .save_registered_segmentation_override(
            &edit(
                &origin.document.chapter,
                0,
                "猫猫",
                &["猫", "猫"],
                OverrideScope::AllOccurrences,
            ),
            200,
        )
        .unwrap();
    let EpubImportOutcome::Created(epub) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>猫猫 猫猫 猫 第一章</p>"),
            "本.epub",
            210,
            fixture,
        )
        .unwrap()
    else {
        panic!("fresh EPUB");
    };
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(cat.id, VocabularyStatus::Learning, 220)
        .unwrap();
    // Isolate cross-chapter deduplication: the first chapter has other words,
    // which must continue to contribute along with the single live 猫 spelling.
    let baseline = storage
        .library_word_distributions()
        .unwrap()
        .into_iter()
        .find(|row| row.document_id == epub.document.document_id)
        .unwrap()
        .counts
        .unwrap();
    let expected = WordStatusCounts {
        new: baseline.new - 1,
        ..baseline
    };
    assert_eq!(
        counts(&mut storage, epub.document.document_id),
        Some(expected)
    );
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(origin.document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(origin.document.chapter.chapter_id)],
        )
        .unwrap();
    // Distribution reads do not decode source archives or validate image BLOBs.
    storage.connection.execute_batch(
        "UPDATE imported_files SET original_bytes = X'00'; UPDATE epub_assets SET bytes = X'00'"
    ).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let rows = storage.effective_library_word_distributions().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].document_id, epub.document.document_id);
    assert_eq!(rows[0].counts, Some(expected));
    assert_eq!(fs::read(database.path()).unwrap(), before);
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
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
    let mut alternate = fixture(&legacy.chapter).unwrap();
    for mode in [SplitMode::A, SplitMode::C] {
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
fn effective_library_words_reject_corrupt_analysis_rules_and_live_status_without_writes() {
    for sql in [
        "UPDATE analyzed_tokens SET surface = '犬'",
        "UPDATE analyzed_tokens SET start_byte = 1",
        "UPDATE chapter_analyses SET engine_revision = ' '",
        "UPDATE segmentation_override_parts SET surface = '犬'",
        "DELETE FROM chapters", // disable FKs below, to simulate corruption
        "UPDATE vocabulary SET status = 'broken'",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫猫");
        storage
            .save_registered_segmentation_override(
                &edit(
                    &imported.document.chapter,
                    0,
                    "猫猫",
                    &["猫", "猫"],
                    OverrideScope::AllOccurrences,
                ),
                200,
            )
            .unwrap();
        storage
            .connection
            .execute_batch("PRAGMA foreign_keys = OFF; PRAGMA ignore_check_constraints = ON")
            .unwrap();
        storage.connection.execute_batch(sql).unwrap();
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        let before = fs::read(database.path()).unwrap();
        let result = storage.effective_library_word_distributions();
        if sql.contains("status") {
            assert!(
                matches!(result, Err(StorageError::InvalidVocabulary)),
                "{sql}: {result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(StorageError::InvalidAnalysis)),
                "{sql}: {result:?}"
            );
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}
