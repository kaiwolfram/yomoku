use super::*;
use yomoku_core::{
    effective_overrides::{
        EffectiveOverrideContext, EffectiveOverrideFailure as Failure, OverrideRuleRevision,
        StoredBoundaryChoice,
    },
    effective_session::{EffectiveOverrideAction as Action, EffectiveOverrideIdentity},
};

fn event(action: Action) -> Event {
    Event::EffectiveReaderOverride(action)
}

fn open(f: &Fixture, page: EffectivePageIdentity) -> Vec<Effect> {
    f.core.process_event(event(Action::Open {
        page,
        anchor: f.original.tokens[0].anchor,
    }))
}

fn prompt(f: &Fixture, page: EffectivePageIdentity) -> EffectiveOverrideIdentity {
    let mut effects = open(f, page);
    let StorageOperation::Effective(Op::LoadOverrideContext { command, anchor }) =
        operation(&effects)
    else {
        panic!("override read")
    };
    let context = Arc::new(EffectiveOverrideContext::from_snapshot(
        command.shared_key(),
        anchor,
        Surface::new("猫犬").unwrap(),
        OverrideRuleRevision {
            newest_id: 5,
            inactive: 0,
        },
        vec![StoredBoundaryChoice {
            mode: SplitMode::B,
            parts: vec![Surface::new("猫犬").unwrap()],
        }],
        f.rules.clone(),
    ));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::OverrideContextLoaded(context)),
    );
    let view = f.core.view().effective_reader_override.unwrap();
    assert_eq!(
        view.prompt.unwrap().default_scope,
        OverrideScope::AllOccurrences
    );
    view.identity.unwrap()
}

fn save(identity: EffectiveOverrideIdentity) -> Event {
    event(Action::Save {
        identity,
        scope: OverrideScope::ThisOccurrence,
        parts: vec!["猫".into(), "犬".into()],
        registered_at_unix_ms: 600,
    })
}

#[test]
fn override_reads_require_current_page_and_reopen_after_dismissal_or_failure() {
    let f = Fixture::new(false);
    let page = f.load();
    assert!(open(
        &f,
        EffectivePageIdentity {
            revision: page.revision + 1,
            ..page
        }
    )
    .is_empty());
    let mut effects = open(&f, page);
    let identity = f
        .core
        .view()
        .effective_reader_override
        .unwrap()
        .identity
        .unwrap();
    assert!(f.core.view().storage_busy);
    f.core.process_event(event(Action::Dismiss { identity }));
    let dismissed = f.core.view().effective_reader_override.unwrap();
    assert!(dismissed.prompt.is_none() && dismissed.pending);
    // A dismissed read still owns the app slot, and malformed responses cannot
    // become an override prompt or consume a later request.
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Listed(vec![], PageLength::default(), Default::default()),
    );
    assert!(!f.core.view().storage_busy);
    let state = f.core.view().effective_reader_override.unwrap();
    assert!(state.prompt.is_none() && !state.pending);
    assert_eq!(
        state.error,
        Some(Failure::Storage(DocumentError::InvalidDocument))
    );
    let fresh = prompt(&f, page);
    assert!(fresh.revision > identity.revision);
    let before = f.core.view().effective_reader_override;
    f.core.process_event(event(Action::Dismiss { identity }));
    f.core.process_event(save(identity));
    assert_eq!(f.core.view().effective_reader_override, before);
    f.core.process_event(Event::CloseDocument);
    assert!(f
        .core
        .view()
        .effective_reader_override
        .unwrap()
        .prompt
        .is_none());
    assert!(open(&f, page).is_empty());
    assert!(Core::<Yomoku>::new().process_event(save(fresh)).is_empty());
}

#[test]
fn override_choice_generation_and_reversal_retire_page_and_require_explicit_reload() {
    for action in 0..3 {
        let f = Fixture::new(false);
        let page = f.load();
        let identity = prompt(&f, page);
        let (action, output) = match action {
            0 => (
                Action::Choose {
                    identity,
                    mode: SplitMode::B,
                    scope: OverrideScope::AllOccurrences,
                    registered_at_unix_ms: 601,
                },
                Output::OverrideSaved {
                    id: 6.try_into().unwrap(),
                    registered: 0,
                },
            ),
            1 => (
                Action::Generate {
                    identity,
                    mode: SplitMode::C,
                },
                Output::OverrideAnalysisStored(SplitMode::C),
            ),
            _ => (
                Action::Reverse {
                    identity,
                    id: f.rules[0].id,
                },
                Output::OverrideReversed(f.rules[0].id),
            ),
        };
        let mut effects = f.core.process_event(event(action));
        assert!(f.core.view().effective_reader_tokens.is_none());
        assert!(f.core.process_event(f.complete(page)).is_empty());
        assert!(f
            .core
            .process_event(Event::LoadReaderTokens(f.base()))
            .is_empty());
        resolve(&f.core, &mut effects, StorageResult::Effective(output));
        let state = f.core.view().effective_reader_override.unwrap();
        assert!(state.reload_required && !state.pending && state.prompt.is_none());
        assert!(f.core.view().effective_reader_tokens.is_none());
        assert!(f.load().revision > page.revision);
        assert!(
            !f.core
                .view()
                .effective_reader_override
                .unwrap()
                .reload_required
        );
    }
}

#[test]
fn retired_override_reload_requirement_does_not_follow_same_book_reopening() {
    let f = Fixture::new(false);
    let page = f.load();
    let identity = prompt(&f, page);
    let mut effects = f.core.process_event(save(identity));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::OverrideSaved {
            id: 6.try_into().unwrap(),
            registered: 2,
        }),
    );
    assert!(
        f.core
            .view()
            .effective_reader_override
            .unwrap()
            .reload_required
    );
    f.core.process_event(Event::CloseDocument);
    assert!(
        !f.core
            .view()
            .effective_reader_override
            .unwrap()
            .reload_required
    );
    let mut effects = f
        .core
        .process_event(Event::OpenReader(f.document.document_id));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Loaded(DocumentReaderSnapshot {
            document: f.document.clone(),
            page_length: PageLength::default(),
            typography: Default::default(),
            analyses: vec![ReaderChapterAnalysis {
                chapter_id: f.document.chapters[0].chapter_id,
                analysis: page.base.analysis,
            }],
            last_read_sequence: None,
        }),
    );
    assert!(
        !f.core
            .view()
            .effective_reader_override
            .unwrap()
            .reload_required
    );
    assert!(f.load().revision > page.revision);
    assert!(f.core.process_event(save(identity)).is_empty());
}

#[test]
fn failed_override_keeps_exact_retry_and_drains_both_time_slots_before_deferred_departure() {
    for cancel in [false, true] {
        let f = Fixture::new(false);
        let page = f.load();
        restore(&f);
        f.core
            .process_event(clock(&f, 0, ReaderClockAction::Resume));
        let identity = prompt(&f, page);
        let mut effects = f.core.process_event(save(identity));
        let frozen = operation(&effects);
        f.core
            .process_event(clock(&f, 250, ReaderClockAction::Checkpoint));
        assert!(f.core.view().reading_time_save_pending);
        assert!(f
            .core
            .process_event(event(Action::Cancel { identity }))
            .is_empty());
        let mut checkpoint = resolve(
            &f.core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        // Queue a newer absolute time while the first save is in flight.
        f.core
            .process_event(clock(&f, 500, ReaderClockAction::Pause));
        f.core.process_event(Event::CloseDocument);
        assert!(f
            .core
            .process_event(event(Action::Retry { identity }))
            .is_empty());
        let mut checkpoint = save_time(&f, &mut checkpoint, 250);
        save_time(&f, &mut checkpoint, 500);
        assert_eq!(f.core.view().content_route, ContentRoute::Reader);
        for e in [
            Event::LoadReaderTokens(f.base()),
            Event::OpenReader(f.document.document_id),
            Event::CreatePaste("other".into()),
            f.complete(page),
        ] {
            assert!(f.core.process_event(e).is_empty());
        }
        // Foreign Cancel cannot drop a retained write or release departure.
        f.core.process_event(event(Action::Cancel {
            identity: EffectiveOverrideIdentity {
                revision: identity.revision + 1,
                ..identity
            },
        }));
        assert!(f.core.view().effective_reader_override.unwrap().pending);
        if cancel {
            f.core.process_event(event(Action::Cancel { identity }));
        } else {
            let mut retry = f.core.process_event(event(Action::Retry { identity }));
            assert_eq!(operation(&retry), frozen);
            resolve(
                &f.core,
                &mut retry,
                StorageResult::Effective(Output::OverrideSaved {
                    id: 6.try_into().unwrap(),
                    registered: 2,
                }),
            );
        }
        assert_eq!(f.core.view().content_route, ContentRoute::Main);
        assert!(!f.core.view().effective_reader_override.unwrap().pending);
    }
}

#[test]
fn stale_override_response_requires_reload_and_read_dismissal_keeps_fifo_ownership() {
    let f = Fixture::new(false);
    let page = f.load();
    let identity = prompt(&f, page);
    let mut effects = f.core.process_event(save(identity));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::OverrideFailed(Failure::StaleContext)),
    );
    let state = f.core.view().effective_reader_override.unwrap();
    assert!(state.reload_required && !state.pending);
    f.core.process_event(event(Action::Retry { identity }));
    assert!(!f.core.view().storage_busy);
    let page = f.load();
    restore(&f);
    f.core
        .process_event(clock(&f, 0, ReaderClockAction::Resume));
    let mut effects = open(&f, page);
    f.core
        .process_event(clock(&f, 300, ReaderClockAction::Pause));
    f.core.process_event(Event::CloseDocument);
    let mut checkpoint = resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(f.core.view().storage_busy);
    save_time(&f, &mut checkpoint, 300);
    assert_eq!(f.core.view().content_route, ContentRoute::Main);
    assert!(f
        .core
        .view()
        .effective_reader_override
        .unwrap()
        .prompt
        .is_none());
}
