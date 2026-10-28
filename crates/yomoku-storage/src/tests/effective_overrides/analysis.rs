use super::*;
use crate::effective_overrides::EffectiveOverrideAnalysisError as Error;
use yomoku_core::analysis::{AnalysisProfile, ChapterAnalysis};

fn alternate(
    chapter: &ChapterText,
    mode: SplitMode,
    profile: &AnalysisProfile,
) -> Result<ChapterAnalysis, &'static str> {
    Ok(ChapterAnalysis {
        mode,
        profile: profile.clone(),
        tokens: chapter
            .text
            .char_indices()
            .filter(|(_, c)| !c.is_whitespace())
            .map(|(start, c)| AnalyzedToken {
                anchor: span(chapter, start, start + c.len_utf8()),
                surface: c.to_string(),
                is_japanese: true,
                reading: Some(format!("読み{c}")),
                dictionary_form: None,
            })
            .collect(),
    })
}

#[test]
fn effective_override_analysis_generates_whole_chapter_and_reuses_stored_modes_read_only_after_reopen(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬 猫犬");
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    let before = state(&storage);
    for mode in [SplitMode::A, SplitMode::C] {
        let prompt = context(&mut storage, &imported, "猫犬");
        let id = storage
            .generate_effective_override_analysis(&prompt, mode, |chapter, selected, profile| {
                assert_eq!(chapter, &imported.document.chapter);
                assert_eq!(selected, mode);
                assert_eq!(profile, &original.profile);
                alternate(chapter, selected, profile)
            })
            .unwrap();
        assert_eq!(
            storage.chapter_analysis(id).unwrap().unwrap().tokens.len(),
            4
        );
    }
    let after = state(&storage);
    assert_eq!(&after[..4], &before[..4]);
    assert_eq!(&after[6..], &before[6..]);
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(original)
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let prompt = context(&mut storage, &imported, "猫犬");
    assert_eq!(
        prompt.choices().iter().map(|c| c.mode).collect::<Vec<_>>(),
        [SplitMode::A, SplitMode::B, SplitMode::C]
    );
    assert_eq!(prompt.choices()[0].parts, parts(&["猫", "犬"]));
    storage
        .connection
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    for mode in [SplitMode::A, SplitMode::C] {
        storage
            .generate_effective_override_analysis::<()>(&prompt, mode, |_, _, _| {
                panic!("stored mode must not load Full")
            })
            .unwrap();
    }
    assert_eq!(state(&storage), after);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn effective_override_analysis_rejects_wrong_profile_mode_anchors_coverage_and_engine_failures_without_writes(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬");
    let prompt = context(&mut storage, &imported, "猫犬");
    let before = state(&storage);
    for invalid in 0..5 {
        let result = storage.generate_effective_override_analysis(
            &prompt,
            SplitMode::A,
            |chapter, mode, profile| {
                let mut output = alternate(chapter, mode, profile)?;
                match invalid {
                    0 => output.profile.engine_revision.push_str("-other"),
                    1 => output.profile.dictionary_revision.push_str("-other"),
                    2 => output.mode = SplitMode::C,
                    3 => output.tokens[0].anchor.chapter_id = 999.try_into().unwrap(),
                    _ => {
                        output.tokens.pop();
                    }
                }
                Ok::<_, &'static str>(output)
            },
        );
        assert!(matches!(
            result,
            Err(Error::Context(EffectiveOverrideError::Storage(
                StorageError::InvalidAnalysis
            )))
        ));
        assert_eq!(state(&storage), before);
    }
    assert!(matches!(
        storage
            .generate_effective_override_analysis(&prompt, SplitMode::C, |_, _, _| Err("offline")),
        Err(Error::Analysis("offline"))
    ));
    assert!(matches!(
        storage.generate_effective_override_analysis::<()>(
            &prompt,
            SplitMode::B,
            |_, _, _| panic!("B is immutable")
        ),
        Err(Error::Context(EffectiveOverrideError::InvalidSelection))
    ));
    assert_eq!(state(&storage), before);
}

#[test]
fn effective_override_analysis_rechecks_intervening_edits_after_computing_without_holding_sqlite_lock(
) {
    for reverse in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫犬");
        let prompt = context(&mut storage, &imported, "猫犬");
        let mut other = Storage::open(database.path()).unwrap();
        let result = storage.generate_effective_override_analysis(
            &prompt,
            SplitMode::A,
            |chapter, mode, profile| {
                let (id, _) =
                    split(&mut other, &prompt, OverrideScope::default(), &["猫", "犬"]).unwrap();
                if reverse {
                    other.reverse_segmentation_override(id).unwrap();
                }
                alternate(chapter, mode, profile)
            },
        );
        assert!(matches!(
            result,
            Err(Error::Context(EffectiveOverrideError::StaleContext))
        ));
        let before = state(&storage);
        assert_eq!(before[4].len(), 1);
        assert!(matches!(
            storage.generate_effective_override_analysis::<()>(
                &prompt,
                SplitMode::A,
                |_, _, _| panic!("stale before Full")
            ),
            Err(Error::Context(EffectiveOverrideError::StaleContext))
        ));
        assert_eq!(state(&storage), before);
    }
}

#[test]
fn effective_override_analysis_statement_and_commit_failures_roll_back_and_retry_frozen_context_after_reopen(
) {
    for commit in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫犬");
        let prompt = context(&mut storage, &imported, "猫犬");
        if commit {
            storage.connection.execute_batch("CREATE TABLE parent (id INTEGER PRIMARY KEY);
                CREATE TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TRIGGER fail_analysis AFTER INSERT ON chapter_analyses BEGIN INSERT INTO child VALUES (1); END;").unwrap();
        } else {
            storage.connection.execute_batch("CREATE TRIGGER fail_analysis AFTER INSERT ON analyzed_tokens BEGIN SELECT RAISE(ABORT, 'analysis test'); END;").unwrap();
        }
        let before = state(&storage);
        assert!(matches!(
            storage.generate_effective_override_analysis(&prompt, SplitMode::A, alternate),
            Err(Error::Context(EffectiveOverrideError::Storage(
                StorageError::Sqlite(_)
            )))
        ));
        assert_eq!(state(&storage), before);
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_analysis")
            .unwrap();
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        storage
            .generate_effective_override_analysis(&prompt, SplitMode::A, alternate)
            .unwrap();
        assert_eq!(state(&storage)[4].len(), 2);
    }
}

#[test]
fn effective_override_analysis_new_metadata_retires_old_context_and_lost_success_cannot_rebase() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬");
    let initial = context(&mut storage, &imported, "猫犬");
    split(
        &mut storage,
        &initial,
        OverrideScope::default(),
        &["猫", "犬"],
    )
    .unwrap();
    let prompt = context(&mut storage, &imported, "猫犬");
    assert!(prompt
        .key()
        .segmentation
        .segments
        .iter()
        .all(|s| s.metadata.is_none()));
    let before = state(&storage);
    storage
        .generate_effective_override_analysis(&prompt, SplitMode::A, alternate)
        .unwrap();
    let after = state(&storage);
    assert_eq!(&after[..4], &before[..4]);
    assert_eq!(&after[6..], &before[6..]);
    let refreshed = context(&mut storage, &imported, "猫犬");
    assert!(refreshed.key().segmentation.segments.iter().all(|s| s
        .metadata
        .as_ref()
        .unwrap()
        .mode
        == SplitMode::A));
    assert_ne!(refreshed.key(), prompt.key());
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(matches!(
        storage.generate_effective_override_analysis::<()>(
            &prompt,
            SplitMode::A,
            |_, _, _| panic!("lost success requires reload")
        ),
        Err(Error::Context(EffectiveOverrideError::StaleContext))
    ));
    storage
        .generate_effective_override_analysis::<()>(&refreshed, SplitMode::A, |_, _, _| {
            panic!("already stored")
        })
        .unwrap();
    assert_eq!(state(&storage), after);
}

#[test]
fn effective_override_analysis_preserves_crossing_unicode_and_omits_alternates_unaligned_with_selection(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "𠮷か\u{3099}\0猫 犬");
    let chapter = &imported.document.chapter;
    let page = span(chapter, 4, 7); // inside original B, including supplementary source
    let selection = span(chapter, 0, chapter.text.find(' ').unwrap());
    let key = key(&mut storage, &imported, page);
    let prompt = storage
        .load_effective_override_context(key, selection)
        .unwrap();
    // C merges across whitespace in this synthetic fixture. The full snapshot
    // is valid, but there is no exact C partition of the requested B span.
    let id = storage
        .generate_effective_override_analysis(&prompt, SplitMode::C, |whole, mode, profile| {
            assert_eq!(whole, chapter);
            let anchor = span(whole, 0, whole.text.len());
            Ok::<_, ()>(ChapterAnalysis {
                mode,
                profile: profile.clone(),
                tokens: vec![AnalyzedToken {
                    anchor,
                    surface: whole.text.clone(),
                    is_japanese: true,
                    reading: None,
                    dictionary_form: None,
                }],
            })
        })
        .unwrap();
    assert_eq!(
        storage.chapter_analysis(id).unwrap().unwrap().tokens[0].surface,
        chapter.text
    );
    let prompt = storage
        .load_effective_override_context(Arc::new(prompt.key().clone()), selection)
        .unwrap();
    assert_eq!(
        prompt.choices().iter().map(|c| c.mode).collect::<Vec<_>>(),
        [SplitMode::B]
    );
    let bytes = fs::read(database.path()).unwrap();
    storage
        .generate_effective_override_analysis::<()>(&prompt, SplitMode::C, |_, _, _| {
            panic!("stored but unaligned must not regenerate")
        })
        .unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn effective_override_analysis_keeps_accepted_completion_frozen_across_new_metadata_and_reopen() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
        pagination::PageLength,
        reader::ReaderPage,
    };
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫犬");
    let initial = context(&mut storage, &imported, "猫犬");
    split(
        &mut storage,
        &initial,
        OverrideScope::default(),
        &["猫", "犬"],
    )
    .unwrap();
    let prompt = context(&mut storage, &imported, "猫犬");
    let page = ReaderPage::open(&imported.document, PageLength::default()).unwrap();
    let time = storage
        .load_pending_reading_time(imported.document.document_id, page.position)
        .unwrap();
    storage
        .save_pending_reading_time(time, PendingReadingTime::from_millis(987).unwrap())
        .unwrap();
    let prepared = storage
        .prepare_effective_page_completion(
            PageCompletionIntent {
                document_id: imported.document.document_id,
                page: page.anchor,
                page_length: PageLength::default(),
                departing_position: page.position,
                completed_at_unix_ms: 500,
                analysis_id: imported.analysis_id,
            },
            prompt.key(),
        )
        .unwrap();
    assert!(prepared.occurrences().iter().all(|o| o.metadata.is_none()));
    storage
        .generate_effective_override_analysis(&prompt, SplitMode::A, alternate)
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&prepared).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
    );
    let bytes = fs::read(database.path()).unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&prepared).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert_eq!(
        storage
            .connection
            .query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        987
    );
    assert_eq!(
        storage
            .connection
            .query_row(
                "SELECT COUNT(*) FROM reading_occurrences WHERE dictionary_form IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
