//! Retained read-only browser on the application's single FIFO storage slot.

use super::*;
use crate::vocabulary::{VocabularyPage, VocabularyQuery, VocabularyRow, VOCABULARY_PAGE_SIZE};

pub(super) struct VocabularyRequest {
    id: u64,
    query: VocabularyQuery,
    offset: u64,
    obsolete: bool,
}

/// Build a replacement separately so rows, count and pagination publish together.
#[derive(Default)]
pub(super) struct VocabularyRefresh {
    target_len: usize,
    rows: Vec<VocabularyRow>,
    matching_count: Option<u64>,
}

impl Model {
    pub(super) fn vocabulary_visible(&self) -> bool {
        self.destination == Destination::Vocabulary
            && self.content_route == ContentRoute::Main
            && !self.settings_open
    }

    pub(super) fn patch_vocabulary_status(
        &mut self,
        id: crate::document::VocabularyId,
        status: crate::vocabulary::VocabularyStatus,
    ) {
        for row in self
            .vocabulary
            .rows
            .iter_mut()
            .chain(
                self.vocabulary_refresh
                    .iter_mut()
                    .flat_map(|refresh| &mut refresh.rows),
            )
            .filter(|row| row.id == id)
        {
            row.status = status;
        }
    }

    pub(super) fn invalidate_vocabulary(&mut self) {
        self.vocabulary_refresh = None;
        self.vocabulary.matching_count = None;
        self.vocabulary.rows.clear();
        self.vocabulary.next_offset = None;
        self.vocabulary.error = None;
        self.vocabulary_failed_offset = None;
        if let Some(request) = self.vocabulary_request_mut() {
            // Query equality alone cannot distinguish A -> B -> A or reentry.
            request.obsolete = true;
        }
    }
}

impl Yomoku {
    pub(super) fn refresh_vocabulary_prefix(&self, model: &mut Model) -> Command<Effect, Event> {
        model.vocabulary_refresh = Some(VocabularyRefresh {
            target_len: model
                .vocabulary
                .rows
                .len()
                .max(VOCABULARY_PAGE_SIZE as usize),
            ..Default::default()
        });
        self.dispatch_vocabulary(0, model)
    }

    pub(super) fn update_vocabulary(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        if !matches!(
            event,
            Event::LoadVocabulary
                | Event::SetVocabularyQuery(_)
                | Event::LoadMoreVocabulary
                | Event::RetryVocabulary
        ) {
            return None;
        }
        if !model.vocabulary_visible()
            || (model.storage_busy() && model.vocabulary_request().is_none())
        {
            return Some(Command::done());
        }
        if let Event::SetVocabularyQuery(query) = event {
            let query = query.clone().canonicalized();
            if model.vocabulary.query != query {
                model.vocabulary.query = query;
                model.invalidate_vocabulary();
                return Some(if model.vocabulary_request().is_some() {
                    render::render()
                } else {
                    self.dispatch_vocabulary(0, model)
                });
            }
        }
        if model.vocabulary_request().is_some() {
            return Some(Command::done());
        }
        let offset = match event {
            Event::RetryVocabulary if model.vocabulary.error.is_some() => {
                model.vocabulary_failed_offset
            }
            Event::LoadMoreVocabulary if model.vocabulary.error.is_none() => {
                model.vocabulary.next_offset
            }
            Event::LoadVocabulary | Event::SetVocabularyQuery(_)
                if model.vocabulary.matching_count.is_none()
                    && model.vocabulary.error.is_none() =>
            {
                Some(0)
            }
            _ => None,
        };
        Some(offset.map_or_else(Command::done, |offset| {
            self.dispatch_vocabulary(offset, model)
        }))
    }

    fn dispatch_vocabulary(&self, offset: u64, model: &mut Model) -> Command<Effect, Event> {
        let Some(id) = model.vocabulary_request_sequence.checked_add(1) else {
            model.vocabulary.error = Some(DocumentError::Unavailable);
            return render::render();
        };
        model.vocabulary_request_sequence = id;
        let query = model.vocabulary.query.clone();
        model.pending_storage = PendingStorage::Vocabulary(VocabularyRequest {
            id,
            query: query.clone(),
            offset,
            obsolete: false,
        });
        model.vocabulary.loading = true;
        model.vocabulary.error = None;
        model.vocabulary_failed_offset = None;

        render::render().and(
            Command::request_from_shell(StorageOperation::LoadVocabulary { query, offset })
                .then_send(move |result| Event::VocabularyFinished(id, result)),
        )
    }

    pub(super) fn finish_vocabulary(
        &self,
        id: u64,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        if !model
            .vocabulary_request()
            .as_ref()
            .is_some_and(|request| request.id == id)
        {
            return Command::done();
        }
        let PendingStorage::Vocabulary(request) = std::mem::take(&mut model.pending_storage) else {
            unreachable!("matched request")
        };
        model.vocabulary.loading = false;
        if request.obsolete {
            return if model.vocabulary_visible() {
                self.dispatch_vocabulary(0, model)
            } else {
                render::render()
            };
        }
        let refreshing = model.vocabulary_refresh.is_some();
        if let StorageResult::VocabularyLoaded(page) = &result {
            let expected_count = model
                .vocabulary_refresh
                .as_ref()
                .map_or(model.vocabulary.matching_count, |refresh| {
                    refresh.matching_count
                });
            if request.offset > 0
                && page.offset == request.offset
                && expected_count != Some(page.matching_count)
            {
                // Restart a replacement if another writer changes counts between pages.
                if refreshing {
                    let refresh = model.vocabulary_refresh.as_mut().expect("refreshing");
                    refresh.rows.clear();
                    refresh.matching_count = None;
                    return self.dispatch_vocabulary(0, model);
                }
                model.invalidate_vocabulary();
                return if model.vocabulary_visible() {
                    self.dispatch_vocabulary(0, model)
                } else {
                    render::render()
                };
            }
        }
        let rows = model
            .vocabulary_refresh
            .as_ref()
            .map_or(model.vocabulary.rows.as_slice(), |refresh| {
                refresh.rows.as_slice()
            });
        match result {
            StorageResult::VocabularyLoaded(page) if valid_page(&page, &request, rows) => {
                if let Some(refresh) = &mut model.vocabulary_refresh {
                    refresh.matching_count = Some(page.matching_count);
                    refresh.rows.extend(page.rows);
                    if refresh.rows.len() < refresh.target_len {
                        if let Some(offset) = page.next_offset {
                            return self.dispatch_vocabulary(offset, model);
                        }
                    }
                    let refresh = model.vocabulary_refresh.take().expect("completed refresh");
                    model.vocabulary.rows = refresh.rows;
                } else {
                    model.vocabulary.rows.extend(page.rows);
                }
                model.vocabulary.matching_count = Some(page.matching_count);
                model.vocabulary.next_offset = page.next_offset;
            }
            StorageResult::Failed(error) => model.vocabulary.error = Some(error),
            _ => model.vocabulary.error = Some(DocumentError::InvalidDocument),
        }
        if model.vocabulary.error.is_some() {
            model.vocabulary_failed_offset = Some(request.offset);
        }
        render::render()
    }
}

fn valid_page(page: &VocabularyPage, request: &VocabularyRequest, rows: &[VocabularyRow]) -> bool {
    let mut ids: std::collections::BTreeSet<_> = rows.iter().map(|row| row.id).collect();
    let length = page.rows.len() as u64;
    let end = request.offset.checked_add(length);
    page.offset == request.offset
        && request.offset == rows.len() as u64
        && length
            == page
                .matching_count
                .saturating_sub(request.offset)
                .min(VOCABULARY_PAGE_SIZE)
        && end.is_some_and(|end| page.next_offset == (end < page.matching_count).then_some(end))
        && page.rows.iter().all(|row| {
            !row.surface.trim().is_empty()
                && row.surface.starts_with(&request.query.prefix)
                && request.query.statuses.contains(&row.status)
                && row.registered_at_unix_ms >= 0
                && row.last_seen_unix_ms.is_none_or(|date| date >= 0)
                && ids.insert(row.id)
        })
}
