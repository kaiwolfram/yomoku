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
    let mut snapshot = snapshot_with_manual_split(true);
    let mut token = snapshot.tokens.last().unwrap().clone();
    token.vocabulary = Some(ReaderVocabulary {
        id: 55.try_into().unwrap(),
        status: VocabularyStatus::New,
    });
    snapshot.tokens.last_mut().unwrap().vocabulary = token.vocabulary;
    let details = EffectiveReaderWordDetails {
        key: snapshot.key().unwrap(),
        token,
        encountered: 7,
        candidates: DictionaryCandidates {
            surface: "猫".into(),
            dictionary_form: None,
            words: vec![],
            names: vec![],
        },
        custom_entry: None,
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

fn intent(selection: EffectiveReaderSelection) -> PageCompletionIntent {
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
fn status_changes_update_live_word_and_page_without_guessing_manual_metadata() {
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
    let mut fresh = snapshot_with_manual_split(true);
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

#[test]
fn status_rejects_mismatched_results_and_requires_explicit_retry_cancel_or_choice() {
    let (mut session, selection, _) = loaded();
    let mut foreign = selection;
    foreign.segment_index += 1;
    for result in [
        Output::StatusChanged(foreign, Some(entry())),
        Output::StatusChanged(
            selection,
            Some(ReaderVocabulary {
                id: 99.try_into().unwrap(),
                ..entry()
            }),
        ),
        Output::StatusChanged(
            selection,
            Some(ReaderVocabulary {
                status: VocabularyStatus::Ignored,
                ..entry()
            }),
        ),
        Output::CustomEntryChanged(selection, None),
    ] {
        let request = begin(&mut session, selection);
        assert!(session.finish_status_request(&request, Ok(result)));
        assert_eq!(
            session.status_state().error,
            Some(DocumentError::InvalidDocument)
        );
        assert!(session.status_state().pending);
        assert!(!session.status_state().busy);
    }
    assert!(session.cancel_status_retry());
    assert!(!session.status_state().pending);
    assert!(session.retry_status_change().is_none());
    assert!(!session.cancel_status_retry());
}

#[test]
fn status_word_custom_and_completion_obligations_gate_each_other() {
    let (mut session, selection, details) = loaded();
    let load = session.begin_word_load(selection).unwrap();
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 500)
        .is_none());
    assert!(
        session.finish_word_request(&load, Ok(Output::WordLoaded(selection, Box::new(details))))
    );
    let save = session
        .begin_custom_entry_save(selection, " meaning ".into(), " よみ ".into())
        .unwrap();
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 500)
        .is_none());
    assert!(session.finish_word_request(&save, Err(DocumentError::Unavailable)));
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 500)
        .is_none());
    assert!(session.cancel_custom_entry_retry());
    let status = begin(&mut session, selection);
    for failed in [false, true] {
        if failed {
            assert!(session.finish_status_request(&status, Err(DocumentError::Unavailable)));
        }
        assert!(!session.begin_completion(selection.page, intent(selection)));
        assert!(session.begin_word_load(selection).is_none());
        assert!(session
            .begin_custom_entry_save(selection, "meaning".into(), "よみ".into())
            .is_none());
    }
    assert!(session.cancel_status_retry());
    assert!(session.begin_completion(selection.page, intent(selection)));
    assert!(session
        .begin_status_change(selection, entry().id, VocabularyStatus::Known, 500)
        .is_none());
}
