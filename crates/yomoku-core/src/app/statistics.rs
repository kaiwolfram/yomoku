//! Retained statistics queries share the app's single native storage slot.

use super::*;
use crate::statistics::StatisticsQuery;

impl Model {
    pub(super) fn statistics_visible(&self) -> bool {
        self.destination == Destination::Statistics
            && self.content_route == ContentRoute::Main
            && !self.settings_open
    }

    pub(super) fn invalidate_statistics(&mut self) {
        self.statistics.query = None;
        self.statistics.data = None;
        self.statistics.error = None;
    }
}

impl Yomoku {
    pub(super) fn update_statistics(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        let (timeframe, as_of_unix_ms) = match event {
            Event::LoadStatistics { as_of_unix_ms } => (model.statistics.timeframe, *as_of_unix_ms),
            Event::SetStatisticsTimeframe {
                timeframe,
                as_of_unix_ms,
            } => (*timeframe, *as_of_unix_ms),
            Event::RetryStatistics => {
                if !model.statistics_visible()
                    || model.storage_busy()
                    || model.statistics.error.is_none()
                {
                    return Some(Command::done());
                }
                return Some(self.dispatch_statistics(model));
            }
            _ => return None,
        };
        if !model.statistics_visible()
            || (model.storage_busy() && model.statistics_request().is_none())
        {
            return Some(Command::done());
        }
        let changed = model.statistics.timeframe != timeframe;
        model.statistics.timeframe = timeframe;
        // An explicit choice also wins over an unfinished startup restore, even
        // when the user taps the currently displayed default timeframe.
        let save = if matches!(event, Event::SetStatisticsTimeframe { .. })
            && (changed || !model.statistics_preference_loaded)
        {
            model.statistics_preference_dirty = true;
            model.statistics_preference_error = None;
            self.dispatch_statistics_preference(model)
        } else {
            Command::done()
        };
        let query = StatisticsQuery {
            timeframe,
            as_of_unix_ms,
        };
        // Automatic loads do not retry a failed identical query. Resume only
        // unattempted reads that were coalesced while hidden in Settings.
        if model.statistics.query == Some(query) {
            let loaded = model.statistics.data.as_ref().is_some_and(|data| {
                data.timeframe == query.timeframe
                    && query.as_of_unix_ms.checked_add(1) == Some(data.end_exclusive_unix_ms)
            });
            return Some(save.and(
                if model.statistics_request().is_none()
                    && !loaded
                    && model.statistics.error.is_none()
                {
                    self.dispatch_statistics(model)
                } else {
                    Command::done()
                },
            ));
        }
        model.statistics.query = Some(query);
        model.statistics.error = None;
        Some(save.and(if model.statistics_request().is_some() {
            // Coalesce rapid selections; the old callback still owns the FIFO.
            render::render()
        } else {
            self.dispatch_statistics(model)
        }))
    }

    fn dispatch_statistics(&self, model: &mut Model) -> Command<Effect, Event> {
        let query = model.statistics.query.expect("requested statistics query");
        let Some(request_id) = model.statistics_request_sequence.checked_add(1) else {
            model.statistics.error = Some(DocumentError::Unavailable);
            return render::render();
        };
        model.statistics_request_sequence = request_id;
        model.pending_storage = PendingStorage::Statistics(request_id, query);
        model.statistics.loading = true;
        model.statistics.error = None;

        render::render().and(
            Command::request_from_shell(StorageOperation::LoadStatistics(query))
                .then_send(move |result| Event::StatisticsFinished(request_id, result)),
        )
    }

    pub(super) fn finish_statistics(
        &self,
        request_id: u64,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        let Some((pending_id, query)) = model.statistics_request() else {
            return Command::done();
        };
        if request_id != pending_id {
            return Command::done();
        }
        model.pending_storage = PendingStorage::Idle;
        model.statistics.loading = false;
        if model.statistics.query != Some(query) {
            // Navigation/mutation can invalidate a read; a changed timeframe can
            // replace it. Neither stale success nor stale failure is presented.
            return if model.statistics_visible() && model.statistics.query.is_some() {
                self.dispatch_statistics(model)
            } else {
                render::render()
            };
        }
        match result {
            StorageResult::StatisticsLoaded(data)
                if data.timeframe == query.timeframe
                    && query.as_of_unix_ms.checked_add(1) == Some(data.end_exclusive_unix_ms) =>
            {
                model.statistics.data = Some(*data);
            }
            StorageResult::Failed(error) => model.statistics.error = Some(error),
            _ => model.statistics.error = Some(DocumentError::InvalidDocument),
        }
        render::render()
    }
}
