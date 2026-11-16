use super::*;
use yomoku_core::{
    completion::PageCompletionIntent,
    dictionary::DictionaryCandidates,
    effective_reader::EffectiveReaderWordDetails,
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::{PageLength, ReadingPosition},
    storage::DocumentError,
};

fn loaded() -> (
    EffectiveReaderSession,
    EffectiveReaderSelection,
    EffectiveReaderWordDetails,
) {
    let mut snapshot = snapshot_with_separate_tokens(true);
    let mut token = snapshot.tokens.last().unwrap().clone();
    token.vocabulary = Some(ReaderVocabulary {
        id: 55.try_into().unwrap(),
        status: VocabularyStatus::New,
    });
    snapshot.tokens.last_mut().unwrap().vocabulary = token.vocabulary;
    let details = EffectiveReaderWordDetails {
        parts: Vec::new(),
        key: snapshot.key().unwrap(),
        token,
        encountered: 7,
        last_seen_unix_ms: None,
        candidates: DictionaryCandidates {
            surface: "猫".into(),
            dictionary_form: None,
            words: vec![],
            names: vec![],
        },
    };
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(request, snapshot));
    let selection = EffectiveReaderSelection {
        page: session.page().unwrap().identity,
        segment_index: details.token.segment_index,
    };
    (session, selection, details)
}

fn begin(
    session: &mut EffectiveReaderSession,
    selection: EffectiveReaderSelection,
) -> yomoku_core::effective_session::EffectiveStatusRequest {
    session
        .begin_status_change(
            selection,
            55.try_into().unwrap(),
            VocabularyStatus::Known,
            500,
        )
        .unwrap()
}

fn entry() -> ReaderVocabulary {
    ReaderVocabulary {
        id: 55.try_into().unwrap(),
        status: VocabularyStatus::Known,
    }
}

pub(super) fn intent(selection: EffectiveReaderSelection) -> PageCompletionIntent {
    PageCompletionIntent {
        document_id: selection.page.base.document_id,
        page: selection.page.base.page,
        page_length: PageLength::default(),
        departing_position: ReadingPosition {
            source_id: selection.page.base.page.source_id,
            chapter_id: selection.page.base.page.chapter_id,
            offset: selection.page.base.page.range.start,
        },
        completed_at_unix_ms: 500,
        analysis_id: match selection.page.base.analysis {
            ReaderAnalysis::Available(id) => id,
            _ => unreachable!(),
        },
    }
}

#[test]
fn status_changes_update_live_word_and_page_without_guessing_missing_readings_or_base_forms() {
    let (mut session, selection, details) = loaded();
    let request = session.begin_word_load(selection).unwrap();
    assert!(session.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    for status in [
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ] {
        let before = session.word_state().details.unwrap();
        let request = session
            .begin_status_change(selection, entry().id, status, 500)
            .unwrap();
        assert!(
            matches!(request.operation(), Op::SetStatus { status: choice, occurred_at_unix_ms: 500, .. } if choice == status)
        );
        assert!(session.status_state().busy);
        assert_eq!(session.word_state().details.as_ref().unwrap(), &before);
        let updated = ReaderVocabulary { status, ..entry() };
        assert!(session.finish_status_request(
            &request,
            Ok(Output::StatusChanged(selection, Some(updated)))
        ));
        let details = session.word_state().details.unwrap();
        assert_eq!(details.token.vocabulary, Some(updated));
        assert_eq!(details.encountered, before.encountered);
        assert_eq!(details.candidates, before.candidates);
        assert!(details.token.reading.is_none() && details.token.dictionary_form.is_none());
        assert_eq!(
            session.page().unwrap().tokens.last().unwrap().vocabulary,
            Some(updated)
        );
        assert!(!session.status_state().pending);
        assert!(!session.finish_status_request(&request, Err(DocumentError::Unavailable)));
    }
}

#[test]
fn status_observing_deleted_vocabulary_clears_saved_history_but_keeps_visible_occurrences() {
    let (mut session, selection, details) = loaded();
    let candidates = details.candidates.clone();
    let request = session.begin_word_load(selection).unwrap();
    assert!(session.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    assert_eq!(session.word_state().details.unwrap().encountered, 8);
    let request = begin(&mut session, selection);
    assert!(session.finish_status_request(&request, Ok(Output::StatusChanged(selection, None))));
    let details = session.word_state().details.unwrap();
    assert!(details.token.vocabulary.is_none());
    assert_eq!(details.encountered, 1);
    assert_eq!(details.candidates, candidates);
    assert!(session
        .page()
        .unwrap()
        .tokens
        .last()
        .unwrap()
        .vocabulary
        .is_none());
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 501)
        .is_none());
}

#[test]
fn status_gates_stale_missing_foreign_and_invalid_choices_without_changing_page() {
    let (mut session, selection, _) = loaded();
    for index in [0, 1, 3, u64::MAX] {
        assert!(session
            .begin_status_change(
                EffectiveReaderSelection {
                    segment_index: index,
                    ..selection
                },
                entry().id,
                VocabularyStatus::Known,
                500
            )
            .is_none());
    }
    let mut foreign = selection;
    foreign.page.revision += 1;
    assert!(session
        .begin_status_change(foreign, entry().id, VocabularyStatus::Known, 500)
        .is_none());
    assert!(session
        .begin_status_change(
            selection,
            99.try_into().unwrap(),
            VocabularyStatus::Known,
            500
        )
        .is_none());
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, -1)
        .is_none());
    assert!(!session.status_state().pending);
    let request = begin(&mut session, selection);
    assert!(session.finish_status_request(&request, Ok(Output::StatusChanged(selection, None))));
    assert!(session
        .page()
        .unwrap()
        .tokens
        .last()
        .unwrap()
        .vocabulary
        .is_none());
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 500)
        .is_none());
}

#[test]
fn failed_status_keeps_committed_fields_and_exact_retry_uses_fresh_callback_attempt() {
    let (mut session, selection, _) = loaded();
    let original = session.page().unwrap().clone();
    let first = begin(&mut session, selection);
    assert!(!session.cancel_status_retry());
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Learning, 501)
        .is_none());
    assert!(session.finish_status_request(&first, Err(DocumentError::Unavailable)));
    assert_eq!(session.page(), Some(&original));
    assert_eq!(
        session.status_state().error,
        Some(DocumentError::Unavailable)
    );
    let retry = session.retry_status_change().unwrap();
    assert_ne!(first, retry);
    assert_eq!(first.operation(), retry.operation());
    assert!(
        !session.finish_status_request(&first, Ok(Output::StatusChanged(selection, Some(entry()))))
    );
    assert!(session.status_state().busy);
    assert!(
        session.finish_status_request(&retry, Ok(Output::StatusChanged(selection, Some(entry()))))
    );
    assert!(!session.status_state().pending);
    assert!(session.status_state().error.is_none());
}

#[test]
fn invalidated_status_callbacks_drain_without_touching_fresh_page_or_restoring_word() {
    let (mut session, selection, details) = loaded();
    let load = session.begin_word_load(selection).unwrap();
    assert!(
        session.finish_word_request(&load, Ok(Output::WordLoaded(selection, Box::new(details))))
    );
    let first = begin(&mut session, selection);
    session.invalidate();
    assert!(session.status_state().busy);
    assert!(session.finish_status_request(&first, Err(DocumentError::Unavailable)));
    let retry = session.retry_status_change().unwrap();
    let mut fresh = snapshot_with_separate_tokens(true);
    fresh.tokens.last_mut().unwrap().vocabulary = Some(ReaderVocabulary {
        status: VocabularyStatus::Learning,
        ..entry()
    });
    let load = session.begin_load(base(&fresh)).unwrap();
    assert!(session.accept(load, fresh));
    let page = session.page().unwrap().clone();
    assert!(
        session.finish_status_request(&retry, Ok(Output::StatusChanged(selection, Some(entry()))))
    );
    assert_eq!(session.page(), Some(&page));
    assert!(session.word_state().selection.is_none());
    assert!(session.word_state().details.is_none());
}
