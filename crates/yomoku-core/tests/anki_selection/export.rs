use super::{Fixture, ReaderAnalysis, VocabularyStatus};
use yomoku_core::anki::*;

fn begin(
    session: &mut AnkiExportSession,
    f: &Fixture,
    deck: i64,
) -> Result<AnkiExportRequest, AnkiBeginError> {
    session.begin_original(
        deck.try_into().unwrap(),
        &f.chapter,
        &f.page,
        f.details.selection(),
        &f.details,
    )
}

fn existing() -> AnkiExistingNotes {
    AnkiExistingNotes::new(vec![9.try_into().unwrap(), 12.try_into().unwrap()]).unwrap()
}

fn fail(session: &mut AnkiExportSession, request: &AnkiExportRequest, error: AnkiExportError) {
    assert!(session.finish(
        request,
        AnkiExportResult::Add(AnkiAddOutcome::NotInserted(error)),
    ));
    assert_eq!(session.progress(), Some(&AnkiExportProgress::Failed(error)));
}

#[test]
fn original_and_effective_commands_freeze_identical_content_and_distinct_selections() {
    let f = Fixture::new("猫と猫。", "猫");
    let mut original = AnkiExportSession::default();
    let mut effective = AnkiExportSession::default();
    let first = begin(&mut original, &f, i64::MAX).unwrap();
    let (page, details) = f.effective();
    let second = effective
        .begin_effective(
            i64::MAX.try_into().unwrap(),
            &f.chapter,
            &page,
            details.selection,
            &details,
        )
        .unwrap();
    assert_eq!(first.export().note(), second.export().note());
    assert_eq!(first.export().note(), &f.original().unwrap());
    assert_eq!(i64::from(first.export().deck()), i64::MAX);
    assert_eq!(first.operation(), AnkiExportOperation::CheckAndAdd);
    assert_eq!(
        first.export().selection(),
        AnkiSelection::Original(f.details.selection())
    );
    assert_eq!(
        second.export().selection(),
        AnkiSelection::Effective(details.selection)
    );
    assert_eq!(
        first.export().note().fields()[3],
        "猫と<strong class=\"yomoku-target\">猫</strong>。"
    );
    assert_eq!(original.export(), Some(first.export()));
    assert_eq!(
        original.progress(),
        Some(&AnkiExportProgress::Pending(
            AnkiExportOperation::CheckAndAdd
        ))
    );
}

#[test]
fn safe_retry_keeps_frozen_fields_deck_and_selection_after_reader_changes() {
    let mut f = Fixture::new("猫と猫。", "猫");
    let mut session = AnkiExportSession::default();
    let first = begin(&mut session, &f, 5).unwrap();
    let frozen = first.export().clone();
    fail(&mut session, &first, AnkiExportError::PermissionDenied);
    // New dictionary data, a page revisit and a different configured deck must
    // never be substituted into this accepted command by Retry.
    f.details.token.reading = Some("changed".into());
    f.page.analysis = ReaderAnalysis::Available(99.try_into().unwrap());
    let retry = session.retry().unwrap();
    assert_ne!(retry, first);
    assert_eq!(retry.export(), &frozen);
    assert_eq!(retry.operation(), AnkiExportOperation::CheckAndAdd);
    assert_eq!(begin(&mut session, &f, 6), Err(AnkiBeginError::Busy));
    assert!(!session.clear());
    assert!(session.retry().is_none());
    assert!(!session.finish(
        &first,
        AnkiExportResult::Add(AnkiAddOutcome::Added(8.try_into().unwrap())),
    ));
    assert_eq!(session.export(), Some(&frozen));
    assert!(session.finish(
        &retry.clone(),
        AnkiExportResult::Add(AnkiAddOutcome::Added(9.try_into().unwrap())),
    ));
    assert_eq!(
        session.progress(),
        Some(&AnkiExportProgress::Added(9.try_into().unwrap()))
    );
    assert!(session.retry().is_none());
    assert!(!session.finish(
        &retry,
        AnkiExportResult::Add(AnkiAddOutcome::NotInserted(
            AnkiExportError::ProviderFailure
        )),
    ));
    assert!(session.clear());
    assert!(session.export().is_none());
}

#[test]
fn every_known_preinsert_failure_requires_explicit_retry_or_can_be_canceled() {
    let f = Fixture::new("猫。", "猫");
    for error in [
        AnkiExportError::ProviderUnavailable,
        AnkiExportError::UnsupportedProvider,
        AnkiExportError::PermissionDenied,
        AnkiExportError::CollectionUnavailable,
        AnkiExportError::DeckUnavailable,
        AnkiExportError::IncompatibleModel,
        AnkiExportError::DuplicateLookupFailed,
        AnkiExportError::ProviderFailure,
        AnkiExportError::InvalidResponse,
    ] {
        let mut session = AnkiExportSession::default();
        let request = begin(&mut session, &f, 5).unwrap();
        fail(&mut session, &request, error);
        let retry = session.retry().unwrap();
        assert_eq!(request.export(), retry.export());
        assert_eq!(retry.operation(), AnkiExportOperation::CheckAndAdd);
        fail(&mut session, &retry, error);
        assert!(session.clear());
        assert!(session.progress().is_none());
        assert!(session.retry().is_none());
    }
}

#[test]
fn uncertain_insert_survives_dismissal_replacement_and_empty_or_failed_reconciliation() {
    let f = Fixture::new("猫。", "猫");
    let other = Fixture::new("犬。", "犬");
    let mut session = AnkiExportSession::default();
    let add = begin(&mut session, &f, 5).unwrap();
    assert!(session.finish(&add, AnkiExportResult::Add(AnkiAddOutcome::Uncertain)));
    for result in [
        AnkiReconciliation::Unresolved,
        AnkiReconciliation::Failed(AnkiExportError::PermissionDenied),
        AnkiReconciliation::Failed(AnkiExportError::DuplicateLookupFailed),
        AnkiReconciliation::Failed(AnkiExportError::ProviderUnavailable),
    ] {
        assert!(!session.clear());
        assert_eq!(
            begin(&mut session, &other, 6),
            Err(AnkiBeginError::UnresolvedInsertion)
        );
        let reconcile = session.retry().unwrap();
        assert_eq!(reconcile.operation(), AnkiExportOperation::Reconcile);
        assert_eq!(reconcile.export(), add.export());
        assert_eq!(begin(&mut session, &other, 6), Err(AnkiBeginError::Busy));
        assert!(session.retry().is_none());
        assert!(!session.finish(
            &add,
            AnkiExportResult::Add(AnkiAddOutcome::NotInserted(
                AnkiExportError::ProviderFailure
            )),
        ));
        let expected = match result {
            AnkiReconciliation::Failed(error) => Some(error),
            _ => None,
        };
        assert!(session.finish(&reconcile, AnkiExportResult::Reconcile(result)));
        assert_eq!(
            session.progress(),
            Some(&AnkiExportProgress::Uncertain(expected))
        );
        assert!(!session.finish(
            &reconcile,
            AnkiExportResult::Reconcile(AnkiReconciliation::Existing(existing())),
        ));
    }
    let reconcile = session.retry().unwrap();
    assert!(session.finish(
        &reconcile,
        AnkiExportResult::Reconcile(AnkiReconciliation::Existing(existing())),
    ));
    assert_eq!(
        session.progress(),
        Some(&AnkiExportProgress::Existing(existing()))
    );
    assert!(session.retry().is_none());
    assert!(session.clear());
    assert!(begin(&mut session, &other, 6).is_ok());
}

#[test]
fn existing_notes_and_successes_finish_without_another_write() {
    let f = Fixture::new("猫。", "猫");
    let mut session = AnkiExportSession::default();
    for outcome in [
        AnkiAddOutcome::Existing(existing()),
        AnkiAddOutcome::Added(i64::MAX.try_into().unwrap()),
    ] {
        let request = begin(&mut session, &f, 5).unwrap();
        assert!(session.finish(&request, AnkiExportResult::Add(outcome.clone())));
        let expected = match outcome {
            AnkiAddOutcome::Existing(ids) => AnkiExportProgress::Existing(ids),
            AnkiAddOutcome::Added(id) => AnkiExportProgress::Added(id),
            _ => unreachable!(),
        };
        assert_eq!(session.progress(), Some(&expected));
        assert!(session.retry().is_none());
        assert!(!session.finish(&request, AnkiExportResult::Add(AnkiAddOutcome::Uncertain)));
        assert_eq!(session.progress(), Some(&expected));
    }
}

#[test]
fn malformed_result_kinds_never_enable_another_insert() {
    let f = Fixture::new("猫。", "猫");
    let mut session = AnkiExportSession::default();
    let add = begin(&mut session, &f, 5).unwrap();
    assert!(session.finish(
        &add,
        AnkiExportResult::Reconcile(AnkiReconciliation::Unresolved),
    ));
    assert_eq!(
        session.progress(),
        Some(&AnkiExportProgress::Uncertain(Some(
            AnkiExportError::InvalidResponse
        )))
    );
    let reconcile = session.retry().unwrap();
    assert_eq!(reconcile.operation(), AnkiExportOperation::Reconcile);
    assert!(session.finish(
        &reconcile,
        AnkiExportResult::Add(AnkiAddOutcome::NotInserted(
            AnkiExportError::ProviderFailure
        )),
    ));
    assert_eq!(
        session.progress(),
        Some(&AnkiExportProgress::Uncertain(Some(
            AnkiExportError::InvalidResponse
        )))
    );
    assert_eq!(
        session.retry().unwrap().operation(),
        AnkiExportOperation::Reconcile
    );
}

#[test]
fn other_sessions_and_cleared_attempts_cannot_consume_the_current_request() {
    let f = Fixture::new("猫。", "猫");
    let mut session = AnkiExportSession::default();
    let mut other = AnkiExportSession::default();
    let first = begin(&mut session, &f, 5).unwrap();
    let foreign = begin(&mut other, &f, 5).unwrap();
    assert_eq!(first.export(), foreign.export());
    assert_ne!(first, foreign);
    assert!(!session.finish(&foreign, AnkiExportResult::Add(AnkiAddOutcome::Uncertain)));
    fail(&mut session, &first, AnkiExportError::ProviderUnavailable);
    assert!(session.clear());
    let next = begin(&mut session, &f, 5).unwrap();
    assert!(!session.finish(&first, AnkiExportResult::Add(AnkiAddOutcome::Uncertain)));
    assert_eq!(
        session.progress(),
        Some(&AnkiExportProgress::Pending(
            AnkiExportOperation::CheckAndAdd
        ))
    );
    assert!(session.finish(
        &next,
        AnkiExportResult::Add(AnkiAddOutcome::Existing(existing()))
    ));
}

#[test]
fn stale_or_corrupt_selections_cannot_replace_retained_results() {
    let mut f = Fixture::new("猫と猫。", "猫");
    let mut session = AnkiExportSession::default();
    let first = begin(&mut session, &f, 5).unwrap();
    fail(&mut session, &first, AnkiExportError::ProviderUnavailable);
    let original = f.details.selection();
    f.page.analysis = ReaderAnalysis::Available(99.try_into().unwrap());
    assert_eq!(
        begin(&mut session, &f, 6),
        Err(AnkiBeginError::InvalidSelection)
    );
    assert_eq!(session.export(), Some(first.export()));
    f.page.analysis = original.key.analysis;
    f.chapter.text = "犬と犬。".into();
    assert_eq!(
        begin(&mut session, &f, 6),
        Err(AnkiBeginError::InvalidSelection)
    );
    f.chapter.text = "猫と猫。".into();
    let (mut page, details) = f.effective();
    for mutation in 0..3 {
        let saved = page.clone();
        match mutation {
            0 => page.identity.revision += 1,
            1 => page.tokens[0].segment_index += 1,
            2 => page.tokens[0].reading = Some("changed".into()),
            _ => unreachable!(),
        }
        assert_eq!(
            session.begin_effective(
                6.try_into().unwrap(),
                &f.chapter,
                &page,
                details.selection,
                &details
            ),
            Err(AnkiBeginError::InvalidSelection)
        );
        assert_eq!(session.export(), Some(first.export()));
        page = saved;
    }
    let retry = session.retry().unwrap();
    assert_eq!(retry.export(), first.export());
}

#[test]
fn export_outcomes_preserve_status_encounters_page_and_dictionary_inputs() {
    for status in [
        None,
        Some(VocabularyStatus::New),
        Some(VocabularyStatus::Learning),
        Some(VocabularyStatus::Familiar),
        Some(VocabularyStatus::Known),
        Some(VocabularyStatus::Ignored),
    ] {
        let mut f = Fixture::new("食べた。", "食べた");
        f.page.tokens[0].vocabulary = status.map(|status| yomoku_core::reader::ReaderVocabulary {
            id: 3.try_into().unwrap(),
            status,
        });
        f.details.token.vocabulary = f.page.tokens[0].vocabulary;
        let before = (f.chapter.clone(), f.page.clone(), f.details.clone());
        let (effective, details) = f.effective();
        let effective_before = (effective.clone(), details.clone());
        for use_effective in [false, true] {
            let mut session = AnkiExportSession::default();
            let request = if use_effective {
                session
                    .begin_effective(
                        5.try_into().unwrap(),
                        &f.chapter,
                        &effective,
                        details.selection,
                        &details,
                    )
                    .unwrap()
            } else {
                begin(&mut session, &f, 5).unwrap()
            };
            fail(
                &mut session,
                &request,
                AnkiExportError::DuplicateLookupFailed,
            );
            let retry = session.retry().unwrap();
            assert!(session.finish(&retry, AnkiExportResult::Add(AnkiAddOutcome::Uncertain)));
            let reconcile = session.retry().unwrap();
            assert!(session.finish(
                &reconcile,
                AnkiExportResult::Reconcile(AnkiReconciliation::Existing(existing()))
            ));
            assert_eq!(request.export().note().surface().as_str(), "食べた");
            assert!(session.clear());
        }
        assert_eq!(before, (f.chapter, f.page, f.details));
        assert_eq!(effective_before, (effective, details));
    }
}

#[test]
fn provider_ids_and_nonempty_existing_results_keep_exact_long_values() {
    for invalid in [i64::MIN, -1, 0] {
        assert!(AnkiDeckId::try_from(invalid).is_err());
        assert!(AnkiNoteId::try_from(invalid).is_err());
    }
    for valid in [1, 9_007_199_254_740_993, i64::MAX] {
        assert_eq!(i64::from(AnkiDeckId::try_from(valid).unwrap()), valid);
        assert_eq!(i64::from(AnkiNoteId::try_from(valid).unwrap()), valid);
    }
    assert!(AnkiExistingNotes::new(vec![]).is_err());
    let ids = AnkiExistingNotes::new(vec![
        12.try_into().unwrap(),
        9.try_into().unwrap(),
        12.try_into().unwrap(),
    ])
    .unwrap();
    assert_eq!(ids.ids(), &[12.try_into().unwrap(), 9.try_into().unwrap()]);
}
