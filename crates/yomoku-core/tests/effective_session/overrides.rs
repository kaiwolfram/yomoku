use std::sync::Arc;

use super::*;
use yomoku_core::{
    effective_overrides::{
        EffectiveOverrideContext, EffectiveOverrideFailure as Failure, OverrideRuleRevision,
        StoredBoundaryChoice,
    },
    effective_session::{
        EffectiveOverrideAction as Shell, EffectiveOverrideProgress as Progress,
        EffectiveOverrideRequest, EffectivePageIdentity,
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

#[test]
fn compact_prompt_projects_only_stored_choices_winning_rules_and_default_global_scope() {
    let (mut session, page, anchor) = loaded(true);
    let request = session
        .handle_override_action(Shell::Open { page, anchor })
        .unwrap();
    let loading = session.override_view();
    assert!(loading.busy && loading.pending && loading.prompt.is_none());
    let expected = context(&request, true);
    session
        .finish_override_request(
            &request,
            Ok(Output::OverrideContextLoaded(Arc::clone(&expected))),
        )
        .unwrap();
    let view = session.override_view();
    let prompt = view.prompt.unwrap();
    assert_eq!(view.identity, Some(prompt.identity));
    assert_eq!(prompt.identity.page, page);
    assert_eq!(prompt.anchor, anchor); // full crossing span, not clipped to page
    assert!(anchor.range.end > page.base.page.range.end);
    assert_eq!(prompt.surface, "猫犬");
    assert_eq!(prompt.default_scope, OverrideScope::AllOccurrences);
    assert_eq!(
        prompt
            .choices
            .iter()
            .map(|choice| choice.mode)
            .collect::<Vec<_>>(),
        vec![SplitMode::A, SplitMode::B]
    );
    assert_eq!(prompt.choices[0].parts, vec!["猫", "犬"]);
    assert_eq!(prompt.winning_rules.len(), 1);
    let rule = &prompt.winning_rules[0];
    assert_eq!(rule.id, expected.rules()[0].id);
    assert_eq!(rule.scope, OverrideScope::AllOccurrences);
    assert_eq!(rule.surface, "猫犬");
    assert_eq!(rule.parts, vec!["猫", "犬"]);
    assert!(!view.busy && !view.pending && view.error.is_none() && !view.reload_required);
}

#[test]
fn reopened_same_page_prompt_rejects_old_choices_manual_generation_reversal_and_dismissal() {
    let (mut session, page, anchor) = loaded(true);
    prompt(&mut session, page, anchor, true);
    let old = session.override_view().identity.unwrap();
    session.handle_override_action(Shell::Dismiss { identity: old });
    assert!(session.override_view().prompt.is_none());
    prompt(&mut session, page, anchor, true);
    let view = session.override_view();
    let fresh = view.identity.unwrap();
    assert_eq!(old.page, fresh.page);
    assert!(fresh.revision > old.revision);
    for action in [
        Shell::Choose {
            identity: old,
            mode: SplitMode::A,
            scope: OverrideScope::default(),
            registered_at_unix_ms: 100,
        },
        Shell::Save {
            identity: old,
            scope: OverrideScope::ThisOccurrence,
            parts: vec!["猫".into(), "犬".into()],
            registered_at_unix_ms: 100,
        },
        Shell::Generate {
            identity: old,
            mode: SplitMode::C,
        },
        Shell::Reverse {
            identity: old,
            id: 5.try_into().unwrap(),
        },
        Shell::Retry { identity: old },
        Shell::Cancel { identity: old },
        Shell::Dismiss { identity: old },
    ] {
        assert!(session.handle_override_action(action).is_none());
        assert_eq!(session.override_view(), view);
    }
    let request = session
        .handle_override_action(Shell::Choose {
            identity: fresh,
            mode: SplitMode::A,
            scope: OverrideScope::ThisOccurrence,
            registered_at_unix_ms: 123,
        })
        .unwrap();
    assert!(
        matches!(request.operation(), Op::SaveOverride { scope: OverrideScope::ThisOccurrence, registered_at_unix_ms: 123, parts: ref p, .. } if p == &parts(&["猫", "犬"]))
    );
    assert!(session.override_view().prompt.is_none());
    assert!(session.page().is_none());
}

#[test]
fn shell_manual_parts_are_literal_validated_and_retry_cancel_authenticate_the_accepted_prompt() {
    let (mut session, page, anchor) = loaded(false);
    prompt(&mut session, page, anchor, false);
    let identity = session.override_view().identity.unwrap();
    for values in [
        vec![],
        vec![""],
        vec![" "],
        vec!["犬", "猫"],
        vec!["猫", "犬 "],
    ] {
        assert!(session
            .handle_override_action(Shell::Save {
                identity,
                scope: OverrideScope::default(),
                parts: values.into_iter().map(str::to_owned).collect(),
                registered_at_unix_ms: 123,
            })
            .is_none());
        assert!(!session.override_view().pending);
        assert!(session.override_view().prompt.is_some());
    }
    let request = session
        .handle_override_action(Shell::Save {
            identity,
            scope: OverrideScope::AllOccurrences,
            parts: vec!["猫".into(), "犬".into()],
            registered_at_unix_ms: 123,
        })
        .unwrap();
    session.handle_override_action(Shell::Cancel { identity });
    assert!(session.override_view().busy); // in-flight writes cannot be abandoned
    session
        .finish_override_request(&request, Err(DocumentError::Unavailable))
        .unwrap();
    let failed = session.override_view();
    assert_eq!(failed.identity, Some(identity));
    assert_eq!(
        failed.error,
        Some(Failure::Storage(DocumentError::Unavailable))
    );
    assert!(failed.pending && !failed.busy && failed.prompt.is_none());
    let mut foreign = identity;
    foreign.revision += 1;
    for action in [
        Shell::Retry { identity: foreign },
        Shell::Cancel { identity: foreign },
    ] {
        assert!(session.handle_override_action(action).is_none());
        assert_eq!(session.override_view(), failed);
    }
    let retry = session
        .handle_override_action(Shell::Retry { identity })
        .unwrap();
    assert_ne!(retry, request);
    assert_eq!(retry.operation(), request.operation());
    session
        .finish_override_request(&retry, Err(DocumentError::Unavailable))
        .unwrap();
    session.handle_override_action(Shell::Cancel { identity });
    let canceled = session.override_view();
    assert!(canceled.reload_required);
    assert!(!canceled.pending && !canceled.busy);
    assert!(canceled.identity.is_none() && canceled.prompt.is_none() && canceled.error.is_none());
}

#[test]
fn compact_generation_reversal_and_stale_results_preserve_native_reload_semantics() {
    for generate in [false, true] {
        let (mut session, page, anchor) = loaded(true);
        prompt(&mut session, page, anchor, true);
        let identity = session.override_view().identity.unwrap();
        let action = if generate {
            Shell::Generate {
                identity,
                mode: SplitMode::C,
            }
        } else {
            Shell::Reverse {
                identity,
                id: 5.try_into().unwrap(),
            }
        };
        let request = session.handle_override_action(action).unwrap();
        assert!(session.override_view().prompt.is_none());
        let result = if generate {
            Output::OverrideAnalysisStored(SplitMode::C)
        } else {
            Output::OverrideReversed(5.try_into().unwrap())
        };
        session
            .finish_override_request(&request, Ok(result))
            .unwrap();
        let done = session.override_view();
        assert!(done.reload_required && !done.pending && done.prompt.is_none());
        assert!(session
            .handle_override_action(Shell::Retry { identity })
            .is_none());
    }
    let (mut session, page, anchor) = loaded(false);
    prompt(&mut session, page, anchor, false);
    let identity = session.override_view().identity.unwrap();
    let request = session
        .handle_override_action(Shell::Generate {
            identity,
            mode: SplitMode::C,
        })
        .unwrap();
    session
        .finish_override_request(&request, Ok(Output::OverrideFailed(Failure::StaleContext)))
        .unwrap();
    let stale = session.override_view();
    assert!(stale.reload_required && !stale.pending && stale.identity.is_none());
    assert_eq!(stale.error, Some(Failure::StaleContext));
    assert!(session
        .handle_override_action(Shell::Retry { identity })
        .is_none());
}

#[test]
fn compact_dismissed_reads_and_page_refreshes_cannot_restore_prompt_actions() {
    let (mut session, page, anchor) = loaded(false);
    let request = session
        .handle_override_action(Shell::Open { page, anchor })
        .unwrap();
    let identity = session.override_view().identity.unwrap();
    let context = context(&request, false);
    session.handle_override_action(Shell::Dismiss { identity });
    session
        .finish_override_request(&request, Ok(Output::OverrideContextLoaded(context)))
        .unwrap();
    assert!(session.override_view().prompt.is_none());
    assert!(session.override_view().identity.is_none());
    prompt(&mut session, page, anchor, false);
    let identity = session.override_view().identity.unwrap();
    let snapshot = snapshot();
    let load = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(load, snapshot));
    assert!(session.override_view().prompt.is_none());
    assert!(session
        .handle_override_action(Shell::Save {
            identity,
            scope: OverrideScope::default(),
            parts: vec!["猫".into(), "犬".into()],
            registered_at_unix_ms: 123
        })
        .is_none());
}

#[test]
fn compact_manual_command_preserves_supplementary_combining_and_nul_parts_without_normalizing() {
    let (mut session, page, _) = loaded(false);
    let anchor = session.resolve(page).unwrap().segmentation.segments[0]
        .boundary
        .anchor;
    let request = session
        .handle_override_action(Shell::Open { page, anchor })
        .unwrap();
    let Op::LoadOverrideContext { command, .. } = request.operation() else {
        unreachable!()
    };
    let context = Arc::new(EffectiveOverrideContext::from_snapshot(
        command.shared_key(),
        anchor,
        Surface::new("𠮷か\u{3099}\0").unwrap(),
        OverrideRuleRevision {
            newest_id: 0,
            inactive: 0,
        },
        vec![StoredBoundaryChoice {
            mode: SplitMode::B,
            parts: parts(&["𠮷か\u{3099}\0"]),
        }],
        vec![],
    ));
    session
        .finish_override_request(&request, Ok(Output::OverrideContextLoaded(context)))
        .unwrap();
    let view = session.override_view();
    let prompt = view.prompt.unwrap();
    assert_eq!(prompt.surface, "𠮷か\u{3099}\0");
    assert_eq!(prompt.choices[0].parts, vec!["𠮷か\u{3099}\0"]);
    let request = session
        .handle_override_action(Shell::Save {
            identity: prompt.identity,
            scope: prompt.default_scope,
            parts: vec!["𠮷".into(), "か\u{3099}\0".into()],
            registered_at_unix_ms: 123,
        })
        .unwrap();
    assert!(
        matches!(request.operation(), Op::SaveOverride { parts: ref p, .. } if p == &parts(&["𠮷", "か\u{3099}\0"]))
    );
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
