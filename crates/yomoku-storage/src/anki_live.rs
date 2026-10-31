//! Transactional live journal ownership. These APIs perform SQLite I/O only;
//! the dispatcher must still authenticate its owner and invoke the provider once.

use rusqlite::TransactionBehavior;
use yomoku_core::anki::{
    AnkiModelCreation, AnkiModelRecovery, AnkiRecovery, AnkiRecoverySnapshot, AnkiSubmission,
};

use crate::{
    anki::{self, AnkiJournalError, AnkiLiveSubmission, AnkiSubmissionOutcome},
    anki_model::{self, AnkiLiveModelCreation, AnkiModelCreationOutcome},
    anki_recovery::read_snapshot,
    Storage,
};

/// Frozen content from native preparation/preflight, never a restored mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiLiveIntent {
    Note(Box<AnkiSubmission>),
    Model(Box<AnkiModelCreation>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiLiveOutcome {
    Note(AnkiSubmissionOutcome),
    Model(AnkiModelCreationOutcome),
}

#[derive(Debug)]
enum LiveToken {
    Note(AnkiLiveSubmission),
    Model(AnkiLiveModelCreation),
}

/// Issued only after a successful begin COMMIT. No Clone, serialization or
/// reconstruction API: the owning worker retains this across one invocation.
/// A recovery snapshot is data only and cannot recover a lost live attempt.
#[derive(Debug)]
pub struct AnkiLiveAttempt {
    token: LiveToken,
    snapshot: Box<AnkiRecoverySnapshot>,
}

impl AnkiLiveAttempt {
    /// The exact pair committed by begin. This is also the expected pair for
    /// outcome acceptance, including frozen content and any other obligation.
    pub fn snapshot(&self) -> &AnkiRecoverySnapshot {
        &self.snapshot
    }
}

impl Storage {
    /// Check both journals, insert pending content and capture the resulting
    /// pair under one immediate transaction. A failed/lost response must restore
    /// read-only; never repeat begin or dispatch from a recovered record.
    pub fn begin_anki_live(
        &mut self,
        intent: &AnkiLiveIntent,
    ) -> Result<AnkiLiveAttempt, AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (token, expected) = match intent {
            AnkiLiveIntent::Note(submission) => {
                let live = anki::begin_in(&transaction, submission)?;
                let expected = AnkiRecoverySnapshot {
                    note: Some(AnkiRecovery {
                        id: live.id(),
                        submission: *submission.clone(),
                        known_note_id: None,
                    }),
                    model: None,
                };
                (LiveToken::Note(live), expected)
            }
            AnkiLiveIntent::Model(creation) => {
                let live = anki_model::begin_in(&transaction, creation)?;
                let expected = AnkiRecoverySnapshot {
                    note: None,
                    model: Some(AnkiModelRecovery {
                        id: live.id(),
                        creation: *creation.clone(),
                        known_model_id: None,
                    }),
                };
                (LiveToken::Model(live), expected)
            }
        };
        let snapshot = Box::new(read_snapshot(&transaction)?);
        if *snapshot != expected {
            return Err(AnkiJournalError::InvalidRecord);
        }
        transaction.commit()?;
        Ok(AnkiLiveAttempt { token, snapshot })
    }

    /// Compare the complete frozen pair before accepting the exact live outcome.
    /// Return the authoritative pair only after COMMIT; any error/lost response
    /// requires read-only restore/reconciliation, never another provider call.
    /// The caller must also reject foreign/stale callback handles in its owner.
    pub fn finish_anki_live(
        &mut self,
        live: &AnkiLiveAttempt,
        outcome: AnkiLiveOutcome,
    ) -> Result<Box<AnkiRecoverySnapshot>, AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if read_snapshot(&transaction)? != *live.snapshot {
            return Err(AnkiJournalError::StaleCallback);
        }
        let mut expected = live.snapshot.clone();
        match (&live.token, outcome) {
            (LiveToken::Note(token), AnkiLiveOutcome::Note(outcome)) => {
                match &outcome {
                    AnkiSubmissionOutcome::Uncertain { known_note_id } => {
                        expected.note.as_mut().expect("live note").known_note_id = *known_note_id;
                    }
                    _ => expected.note = None,
                }
                anki::finish_in(&transaction, token, outcome)?;
            }
            (LiveToken::Model(token), AnkiLiveOutcome::Model(outcome)) => {
                match &outcome {
                    AnkiModelCreationOutcome::Uncertain { known_model_id } => {
                        expected.model.as_mut().expect("live model").known_model_id =
                            *known_model_id;
                    }
                    _ => expected.model = None,
                }
                anki_model::finish_in(&transaction, token, outcome)?;
            }
            _ => return Err(AnkiJournalError::StaleCallback),
        }
        let snapshot = Box::new(read_snapshot(&transaction)?);
        if snapshot != expected {
            return Err(AnkiJournalError::InvalidRecord);
        }
        transaction.commit()?;
        Ok(snapshot)
    }
}
