//! Retained statistics queries share the app's single native storage slot.

use super::*;
use crate::statistics::StatisticsQuery;

impl Model {
    fn statistics_visible(&self) -> bool {
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
                    || model.storage_busy
                    || model.statistics.error.is_none()
                {
                    return Some(Command::done());
                }
                return Some(self.dispatch_statistics(model));
            }
            _ => return None,
        };
        if !model.statistics_visible() || (model.storage_busy && model.statistics_request.is_none())
        {
            return Some(Command::done());
        }
        let query = StatisticsQuery {
            timeframe,
            as_of_unix_ms,
        };
        // An automatic load never retries a failed identical query. Its explicit
        // Retry event owns that action, so a render cannot create a retry loop.
        if model.statistics.query == Some(query) {
            // A coalesced selection may have become hidden before its turn.
            // Resume that unattempted read after Settings closes, without
            // replaying a completed/failed query or duplicating an active one.
            return Some(
                if model.statistics_request.is_none()
                    && model.statistics.data.is_none()
                    && model.statistics.error.is_none()
                {
                    self.dispatch_statistics(model)
                } else {
                    Command::done()
                },
            );
        }
        model.statistics.timeframe = timeframe;
        model.statistics.query = Some(query);
        model.statistics.data = None;
        model.statistics.error = None;
        Some(if model.statistics_request.is_some() {
            // Coalesce rapid selections; the old callback still owns the FIFO.
            render::render()
        } else {
            self.dispatch_statistics(model)
        })
    }

    fn dispatch_statistics(&self, model: &mut Model) -> Command<Effect, Event> {
        let query = model.statistics.query.expect("requested statistics query");
        let Some(request_id) = model.statistics_request_sequence.checked_add(1) else {
            model.statistics.error = Some(DocumentError::Unavailable);
            return render::render();
        };
        model.statistics_request_sequence = request_id;
        model.statistics_request = Some((request_id, query));
        model.statistics.loading = true;
        model.statistics.error = None;
        model.storage_busy = true;

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
        let Some((pending_id, query)) = model.statistics_request else {
            return Command::done();
        };
        if request_id != pending_id {
            return Command::done();
        }
        model.statistics_request = None;
        model.storage_busy = false;
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
