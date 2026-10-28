use super::*;
use yomoku_core::effective_session::EffectiveOverrideProgress;

#[test]
fn effective_worker_callback_adapter_routes_completion_status_deletion_and_override_lifecycles() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let identity = session.page().unwrap().identity;
    assert!(session.begin_completion(identity, fixture.intent));
    let prepare = session.dispatch_completion().unwrap();
    assert_eq!(
        finish_completion(&router, &mut session, &prepare),
        CompletionProgress::Prepared
    );
    // Prepared progress never dispatches the commit automatically.
    assert!(session.completion_state().pending && !session.completion_state().busy);
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let commit = session.dispatch_completion().unwrap();
    let CompletionProgress::Completed { outcome, .. } =
        finish_completion(&router, &mut session, &commit)
    else {
        panic!("completed")
    };
    assert!(matches!(
        outcome,
        AnalyzedCompletionOutcome::Recorded { .. }
    ));
    assert!(session
        .finish_request(
            &EffectiveReaderRequest::Completion(commit),
            StorageResult::Failed(DocumentError::Unavailable)
        )
        .is_none());

    // Load observes exact-surface history; a later status response can report that
    // the entry and its history were deleted from another session.
    let before = std::fs::read(fixture.path()).unwrap();
    load(&router, &mut session, fixture.base);
    let selection = select(&session, "猫").selection();
    let lookup = session.begin_word_load(selection).unwrap();
    finish_word(&router, &mut session, &lookup);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let details = session.word_state().details.unwrap();
    assert_eq!(details.encountered, 1);
    let id = details.token.vocabulary.unwrap().id;
    let candidates = details.candidates;
    fixture
        .sql()
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(id)])
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let status = session
        .begin_status_change(selection, id, VocabularyStatus::Ignored, 600)
        .unwrap();
    assert_eq!(
        finish_request(
            &router,
            &mut session,
            EffectiveReaderRequest::Status(status)
        ),
        EffectiveReaderProgress::StatusFinished
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let details = session.word_state().details.unwrap();
    assert_eq!(details.encountered, 0);
    assert!(details.token.vocabulary.is_none());
    assert_eq!(details.candidates, candidates);
    assert!(!session.status_state().pending);

    let anchor = TextAnchor {
        range: TextRange {
            start: TextOffset::START,
            end: 9.try_into().unwrap(),
        },
        ..fixture.base.page
    };
    let context = session.begin_override_load(selection.page, anchor).unwrap();
    assert_eq!(
        finish_request(
            &router,
            &mut session,
            EffectiveReaderRequest::Override(context)
        ),
        EffectiveReaderProgress::Override(EffectiveOverrideProgress::Loaded)
    );
    let save = session
        .begin_override_save(
            OverrideScope::ThisOccurrence,
            ["食", "べた"]
                .map(|part| Surface::new(part).unwrap())
                .into(),
            700,
        )
        .unwrap();
    assert!(session.page().is_none());
    let EffectiveReaderProgress::Override(EffectiveOverrideProgress::Saved { id, registered }) =
        finish_request(
            &router,
            &mut session,
            EffectiveReaderRequest::Override(save),
        )
    else {
        panic!("saved")
    };
    assert_eq!(registered, 2);
    assert!(session.override_state().reload_required);
    load(&router, &mut session, fixture.base);
    let context = session
        .begin_override_load(session.page().unwrap().identity, anchor)
        .unwrap();
    assert_eq!(
        finish_request(
            &router,
            &mut session,
            EffectiveReaderRequest::Override(context)
        ),
        EffectiveReaderProgress::Override(EffectiveOverrideProgress::Loaded)
    );
    let reverse = session.begin_override_reverse(id).unwrap();
    assert_eq!(
        finish_request(
            &router,
            &mut session,
            EffectiveReaderRequest::Override(reverse)
        ),
        EffectiveReaderProgress::Override(EffectiveOverrideProgress::Reversed(id))
    );
    assert!(session.page().is_none());
    assert!(session.override_state().reload_required);
    // Boundary changes do not recreate the independently deleted vocabulary or
    // rewrite prior completion partitions/counts.
    assert!(fixture
        .storage()
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .is_none());
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        5
    );
}
