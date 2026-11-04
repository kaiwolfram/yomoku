use super::*;
use yomoku_core::{
    effective_session::{
        EffectiveCompletionProgress, EffectiveReaderProgress as Progress,
        EffectiveReaderRequest as Request,
    },
    effective_storage::EffectiveStorageResult as Output,
    storage::{DocumentError, StorageResult},
};

#[test]
fn token_callbacks_use_captured_attempts_and_require_explicit_retry_after_bad_results() {
    let snapshot = snapshot();
    let mut session = EffectiveReaderSession::default();
    let old = session.begin_load(base(&snapshot)).unwrap();
    let fresh = session.begin_load(base(&snapshot)).unwrap();
    // Even a response carrying the NEW attempt cannot authorize an old callback.
    assert_eq!(
        session.finish_request(
            &Request::Tokens(old),
            StorageResult::Effective(Output::TokensLoaded(fresh, snapshot.clone())),
        ),
        None
    );
    assert_eq!(
        session.finish_request(
            &Request::Tokens(fresh),
            StorageResult::Effective(Output::TokensLoaded(old, snapshot.clone())),
        ),
        Some(Progress::TokensFailed(DocumentError::InvalidDocument))
    );
    assert!(session.page().is_none());
    assert_eq!(
        session.finish_request(
            &Request::Tokens(fresh),
            StorageResult::Effective(Output::TokensLoaded(fresh, snapshot.clone())),
        ),
        None
    );
    let retry = session.begin_load(base(&snapshot)).unwrap();
    assert_eq!(
        session.finish_request(
            &Request::Tokens(retry),
            StorageResult::Effective(Output::TokensLoaded(retry, snapshot)),
        ),
        Some(Progress::TokensLoaded)
    );
    assert_eq!(
        session.finish_request(&Request::Tokens(fresh), StorageResult::ImportCancelled),
        None
    );
    assert!(session.page().is_some());
}

#[test]
fn word_errors_do_not_consume_a_newer_callback_or_erase_exact_custom_retries() {
    let (mut session, selection, details) = words::loaded(true);
    let load = Request::Word(session.begin_word_load(selection).unwrap());
    assert_eq!(
        session.finish_request(&load, StorageResult::ImportCancelled),
        Some(Progress::WordFinished)
    );
    assert_eq!(
        session.word_state().error,
        Some(DocumentError::InvalidDocument)
    );
    let retry = Request::Word(session.begin_word_load(selection).unwrap());
    assert_eq!(
        session.finish_request(&load, StorageResult::Failed(DocumentError::Unavailable)),
        None
    );
    assert!(session.word_state().busy);
    assert_eq!(
        session.finish_request(
            &retry,
            StorageResult::Effective(Output::WordLoaded(selection, Box::new(details))),
        ),
        Some(Progress::WordFinished)
    );
    let save = Request::Word(
        session
            .begin_custom_entry_save(selection, " literal 📖\0 ".into(), " よみ\0 ".into())
            .unwrap(),
    );
    assert_eq!(
        session.finish_request(&save, StorageResult::ImportCancelled),
        Some(Progress::WordFinished)
    );
    assert!(session.word_state().custom_entry_pending);
    let retry = Request::Word(session.retry_custom_entry().unwrap());
    assert_ne!(retry, save);
    assert_eq!(retry.operation(), save.operation());
    assert_eq!(
        session.finish_request(&save, StorageResult::Failed(DocumentError::Unavailable)),
        None
    );
    assert!(session.word_state().busy);
    assert_eq!(
        session.finish_request(&retry, StorageResult::Failed(DocumentError::Unavailable)),
        Some(Progress::WordFinished)
    );
    assert_eq!(
        session.word_state().custom_entry_error,
        Some(DocumentError::Unavailable)
    );
    assert!(session.cancel_custom_entry_retry());
}

#[test]
fn status_errors_preserve_frozen_choice_until_retry_or_cancel() {
    let mut snapshot = snapshot_with_manual_split(true);
    let id = 55.try_into().unwrap();
    snapshot.tokens.last_mut().unwrap().vocabulary = Some(ReaderVocabulary {
        id,
        status: VocabularyStatus::New,
    });
    let mut session = EffectiveReaderSession::default();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    let selection = EffectiveReaderSelection {
        page: session.page().unwrap().identity,
        segment_index: 2,
    };
    let request = Request::Status(
        session
            .begin_status_change(selection, id, VocabularyStatus::Known, 500)
            .unwrap(),
    );
    assert_eq!(
        session.finish_request(&request, StorageResult::ImportCancelled),
        Some(Progress::StatusFinished)
    );
    assert_eq!(
        session.status_state().error,
        Some(DocumentError::InvalidDocument)
    );
    let retry = Request::Status(session.retry_status_change().unwrap());
    assert_ne!(request, retry);
    assert_eq!(request.operation(), retry.operation());
    assert_eq!(
        session.finish_request(&request, StorageResult::Failed(DocumentError::Unavailable)),
        None
    );
    assert!(session.status_state().busy);
    assert_eq!(
        session.finish_request(
            &retry,
            StorageResult::Effective(Output::StatusChanged(selection, None))
        ),
        Some(Progress::StatusFinished)
    );
    assert!(!session.status_state().pending);
}

#[test]
fn completion_failures_do_not_dispatch_followups_or_lose_the_frozen_intent() {
    let (mut session, selection, _) = words::loaded(true);
    assert!(session.begin_completion(selection.page, statuses::intent(selection)));
    let request = Request::Completion(session.dispatch_completion().unwrap());
    assert_eq!(
        session.finish_request(&request, StorageResult::ImportCancelled),
        Some(Progress::Completion(EffectiveCompletionProgress::Failed(
            DocumentError::InvalidDocument
        )))
    );
    assert!(session.completion_state().pending);
    assert!(!session.completion_state().busy);
    let retry = Request::Completion(session.dispatch_completion().unwrap());
    assert_ne!(request, retry);
    assert_eq!(request.operation(), retry.operation());
    assert_eq!(
        session.finish_request(&request, StorageResult::ImportCancelled),
        None
    );
    assert!(session.completion_state().busy);
    assert_eq!(
        session.finish_request(&retry, StorageResult::Failed(DocumentError::Unavailable)),
        Some(Progress::Completion(EffectiveCompletionProgress::Failed(
            DocumentError::Unavailable
        )))
    );
    assert!(session.completion_state().pending);
    assert!(!session.completion_state().busy);
}
