use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Model, Yomoku},
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{
        VocabularyPage, VocabularyQuery, VocabularyRow, VocabularySort, VocabularyStatus,
    },
};

fn cores() -> [Core<Yomoku>; 2] {
    [Core::new(), Core::new_with(Yomoku, Model::default())]
}
fn enter(core: &Core<Yomoku>) {
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
}
fn query(prefix: &str) -> VocabularyQuery {
    VocabularyQuery {
        prefix: prefix.into(),
        ..Default::default()
    }
}
fn operation(effects: &[Effect]) -> Option<&StorageOperation> {
    effects.iter().find_map(|effect| match effect {
        Effect::Storage(request) => Some(&request.operation),
        _ => None,
    })
}
fn resolve(core: &Core<Yomoku>, effects: &mut [Effect], result: StorageResult) -> Vec<Effect> {
    let request = effects
        .iter_mut()
        .find_map(|effect| match effect {
            Effect::Storage(request) => Some(request),
            _ => None,
        })
        .expect("storage request");
    core.resolve(request, result).unwrap()
}
fn loaded(prefix: &str, offset: u64, count: u64) -> StorageResult {
    let end = offset.saturating_add(100).min(count);
    StorageResult::VocabularyLoaded(VocabularyPage {
        matching_count: count,
        offset,
        rows: (offset..end)
            .map(|index| VocabularyRow {
                id: i64::try_from(index + 1).unwrap().try_into().unwrap(),
                surface: format!("{prefix}猫{index}"),
                status: VocabularyStatus::New,
                registered_at_unix_ms: 123,
                encountered_count: index,
                last_seen_unix_ms: (index > 0).then_some(456),
            })
            .collect(),
        next_offset: (end < count).then_some(end),
    })
}
fn request(query: VocabularyQuery, offset: u64) -> StorageOperation {
    StorageOperation::LoadVocabulary { query, offset }
}

#[test]
fn lazy_defaults_empty_results_settings_and_three_pages_are_retained() {
    for core in cores() {
        assert_eq!(core.view().vocabulary.query, VocabularyQuery::default());
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        enter(&core);
        let mut effects = core.process_event(Event::LoadVocabulary);
        assert_eq!(operation(&effects), Some(&request(query(""), 0)));
        assert!(core.view().vocabulary.loading && core.view().storage_busy);
        for event in [
            Event::LoadVocabulary,
            Event::LoadMoreVocabulary,
            Event::RetryVocabulary,
        ] {
            assert!(core.process_event(event).is_empty());
        }
        resolve(&core, &mut effects, loaded("", 0, 205));
        assert_eq!(core.view().vocabulary.matching_count, Some(205));
        assert_eq!(core.view().vocabulary.rows.len(), 100);
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        for offset in [100, 200] {
            let mut effects = core.process_event(Event::LoadMoreVocabulary);
            assert_eq!(operation(&effects), Some(&request(query(""), offset)));
            assert_eq!(core.view().vocabulary.rows.len() as u64, offset);
            resolve(&core, &mut effects, loaded("", offset, 205));
        }
        let state = core.view().vocabulary;
        assert_eq!(state.rows.len(), 205);
        assert_eq!(state.next_offset, None);
        assert!(core.process_event(Event::LoadMoreVocabulary).is_empty());
        for event in [Event::OpenSettings, Event::CloseSettings] {
            core.process_event(event);
            assert_eq!(core.view().vocabulary, state);
        }
        let mut effects = core.process_event(Event::SetVocabularyQuery(query("absent")));
        assert!(core.view().vocabulary.rows.is_empty());
        assert_eq!(core.view().vocabulary.matching_count, None);
        resolve(&core, &mut effects, loaded("absent", 0, 0));
        assert_eq!(core.view().vocabulary.matching_count, Some(0));
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        assert!(core.process_event(Event::LoadMoreVocabulary).is_empty());
    }
}

#[test]
fn first_and_append_failures_require_explicit_exact_retry_and_keep_document_errors() {
    for core in cores() {
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        resolve(
            &core,
            &mut unrelated,
            StorageResult::Failed(DocumentError::NotFound),
        );
        enter(&core);
        let mut effects = core.process_event(Event::LoadVocabulary);
        resolve(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().vocabulary.matching_count, None);
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        let mut retry = core.process_event(Event::RetryVocabulary);
        assert_eq!(operation(&retry), Some(&request(query(""), 0)));
        resolve(&core, &mut retry, loaded("", 0, 205));
        let rows = core.view().vocabulary.rows;
        let mut effects = core.process_event(Event::LoadMoreVocabulary);
        resolve(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::InvalidDocument),
        );
        assert_eq!(core.view().vocabulary.rows, rows);
        assert_eq!(core.view().vocabulary.next_offset, Some(100));
        for event in [
            Event::LoadVocabulary,
            Event::LoadMoreVocabulary,
            Event::SetVocabularyQuery(query("")),
        ] {
            assert!(core.process_event(event).is_empty());
        }
        core.process_event(Event::OpenSettings);
        assert!(core.process_event(Event::RetryVocabulary).is_empty());
        core.process_event(Event::CloseSettings);
        let mut retry = core.process_event(Event::RetryVocabulary);
        assert_eq!(operation(&retry), Some(&request(query(""), 100)));
        resolve(&core, &mut retry, loaded("", 100, 205));
        assert_eq!(core.view().vocabulary.rows.len(), 200);
        assert_eq!(core.view().vocabulary.error, None);
        assert_eq!(core.view().document_error, Some(DocumentError::NotFound));
    }
}

#[test]
fn coalesced_queries_discard_old_results_even_when_query_returns_to_original() {
    for core in cores() {
        enter(&core);
        let mut first = core.process_event(Event::SetVocabularyQuery(query("猫")));
        for prefix in ["犬", "𠮷\0_%", "猫"] {
            let effects = core.process_event(Event::SetVocabularyQuery(query(prefix)));
            assert!(operation(&effects).is_none());
            assert_eq!(core.view().vocabulary.query.prefix, prefix);
        }
        let mut next = resolve(&core, &mut first, loaded("猫", 0, 5));
        assert_eq!(operation(&next), Some(&request(query("猫"), 0)));
        assert!(core.view().vocabulary.rows.is_empty());
        core.process_event(Event::SetVocabularyQuery(query("犬")));
        let mut last = resolve(
            &core,
            &mut next,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(operation(&last), Some(&request(query("犬"), 0)));
        assert_eq!(core.view().vocabulary.error, None);
        resolve(&core, &mut last, loaded("犬", 0, 2));
        assert!(core
            .view()
            .vocabulary
            .rows
            .iter()
            .all(|row| row.surface.starts_with("犬")));
    }
}

#[test]
fn filters_are_sets_all_sorts_and_literal_prefix_survive_navigation() {
    for core in cores() {
        enter(&core);
        for sort in [
            VocabularySort::Newest,
            VocabularySort::Frequency,
            VocabularySort::LastSeen,
            VocabularySort::Alphabetical,
            VocabularySort::Oldest,
            VocabularySort::FrequencyAscending,
            VocabularySort::LastSeenAscending,
            VocabularySort::AlphabeticalDescending,
        ] {
            let q = VocabularyQuery {
                prefix: " 𠮷e\u{301}\0_%".into(),
                statuses: vec![
                    VocabularyStatus::Ignored,
                    VocabularyStatus::Known,
                    VocabularyStatus::Ignored,
                ],
                sort,
            };
            let mut effects = core.process_event(Event::SetVocabularyQuery(q.clone()));
            let canonical = q.clone().canonicalized();
            assert_eq!(operation(&effects), Some(&request(canonical.clone(), 0)));
            resolve(&core, &mut effects, loaded(&q.prefix, 0, 0));
            assert!(core.process_event(Event::SetVocabularyQuery(q)).is_empty());
            core.process_event(Event::SelectDestination(Destination::Library));
            enter(&core);
            assert_eq!(core.view().vocabulary.query, canonical);
            assert_eq!(core.view().vocabulary.matching_count, None);
            let mut effects = core.process_event(Event::LoadVocabulary);
            assert_eq!(operation(&effects), Some(&request(canonical, 0)));
            resolve(&core, &mut effects, loaded("", 0, 0));
        }
        let mut effects = core.process_event(Event::SetVocabularyQuery(VocabularyQuery {
            statuses: vec![],
            ..query("")
        }));
        resolve(&core, &mut effects, loaded("", 0, 0));
        assert!(core.view().vocabulary.query.statuses.is_empty());
    }
}

#[test]
fn stale_callbacks_cannot_release_new_requests_or_accept_generic_storage_results() {
    for core in cores() {
        enter(&core);
        let mut first = core.process_event(Event::LoadVocabulary);
        resolve(&core, &mut first, loaded("", 0, 205));
        let mut next = core.process_event(Event::LoadMoreVocabulary);
        for event in [
            Event::VocabularyFinished(1, loaded("", 0, 205)),
            Event::StorageFinished(StorageResult::Failed(DocumentError::Unavailable)),
        ] {
            assert!(core.process_event(event).is_empty());
            assert!(core.view().storage_busy && core.view().vocabulary.loading);
        }
        resolve(&core, &mut next, loaded("", 100, 205));
        let before = core.view();
        assert!(core
            .process_event(Event::VocabularyFinished(2, loaded("", 100, 205)))
            .is_empty());
        assert_eq!(core.view(), before);
    }
}

#[test]
fn unrelated_busy_work_is_preserved_and_hidden_coalesced_query_resumes_after_settings() {
    for core in cores() {
        enter(&core);
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        assert!(core
            .process_event(Event::SetVocabularyQuery(query("猫")))
            .is_empty());
        assert_eq!(core.view().vocabulary.query, query(""));
        resolve(
            &core,
            &mut unrelated,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        let mut first = core.process_event(Event::LoadVocabulary);
        core.process_event(Event::SetVocabularyQuery(query("犬")));
        core.process_event(Event::OpenSettings);
        let followup = resolve(&core, &mut first, loaded("", 0, 205));
        assert!(operation(&followup).is_none());
        assert!(!core.view().storage_busy && !core.view().vocabulary.loading);
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        core.process_event(Event::CloseSettings);
        let mut resumed = core.process_event(Event::LoadVocabulary);
        assert_eq!(operation(&resumed), Some(&request(query("犬"), 0)));
        resolve(&core, &mut resumed, loaded("犬", 0, 2));
        core.process_event(Event::SelectDestination(Destination::Library));
        assert!(core.process_event(Event::LoadMoreVocabulary).is_empty());
        enter(&core);
        let mut pending = core.process_event(Event::LoadVocabulary);
        core.process_event(Event::SelectDestination(Destination::Statistics));
        enter(&core);
        let mut refreshed = resolve(&core, &mut pending, loaded("犬", 0, 2));
        assert_eq!(operation(&refreshed), Some(&request(query("犬"), 0)));
        resolve(&core, &mut refreshed, loaded("犬", 0, 3));
        assert_eq!(core.view().vocabulary.matching_count, Some(3));
    }
}

#[test]
fn malformed_page_never_partially_appends_and_retry_keeps_exact_offset() {
    for core in cores() {
        enter(&core);
        let mut first = core.process_event(Event::LoadVocabulary);
        resolve(&core, &mut first, loaded("", 0, 205));
        let rows = core.view().vocabulary.rows;
        let mut next = core.process_event(Event::LoadMoreVocabulary);
        let StorageResult::VocabularyLoaded(mut page) = loaded("", 100, 205) else {
            unreachable!()
        };
        page.rows.last_mut().unwrap().id = rows[0].id;
        resolve(&core, &mut next, StorageResult::VocabularyLoaded(page));
        assert_eq!(core.view().vocabulary.rows, rows);
        assert_eq!(
            core.view().vocabulary.error,
            Some(DocumentError::InvalidDocument)
        );
        let mut retry = core.process_event(Event::RetryVocabulary);
        assert_eq!(operation(&retry), Some(&request(query(""), 100)));
        resolve(&core, &mut retry, loaded("", 100, 205));
        assert_eq!(core.view().vocabulary.rows.len(), 200);
    }
}

#[test]
fn changed_count_restarts_at_zero_and_import_invalidates_without_resetting_query() {
    for core in cores() {
        enter(&core);
        let mut first = core.process_event(Event::SetVocabularyQuery(query("猫")));
        resolve(&core, &mut first, loaded("猫", 0, 205));
        let mut more = core.process_event(Event::LoadMoreVocabulary);
        let mut restarted = resolve(&core, &mut more, loaded("猫", 100, 206));
        assert_eq!(operation(&restarted), Some(&request(query("猫"), 0)));
        assert!(core.view().vocabulary.rows.is_empty());
        resolve(&core, &mut restarted, loaded("猫", 0, 206));
        assert_eq!(core.view().vocabulary.rows.len(), 100);
        core.process_event(Event::OpenPaste);
        let mut imported = core.process_event(Event::CreatePaste("猫".into()));
        resolve(
            &core,
            &mut imported,
            StorageResult::Created(yomoku_core::document::LibraryDocument {
                document_id: 1.try_into().unwrap(),
                title: "猫".into(),
                progress_percent: 0,
                finished_at_least_once: false,
                last_read_sequence: None,
            }),
        );
        assert_eq!(core.view().vocabulary.query, query("猫"));
        assert_eq!(core.view().vocabulary.matching_count, None);
        assert!(core.view().vocabulary.rows.is_empty());
    }
}

fn all_ids(count: i64) -> Vec<yomoku_core::document::VocabularyId> {
    (1..=count).map(|id| id.try_into().unwrap()).collect()
}
fn ready(core: &Core<Yomoku>) {
    enter(core);
    let mut effects = core.process_event(Event::LoadVocabulary);
    resolve(core, &mut effects, loaded("", 0, 1205));
}
fn select_all(core: &Core<Yomoku>) {
    let mut effects = core.process_event(Event::SelectAllVocabulary);
    assert_eq!(
        operation(&effects),
        Some(&StorageOperation::VocabularyMatchingIds(query("")))
    );
    assert!(core.view().vocabulary.selecting_all && core.view().storage_busy);
    resolve(
        core,
        &mut effects,
        StorageResult::VocabularyMatchingIds(all_ids(1205)),
    );
}
fn prompt(
    core: &Core<Yomoku>,
    action: yomoku_core::vocabulary::VocabularyBulkAction,
) -> yomoku_core::vocabulary::VocabularyBulkCommand {
    core.process_event(Event::RequestVocabularyBulkAction(action));
    core.view().vocabulary.confirmation.unwrap()
}

#[test]
fn vocabulary_selection_captures_unloaded_matches_and_retains_hidden_ids_and_settings() {
    for core in cores() {
        assert!(core.process_event(Event::SelectAllVocabulary).is_empty());
        ready(&core);
        assert!(core
            .process_event(Event::ToggleVocabularySelection(9000.try_into().unwrap()))
            .is_empty());
        core.process_event(Event::ToggleVocabularySelection(1.try_into().unwrap()));
        core.process_event(Event::ToggleVocabularySelection(1.try_into().unwrap()));
        assert_eq!(core.view().vocabulary.selection, Some(vec![]));
        select_all(&core);
        assert_eq!(core.view().vocabulary.rows.len(), 100);
        assert_eq!(core.view().vocabulary.selection, Some(all_ids(1205)));
        let mut filtered = core.process_event(Event::SetVocabularyQuery(query("犬")));
        resolve(&core, &mut filtered, loaded("犬", 0, 1));
        core.process_event(Event::ToggleVocabularySelection(1.try_into().unwrap()));
        let mut all = core.process_event(Event::SelectAllVocabulary);
        resolve(
            &core,
            &mut all,
            StorageResult::VocabularyMatchingIds(vec![2000.try_into().unwrap()]),
        );
        let mut expected = all_ids(1205);
        expected.remove(0);
        expected.push(2000.try_into().unwrap());
        assert_eq!(core.view().vocabulary.selection, Some(expected));
        let retained = core.view().vocabulary;
        for event in [Event::OpenSettings, Event::CloseSettings] {
            core.process_event(event);
            assert_eq!(core.view().vocabulary, retained);
        }
        core.process_event(Event::CancelVocabularySelection);
        assert_eq!(core.view().vocabulary.selection, None);
        select_all_for_query(&core, query("犬"));
        core.process_event(Event::SelectDestination(Destination::Library));
        assert_eq!(core.view().vocabulary.selection, None);
    }
}
fn select_all_for_query(core: &Core<Yomoku>, query: VocabularyQuery) {
    let mut effects = core.process_event(Event::SelectAllVocabulary);
    assert_eq!(
        operation(&effects),
        Some(&StorageOperation::VocabularyMatchingIds(query))
    );
    resolve(
        core,
        &mut effects,
        StorageResult::VocabularyMatchingIds(all_ids(2)),
    );
}

#[test]
fn vocabulary_confirmation_cancel_and_exact_payload_gate_all_unrelated_work() {
    use yomoku_core::vocabulary::VocabularyBulkAction;
    for core in cores() {
        ready(&core);
        assert!(core
            .process_event(Event::RequestVocabularyBulkAction(
                VocabularyBulkAction::Delete
            ))
            .is_empty());
        select_all(&core);
        let command = prompt(&core, VocabularyBulkAction::Delete);
        let before = core.view();
        for event in [
            Event::SelectDestination(Destination::Library),
            Event::OpenSettings,
            Event::OpenPaste,
            Event::CreatePaste("猫".into()),
            Event::RefreshLibrary,
            Event::SetVocabularyQuery(query("犬")),
            Event::LoadMoreVocabulary,
            Event::ToggleVocabularySelection(1.try_into().unwrap()),
            Event::SelectAllVocabulary,
            Event::RequestVocabularyBulkAction(VocabularyBulkAction::SetStatus {
                status: VocabularyStatus::Known,
                occurred_at_unix_ms: 555,
            }),
            Event::CancelVocabularySelection,
            Event::RetryVocabularyBulkAction,
            Event::StorageFinished(StorageResult::Failed(DocumentError::NotFound)),
        ] {
            assert!(core.process_event(event).is_empty());
            assert_eq!(core.view(), before);
        }
        let mut altered = command.clone();
        altered.ids.pop();
        assert!(core
            .process_event(Event::ConfirmVocabularyBulkAction(altered))
            .is_empty());
        core.process_event(Event::CancelVocabularyBulkAction);
        assert!(core.view().vocabulary.confirmation.is_none());
        assert_eq!(core.view().vocabulary.selection, Some(command.ids.clone()));
        assert!(core
            .process_event(Event::ConfirmVocabularyBulkAction(command.clone()))
            .is_empty());
        let command = prompt(&core, VocabularyBulkAction::Delete);
        let mut effects = core.process_event(Event::ConfirmVocabularyBulkAction(command.clone()));
        assert_eq!(
            operation(&effects),
            Some(&StorageOperation::ApplyVocabularyBulk(command.clone()))
        );
        let pending = core.view();
        for event in [
            Event::CancelVocabularyBulkAction,
            Event::CancelVocabularySelection,
            Event::RetryVocabularyBulkAction,
            Event::ConfirmVocabularyBulkAction(command.clone()),
            Event::VocabularyBulkFinished(0, StorageResult::VocabularyBulkApplied(command.clone())),
        ] {
            assert!(core.process_event(event).is_empty());
            assert_eq!(core.view(), pending);
        }
        resolve(
            &core,
            &mut effects,
            StorageResult::VocabularyBulkApplied(command),
        );
        let finished = core.view();
        assert!(!finished.storage_busy);
        assert!(finished.vocabulary.rows.is_empty());
        assert_eq!(finished.vocabulary.matching_count, None);
        assert_eq!(finished.vocabulary.selection, None);
        assert_eq!(finished.vocabulary.confirmation, None);
        assert_eq!(finished.vocabulary.bulk_error, None);
        assert!(core
            .process_event(Event::VocabularyBulkFinished(
                2,
                StorageResult::Failed(DocumentError::Unavailable)
            ))
            .is_empty());
        assert_eq!(core.view(), finished);
        let mut refreshed = core.process_event(Event::LoadVocabulary);
        resolve(&core, &mut refreshed, loaded("", 0, 0));
        assert_eq!(core.view().vocabulary.matching_count, Some(0));
    }
}

#[test]
fn vocabulary_select_all_failure_keeps_exact_query_until_explicit_retry_or_cancel() {
    for core in cores() {
        ready(&core);
        core.process_event(Event::ToggleVocabularySelection(1.try_into().unwrap()));
        let mut effects = core.process_event(Event::SelectAllVocabulary);
        assert!(core
            .process_event(Event::CancelVocabularySelection)
            .is_empty());
        // Malformed native results cannot introduce duplicate/noncanonical IDs.
        resolve(
            &core,
            &mut effects,
            StorageResult::VocabularyMatchingIds(vec![
                2.try_into().unwrap(),
                1.try_into().unwrap(),
            ]),
        );
        assert_eq!(
            core.view().vocabulary.bulk_error,
            Some(DocumentError::InvalidDocument)
        );
        assert_eq!(core.view().vocabulary.selection, Some(all_ids(1)));
        for event in [
            Event::SelectAllVocabulary,
            Event::SetVocabularyQuery(query("犬")),
            Event::OpenSettings,
            Event::RefreshLibrary,
        ] {
            assert!(core.process_event(event).is_empty());
        }
        let mut retry = core.process_event(Event::RetryVocabularySelection);
        assert_eq!(
            operation(&retry),
            Some(&StorageOperation::VocabularyMatchingIds(query("")))
        );
        resolve(
            &core,
            &mut retry,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(
            core.view().vocabulary.bulk_error,
            Some(DocumentError::Unavailable)
        );
        core.process_event(Event::CancelVocabularySelection);
        assert_eq!(core.view().vocabulary.bulk_error, None);
        assert!(core
            .process_event(Event::RetryVocabularySelection)
            .is_empty());
        select_all(&core);
    }
}

#[test]
fn vocabulary_status_retries_freeze_ids_status_date_and_reject_wrong_success() {
    use yomoku_core::vocabulary::VocabularyBulkAction;
    for core in cores() {
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        resolve(
            &core,
            &mut unrelated,
            StorageResult::Failed(DocumentError::NotFound),
        );
        ready(&core);
        select_all(&core);
        assert!(core
            .process_event(Event::RequestVocabularyBulkAction(
                VocabularyBulkAction::SetStatus {
                    status: VocabularyStatus::Known,
                    occurred_at_unix_ms: -1
                }
            ))
            .is_empty());
        for status in VocabularyQuery::default().statuses {
            let command = prompt(
                &core,
                VocabularyBulkAction::SetStatus {
                    status,
                    occurred_at_unix_ms: 555,
                },
            );
            let mut effects =
                core.process_event(Event::ConfirmVocabularyBulkAction(command.clone()));
            resolve(
                &core,
                &mut effects,
                StorageResult::Failed(DocumentError::Unavailable),
            );
            let failed = core.view();
            assert_eq!(failed.vocabulary.confirmation, Some(command.clone()));
            assert_eq!(
                failed.vocabulary.bulk_error,
                Some(DocumentError::Unavailable)
            );
            assert_eq!(failed.document_error, Some(DocumentError::NotFound));
            assert!(core
                .process_event(Event::ConfirmVocabularyBulkAction(command.clone()))
                .is_empty());
            let mut retry = core.process_event(Event::RetryVocabularyBulkAction);
            assert_eq!(
                operation(&retry),
                Some(&StorageOperation::ApplyVocabularyBulk(command.clone()))
            );
            let mut altered = command.clone();
            altered.ids.pop();
            resolve(
                &core,
                &mut retry,
                StorageResult::VocabularyBulkApplied(altered),
            );
            assert_eq!(
                core.view().vocabulary.bulk_error,
                Some(DocumentError::InvalidDocument)
            );
            let mut retry = core.process_event(Event::RetryVocabularyBulkAction);
            assert_eq!(
                operation(&retry),
                Some(&StorageOperation::ApplyVocabularyBulk(command.clone()))
            );
            resolve(
                &core,
                &mut retry,
                StorageResult::Failed(DocumentError::Unavailable),
            );
            core.process_event(Event::CancelVocabularyBulkAction);
            assert_eq!(core.view().vocabulary.selection, Some(all_ids(1205)));
        }
    }
}
