//! Read-only projections of completed reading history. No clocks or storage I/O.

/// Lifetime totals from complete analyzed events, including deliberate rereads.
/// Occurrences retain repetitions and exclude erased vocabulary history. Time
/// belongs to each completed page once, including pages with no Japanese words.
/// Registration, partial time and unused completion reservations contribute zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LifetimeReadingTotals {
    pub japanese_occurrences: u64,
    pub active_time_ms: u64,
}
