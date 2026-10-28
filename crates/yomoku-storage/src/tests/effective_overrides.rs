use yomoku_core::{
    analysis::{AnalyzedToken, OverrideScope, SplitMode},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveReaderTokenKey,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, registered_overrides::state, *};
use crate::effective_overrides::{EffectiveOverrideContext, EffectiveOverrideError};

mod analysis;

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

fn span(chapter: &ChapterText, start: usize, end: usize) -> TextAnchor {
    TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::at(&chapter.text, start).unwrap(),
            end: TextOffset::at(&chapter.text, end).unwrap(),
        },
    }
}

fn key(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
    page: TextAnchor,
) -> Arc<EffectiveReaderTokenKey> {
    Arc::new(
        storage
            .load_effective_reader_tokens(imported.document.document_id, page)
            .unwrap()
            .unwrap()
            .key()
            .unwrap(),
    )
}

fn context(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
    surface: &str,
) -> EffectiveOverrideContext {
    let chapter = &imported.document.chapter;
    let page = span(chapter, 0, chapter.text.len());
    let key = key(storage, imported, page);
    let start = chapter.text.find(surface).unwrap();
    storage
        .load_effective_override_context(key.clone(), span(chapter, start, start + surface.len()))
        .unwrap()
}

fn parts(values: &[&str]) -> Vec<Surface> {
    values
        .iter()
        .map(|value| Surface::new(*value).unwrap())
        .collect()
}

fn split(
    storage: &mut Storage,
    context: &EffectiveOverrideContext,
    scope: OverrideScope,
    values: &[&str],
) -> Result<(yomoku_core::document::OverrideId, usize), EffectiveOverrideError> {
    storage.save_effective_override(context, scope, parts(values), 120)
}

fn surfaces(storage: &mut Storage, imported: &AnalyzedPaste) -> Vec<String> {
    let chapter = &imported.document.chapter;
    storage
        .load_effective_reader_tokens(
            imported.document.document_id,
            span(chapter, 0, chapter.text.len()),
        )
        .unwrap()
        .unwrap()
        .tokens
        .into_iter()
        .map(|token| token.surface)
        .collect()
}

#[test]
fn effective_override_context_is_read_only_and_offers_only_exact_same_profile_stored_choices() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた");
    let chapter = &imported.document.chapter;
    let mut alternate = fixture(chapter).unwrap();
    alternate.mode = SplitMode::A;
    alternate.tokens = [span(chapter, 0, 6), span(chapter, 6, 9)]
        .into_iter()
        .map(|anchor| AnalyzedToken {
            anchor,
            surface: anchor.slice(chapter).unwrap().into(),
            is_japanese: true,
            reading: None,
            dictionary_form: None,
        })
        .collect();
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    alternate.mode = SplitMode::C;
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    alternate.mode = SplitMode::A;
    alternate.profile.engine_revision.push_str("-foreign");
    alternate.tokens = fixture(chapter).unwrap().tokens;
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    let before = state(&storage);
    let bytes = fs::read(database.path()).unwrap();
    storage
        .connection
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    let context = context(&mut storage, &imported, "食べた");
    assert_eq!(context.anchor(), span(chapter, 0, 9));
    assert_eq!(context.surface(), &Surface::new("食べた").unwrap());
    assert_eq!(
        context
            .choices()
            .iter()
            .map(|choice| choice.mode)
            .collect::<Vec<_>>(),
        [SplitMode::A, SplitMode::B, SplitMode::C]
    );
    assert_eq!(context.choices()[0].parts, parts(&["食べ", "た"]));
    assert_eq!(context.choices()[1].parts, parts(&["食べた"]));
    assert!(context.rules().is_empty());
    assert_eq!(state(&storage), before);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn effective_override_save_registers_winners_once_and_reversal_preserves_history_and_statuses() {
    for scope in [OverrideScope::AllOccurrences, OverrideScope::ThisOccurrence] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた 食べた");
        import(&mut storage, "食べ");
        let existing = storage
            .vocabulary_entry(&Surface::new("食べ").unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(existing.id, VocabularyStatus::Familiar, 110)
            .unwrap();
        let before = state(&storage);
        let prompt = context(&mut storage, &imported, "食べた");
        let (id, registered) = split(&mut storage, &prompt, scope, &["食べ", "た"]).unwrap();
        assert_eq!(registered, 1);
        assert_eq!(
            surfaces(&mut storage, &imported),
            if scope == OverrideScope::AllOccurrences {
                vec!["食べ", "た", "食べ", "た"]
            } else {
                vec!["食べ", "た", "食べた"]
            }
        );
        assert_eq!(
            storage
                .vocabulary_entry(&Surface::new("食べ").unwrap())
                .unwrap()
                .unwrap()
                .status,
            VocabularyStatus::Familiar
        );
        assert_eq!(&state(&storage)[4..], &before[4..]);
        let reverse = context(&mut storage, &imported, "食べた");
        assert_eq!(
            reverse
                .rules()
                .iter()
                .map(|rule| rule.id)
                .collect::<Vec<_>>(),
            [id]
        );
        let registered_state = state(&storage);
        storage.reverse_effective_override(&reverse, id).unwrap();
        assert_eq!(&state(&storage)[1..], &registered_state[1..]);
        assert_eq!(surfaces(&mut storage, &imported), ["食べた", "食べた"]);
        // Both old prompts are now stale, including after the page returned to B.
        let before = state(&storage);
        assert!(matches!(
            split(&mut storage, &prompt, scope, &["食", "べた"]),
            Err(EffectiveOverrideError::StaleContext)
        ));
        assert!(matches!(
            storage.reverse_effective_override(&reverse, id),
            Err(EffectiveOverrideError::StaleContext)
        ));
        assert_eq!(state(&storage), before);
    }
}

#[test]
fn effective_override_intervening_reversal_invalidates_identical_page_and_unrelated_prompts() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫");
    let other = import(&mut storage, "犬犬");
    let old = context(&mut storage, &imported, "猫猫");
    let other_prompt = context(&mut storage, &other, "犬犬");
    let original = key(&mut storage, &imported, old.anchor());
    let (id, _) = split(
        &mut storage,
        &other_prompt,
        OverrideScope::default(),
        &["犬", "犬"],
    )
    .unwrap();
    storage.reverse_segmentation_override(id).unwrap();
    assert_eq!(key(&mut storage, &imported, old.anchor()), original);
    let before = state(&storage);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(matches!(
        split(&mut storage, &old, OverrideScope::default(), &["猫", "猫"]),
        Err(EffectiveOverrideError::StaleContext)
    ));
    assert_eq!(state(&storage), before);
    let fresh = context(&mut storage, &imported, "猫猫");
    assert_eq!(
        split(
            &mut storage,
            &fresh,
            OverrideScope::default(),
            &["猫", "猫"]
        )
        .unwrap()
        .1,
        1
    );
}

#[test]
fn effective_override_failed_write_retries_after_reopen_but_lost_success_never_recreates_deleted_vocab(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫");
    let prompt = context(&mut storage, &imported, "猫猫");
    let before = state(&storage);
    storage
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_registration BEFORE INSERT ON vocabulary
        BEGIN SELECT RAISE(ABORT, 'injected registration failure'); END;",
        )
        .unwrap();
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["猫", "猫"]
        ),
        Err(EffectiveOverrideError::Storage(StorageError::Sqlite(_)))
    ));
    assert_eq!(state(&storage), before);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let (id, n) = split(
        &mut storage,
        &prompt,
        OverrideScope::default(),
        &["猫", "猫"],
    )
    .unwrap();
    assert_eq!(n, 1);
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = ?1", ["猫"])
        .unwrap();
    let before = state(&storage);
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["猫", "猫"]
        ),
        Err(EffectiveOverrideError::StaleContext)
    ));
    assert_eq!(state(&storage), before);
    // An explicit reload of the same active rule remains a write-free no-op.
    let refreshed = context(&mut storage, &imported, "猫猫");
    assert_eq!(
        split(
            &mut storage,
            &refreshed,
            OverrideScope::default(),
            &["猫", "猫"]
        )
        .unwrap(),
        (id, 0)
    );
    assert_eq!(state(&storage), before);
}

#[test]
fn effective_override_deferred_commit_failure_rolls_back_rule_and_registration_and_retries() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫");
    let prompt = context(&mut storage, &imported, "猫猫");
    storage.connection.execute_batch("CREATE TABLE fail_parent (id INTEGER PRIMARY KEY);
        CREATE TABLE fail_child (id INTEGER REFERENCES fail_parent(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER fail_commit AFTER INSERT ON segmentation_overrides BEGIN INSERT INTO fail_child VALUES (1); END;").unwrap();
    let before = state(&storage);
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["猫", "猫"]
        ),
        Err(EffectiveOverrideError::Storage(StorageError::Sqlite(_)))
    ));
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_commit")
        .unwrap();
    let (id, _) = split(
        &mut storage,
        &prompt,
        OverrideScope::default(),
        &["猫", "猫"],
    )
    .unwrap();
    let reverse = context(&mut storage, &imported, "猫猫");
    storage.connection.execute_batch("CREATE TRIGGER fail_reverse AFTER UPDATE ON segmentation_overrides BEGIN INSERT INTO fail_child VALUES (1); END;").unwrap();
    let before = state(&storage);
    assert!(matches!(
        storage.reverse_effective_override(&reverse, id),
        Err(EffectiveOverrideError::Storage(StorageError::Sqlite(_)))
    ));
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_reverse")
        .unwrap();
    storage.reverse_effective_override(&reverse, id).unwrap();
}

#[test]
fn effective_override_context_rejects_foreign_non_japanese_partial_off_page_and_stale_spans() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫 English 。 犬犬");
    let chapter = &imported.document.chapter;
    let page = span(chapter, 0, 6);
    let key = key(&mut storage, &imported, page);
    let other = import(&mut storage, "猫猫");
    let before = state(&storage);
    for anchor in [
        span(&other.document.chapter, 0, 6),
        span(chapter, 0, 3),
        span(chapter, 7, 14),
        span(chapter, 15, 18),
        span(chapter, 0, chapter.text.len()),
        span(chapter, chapter.text.len() - 6, chapter.text.len()),
    ] {
        assert!(
            matches!(
                storage.load_effective_override_context(key.clone(), anchor),
                Err(EffectiveOverrideError::InvalidSelection)
            ),
            "{anchor:?}"
        );
    }
    assert_eq!(state(&storage), before);
    let prompt = storage
        .load_effective_override_context(key.clone(), page)
        .unwrap();
    split(
        &mut storage,
        &prompt,
        OverrideScope::default(),
        &["猫", "猫"],
    )
    .unwrap();
    assert!(matches!(
        storage.load_effective_override_context(key.clone(), page),
        Err(EffectiveOverrideError::StaleContext)
    ));
}

#[test]
fn effective_override_page_crossing_keeps_literal_unicode_and_omits_unaligned_alternate() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "𠮷か\u{3099}\0猫");
    let chapter = &imported.document.chapter;
    let page = span(chapter, "𠮷".len(), chapter.text.len() - "猫".len());
    let key = key(&mut storage, &imported, page);
    let prompt = storage
        .load_effective_override_context(key.clone(), span(chapter, 0, chapter.text.len()))
        .unwrap();
    assert_eq!(prompt.surface().as_str(), chapter.text);
    split(
        &mut storage,
        &prompt,
        OverrideScope::ThisOccurrence,
        &["𠮷", "か\u{3099}\0猫"],
    )
    .unwrap();
    assert_eq!(surfaces(&mut storage, &imported), ["𠮷", "か\u{3099}\0猫"]);

    let imported = import(&mut storage, "猫犬");
    let chapter = &imported.document.chapter;
    let mut original = fixture(chapter).unwrap();
    original.tokens = [span(chapter, 0, 3), span(chapter, 3, 6)]
        .into_iter()
        .map(|anchor| AnalyzedToken {
            anchor,
            surface: anchor.slice(chapter).unwrap().into(),
            is_japanese: true,
            reading: None,
            dictionary_form: None,
        })
        .collect();
    // Replace the synthetic snapshot before obtaining a reader key.
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    storage
        .store_chapter_analysis(chapter.chapter_id, &original)
        .unwrap();
    let mut alternate = fixture(chapter).unwrap();
    alternate.mode = SplitMode::C;
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    let prompt = context(&mut storage, &imported, "猫");
    assert_eq!(
        prompt
            .choices()
            .iter()
            .map(|choice| choice.mode)
            .collect::<Vec<_>>(),
        [SplitMode::B]
    );
}

#[test]
fn effective_override_only_reverses_displayed_winners_and_retains_deleted_origin_globals() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let origin = import(&mut storage, "猫猫 犬犬");
    let cat = context(&mut storage, &origin, "猫猫");
    let (cat_id, _) = split(&mut storage, &cat, OverrideScope::default(), &["猫", "猫"]).unwrap();
    let dog = context(&mut storage, &origin, "犬犬");
    let (dog_id, _) = split(&mut storage, &dog, OverrideScope::default(), &["犬", "犬"]).unwrap();
    let visible = context(&mut storage, &origin, "猫猫");
    let before = state(&storage);
    assert!(matches!(
        storage.reverse_effective_override(&visible, dog_id),
        Err(EffectiveOverrideError::InvalidSelection)
    ));
    assert_eq!(state(&storage), before);
    let reader = import(&mut storage, "猫猫");
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(origin.document.chapter.chapter_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(origin.document.document_id)],
        )
        .unwrap();
    let visible = context(&mut storage, &reader, "猫猫");
    assert_eq!(visible.rules()[0].id, cat_id);
    storage
        .reverse_effective_override(&visible, cat_id)
        .unwrap();
    assert_eq!(surfaces(&mut storage, &reader), ["猫猫"]);
}

#[test]
fn effective_override_concurrent_prompts_cannot_replace_winner_after_another_commit() {
    let database = TestDatabase::new();
    let mut first = Storage::open(database.path()).unwrap();
    let imported = import(&mut first, "食べた");
    let first_prompt = context(&mut first, &imported, "食べた");
    let mut second = Storage::open(database.path()).unwrap();
    let second_prompt = context(&mut second, &imported, "食べた");
    let barrier = Arc::new(Barrier::new(2));
    let other_barrier = barrier.clone();
    let one = std::thread::spawn(move || {
        other_barrier.wait();
        split(
            &mut first,
            &first_prompt,
            OverrideScope::default(),
            &["食べ", "た"],
        )
    });
    barrier.wait();
    let two = split(
        &mut second,
        &second_prompt,
        OverrideScope::default(),
        &["食", "べた"],
    );
    let one = one.join().unwrap();
    assert!(matches!(
        (&one, &two),
        (Ok(_), Err(EffectiveOverrideError::StaleContext))
            | (Err(EffectiveOverrideError::StaleContext), Ok(_))
    ));
    assert_eq!(
        second
            .connection
            .query_row("SELECT COUNT(*) FROM segmentation_overrides", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn effective_override_metadata_change_retires_prompt_without_any_rule_edit() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた");
    let prompt = context(&mut storage, &imported, "食べた");
    let (id, _) = split(
        &mut storage,
        &prompt,
        OverrideScope::default(),
        &["食べ", "た"],
    )
    .unwrap();
    let prompt = context(&mut storage, &imported, "食べた");
    let chapter = &imported.document.chapter;
    let mut alternate = fixture(chapter).unwrap();
    alternate.mode = SplitMode::A;
    alternate.tokens = [span(chapter, 0, 6), span(chapter, 6, 9)]
        .into_iter()
        .map(|anchor| AnalyzedToken {
            anchor,
            surface: anchor.slice(chapter).unwrap().into(),
            is_japanese: true,
            reading: Some("exact-reading".into()),
            dictionary_form: None,
        })
        .collect();
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    let before = state(&storage);
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["食", "べた"]
        ),
        Err(EffectiveOverrideError::StaleContext)
    ));
    assert!(matches!(
        storage.reverse_effective_override(&prompt, id),
        Err(EffectiveOverrideError::StaleContext)
    ));
    assert_eq!(state(&storage), before);
}

#[test]
fn effective_override_status_and_time_changes_do_not_retire_prompt_or_rewrite_completed_reading() {
    use yomoku_core::{
        active_time::PendingReadingTime, completion::PageCompletion, pagination::ReadingPosition,
    };
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた");
    let chapter = &imported.document.chapter;
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::START,
    };
    let time = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(time, PendingReadingTime::from_millis(1234).unwrap())
        .unwrap();
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: imported.document.document_id,
        page: span(chapter, 0, chapter.text.len()),
        page_length: storage.page_length().unwrap(),
        departing_position: position,
        completed_at_unix_ms: 110,
        pending_time: PendingReadingTime::from_millis(1234).unwrap(),
    };
    storage
        .record_analyzed_page_completion(completion, imported.analysis_id)
        .unwrap();
    storage
        .save_reading_position(imported.document.document_id, position)
        .unwrap();
    let prompt = context(&mut storage, &imported, "食べた");
    let word = storage
        .vocabulary_entry(&Surface::new("食べた").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(word.id, VocabularyStatus::Ignored, 115)
        .unwrap();
    let time = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(time, PendingReadingTime::from_millis(567).unwrap())
        .unwrap();
    let before = state(&storage);
    let reader = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap();
    let (id, _) = split(
        &mut storage,
        &prompt,
        OverrideScope::default(),
        &["食べ", "た"],
    )
    .unwrap();
    assert_eq!(&state(&storage)[4..], &before[4..]);
    assert_eq!(
        storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap(),
        reader
    );
    let prompt = context(&mut storage, &imported, "食べた");
    let before_reverse = state(&storage);
    storage.reverse_effective_override(&prompt, id).unwrap();
    assert_eq!(&state(&storage)[1..], &before_reverse[1..]);
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new("食べた").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Ignored
    );
}

#[test]
fn effective_override_rejects_invalid_parts_timestamp_missing_analysis_and_deleted_document_without_writes(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫");
    let prompt = context(&mut storage, &imported, "猫猫");
    let before = state(&storage);
    for values in [vec![], vec!["犬"], vec!["猫", "犬"]] {
        assert!(split(&mut storage, &prompt, OverrideScope::default(), &values).is_err());
        assert_eq!(state(&storage), before);
    }
    assert!(storage
        .save_effective_override(&prompt, OverrideScope::default(), parts(&["猫", "猫"]), -1)
        .is_err());
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    let before = state(&storage);
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["猫", "猫"]
        ),
        Err(EffectiveOverrideError::StaleContext)
    ));
    assert_eq!(state(&storage), before);
    let imported = import(&mut storage, "犬犬");
    let prompt = context(&mut storage, &imported, "犬犬");
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    let before = state(&storage);
    assert!(matches!(
        split(
            &mut storage,
            &prompt,
            OverrideScope::default(),
            &["犬", "犬"]
        ),
        Err(EffectiveOverrideError::InvalidSelection)
    ));
    assert_eq!(state(&storage), before);
}
