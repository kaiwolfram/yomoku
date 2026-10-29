//! Read-only projections of reading and vocabulary history. No clocks or I/O.

use std::collections::BTreeMap;

use crate::{document::VocabularyId, library::WordStatusCounts, vocabulary::VocabularyStatus};

/// Lifetime totals from complete analyzed events, including deliberate rereads.
/// Occurrences retain repetitions and exclude erased vocabulary history. Time
/// belongs to each completed page once, including pages with no Japanese words.
/// Registration, partial time and unused completion reservations contribute zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LifetimeReadingTotals {
    pub japanese_occurrences: u64,
    pub active_time_ms: u64,
}

/// One validated v1 registration or actual status transition. Storage validates
/// the version and cause/provenance before supplying events in durable ID order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VocabularyStatusChange {
    pub event_id: u64,
    pub vocabulary_id: VocabularyId,
    pub occurred_at_unix_ms: i64,
    pub old_status: Option<VocabularyStatus>,
    pub new_status: VocabularyStatus,
}

/// Absolute counts after all changes at this instant. Before the first point all
/// counts are zero; no date range/bucket or synthetic starting point is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusTimelinePoint {
    pub occurred_at_unix_ms: i64,
    pub counts: WordStatusCounts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidStatusHistory;

impl std::fmt::Display for InvalidStatusHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid vocabulary status history")
    }
}

impl std::error::Error for InvalidStatusHistory {}

/// Streaming replay, bounded by surviving vocabulary and distinct event dates,
/// rather than all historical transitions. Discard on error; no partial result
/// should be displayed. Erased vocabulary must be absent from the input.
#[derive(Default)]
pub struct StatusTimelineBuilder {
    last_event_id: u64,
    entries: BTreeMap<VocabularyId, (VocabularyStatus, i64)>,
    changes: BTreeMap<i64, [i128; 5]>,
}

impl StatusTimelineBuilder {
    pub fn push(&mut self, event: VocabularyStatusChange) -> Result<(), InvalidStatusHistory> {
        if event.event_id <= self.last_event_id || event.occurred_at_unix_ms < 0 {
            return Err(InvalidStatusHistory);
        }
        let previous = self.entries.get(&event.vocabulary_id).copied();
        match (previous, event.old_status) {
            (None, None) if event.new_status == VocabularyStatus::New => {}
            (Some((status, _)), Some(old)) if status == old && old != event.new_status => {}
            _ => return Err(InvalidStatusHistory),
        }
        // A wall-clock correction or delayed frozen completion cannot make an
        // entry change before its own registration/preceding committed change.
        // Only this projection is adjusted; durable event dates remain exact.
        let timestamp = previous.map_or(event.occurred_at_unix_ms, |(_, date)| {
            date.max(event.occurred_at_unix_ms)
        });
        let deltas = self.changes.entry(timestamp).or_default();
        if let Some(old) = event.old_status {
            deltas[status_index(old)] -= 1;
        }
        deltas[status_index(event.new_status)] += 1;
        self.entries
            .insert(event.vocabulary_id, (event.new_status, timestamp));
        self.last_event_id = event.event_id;
        Ok(())
    }

    pub fn finish(self) -> Result<Vec<StatusTimelinePoint>, InvalidStatusHistory> {
        let mut counts = [0_i128; 5];
        self.changes
            .into_iter()
            .map(|(occurred_at_unix_ms, deltas)| {
                for (count, delta) in counts.iter_mut().zip(deltas) {
                    *count += delta;
                }
                let [new, learning, familiar, known, ignored] =
                    counts.map(|count| u64::try_from(count).map_err(|_| InvalidStatusHistory));
                Ok(StatusTimelinePoint {
                    occurred_at_unix_ms,
                    counts: WordStatusCounts {
                        new: new?,
                        learning: learning?,
                        familiar: familiar?,
                        known: known?,
                        ignored: ignored?,
                    },
                })
            })
            .collect()
    }
}

fn status_index(status: VocabularyStatus) -> usize {
    match status {
        VocabularyStatus::New => 0,
        VocabularyStatus::Learning => 1,
        VocabularyStatus::Familiar => 2,
        VocabularyStatus::Known => 3,
        VocabularyStatus::Ignored => 4,
    }
}
