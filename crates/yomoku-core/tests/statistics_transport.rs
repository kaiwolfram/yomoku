use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Model, Yomoku},
    statistics::{statistics_buckets, StatisticsQuery, StatisticsTimeframe as Timeframe},
    storage::{DocumentError, StorageOperation, StorageResult},
};

const NOW: i64 = 1_800_000_000_123;

fn cores() -> [Core<Yomoku>; 2] {
    [Core::new(), Core::new_with(Yomoku, Model::default())]
}

fn query(timeframe: Timeframe, as_of_unix_ms: i64) -> StatisticsQuery {
    StatisticsQuery {
        timeframe,
        as_of_unix_ms,
    }
}

fn resolve(core: &Core<Yomoku>, effects: &mut [Effect], result: StorageResult) -> Vec<Effect> {
    let request = effects
        .iter_mut()
        .find_map(|effect| match effect {
            Effect::Storage(request)
                if !matches!(
                    request.operation,
                    StorageOperation::SaveStatisticsTimeframe(_)
                        | StorageOperation::LoadStatisticsTimeframe
                ) =>
            {
                Some(request)
            }
            _ => None,
        })
        .expect("storage request");
    core.resolve(request, result).unwrap()
}

fn loaded(query: StatisticsQuery) -> StorageResult {
    StorageResult::StatisticsLoaded(Box::new(
        statistics_buckets(query.timeframe, query.as_of_unix_ms, &[], &[]).unwrap(),
    ))
}

fn operation(effects: &[Effect]) -> Option<&StorageOperation> {
    effects.iter().find_map(|effect| match effect {
        Effect::Storage(request)
            if !matches!(
                request.operation,
                StorageOperation::SaveStatisticsTimeframe(_)
                    | StorageOperation::LoadStatisticsTimeframe
            ) =>
        {
            Some(&request.operation)
        }
        _ => None,
    })
}

fn enter(core: &Core<Yomoku>) {
    core.process_event(Event::SelectDestination(Destination::Statistics));
}

#[test]
fn default_week_is_lazy_retained_and_identical_loads_never_duplicate() {
    for core in cores() {
        assert_eq!(core.view().statistics.timeframe, Timeframe::LastWeek);
        assert_eq!(core.view().statistics.data, None);
        assert!(core
            .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
            .is_empty());
        enter(&core);
        let q = query(Timeframe::LastWeek, NOW);
        let mut effects = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        assert_eq!(
            operation(&effects),
            Some(&StorageOperation::LoadStatistics(q))
        );
        assert!(core.view().statistics.loading && core.view().storage_busy);
        assert!(core
            .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
            .is_empty());
        assert!(core.process_event(Event::RetryStatistics).is_empty());
        resolve(&core, &mut effects, loaded(q));
        let state = core.view().statistics;
        assert_eq!(
            state.data,
            Some(statistics_buckets(q.timeframe, NOW, &[], &[]).unwrap())
        );
        assert!(!state.loading && !core.view().storage_busy);
        for event in [Event::OpenSettings, Event::CloseSettings] {
            core.process_event(event);
            assert_eq!(core.view().statistics, state);
        }
        assert!(core
            .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
            .is_empty());
    }
}

#[test]
fn failures_require_explicit_retry_and_freeze_the_original_cutoff() {
    for core in cores() {
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        resolve(
            &core,
            &mut unrelated,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        enter(&core);
        let q = query(Timeframe::LastWeek, NOW);
        let mut effects = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        resolve(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::InvalidDocument),
        );
        assert_eq!(
            core.view().statistics.error,
            Some(DocumentError::InvalidDocument)
        );
        assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
        assert!(core
            .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
            .is_empty());
        let mut retry = core.process_event(Event::RetryStatistics);
        assert_eq!(
            operation(&retry),
            Some(&StorageOperation::LoadStatistics(q))
        );
        assert_eq!(core.view().statistics.error, None);
        resolve(&core, &mut retry, loaded(q));
        assert!(core.view().statistics.data.is_some());
        assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    }
}

#[test]
fn rapid_changes_coalesce_and_discard_old_successes_and_failures() {
    for core in cores() {
        enter(&core);
        let first = query(Timeframe::LastWeek, NOW);
        let mut effects = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        for timeframe in [
            Timeframe::LastMonth,
            Timeframe::LastQuarter,
            Timeframe::AllTime,
        ] {
            let changed = core.process_event(Event::SetStatisticsTimeframe {
                timeframe,
                as_of_unix_ms: NOW + 1,
            });
            assert!(operation(&changed).is_none());
            assert_eq!(core.view().statistics.timeframe, timeframe);
            assert!(core.view().statistics.loading);
        }
        let latest = query(Timeframe::AllTime, NOW + 1);
        let mut next = resolve(&core, &mut effects, loaded(first));
        assert_eq!(
            operation(&next),
            Some(&StorageOperation::LoadStatistics(latest))
        );
        assert!(core.view().statistics.data.is_none());
        core.process_event(Event::SetStatisticsTimeframe {
            timeframe: Timeframe::LastMonth,
            as_of_unix_ms: NOW + 2,
        });
        let mut final_read = resolve(
            &core,
            &mut next,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().statistics.error, None);
        let final_query = query(Timeframe::LastMonth, NOW + 2);
        assert_eq!(
            operation(&final_read),
            Some(&StorageOperation::LoadStatistics(final_query))
        );
        resolve(&core, &mut final_read, loaded(final_query));
        assert_eq!(core.view().statistics.query, Some(final_query));
        assert_eq!(
            core.view().statistics.data.unwrap().timeframe,
            Timeframe::LastMonth
        );
    }
}

#[test]
fn stale_callback_cannot_release_new_request_even_with_identical_query() {
    for core in cores() {
        enter(&core);
        let q = query(Timeframe::LastWeek, NOW);
        let mut first = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        resolve(&core, &mut first, loaded(q));
        core.process_event(Event::SelectDestination(Destination::Library));
        enter(&core);
        let mut second = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        assert!(core
            .process_event(Event::StatisticsFinished(1, loaded(q)))
            .is_empty());
        assert!(core
            .process_event(Event::StorageFinished(
                0,
                StorageResult::Failed(DocumentError::Unavailable)
            ))
            .is_empty());
        assert!(core.view().storage_busy && core.view().statistics.loading);
        resolve(&core, &mut second, loaded(q));
        let expected = core.view();
        assert!(core
            .process_event(Event::StatisticsFinished(
                2,
                StorageResult::Failed(DocumentError::Unavailable)
            ))
            .is_empty());
        assert_eq!(core.view(), expected);
    }
}

#[test]
fn hidden_requests_and_unrelated_busy_work_are_not_replaced() {
    for core in cores() {
        enter(&core);
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        assert!(core
            .process_event(Event::SetStatisticsTimeframe {
                timeframe: Timeframe::AllTime,
                as_of_unix_ms: NOW
            })
            .is_empty());
        assert_eq!(core.view().statistics.timeframe, Timeframe::LastWeek);
        resolve(
            &core,
            &mut unrelated,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        for event in [
            Event::OpenSettings,
            Event::SelectDestination(Destination::Vocabulary),
            Event::OpenPaste,
        ] {
            core.process_event(event);
            assert!(core
                .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
                .is_empty());
            assert!(core.process_event(Event::RetryStatistics).is_empty());
            assert!(!core.view().storage_busy);
        }
    }
}

#[test]
fn return_refreshes_external_mutations_and_invalidates_an_inflight_old_cutoff() {
    for core in cores() {
        enter(&core);
        let mut first = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        core.process_event(Event::SelectDestination(Destination::Vocabulary));
        enter(&core);
        assert_eq!(core.view().statistics.query, None);
        assert_eq!(core.view().statistics.data, None);
        let changed = core.process_event(Event::LoadStatistics {
            as_of_unix_ms: NOW + 100,
        });
        assert!(operation(&changed).is_none());
        let mut next = resolve(&core, &mut first, loaded(query(Timeframe::LastWeek, NOW)));
        let q = query(Timeframe::LastWeek, NOW + 100);
        assert_eq!(operation(&next), Some(&StorageOperation::LoadStatistics(q)));
        resolve(&core, &mut next, loaded(q));
        core.process_event(Event::SelectDestination(Destination::Library));
        enter(&core);
        assert_eq!(core.view().statistics.data, None);
        assert_eq!(core.view().statistics.timeframe, Timeframe::LastWeek);
    }
}

#[test]
fn changed_query_rejects_mismatched_results_and_does_not_launch_hidden_followup() {
    for core in cores() {
        enter(&core);
        let mut effects = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        resolve(
            &core,
            &mut effects,
            loaded(query(Timeframe::LastMonth, NOW)),
        );
        assert_eq!(
            core.view().statistics.error,
            Some(DocumentError::InvalidDocument)
        );
        assert_eq!(core.view().statistics.data, None);
        let mut effects = core.process_event(Event::RetryStatistics);
        core.process_event(Event::SetStatisticsTimeframe {
            timeframe: Timeframe::AllTime,
            as_of_unix_ms: NOW,
        });
        core.process_event(Event::SelectDestination(Destination::Library));
        let next = resolve(&core, &mut effects, loaded(query(Timeframe::LastWeek, NOW)));
        assert!(operation(&next).is_none());
        assert!(!core.view().statistics.loading && !core.view().storage_busy);
        assert_eq!(core.view().statistics.data, None);
    }
}

#[test]
fn committed_import_invalidates_retained_statistics_without_resetting_timeframe() {
    for core in cores() {
        enter(&core);
        let q = query(Timeframe::AllTime, NOW);
        let mut effects = core.process_event(Event::SetStatisticsTimeframe {
            timeframe: q.timeframe,
            as_of_unix_ms: NOW,
        });
        resolve(&core, &mut effects, loaded(q));
        core.process_event(Event::OpenPaste);
        assert!(core.view().statistics.data.is_some());
        let mut effects = core.process_event(Event::CreatePaste("猫".into()));
        resolve(
            &core,
            &mut effects,
            StorageResult::Created(yomoku_core::document::LibraryDocument {
                document_id: 1.try_into().unwrap(),
                title: "猫".into(),
                progress_percent: 0,
                is_read: false,
                last_read_sequence: None,
            }),
        );
        let state = core.view().statistics;
        assert_eq!(state.timeframe, Timeframe::AllTime);
        assert_eq!(state.query, None);
        assert_eq!(state.data, None);
    }
}

#[test]
fn settings_close_can_resume_an_unattempted_coalesced_query() {
    for core in cores() {
        enter(&core);
        let mut effects = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        core.process_event(Event::SetStatisticsTimeframe {
            timeframe: Timeframe::LastMonth,
            as_of_unix_ms: NOW,
        });
        core.process_event(Event::OpenSettings);
        let next = resolve(&core, &mut effects, loaded(query(Timeframe::LastWeek, NOW)));
        assert!(operation(&next).is_none());
        assert!(!core.view().storage_busy);
        assert!(core
            .process_event(Event::LoadStatistics { as_of_unix_ms: NOW })
            .is_empty());
        core.process_event(Event::CloseSettings);
        let mut resumed = core.process_event(Event::LoadStatistics { as_of_unix_ms: NOW });
        let q = query(Timeframe::LastMonth, NOW);
        assert_eq!(
            operation(&resumed),
            Some(&StorageOperation::LoadStatistics(q))
        );
        resolve(&core, &mut resumed, loaded(q));
        assert!(core.view().statistics.data.is_some());
    }
}
