//! Hidden reading-time accounting. The caller supplies monotonic milliseconds;
//! this module reads no clock and performs no persistence or statistics writes.

use serde::{Deserialize, Serialize};

use crate::{document::DocumentId, pagination::ReadingPosition};

/// Time without an interaction that can still count as active reading.
pub const IDLE_TIMEOUT_MS: u64 = 5 * 60 * 1_000;

/// Platform observations use one monotonic epoch per live reader clock. Only
/// durations, never these timestamps, cross the persistence boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderClockAction {
    Resume,
    Interact,
    Checkpoint,
    Pause,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderClockSample {
    pub document_id: DocumentId,
    pub position: ReadingPosition,
    pub monotonic_ms: i64,
    pub action: ReaderClockAction,
}

/// Only this accumulated duration is suitable for a durable partial-page save.
/// Never persist the accumulator's monotonic timestamps across process/reboot.
/// Page/post identity and transactional completion belong to the caller.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(try_from = "i64", into = "i64")]
pub struct PendingReadingTime(i64);

impl TryFrom<i64> for PendingReadingTime {
    type Error = ActiveTimeError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::from_millis(value)
    }
}

impl From<PendingReadingTime> for i64 {
    fn from(value: PendingReadingTime) -> Self {
        value.milliseconds()
    }
}

/// Absolute pending duration at one exact position, never a monotonic timestamp.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReadingTimeUpdate {
    pub document_id: DocumentId,
    pub position: ReadingPosition,
    pub pending: PendingReadingTime,
}

/// Optimistic repository token. Hosts submit durations, never mint revisions.
/// Storage validates this value against the durable row before every write.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct SavedReadingTime {
    pub value: ReadingTimeUpdate,
    pub revision: i64,
}

impl SavedReadingTime {
    pub fn pending(self) -> PendingReadingTime {
        self.value.pending
    }

    pub fn position(self) -> ReadingPosition {
        self.value.position
    }
}

impl PendingReadingTime {
    pub fn from_millis(milliseconds: i64) -> Result<Self, ActiveTimeError> {
        if milliseconds < 0 {
            return Err(ActiveTimeError::NegativeDuration);
        }
        Ok(Self(milliseconds))
    }

    pub fn milliseconds(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActiveTimeError {
    NegativeDuration,
    ClockWentBackwards,
    DurationOverflow,
}

impl std::fmt::Display for ActiveTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NegativeDuration => "pending reading time must not be negative",
            Self::ClockWentBackwards => "reading time requires a monotonic clock",
            Self::DurationOverflow => "pending reading time exceeds signed milliseconds",
        })
    }
}

impl std::error::Error for ActiveTimeError {}

/// One page/post's pending time, initially paused even when restored.
///
/// Resume only while the reader is open in the foreground; pause on background
/// or reader close. Dictionary popups remain within the reader: opening, using
/// and closing them are interactions, not pauses. Checkpoint before saving.
/// All calls for this instance must use the same monotonic clock epoch.
///
/// Checkpoints never drain time or turn it into completed-reading statistics.
/// Future completion must freeze the duration with its command ID and commit it
/// atomically with encounters, status changes and position before retiring it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ActiveReadingTime {
    pending: PendingReadingTime,
    last_observed_ms: Option<u64>,
    last_interaction_ms: Option<u64>,
}

impl ActiveReadingTime {
    pub fn pending(&self) -> PendingReadingTime {
        self.pending
    }

    pub fn from_pending(pending: PendingReadingTime) -> Self {
        Self {
            pending,
            ..Self::default()
        }
    }

    /// Opening/resuming starts the idle window. Duplicate resume notifications
    /// while already active do not extend that window.
    pub fn resume(&mut self, now_ms: u64) -> Result<PendingReadingTime, ActiveTimeError> {
        self.observe(now_ms, Action::Resume)
    }

    /// Settle the active interval, then stop. Repeated pauses are harmless.
    pub fn pause(&mut self, now_ms: u64) -> Result<PendingReadingTime, ActiveTimeError> {
        self.observe(now_ms, Action::Pause)
    }

    /// Reset idle on any reader interaction, including dictionary interactions.
    /// After idle, start counting again from now; never backfill the idle gap.
    /// An interaction while paused does not resume reading.
    pub fn interact(&mut self, now_ms: u64) -> Result<PendingReadingTime, ActiveTimeError> {
        self.observe(now_ms, Action::Interact)
    }

    /// Account through now without changing lifecycle or resetting idle.
    /// The returned value can be saved as pending time; it is not a completion.
    pub fn checkpoint(&mut self, now_ms: u64) -> Result<PendingReadingTime, ActiveTimeError> {
        self.observe(now_ms, Action::Checkpoint)
    }

    fn observe(
        &mut self,
        now_ms: u64,
        action: Action,
    ) -> Result<PendingReadingTime, ActiveTimeError> {
        if self.last_observed_ms.is_some_and(|last| now_ms < last) {
            return Err(ActiveTimeError::ClockWentBackwards);
        }
        let elapsed = match (self.last_interaction_ms, self.last_observed_ms) {
            (Some(interaction), Some(last)) => {
                // Differences avoid overflowing an absolute idle deadline near
                // u64::MAX. Both timestamps are >= interaction by construction.
                (now_ms - interaction).min(IDLE_TIMEOUT_MS)
                    - (last - interaction).min(IDLE_TIMEOUT_MS)
            }
            _ => 0,
        };
        let pending = self
            .pending
            .0
            .checked_add(elapsed as i64)
            .ok_or(ActiveTimeError::DurationOverflow)?;
        // Validate everything before mutation, so errors preserve retry state.
        self.pending = PendingReadingTime(pending);
        self.last_observed_ms = Some(now_ms);
        match action {
            Action::Resume => {
                self.last_interaction_ms.get_or_insert(now_ms);
            }
            Action::Interact if self.last_interaction_ms.is_some() => {
                self.last_interaction_ms = Some(now_ms);
            }
            Action::Pause => self.last_interaction_ms = None,
            Action::Interact | Action::Checkpoint => {}
        }
        Ok(self.pending)
    }
}

#[derive(Clone, Copy)]
enum Action {
    Resume,
    Pause,
    Interact,
    Checkpoint,
}
