use super::*;
use crate::anki::{AnkiLiveProgress, AnkiRecoveryPhase, AnkiRecoveryView};

impl Model {
    fn anki_recovery_needs_attention(&self) -> bool {
        matches!(&self.anki_recovery, AnkiRecoveryState::Failed(_))
            || matches!(&self.anki_recovery, AnkiRecoveryState::Restored(pair)
                if pair.note.is_some() || pair.model.is_some())
            || self.anki_live.is_some()
            || self.anki_recovery_read_error.is_some()
            || self.anki_recovery_commit_error.is_some()
    }

    pub(in crate::app) fn remember_anki_recovery_attention(&mut self) {
        self.anki_recovery_notice |= self.anki_recovery_needs_attention();
    }

    pub(super) fn can_retry_anki_recovery(&self) -> bool {
        let live_failed = matches!(self.anki_live_progress(), Some(AnkiLiveProgress::Failed(_)));
        self.anki_recovery_commit.is_none()
            && (!self.anki_live_blocks_recovery() || live_failed)
            && (live_failed
                || self.anki_recovery_read.is_some()
                || matches!(self.anki_recovery, AnkiRecoveryState::Failed(_))
                || self.anki_recovery_read_error.is_some()
                || self.anki_recovery_commit_error.is_some())
    }

    pub(super) fn can_check_anki_recovery(&self) -> bool {
        matches!(self.anki_recovery, AnkiRecoveryState::Restored(_))
            && !self.anki_live_blocks_recovery()
            && self.anki_recovery_read.is_none()
            && self.anki_recovery_commit.is_none()
            && self.anki_recovery_commit_error.is_none()
    }

    pub(in crate::app) fn anki_recovery_view(&self) -> Option<AnkiRecoveryView> {
        if !self.settings_open
            || !(self.anki_recovery_notice || self.anki_recovery_needs_attention())
        {
            return None;
        }
        let (note_pending, model_pending) = match &self.anki_recovery {
            AnkiRecoveryState::Restored(pair) => (pair.note.is_some(), pair.model.is_some()),
            _ => (false, false),
        };
        let storage_error = self.anki_recovery_commit_error.or_else(|| {
            match (self.anki_live_progress(), &self.anki_recovery) {
                (Some(AnkiLiveProgress::Failed(error)), _) => Some(error),
                (_, AnkiRecoveryState::Failed(error)) => Some(*error),
                _ => None,
            }
        });
        let phase = if self.anki_recovery_commit.is_some() {
            AnkiRecoveryPhase::Saving
        } else if self.anki_recovery_read.is_some() {
            AnkiRecoveryPhase::Checking
        } else if storage_error.is_some() || self.anki_recovery_read_error.is_some() {
            AnkiRecoveryPhase::Failed
        } else if self.anki_live_blocks_recovery() {
            AnkiRecoveryPhase::Busy
        } else {
            match self.anki_recovery {
                AnkiRecoveryState::Unrestored | AnkiRecoveryState::Loading => {
                    AnkiRecoveryPhase::Loading
                }
                AnkiRecoveryState::Failed(_) => AnkiRecoveryPhase::Failed,
                AnkiRecoveryState::Restored(_) if note_pending || model_pending => {
                    AnkiRecoveryPhase::Unresolved
                }
                AnkiRecoveryState::Restored(_) => AnkiRecoveryPhase::Ready,
            }
        };
        let can_check = self.can_check_anki_recovery();
        Some(AnkiRecoveryView {
            phase,
            note_pending,
            model_pending,
            storage_error,
            provider_error: self.anki_recovery_read_error,
            can_retry: self.can_retry_anki_recovery(),
            can_check_note: can_check && note_pending,
            can_check_model: can_check && model_pending,
        })
    }
}

#[cfg(test)]
mod tests;
