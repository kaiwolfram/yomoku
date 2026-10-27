use super::*;
use yomoku_core::{
    dictionary::DictionaryCandidates,
    effective_reader::EffectiveReaderWordDetails,
    effective_session::EffectiveWordRequest,
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    reader::ReaderCustomEntry,
    storage::DocumentError,
};

fn loaded(
    manual: bool,
) -> (
    EffectiveReaderSession,
    EffectiveReaderSelection,
    EffectiveReaderWordDetails,
) {
    let snapshot = snapshot_with_manual_split(manual);
    let token = snapshot.tokens.last().unwrap().clone();
    let mut pages = EffectiveReaderSession::default();
    let request = pages.begin_load(base(&snapshot)).unwrap();
    let details = EffectiveReaderWordDetails {
        key: snapshot.key().unwrap(),
        candidates: DictionaryCandidates {
            surface: token.surface.clone(),
            dictionary_form: token.metadata.as_ref().and_then(|metadata| {
                metadata
                    .dictionary_form
                    .as_ref()
                    .map(Surface::as_str)
                    .map(str::to_owned)
            }),
            words: vec![],
            names: vec![],
        },
        token,
        encountered: 7,
        custom_entry: None,
    };
    assert!(pages.accept(request, snapshot));
    let selection = EffectiveReaderSelection {
        page: pages.page().unwrap().identity,
        segment_index: details.token.segment_index,
    };
    (pages, selection, details)
}

fn finish_load(
    pages: &mut EffectiveReaderSession,
    selection: EffectiveReaderSelection,
    details: EffectiveReaderWordDetails,
) -> EffectiveWordRequest {
    let request = pages.begin_word_load(selection).unwrap();
    assert!(matches!(request.operation(), Op::LoadWord(_)));
    assert!(pages.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    assert!(!pages.word_state().busy);
    request
}

fn entry(surface: &str) -> ReaderCustomEntry {
    ReaderCustomEntry {
        surface: surface.into(),
        english_meaning: " literal\0meaning 📖 ".into(),
        reading: " よみか\u{3099}\0 ".into(),
    }
}

#[test]
fn word_projection_is_compact_and_preserves_resolved_unresolved_and_live_fields() {
    for manual in [false, true] {
        for status in [
            None,
            Some(VocabularyStatus::New),
            Some(VocabularyStatus::Learning),
            Some(VocabularyStatus::Familiar),
            Some(VocabularyStatus::Known),
            Some(VocabularyStatus::Ignored),
        ] {
            let (mut pages, selection, mut details) = loaded(manual);
            details.token.vocabulary = status.map(|status| ReaderVocabulary {
                id: 55.try_into().unwrap(),
                status,
            });
            details.custom_entry = Some(entry(&details.token.surface));
            let expected = details.clone();
            finish_load(&mut pages, selection, details);
            let word = pages.word_state().details.unwrap();
            assert_eq!(word.selection, selection);
            assert_eq!(word.token, expected.token.into());
            assert_eq!(word.encountered, 7);
            assert_eq!(word.candidates, expected.candidates);
            assert_eq!(word.custom_entry, expected.custom_entry);
            assert_eq!(word.token.reading.is_none(), manual);
            assert_eq!(word.token.dictionary_form.is_none(), manual);
            assert_eq!(
                pages.page().unwrap().tokens.last().unwrap().vocabulary,
                word.token.vocabulary
            );
        }
    }
}

#[test]
fn dismissed_refreshed_and_duplicate_word_callbacks_cannot_resurrect_selection() {
    let (mut pages, selection, details) = loaded(true);
    let request = pages.begin_word_load(selection).unwrap();
    assert!(pages.begin_word_load(selection).is_none());
    pages.dismiss_word();
    assert!(pages.word_state().busy);
    assert!(pages.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details.clone())))
    ));
    assert!(pages.word_state().selection.is_none());
    assert!(pages.word_state().details.is_none());
    assert!(!pages.word_state().busy);
    let second = pages.begin_word_load(selection).unwrap();
    assert_ne!(second, request);
    assert!(!pages.finish_word_request(&request, Err(DocumentError::Unavailable)));
    assert!(pages.word_state().busy);
    pages.invalidate();
    assert!(pages.finish_word_request(
        &second,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    assert!(pages.word_state().selection.is_none());
    assert!(pages.word_state().details.is_none());
    assert!(!pages.word_state().busy);
    let snapshot = snapshot_with_manual_split(true);
    let load = pages.begin_load(base(&snapshot)).unwrap();
    assert!(pages.accept(load, snapshot));
    assert!(pages.begin_word_load(selection).is_none());
    let fresh = EffectiveReaderSelection {
        page: pages.page().unwrap().identity,
        ..selection
    };
    let third = pages.begin_word_load(fresh).unwrap();
    assert!(!pages.finish_word_request(&second, Err(DocumentError::Unavailable)));
    assert!(pages.finish_word_request(&third, Err(DocumentError::Unavailable)));
    assert_eq!(pages.word_state().error, Some(DocumentError::Unavailable));
    assert!(!pages.word_state().busy);
    assert!(pages.retry_custom_entry().is_none());
    assert!(pages.begin_word_load(fresh).is_some()); // explicit lookup retry only
}

#[test]
fn foreign_word_results_fail_closed_without_consuming_another_request() {
    for case in 0..13 {
        let (mut pages, selection, mut details) = loaded(true);
        let request = pages.begin_word_load(selection).unwrap();
        let mut returned = selection;
        match case {
            0 => returned.page.revision += 1,
            1 => details.key.document_id = 99.try_into().unwrap(),
            2 => details.key.segmentation.segments[0]
                .boundary
                .surface
                .push('別'),
            3 => details.token.segment_index += 1,
            4 => details.token.surface.push('別'),
            5 => details.token.display_range = range(0, 1),
            6 => details.token.metadata = snapshot().tokens.last().unwrap().metadata.clone(),
            7 => details.candidates.surface.push('別'),
            8 => details.token.anchor.range = range(0, 1),
            9 => details.candidates.dictionary_form = Some("guessed".into()),
            10 => details.custom_entry = Some(entry("別")),
            11 => {
                let mut custom = entry(&details.token.surface);
                custom.english_meaning = "\t　".into();
                details.custom_entry = Some(custom);
            }
            _ => {
                let mut custom = entry(&details.token.surface);
                custom.reading = "\n".into();
                details.custom_entry = Some(custom);
            }
        }
        assert!(pages.finish_word_request(
            &request,
            Ok(Output::WordLoaded(returned, Box::new(details)))
        ));
        assert_eq!(
            pages.word_state().error,
            Some(DocumentError::InvalidDocument),
            "case {case}"
        );
        assert!(pages.word_state().details.is_none());
        let next = pages.begin_word_load(selection).unwrap();
        assert!(!pages.finish_word_request(&request, Err(DocumentError::Unavailable)));
        assert!(pages.finish_word_request(&next, Err(DocumentError::Unavailable)));
    }
}

#[test]
fn custom_commands_freeze_literal_values_gate_mutations_and_retry_with_fresh_attempts() {
    let (mut pages, selection, details) = loaded(true);
    finish_load(&mut pages, selection, details.clone());
    let saved = entry(&details.token.surface);
    let save = pages
        .begin_custom_entry_save(
            selection,
            saved.english_meaning.clone(),
            saved.reading.clone(),
        )
        .unwrap();
    assert!(pages.word_state().custom_entry_pending);
    assert!(pages.word_state().details.unwrap().custom_entry.is_none());
    assert!(pages.begin_word_load(selection).is_none());
    assert!(pages
        .begin_custom_entry_save(selection, "other".into(), "other".into())
        .is_none());
    assert!(pages.begin_custom_entry_delete(selection).is_none());
    assert!(pages.retry_custom_entry().is_none());
    assert!(pages.finish_word_request(&save, Err(DocumentError::Unavailable)));
    let failed = pages.word_state();
    assert!(failed.custom_entry_pending && !failed.busy);
    assert_eq!(failed.custom_entry_error, Some(DocumentError::Unavailable));
    assert!(failed.details.unwrap().custom_entry.is_none());
    let retry = pages.retry_custom_entry().unwrap();
    assert_ne!(retry, save);
    assert_eq!(retry.operation(), save.operation());
    assert!(!pages.finish_word_request(&save, Err(DocumentError::Unavailable)));
    assert!(pages.word_state().busy);
    // A malformed success must retain the original retry obligation.
    assert!(pages.finish_word_request(
        &retry,
        Ok(Output::CustomEntryChanged(selection, Some(entry("別"))))
    ));
    assert_eq!(
        pages.word_state().custom_entry_error,
        Some(DocumentError::InvalidDocument)
    );
    let retry = pages.retry_custom_entry().unwrap();
    assert_eq!(retry.operation(), save.operation());
    assert!(pages.finish_word_request(
        &retry,
        Ok(Output::CustomEntryChanged(selection, Some(saved.clone())))
    ));
    assert!(!pages.word_state().custom_entry_pending);
    assert_eq!(
        pages.word_state().details.unwrap().custom_entry,
        Some(saved.clone())
    );
    // A failed deletion preserves the committed entry and its frozen command.
    let delete = pages.begin_custom_entry_delete(selection).unwrap();
    assert!(pages.finish_word_request(&delete, Err(DocumentError::Unavailable)));
    assert_eq!(
        pages.word_state().details.unwrap().custom_entry,
        Some(saved)
    );
    pages.invalidate();
    assert!(pages.word_state().custom_entry_pending);
    let retry = pages.retry_custom_entry().unwrap();
    assert_eq!(retry.operation(), delete.operation());
    assert!(pages.finish_word_request(&retry, Ok(Output::CustomEntryChanged(selection, None))));
    assert!(!pages.word_state().custom_entry_pending);
    assert!(pages.word_state().selection.is_none());
    assert!(pages.word_state().details.is_none());
}

#[test]
fn custom_controls_require_a_loaded_missing_match_and_valid_literal_fields() {
    let (mut pages, selection, details) = loaded(true);
    assert!(pages
        .begin_custom_entry_save(selection, "meaning".into(), "reading".into())
        .is_none());
    finish_load(&mut pages, selection, details);
    let before = pages.word_state();
    for (meaning, reading) in [("", "reading"), ("meaning", "\n \t"), ("　", "reading")] {
        assert!(pages
            .begin_custom_entry_save(selection, meaning.into(), reading.into())
            .is_none());
        assert_eq!(pages.word_state(), before);
    }
    assert!(pages.begin_custom_entry_delete(selection).is_none());
    let mut foreign = selection;
    foreign.segment_index = 1; // punctuation
    assert!(pages.begin_word_load(foreign).is_none());
    assert!(pages
        .begin_custom_entry_save(foreign, "meaning".into(), "reading".into())
        .is_none());
    foreign = selection;
    foreign.page.revision += 1;
    assert!(pages
        .begin_custom_entry_save(foreign, "meaning".into(), "reading".into())
        .is_none());
    assert_eq!(pages.word_state(), before);
    pages.dismiss_word();
    assert!(pages
        .begin_custom_entry_save(selection, "meaning".into(), "reading".into())
        .is_none());
}

#[test]
fn cancel_abandons_only_a_failed_custom_retry_and_preserves_the_committed_entry() {
    let (mut pages, selection, mut details) = loaded(true);
    let committed = entry(&details.token.surface);
    details.custom_entry = Some(committed.clone());
    finish_load(&mut pages, selection, details);
    assert!(!pages.cancel_custom_entry_retry());
    let delete = pages.begin_custom_entry_delete(selection).unwrap();
    assert!(!pages.cancel_custom_entry_retry());
    assert!(pages.finish_word_request(&delete, Err(DocumentError::Unavailable)));
    assert!(pages.cancel_custom_entry_retry());
    assert!(!pages.word_state().custom_entry_pending);
    assert!(pages.word_state().custom_entry_error.is_none());
    assert_eq!(
        pages.word_state().details.unwrap().custom_entry,
        Some(committed)
    );
    assert!(pages.retry_custom_entry().is_none());
    assert!(!pages.finish_word_request(&delete, Ok(Output::CustomEntryChanged(selection, None))));
    let replace = pages
        .begin_custom_entry_save(selection, "new explicit meaning".into(), "reading".into())
        .unwrap();
    assert!(pages.finish_word_request(&replace, Err(DocumentError::Unavailable)));
    let newer = pages
        .begin_custom_entry_save(
            selection,
            "another explicit meaning".into(),
            "reading".into(),
        )
        .unwrap();
    assert_ne!(newer.operation(), replace.operation());
    assert!(!pages.finish_word_request(&replace, Ok(Output::CustomEntryChanged(selection, None))));
    assert!(pages.finish_word_request(&newer, Err(DocumentError::Unavailable)));
    // A new explicit lookup also supersedes a failed retry, while stale callbacks
    // cannot consume that lookup or retarget its selection.
    let lookup = pages.begin_word_load(selection).unwrap();
    assert!(!pages.finish_word_request(&newer, Err(DocumentError::Unavailable)));
    assert!(pages.word_state().busy);
    assert!(!pages.word_state().custom_entry_pending);
    assert!(pages.finish_word_request(&lookup, Err(DocumentError::Unavailable)));
}

#[test]
fn name_matches_also_suppress_custom_controls() {
    use yomoku_core::dictionary::{NameCandidate, NameEntry};
    let (mut pages, selection, mut details) = loaded(true);
    details.candidates.names.push(NameCandidate {
        entry_id: 1,
        entry: NameEntry {
            kanji: vec![],
            readings: vec![],
            names: vec![],
        },
        forms: vec![],
    });
    finish_load(&mut pages, selection, details);
    assert!(pages
        .begin_custom_entry_save(selection, "meaning".into(), "reading".into())
        .is_none());
    assert!(pages.begin_custom_entry_delete(selection).is_none());
}
