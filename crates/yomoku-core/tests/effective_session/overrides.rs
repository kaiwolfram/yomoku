use std::sync::Arc;

use super::*;
use yomoku_core::{
    effective_overrides::{
        EffectiveOverrideContext, EffectiveOverrideFailure as Failure, OverrideRuleRevision,
        StoredBoundaryChoice,
    },
    effective_session::{
        EffectiveOverrideProgress as Progress, EffectiveOverrideRequest, EffectivePageIdentity,
    },
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    storage::DocumentError,
};

fn loaded(manual: bool) -> (EffectiveReaderSession, EffectivePageIdentity, TextAnchor) {
    let mut session = EffectiveReaderSession::default();
    let mut snapshot = snapshot_with_manual_split(manual);
    snapshot.tokens.last_mut().unwrap().vocabulary = Some(ReaderVocabulary {
        id: 55.try_into().unwrap(),
        status: VocabularyStatus::New,
    });
    let key = snapshot.key().unwrap();
    let anchor = TextAnchor {
        range: TextRange {
            start: key.segmentation.segments[2].boundary.anchor.range.start,
            end: key
                .segmentation
                .segments
                .last()
                .unwrap()
                .boundary
                .anchor
                .range
                .end,
        },
        ..key.page
    };
    let request = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(request, snapshot));
    let identity = session.page().unwrap().identity;
    (session, identity, anchor)
}

fn parts(values: &[&str]) -> Vec<Surface> {
    values
        .iter()
        .map(|value| Surface::new(*value).unwrap())
        .collect()
}

fn context(request: &EffectiveOverrideRequest, manual: bool) -> Arc<EffectiveOverrideContext> {
    let Op::LoadOverrideContext { command, anchor } = request.operation() else {
        panic!("load")
    };
    let rules = if manual {
        vec![StoredOverride {
            id: 5.try_into().unwrap(),
            active: true,
            edit: SegmentationOverride {
                scope: OverrideScope::default(),
                anchor,
                surface: Surface::new("猫犬").unwrap(),
                parts: parts(&["猫", "犬"]),
            },
        }]
    } else {
        vec![]
    };
    Arc::new(EffectiveOverrideContext::from_snapshot(
        command.shared_key(),
        anchor,
        Surface::new("猫犬").unwrap(),
        OverrideRuleRevision {
            newest_id: i64::from(manual) * 5,
            inactive: 0,
        },
        vec![
            StoredBoundaryChoice {
                mode: SplitMode::A,
                parts: parts(&["猫", "犬"]),
            },
            StoredBoundaryChoice {
                mode: SplitMode::B,
                parts: parts(&["猫犬"]),
            },
        ],
        rules,
    ))
}

fn prompt(
    session: &mut EffectiveReaderSession,
    identity: EffectivePageIdentity,
    anchor: TextAnchor,
    manual: bool,
) {
    let request = session.begin_override_load(identity, anchor).unwrap();
    let context = context(&request, manual);
    assert_eq!(
        session.finish_override_request(&request, Ok(Output::OverrideContextLoaded(context))),
        Some(Progress::Loaded)
    );
}

#[test]
fn prompt_read_authenticates_key_anchor_and_fresh_attempts_without_mutating_page() {
    let (mut session, identity, anchor) = loaded(false);
    let page = session.page().unwrap().clone();
    let request = session.begin_override_load(identity, anchor).unwrap();
    assert!(session.override_state().busy);
    assert!(session.begin_override_load(identity, anchor).is_none());
    let good = context(&request, false);
    let mut wrong_anchor = anchor;
    wrong_anchor.range.start = 0.try_into().unwrap();
    let Op::LoadOverrideContext { command, .. } = request.operation() else {
        unreachable!()
    };
    let bad = EffectiveOverrideContext::from_snapshot(
        command.shared_key(),
        wrong_anchor,
        good.surface().clone(),
        good.rule_revision(),
        good.choices().to_vec(),
        vec![],
    );
    assert_eq!(
        session.finish_override_request(&request, Ok(Output::OverrideContextLoaded(Arc::new(bad)))),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::InvalidDocument
        )))
    );
    assert!(!session.override_state().pending);
    assert!(session.override_context().is_none());
    let retry = session.begin_override_load(identity, anchor).unwrap();
    assert_ne!(request, retry);
    assert_eq!(request.operation(), retry.operation());
    assert!(session
        .finish_override_request(
            &request,
            Ok(Output::OverrideContextLoaded(Arc::clone(&good)))
        )
        .is_none());
    assert!(session.override_state().busy);
    assert_eq!(
        session.finish_override_request(&retry, Ok(Output::OverrideContextLoaded(good))),
        Some(Progress::Loaded)
    );
    assert_eq!(session.override_context().unwrap().choices().len(), 2);
    assert_eq!(session.page(), Some(&page));
}

#[test]
fn dismissed_or_invalidated_reads_drain_without_resurrecting_prompt_or_retiring_fresh_page() {
    for invalidate in [false, true] {
        let (mut session, identity, anchor) = loaded(false);
        let request = session.begin_override_load(identity, anchor).unwrap();
        let context = context(&request, false);
        if invalidate {
            let snapshot = snapshot();
            let load = session.begin_load(base(&snapshot)).unwrap();
            assert!(session.accept(load, snapshot));
        } else {
            session.dismiss_override();
        }
        let page = session.page().unwrap().clone();
        assert_eq!(
            session.finish_override_request(&request, Ok(Output::OverrideContextLoaded(context))),
            Some(Progress::Loaded)
        );
        assert!(session.override_context().is_none());
        assert_eq!(session.page(), Some(&page));
        assert!(session
            .finish_override_request(&request, Err(DocumentError::Unavailable))
            .is_none());
    }
    let (mut session, identity, anchor) = loaded(false);
    let request = session.begin_override_load(identity, anchor).unwrap();
    let snapshot = snapshot();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    let page = session.page().unwrap().clone();
    session
        .finish_override_request(&request, Ok(Output::OverrideFailed(Failure::StaleContext)))
        .unwrap();
    assert_eq!(session.page(), Some(&page));
}

#[test]
fn choices_manual_parts_and_reversals_require_a_current_valid_prompt() {
    let (mut session, identity, anchor) = loaded(false);
    let mut foreign = identity;
    foreign.revision += 1;
    assert!(session.begin_override_load(foreign, anchor).is_none());
    let mut foreign_anchor = anchor;
    foreign_anchor.chapter_id = 999.try_into().unwrap();
    assert!(session
        .begin_override_load(identity, foreign_anchor)
        .is_none());
    assert!(session
        .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 100)
        .is_none());
    prompt(&mut session, identity, anchor, false);
    for invalid in [vec![], parts(&["犬", "猫"]), parts(&["猫"])] {
        assert!(session
            .begin_override_save(OverrideScope::default(), invalid, 100)
            .is_none());
    }
    assert!(session
        .begin_override_save(OverrideScope::default(), parts(&["猫犬"]), -1)
        .is_none());
    assert!(session
        .begin_override_choice(SplitMode::C, OverrideScope::default(), 100)
        .is_none());
    assert!(session
        .begin_override_reverse(5.try_into().unwrap())
        .is_none());
    let request = session
        .begin_override_choice(SplitMode::A, OverrideScope::default(), 100)
        .unwrap();
    assert!(
        matches!(request.operation(), Op::SaveOverride { scope: OverrideScope::AllOccurrences, registered_at_unix_ms: 100, parts: ref p, .. } if p == &parts(&["猫", "犬"]))
    );
    assert!(session.page().is_none());
    assert!(session.override_context().is_none());
    assert!(!session.cancel_override_retry());
}

#[test]
fn failed_writes_share_frozen_context_and_values_across_invalidation_and_explicit_retry() {
    let (mut session, identity, anchor) = loaded(false);
    prompt(&mut session, identity, anchor, false);
    let request = session
        .begin_override_save(OverrideScope::ThisOccurrence, parts(&["猫", "犬"]), 123)
        .unwrap();
    assert_eq!(
        session.finish_override_request(&request, Err(DocumentError::Unavailable)),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::Unavailable
        )))
    );
    session.invalidate();
    assert!(session.override_state().pending);
    let retry = session.retry_override_write().unwrap();
    assert_ne!(request, retry);
    assert_eq!(request.operation(), retry.operation());
    let (
        Op::SaveOverride { context: first, .. },
        Op::SaveOverride {
            context: second, ..
        },
    ) = (request.operation(), retry.operation())
    else {
        unreachable!()
    };
    assert!(Arc::ptr_eq(&first, &second));
    assert!(session
        .finish_override_request(
            &request,
            Ok(Output::OverrideSaved {
                id: 6.try_into().unwrap(),
                registered: 2
            })
        )
        .is_none());
    let snapshot = snapshot();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    assert!(session.page().is_some());
    assert_eq!(
        session.finish_override_request(
            &retry,
            Ok(Output::OverrideSaved {
                id: 6.try_into().unwrap(),
                registered: 2
            })
        ),
        Some(Progress::Saved {
            id: 6.try_into().unwrap(),
            registered: 2
        })
    );
    assert!(session.page().is_none());
    assert!(session.override_state().reload_required);
    assert!(!session.override_state().pending);
    assert!(session.retry_override_write().is_none());
}

#[test]
fn stale_writes_require_reload_and_cannot_replay_old_fields_even_after_new_page_load() {
    for error in [Failure::StaleContext, Failure::InvalidSelection] {
        let (mut session, identity, anchor) = loaded(false);
        prompt(&mut session, identity, anchor, false);
        let write = session
            .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 123)
            .unwrap();
        assert_eq!(
            session.finish_override_request(&write, Ok(Output::OverrideFailed(error))),
            Some(Progress::Failed(error))
        );
        assert!(session.override_state().reload_required);
        assert!(!session.override_state().pending);
        assert!(session.retry_override_write().is_none());
        let snapshot = snapshot();
        let load = session.begin_load(base(&snapshot)).unwrap();
        assert!(session.accept(load, snapshot));
        assert!(session
            .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 124)
            .is_none());
        let identity = session.page().unwrap().identity;
        prompt(&mut session, identity, anchor, false);
        assert!(!session.override_state().reload_required);
        assert!(session
            .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 124)
            .is_some());
    }
}

#[test]
fn reversal_authenticates_winning_rule_and_result_and_failed_request_can_cancel() {
    let (mut session, identity, anchor) = loaded(true);
    prompt(&mut session, identity, anchor, true);
    assert!(session
        .begin_override_reverse(99.try_into().unwrap())
        .is_none());
    let request = session
        .begin_override_reverse(5.try_into().unwrap())
        .unwrap();
    assert_eq!(
        session.finish_override_request(
            &request,
            Ok(Output::OverrideReversed(99.try_into().unwrap()))
        ),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::InvalidDocument
        )))
    );
    let retry = session.retry_override_write().unwrap();
    assert_eq!(request.operation(), retry.operation());
    assert_eq!(
        session
            .finish_override_request(&retry, Ok(Output::OverrideReversed(5.try_into().unwrap()))),
        Some(Progress::Reversed(5.try_into().unwrap()))
    );
    assert!(session.override_state().reload_required);
    let (mut session, identity, anchor) = loaded(false);
    prompt(&mut session, identity, anchor, false);
    let request = session
        .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 123)
        .unwrap();
    session
        .finish_override_request(&request, Err(DocumentError::Unavailable))
        .unwrap();
    assert!(session.cancel_override_retry());
    assert!(!session.override_state().pending);
    assert!(session.override_state().reload_required);
    assert!(session.retry_override_write().is_none());
}

#[test]
fn override_io_and_accepted_word_status_custom_completion_commands_gate_each_other() {
    let (mut custom, selection, details) = super::words::loaded(false);
    let word = custom.begin_word_load(selection).unwrap();
    assert!(custom.finish_word_request(&word, Ok(Output::WordLoaded(selection, Box::new(details)))));
    let save = custom
        .begin_custom_entry_save(selection, "meaning".into(), "よみ".into())
        .unwrap();
    assert!(custom
        .begin_override_load(selection.page, selection.page.base.page)
        .is_none());
    custom.finish_word_request(&save, Err(DocumentError::Unavailable));
    assert!(custom
        .begin_override_load(selection.page, selection.page.base.page)
        .is_none());
    assert!(custom.cancel_custom_entry_retry());
    let read = custom
        .begin_override_load(selection.page, selection.page.base.page)
        .unwrap();
    assert!(custom
        .begin_custom_entry_save(selection, "meaning".into(), "よみ".into())
        .is_none());
    custom
        .finish_override_request(&read, Err(DocumentError::Unavailable))
        .unwrap();

    let (mut session, identity, anchor) = loaded(false);
    let selection = EffectiveReaderSelection {
        page: identity,
        segment_index: 2,
    };
    let word = session.begin_word_load(selection).unwrap();
    assert!(session.begin_override_load(identity, anchor).is_none());
    session.finish_word_request(&word, Err(DocumentError::Unavailable));
    let status = session
        .begin_status_change(
            selection,
            55.try_into().unwrap(),
            VocabularyStatus::Known,
            100,
        )
        .unwrap();
    assert!(session.begin_override_load(identity, anchor).is_none());
    session.finish_status_request(&status, Err(DocumentError::Unavailable));
    assert!(session.begin_override_load(identity, anchor).is_none());
    session.cancel_status_retry();
    let read = session.begin_override_load(identity, anchor).unwrap();
    assert!(session.begin_word_load(selection).is_none());
    assert!(session
        .begin_status_change(
            selection,
            55.try_into().unwrap(),
            VocabularyStatus::Known,
            100
        )
        .is_none());
    assert!(!session.begin_completion(identity, super::statuses::intent(selection)));
    let context = context(&read, false);
    session
        .finish_override_request(&read, Ok(Output::OverrideContextLoaded(context)))
        .unwrap();
    let write = session
        .begin_override_save(OverrideScope::default(), parts(&["猫", "犬"]), 123)
        .unwrap();
    session
        .finish_override_request(&write, Err(DocumentError::Unavailable))
        .unwrap();
    // Fresh rendered pages still cannot bypass an unresolved accepted write.
    let snapshot = snapshot();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    let selection = EffectiveReaderSelection {
        page: session.page().unwrap().identity,
        segment_index: 2,
    };
    assert!(session.begin_word_load(selection).is_none());
    assert!(!session.begin_completion(selection.page, super::statuses::intent(selection)));
    assert!(session.cancel_override_retry());
    assert!(session.page().is_none());
    let snapshot = super::snapshot();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    let selection = EffectiveReaderSelection {
        page: session.page().unwrap().identity,
        segment_index: 2,
    };
    assert!(session.begin_completion(selection.page, super::statuses::intent(selection)));
    assert!(session
        .begin_override_load(selection.page, anchor)
        .is_none());
}

#[test]
fn alternate_generation_freezes_mode_context_retries_with_fresh_attempt_and_requires_reload() {
    let (mut session, identity, anchor) = loaded(false);
    assert!(session.begin_override_analysis(SplitMode::C).is_none());
    prompt(&mut session, identity, anchor, false);
    assert!(session.begin_override_analysis(SplitMode::B).is_none());
    assert!(session.begin_override_analysis(SplitMode::A).is_none()); // already offered
    let request = session.begin_override_analysis(SplitMode::C).unwrap();
    assert!(session.page().is_none());
    assert!(session.override_context().is_none());
    assert!(!session.cancel_override_retry());
    assert!(session.begin_override_analysis(SplitMode::C).is_none());
    assert_eq!(
        session.finish_override_request(&request, Err(DocumentError::Unavailable)),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::Unavailable
        )))
    );
    session.invalidate();
    let retry = session.retry_override_write().unwrap();
    assert_ne!(request, retry);
    let (
        Op::GenerateOverrideAnalysis {
            context: first,
            mode: SplitMode::C,
        },
        Op::GenerateOverrideAnalysis {
            context: second,
            mode: SplitMode::C,
        },
    ) = (request.operation(), retry.operation())
    else {
        panic!("generation")
    };
    assert!(Arc::ptr_eq(&first, &second));
    assert!(session
        .finish_override_request(&request, Ok(Output::OverrideAnalysisStored(SplitMode::C)))
        .is_none());
    // A mismatched mode cannot complete the retained request.
    assert_eq!(
        session.finish_override_request(&retry, Ok(Output::OverrideAnalysisStored(SplitMode::A))),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::InvalidDocument
        )))
    );
    let retry = session.retry_override_write().unwrap();
    assert_eq!(
        session.finish_override_request(&retry, Ok(Output::OverrideAnalysisStored(SplitMode::C))),
        Some(Progress::AnalysisStored(SplitMode::C))
    );
    assert!(!session.override_state().pending);
    assert!(session.override_state().reload_required);
    assert!(session.page().is_none());
    assert!(session.retry_override_write().is_none());
}

#[test]
fn alternate_generation_stale_or_cancel_requires_reload_and_gates_fresh_reader_work() {
    for stale in [false, true] {
        let (mut session, identity, anchor) = loaded(false);
        prompt(&mut session, identity, anchor, false);
        let request = session.begin_override_analysis(SplitMode::C).unwrap();
        session.dismiss_override();
        // Even a refreshed page cannot bypass this accepted cache write.
        let snapshot = snapshot();
        let load = session.begin_load(base(&snapshot)).unwrap();
        assert!(session.accept(load, snapshot));
        let identity = session.page().unwrap().identity;
        let selection = EffectiveReaderSelection {
            page: identity,
            segment_index: 2,
        };
        assert!(session.begin_word_load(selection).is_none());
        assert!(session
            .begin_status_change(
                selection,
                55.try_into().unwrap(),
                VocabularyStatus::Known,
                123
            )
            .is_none());
        assert!(!session.begin_completion(identity, super::statuses::intent(selection)));
        assert!(session.begin_override_load(identity, anchor).is_none());
        let failure = if stale {
            Failure::StaleContext
        } else {
            Failure::Storage(DocumentError::Unavailable)
        };
        session
            .finish_override_request(&request, Ok(Output::OverrideFailed(failure)))
            .unwrap();
        if stale {
            assert!(session.retry_override_write().is_none());
        } else {
            assert!(session.override_state().pending);
            assert!(session.cancel_override_retry());
        }
        assert!(session.override_state().reload_required);
        assert!(session.page().is_none());
        assert!(session.override_context().is_none());
    }
}
