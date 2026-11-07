//! Coalesced preference writes use an independent slot on the existing storage FIFO.
use super::*;

impl Yomoku {
    pub(super) fn update_statistics_preference(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::LoadStatisticsTimeframe => {
                if model.statistics_preference_loaded || model.statistics_preference_error.is_some()
                {
                    return Some(Command::done());
                }
                Some(self.dispatch_statistics_preference(model))
            }
            Event::RetryStatistics
                if model.statistics_visible() && model.statistics_preference_error.is_some() =>
            {
                model.statistics_preference_error = None;
                let save = self.dispatch_statistics_preference(model);
                let query = self
                    .update_statistics(event, model)
                    .unwrap_or_else(Command::done);
                Some(save.and(query))
            }
            Event::StatisticsTimeframeFinished(id, result) => {
                let Some((pending_id, requested)) = model.statistics_preference_request else {
                    return Some(Command::done());
                };
                if *id != pending_id {
                    return Some(Command::done());
                }
                model.statistics_preference_request = None;
                let value = match (requested, result) {
                    (None, StorageResult::StatisticsTimeframeLoaded(value)) => Ok(*value),
                    (Some(expected), StorageResult::StatisticsTimeframeSaved(value))
                        if expected == *value =>
                    {
                        Ok(*value)
                    }
                    (_, StorageResult::Failed(error)) => Err(*error),
                    _ => Err(DocumentError::InvalidSettings),
                };
                let superseded = model.statistics_preference_dirty
                    && requested != Some(model.statistics.timeframe);
                match value {
                    Ok(value) => {
                        model.statistics_preference_loaded = true;
                        if requested.is_none()
                            && !model.statistics_preference_dirty
                            && model.statistics.timeframe != value
                        {
                            model.statistics.timeframe = value;
                            model.invalidate_statistics();
                        }
                        if requested == Some(model.statistics.timeframe) {
                            model.statistics_preference_dirty = false;
                        }
                        model.statistics_preference_error = None;
                    }
                    Err(error) => model.statistics_preference_error = Some(error),
                }
                // The latest accepted choice must finish even if Statistics is now hidden.
                Some(if superseded {
                    model.statistics_preference_error = None;
                    self.dispatch_statistics_preference(model)
                } else {
                    render::render()
                })
            }
            _ => None,
        }
    }

    pub(super) fn dispatch_statistics_preference(
        &self,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        if model.statistics_preference_request.is_some() {
            return Command::done();
        }
        let Some(id) = model.statistics_preference_sequence.checked_add(1) else {
            model.statistics_preference_error = Some(DocumentError::Unavailable);
            return render::render();
        };
        let desired = model
            .statistics_preference_dirty
            .then_some(model.statistics.timeframe);
        model.statistics_preference_sequence = id;
        model.statistics_preference_request = Some((id, desired));
        let operation = desired.map_or(
            StorageOperation::LoadStatisticsTimeframe,
            StorageOperation::SaveStatisticsTimeframe,
        );
        render::render().and(
            Command::request_from_shell(operation)
                .then_send(move |result| Event::StatisticsTimeframeFinished(id, result)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::StatisticsTimeframe::*;

    fn update(model: &mut Model, event: Event) -> Vec<StorageOperation> {
        Yomoku
            .update(event, model)
            .effects()
            .filter_map(|effect| match effect {
                Effect::Storage(request) => Some(request.operation),
                _ => None,
            })
            .collect()
    }
    fn choose(
        model: &mut Model,
        timeframe: crate::statistics::StatisticsTimeframe,
    ) -> Vec<StorageOperation> {
        update(
            model,
            Event::SetStatisticsTimeframe {
                timeframe,
                as_of_unix_ms: 1_800_000_000_123,
            },
        )
    }
    fn finish(model: &mut Model, id: u64, result: StorageResult) -> Vec<StorageOperation> {
        update(model, Event::StatisticsTimeframeFinished(id, result))
    }
    #[test]
    fn startup_restores_once_and_does_not_override_an_accepted_selection() {
        let mut model = Model::default();
        assert_eq!(
            update(&mut model, Event::LoadStatisticsTimeframe),
            [StorageOperation::LoadStatisticsTimeframe]
        );
        assert!(update(&mut model, Event::LoadStatisticsTimeframe).is_empty());
        finish(
            &mut model,
            1,
            StorageResult::StatisticsTimeframeLoaded(LastWeek),
        );
        assert_eq!(model.statistics.timeframe, LastWeek);
        assert!(update(&mut model, Event::LoadStatisticsTimeframe).is_empty());

        let mut model = Model::default();
        update(&mut model, Event::LoadStatisticsTimeframe);
        update(
            &mut model,
            Event::SelectDestination(Destination::Statistics),
        );
        choose(&mut model, AllTime);
        assert_eq!(
            finish(
                &mut model,
                1,
                StorageResult::StatisticsTimeframeLoaded(LastMonth)
            ),
            [StorageOperation::SaveStatisticsTimeframe(AllTime)]
        );
        assert_eq!(model.statistics.timeframe, AllTime);
    }
    #[test]
    fn rapid_selections_coalesce_and_finish_latest_after_navigation_even_when_old_save_fails() {
        for failed in [false, true] {
            let mut model = Model::default();
            update(
                &mut model,
                Event::SelectDestination(Destination::Statistics),
            );
            let first = choose(&mut model, LastWeek);
            assert!(first.contains(&StorageOperation::SaveStatisticsTimeframe(LastWeek)));
            assert!(choose(&mut model, LastMonth).is_empty());
            assert!(choose(&mut model, AllTime).is_empty());
            update(&mut model, Event::SelectDestination(Destination::Library));
            let result = if failed {
                StorageResult::Failed(DocumentError::Unavailable)
            } else {
                StorageResult::StatisticsTimeframeSaved(LastWeek)
            };
            assert_eq!(
                finish(&mut model, 1, result),
                [StorageOperation::SaveStatisticsTimeframe(AllTime)]
            );
            assert!(finish(
                &mut model,
                1,
                StorageResult::StatisticsTimeframeSaved(LastWeek)
            )
            .is_empty());
            assert!(model.statistics_preference_request.is_some());
            finish(
                &mut model,
                2,
                StorageResult::StatisticsTimeframeSaved(AllTime),
            );
            assert!(!model.statistics_preference_dirty);
            assert_eq!(model.statistics_preference_error, None);
            assert_eq!(model.statistics.timeframe, AllTime);
        }
    }
    #[test]
    fn preference_errors_require_explicit_retry_and_do_not_touch_reader_errors() {
        let mut model = Model::default();
        update(&mut model, Event::LoadStatisticsTimeframe);
        finish(
            &mut model,
            1,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(
            Yomoku.view(&model).statistics.error,
            Some(DocumentError::Unavailable)
        );
        assert!(update(&mut model, Event::LoadStatisticsTimeframe).is_empty());
        update(
            &mut model,
            Event::SelectDestination(Destination::Statistics),
        );
        assert_eq!(
            update(&mut model, Event::RetryStatistics),
            [StorageOperation::LoadStatisticsTimeframe]
        );
        finish(
            &mut model,
            2,
            StorageResult::StatisticsTimeframeLoaded(LastYear),
        );
        choose(&mut model, LastWeek);
        finish(
            &mut model,
            3,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(model.statistics_preference_dirty);
        // Finish the independent chart read so Retry may also retry chart failures.
        model.storage_busy = false;
        assert_eq!(
            update(&mut model, Event::RetryStatistics),
            [StorageOperation::SaveStatisticsTimeframe(LastWeek)]
        );
        finish(
            &mut model,
            4,
            StorageResult::StatisticsTimeframeSaved(LastWeek),
        );
        assert_eq!(Yomoku.view(&model).statistics.error, None);
        assert_eq!(model.document_error, None);
    }
}

#[cfg(test)]
#[test]
fn explicit_default_choice_wins_over_late_startup_restore() {
    use crate::statistics::StatisticsTimeframe::*;
    let mut model = Model::default();
    drop(Yomoku.update(Event::LoadStatisticsTimeframe, &mut model));
    drop(Yomoku.update(
        Event::SelectDestination(Destination::Statistics),
        &mut model,
    ));
    drop(Yomoku.update(
        Event::SetStatisticsTimeframe {
            timeframe: LastYear,
            as_of_unix_ms: 100,
        },
        &mut model,
    ));
    drop(Yomoku.update(
        Event::StatisticsTimeframeFinished(1, StorageResult::StatisticsTimeframeLoaded(LastWeek)),
        &mut model,
    ));
    assert_eq!(model.statistics.timeframe, LastYear);
    assert_eq!(
        model.statistics_preference_request,
        Some((2, Some(LastYear)))
    );
}
