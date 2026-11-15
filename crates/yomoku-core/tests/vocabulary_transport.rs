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
            Event::StorageFinished(0, StorageResult::Failed(DocumentError::Unavailable)),
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

fn ready(core: &Core<Yomoku>) {
    enter(core);
    let mut effects = core.process_event(Event::LoadVocabulary);
    resolve(core, &mut effects, loaded("", 0, 1205));
}
fn word_details(core: &Core<Yomoku>) -> StorageResult {
    let row = core.view().vocabulary.word.unwrap().row;
    StorageResult::VocabularyWordLoaded(Box::new(yomoku_core::vocabulary::VocabularyWordDetails {
        candidates: yomoku_core::dictionary::DictionaryCandidates {
            surface: row.surface.clone(),
            dictionary_form: None,
            words: vec![],
            names: vec![],
        },
        row,
    }))
}

#[test]
fn word_lookup_is_read_only_and_current_status_dismisses_without_storage() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready(&core);
        let before = core.view().vocabulary;
        let row = before.rows[0].clone();
        let mut effects = core.process_event(Event::OpenVocabularyWord(row.id));
        assert_eq!(
            operation(&effects),
            Some(&StorageOperation::LoadVocabularyWord(row.id))
        );
        assert_eq!(core.view().vocabulary.word.as_ref().unwrap().row, row);
        let details = word_details(&core);
        resolve(&core, &mut effects, details);
        let loaded = core.view().vocabulary;
        assert_eq!(loaded.rows, before.rows);
        assert!(loaded.word.as_ref().unwrap().candidates.is_some());
        let effects = core.process_event(Event::SetVocabularyWordStatus(
            VocabularyWordStatusCommand {
                id: row.id,
                status: row.status,
                occurred_at_unix_ms: 999,
            },
        ));
        assert!(operation(&effects).is_none());
        assert!(core.view().vocabulary.word.is_none());
        assert_eq!(core.view().vocabulary.rows, before.rows);
    }
}

#[test]
fn dismissed_and_departed_word_reads_release_only_their_slot_without_reopening() {
    for exit in [
        Event::DismissVocabularyWord,
        Event::SelectDestination(Destination::Library),
        Event::OpenSettings,
    ] {
        for core in cores() {
            ready(&core);
            let id = core.view().vocabulary.rows[0].id;
            let mut effects = core.process_event(Event::OpenVocabularyWord(id));
            let details = word_details(&core);
            core.process_event(exit.clone());
            assert!(core.view().vocabulary.word.is_none());
            assert!(core.view().storage_busy);
            let before = core.view();
            core.process_event(Event::VocabularyWordFinished(
                0,
                StorageResult::Failed(DocumentError::NotFound),
            ));
            core.process_event(Event::StorageFinished(
                0,
                StorageResult::Failed(DocumentError::NotFound),
            ));
            assert_eq!(core.view(), before);
            resolve(&core, &mut effects, details);
            assert!(!core.view().storage_busy);
            assert!(core.view().vocabulary.word.is_none());
        }
    }
}

#[test]
fn word_lookup_failures_only_retry_explicitly_and_validate_identity() {
    for core in cores() {
        ready(&core);
        let id = core.view().vocabulary.rows[0].id;
        let mut effects = core.process_event(Event::OpenVocabularyWord(id));
        let mut wrong = match word_details(&core) {
            StorageResult::VocabularyWordLoaded(details) => details,
            _ => unreachable!(),
        };
        wrong.row.id = 999.try_into().unwrap();
        resolve(
            &core,
            &mut effects,
            StorageResult::VocabularyWordLoaded(wrong),
        );
        assert_eq!(
            core.view().vocabulary.word.as_ref().unwrap().error,
            Some(DocumentError::InvalidDocument)
        );
        assert!(core.process_event(Event::LoadVocabulary).is_empty());
        let mut retry = core.process_event(Event::RetryVocabularyWord);
        resolve(
            &core,
            &mut retry,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        let mut retry = core.process_event(Event::RetryVocabularyWord);
        let details = word_details(&core);
        resolve(&core, &mut retry, details);
        assert!(core.view().vocabulary.word.unwrap().candidates.is_some());
        assert!(core.process_event(Event::RetryVocabularyWord).is_empty());
    }
}

#[test]
fn direct_status_retains_exact_retry_after_dismissal_and_patches_results_on_success() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready(&core);
        let row = core.view().vocabulary.rows[0].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let command = VocabularyWordStatusCommand {
            id: row.id,
            status: VocabularyStatus::Learning,
            occurred_at_unix_ms: 12345,
        };
        let mut change = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        assert_eq!(
            operation(&change),
            Some(&StorageOperation::SetVocabularyWordStatus(command.clone()))
        );
        resolve(
            &core,
            &mut change,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().vocabulary.word_status, Some(command.clone()));
        assert_eq!(
            core.view().vocabulary.word.as_ref().unwrap().row.status,
            row.status
        );
        assert!(core
            .process_event(Event::SetVocabularyWordStatus(
                VocabularyWordStatusCommand {
                    status: VocabularyStatus::Known,
                    occurred_at_unix_ms: 99999,
                    ..command.clone()
                }
            ))
            .is_empty());
        core.process_event(Event::DismissVocabularyWord);
        let mut retry = core.process_event(Event::RetryVocabularyWordStatus);
        assert_eq!(
            operation(&retry),
            Some(&StorageOperation::SetVocabularyWordStatus(command.clone()))
        );
        resolve(
            &core,
            &mut retry,
            StorageResult::VocabularyWordStatusChanged(command),
        );
        let view = core.view();
        assert!(!view.storage_busy);
        assert!(view.vocabulary.word.is_none());
        assert!(
            view.vocabulary.word_status.is_none() && view.vocabulary.word_status_error.is_none()
        );
        assert_eq!(view.vocabulary.rows.len(), 100);
        assert_eq!(view.vocabulary.matching_count, Some(1205));
        assert_eq!(view.vocabulary.rows[0].status, VocabularyStatus::Learning);
    }
}

#[test]
fn direct_status_keeps_word_open_after_filter_refresh_and_supports_cancel() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        enter(&core);
        let query = VocabularyQuery {
            statuses: vec![VocabularyStatus::New],
            ..Default::default()
        };
        let mut page = core.process_event(Event::SetVocabularyQuery(query));
        resolve(&core, &mut page, loaded("", 0, 1));
        let row = core.view().vocabulary.rows[0].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let command = VocabularyWordStatusCommand {
            id: row.id,
            status: VocabularyStatus::Ignored,
            occurred_at_unix_ms: 999,
        };
        let mut change = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        resolve(
            &core,
            &mut change,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        core.process_event(Event::CancelVocabularyWordStatus);
        assert!(core.view().vocabulary.word_status.is_none());
        assert!(core
            .process_event(Event::RetryVocabularyWordStatus)
            .is_empty());
        let mut change = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        let mut refresh = resolve(
            &core,
            &mut change,
            StorageResult::VocabularyWordStatusChanged(command),
        );
        assert_eq!(core.view().vocabulary.rows, vec![row.clone()]);
        assert_eq!(core.view().vocabulary.matching_count, Some(1));
        resolve(&core, &mut refresh, loaded("", 0, 0));
        assert_eq!(core.view().vocabulary.matching_count, Some(0));
        let word = core.view().vocabulary.word.unwrap();
        assert_eq!(word.row.id, row.id);
        assert_eq!(word.row.status, VocabularyStatus::Ignored);
        assert!(word.candidates.is_some());
        core.process_event(Event::DismissVocabularyWord);
        assert!(core.view().vocabulary.word.is_none());
    }
}

#[test]
fn stale_word_callbacks_cannot_replace_a_new_lookup_or_acknowledge_another_status() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready(&core); // Page request 1, first lookup 2.
        let id = core.view().vocabulary.rows[0].id;
        let mut first = core.process_event(Event::OpenVocabularyWord(id));
        let old_details = word_details(&core);
        core.process_event(Event::DismissVocabularyWord);
        assert!(core.process_event(Event::OpenVocabularyWord(id)).is_empty());
        resolve(&core, &mut first, old_details.clone());
        let mut current = core.process_event(Event::OpenVocabularyWord(id));
        let pending = core.view();
        core.process_event(Event::VocabularyWordFinished(2, old_details));
        assert_eq!(core.view(), pending);
        let details = word_details(&core);
        resolve(&core, &mut current, details);
        let command = VocabularyWordStatusCommand {
            id,
            status: VocabularyStatus::Known,
            occurred_at_unix_ms: 456,
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        let mut wrong = command.clone();
        wrong.occurred_at_unix_ms += 1;
        resolve(
            &core,
            &mut write,
            StorageResult::VocabularyWordStatusChanged(wrong),
        );
        assert_eq!(core.view().vocabulary.word_status, Some(command.clone()));
        assert_eq!(
            core.view().vocabulary.word_status_error,
            Some(DocumentError::InvalidDocument)
        );
        let retry = core.process_event(Event::RetryVocabularyWordStatus);
        assert_eq!(
            operation(&retry),
            Some(&StorageOperation::SetVocabularyWordStatus(command))
        );
    }
}

fn ready_prefix(core: &Core<Yomoku>, filtered: bool) {
    enter(core);
    let mut q = query("");
    if filtered {
        q.statuses = vec![VocabularyStatus::New];
    }
    let mut effects = core.process_event(Event::SetVocabularyQuery(q));
    if operation(&effects).is_none() {
        effects = core.process_event(Event::LoadVocabulary);
    }
    resolve(core, &mut effects, loaded("", 0, 305));
    for offset in [100, 200] {
        let mut more = core.process_event(Event::LoadMoreVocabulary);
        resolve(core, &mut more, loaded("", offset, 305));
    }
}

fn dataset_page(rows: &[VocabularyRow], offset: usize) -> StorageResult {
    let end = (offset + 100).min(rows.len());
    StorageResult::VocabularyLoaded(VocabularyPage {
        matching_count: rows.len() as u64,
        offset: offset as u64,
        rows: rows[offset..end].to_vec(),
        next_offset: (end < rows.len()).then_some(end as u64),
    })
}

#[test]
fn status_changes_beyond_page_one_patch_only_the_row_without_reads_or_pagination_changes() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready_prefix(&core, false);
        let original = core.view().vocabulary;
        let row = original.rows[150].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let candidates = core.view().vocabulary.word.unwrap().candidates;
        for (index, status) in [
            VocabularyStatus::Learning,
            VocabularyStatus::Known,
            VocabularyStatus::New,
        ]
        .into_iter()
        .enumerate()
        {
            let command = VocabularyWordStatusCommand {
                id: row.id,
                status,
                occurred_at_unix_ms: 1000 + index as i64,
            };
            let mut write = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
            let effects = resolve(
                &core,
                &mut write,
                StorageResult::VocabularyWordStatusChanged(command),
            );
            assert!(operation(&effects).is_none());
            assert!(core.process_event(Event::LoadVocabulary).is_empty());
            let state = core.view().vocabulary;
            let mut expected = original.rows.clone();
            expected[150].status = status;
            assert_eq!(state.rows, expected);
            assert_eq!(state.matching_count, original.matching_count);
            assert_eq!(state.next_offset, Some(300));
            assert_eq!(state.word.unwrap().candidates, candidates);
        }
    }
}

#[test]
fn filter_removal_and_reentry_replace_entire_loaded_prefix_atomically_with_retry_and_dismissal() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready_prefix(&core, true);
        let original = core.view().vocabulary;
        let row = original.rows[150].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let candidates = core.view().vocabulary.word.unwrap().candidates;
        let command = VocabularyWordStatusCommand {
            id: row.id,
            status: VocabularyStatus::Learning,
            occurred_at_unix_ms: 999,
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        resolve(
            &core,
            &mut write,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().vocabulary.rows, original.rows);
        let mut write = core.process_event(Event::RetryVocabularyWordStatus);
        let mut refresh = resolve(
            &core,
            &mut write,
            StorageResult::VocabularyWordStatusChanged(command.clone()),
        );
        let StorageResult::VocabularyLoaded(all) = loaded("", 0, 305) else {
            unreachable!()
        };
        let mut dataset = all.rows;
        for offset in [100, 200, 300] {
            let StorageResult::VocabularyLoaded(page) = loaded("", offset, 305) else {
                unreachable!()
            };
            dataset.extend(page.rows);
        }
        let removed: Vec<_> = dataset
            .iter()
            .filter(|item| item.id != row.id)
            .cloned()
            .collect();
        assert_eq!(
            operation(&refresh),
            Some(&request(original.query.clone(), 0))
        );
        refresh = resolve(&core, &mut refresh, dataset_page(&removed, 0));
        assert_eq!(core.view().vocabulary.rows, original.rows);
        assert_eq!(core.view().vocabulary.matching_count, Some(305));
        assert_eq!(core.view().vocabulary.next_offset, Some(300));
        resolve(
            &core,
            &mut refresh,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().vocabulary.rows, original.rows);
        assert!(core.process_event(Event::LoadMoreVocabulary).is_empty());
        refresh = core.process_event(Event::RetryVocabulary);
        assert_eq!(
            operation(&refresh),
            Some(&request(original.query.clone(), 100))
        );
        refresh = resolve(&core, &mut refresh, dataset_page(&removed, 100));
        assert_eq!(core.view().vocabulary.rows, original.rows);
        resolve(&core, &mut refresh, dataset_page(&removed, 200));
        let state = core.view().vocabulary;
        assert_eq!(state.rows, removed[..300]);
        assert_eq!(state.matching_count, Some(304));
        assert_eq!(state.next_offset, Some(300));
        assert_eq!(
            state.word.as_ref().unwrap().row.status,
            VocabularyStatus::Learning
        );
        assert_eq!(state.word.unwrap().candidates, candidates);
        // The still-open sheet can return the removed word to the same filter.
        let reentry = VocabularyWordStatusCommand {
            status: VocabularyStatus::New,
            occurred_at_unix_ms: 1000,
            ..command
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(reentry.clone()));
        let mut refresh = resolve(
            &core,
            &mut write,
            StorageResult::VocabularyWordStatusChanged(reentry),
        );
        core.process_event(Event::DismissVocabularyWord);
        for offset in [0, 100, 200] {
            assert_eq!(
                operation(&refresh),
                Some(&request(original.query.clone(), offset))
            );
            refresh = resolve(&core, &mut refresh, dataset_page(&dataset, offset as usize));
        }
        assert!(operation(&refresh).is_none());
        assert_eq!(core.view().vocabulary.rows, original.rows);
        assert_eq!(core.view().vocabulary.matching_count, Some(305));
        assert!(core.view().vocabulary.word.is_none());
        let mut more = core.process_event(Event::LoadMoreVocabulary);
        assert_eq!(operation(&more), Some(&request(original.query, 300)));
        resolve(&core, &mut more, dataset_page(&dataset, 300));
        assert_eq!(core.view().vocabulary.rows, dataset);
        assert_eq!(core.view().vocabulary.next_offset, None);
    }
}

#[test]
fn an_obsolete_prefix_refresh_cannot_publish_after_query_change_or_reentry() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready_prefix(&core, true);
        let row = core.view().vocabulary.rows[150].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let command = VocabularyWordStatusCommand {
            id: row.id,
            status: VocabularyStatus::Known,
            occurred_at_unix_ms: 999,
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        let mut refresh = resolve(
            &core,
            &mut write,
            StorageResult::VocabularyWordStatusChanged(command),
        );
        core.process_event(Event::DismissVocabularyWord);
        core.process_event(Event::SetVocabularyQuery(query("犬")));
        let mut fresh = resolve(&core, &mut refresh, loaded("", 0, 304));
        assert_eq!(operation(&fresh), Some(&request(query("犬"), 0)));
        assert!(core.view().vocabulary.rows.is_empty());
        resolve(&core, &mut fresh, loaded("犬", 0, 1));
        assert_eq!(core.view().vocabulary.rows[0].surface, "犬猫0");
    }
}

#[test]
fn reentry_after_failed_prefix_refresh_uses_written_status_instead_of_retained_row_membership() {
    use yomoku_core::vocabulary::VocabularyWordStatusCommand;
    for core in cores() {
        ready_prefix(&core, true);
        let original = core.view().vocabulary;
        let row = original.rows[150].clone();
        let mut lookup = core.process_event(Event::OpenVocabularyWord(row.id));
        let details = word_details(&core);
        resolve(&core, &mut lookup, details);
        let command = VocabularyWordStatusCommand {
            id: row.id,
            status: VocabularyStatus::Learning,
            occurred_at_unix_ms: 999,
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(command.clone()));
        let mut refresh = resolve(
            &core,
            &mut write,
            StorageResult::VocabularyWordStatusChanged(command.clone()),
        );
        refresh = resolve(&core, &mut refresh, loaded("", 0, 304));
        resolve(
            &core,
            &mut refresh,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().vocabulary.rows, original.rows);
        let reentry = VocabularyWordStatusCommand {
            status: VocabularyStatus::New,
            occurred_at_unix_ms: 1000,
            ..command
        };
        let mut write = core.process_event(Event::SetVocabularyWordStatus(reentry.clone()));
        // Retry after sheet dismissal must retain the source membership as well.
        resolve(
            &core,
            &mut write,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        core.process_event(Event::DismissVocabularyWord);
        let mut retry = core.process_event(Event::RetryVocabularyWordStatus);
        refresh = resolve(
            &core,
            &mut retry,
            StorageResult::VocabularyWordStatusChanged(reentry),
        );
        assert_eq!(
            operation(&refresh),
            Some(&request(original.query.clone(), 0))
        );
        for offset in [0, 100, 200] {
            refresh = resolve(&core, &mut refresh, loaded("", offset, 305));
        }
        assert_eq!(core.view().vocabulary.rows, original.rows);
        assert_eq!(core.view().vocabulary.matching_count, Some(305));
        assert_eq!(core.view().vocabulary.next_offset, Some(300));
        assert!(core.view().vocabulary.error.is_none());
        assert!(core.view().vocabulary.word.is_none());
    }
}
