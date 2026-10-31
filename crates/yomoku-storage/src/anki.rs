//! Durable external-note submission journal, not a mutation queue. No method
//! invokes Anki or produces a provider command from a restored record.

use std::{error::Error, fmt};

use rusqlite::{params, Connection, TransactionBehavior};
use yomoku_core::anki::{AnkiExistingNotes, AnkiNoteId, AnkiReconciliation, AnkiSubmission};

use crate::{Storage, StorageError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnkiSubmissionId(i64);

impl AnkiSubmissionId {
    pub fn value(self) -> i64 {
        self.0
    }
}

/// Created only by a newly committed preparation, never by a recovery read.
/// Keep in the live provider callback; no serialization or reconstruction API.
#[derive(Debug)]
pub struct AnkiLiveSubmission {
    id: AnkiSubmissionId,
}

impl AnkiLiveSubmission {
    pub fn id(&self) -> AnkiSubmissionId {
        self.id
    }
}

#[derive(Debug)]
pub enum AnkiJournalError {
    Storage(StorageError),
    InvalidRecord,
    UnresolvedSubmission,
    StaleCallback,
}

impl fmt::Display for AnkiJournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Storage(_) => "Anki journal storage failed",
            Self::InvalidRecord => "invalid stored Anki submission",
            Self::UnresolvedSubmission => "Anki submission requires reconciliation",
            Self::StaleCallback => "stale Anki submission callback",
        })
    }
}

impl Error for AnkiJournalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for AnkiJournalError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Storage(value.into())
    }
}

/// Callback classification from the isolated provider boundary. `NotInserted`
/// requires positive knowledge that NO mutation was submitted. Every failure
/// after submission, including deck placement/readback, must be `Uncertain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiSubmissionOutcome {
    NotInserted,
    Added(AnkiNoteId),
    Existing(AnkiExistingNotes),
    Uncertain { known_note_id: Option<AnkiNoteId> },
}

/// Recovery contains only frozen data for read-only reconciliation. Pending and
/// uncertain records are deliberately indistinguishable here. An empty/failed
/// observation cannot establish that a prior provider write will never commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiRecovery {
    pub id: AnkiSubmissionId,
    pub submission: AnkiSubmission,
    pub known_note_id: Option<AnkiNoteId>,
}

impl Storage {
    /// Commit BEFORE calling the note inserter, once, in the same live attempt.
    /// A successful return is for that caller only; never replay it on restart
    /// or after losing the commit response. Repeated calls are refused while an
    /// obligation exists, even for an identical payload. No auto retry here.
    pub fn begin_anki_submission(
        &mut self,
        submission: &AnkiSubmission,
    ) -> Result<AnkiLiveSubmission, AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if read_recovery(&transaction)?.is_some() {
            return Err(AnkiJournalError::UnresolvedSubmission);
        }
        let payload =
            serde_json::to_string(submission).map_err(|_| AnkiJournalError::InvalidRecord)?;
        let changed = transaction.execute(
            "INSERT INTO anki_submissions (payload, state) VALUES (?1, 'pending')",
            [payload],
        )?;
        let id = AnkiSubmissionId(transaction.last_insert_rowid());
        if id.0 <= 0 || changed != 1 {
            return Err(AnkiJournalError::InvalidRecord);
        }
        transaction.commit()?;
        Ok(AnkiLiveSubmission { id })
    }

    /// A successful empty result alone permits a fresh user-initiated export.
    /// Any error must gate live submission; never treat failed restore as empty.
    /// This read leaves pending rows pending and performs no mutation/replay.
    pub fn anki_recovery(&self) -> Result<Option<AnkiRecovery>, AnkiJournalError> {
        read_recovery(&self.connection)
    }

    /// Accept one callback for the exact still-pending durable ID. Duplicate,
    /// old and post-reconciliation callbacks cannot release a newer obligation.
    /// If commit fails, retain the pending record and reconcile read-only.
    pub fn finish_anki_submission(
        &mut self,
        live: &AnkiLiveSubmission,
        outcome: AnkiSubmissionOutcome,
    ) -> Result<(), AnkiJournalError> {
        let id = live.id;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recovery = read_recovery(&transaction)?.ok_or(AnkiJournalError::StaleCallback)?;
        if recovery.id != id {
            return Err(AnkiJournalError::StaleCallback);
        }
        let (state, note, existing) = match outcome {
            AnkiSubmissionOutcome::NotInserted => ("not_inserted", None, None),
            AnkiSubmissionOutcome::Added(id) => ("added", Some(id), None),
            AnkiSubmissionOutcome::Existing(ids) => ("existing", None, Some(ids)),
            AnkiSubmissionOutcome::Uncertain { known_note_id } => {
                ("uncertain", known_note_id, None)
            }
        };
        let changed = transaction.execute(
            "UPDATE anki_submissions SET state = ?2, known_note_id = ?3, existing_ids = ?4
             WHERE id = ?1 AND state = 'pending'",
            params![id.0, state, note.map(i64::from), encode_existing(existing)?],
        )?;
        if changed != 1 {
            return Err(AnkiJournalError::StaleCallback);
        }
        transaction.commit()?;
        Ok(())
    }

    /// Reconcile pending/uncertain records by exact-surface reads only. Existing
    /// establishes existence, NOT successful placement in the requested deck.
    /// Preserve any known inserted ID, even when another matching note is found.
    /// Empty/failed reads retain the obligation and never authorize insertion.
    pub fn reconcile_anki_submission(
        &mut self,
        id: AnkiSubmissionId,
        observation: AnkiReconciliation,
    ) -> Result<(), AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recovery = read_recovery(&transaction)?.ok_or(AnkiJournalError::StaleCallback)?;
        if recovery.id != id {
            return Err(AnkiJournalError::StaleCallback);
        }
        let (state, existing) = match observation {
            AnkiReconciliation::Existing(ids) => ("existing", Some(ids)),
            AnkiReconciliation::Unresolved | AnkiReconciliation::Failed(_) => ("uncertain", None),
        };
        let changed = transaction.execute(
            "UPDATE anki_submissions SET state = ?2, existing_ids = ?3 WHERE id = ?1",
            params![id.0, state, encode_existing(existing)?],
        )?;
        if changed != 1 {
            return Err(AnkiJournalError::StaleCallback);
        }
        transaction.commit()?;
        Ok(())
    }
}

fn encode_existing(ids: Option<AnkiExistingNotes>) -> Result<Option<String>, AnkiJournalError> {
    ids.map(|ids| {
        serde_json::to_string(&ids.ids().iter().copied().map(i64::from).collect::<Vec<_>>())
            .map_err(|_| AnkiJournalError::InvalidRecord)
    })
    .transpose()
}

fn read_recovery(connection: &Connection) -> Result<Option<AnkiRecovery>, AnkiJournalError> {
    let mut statement = connection.prepare(
        "SELECT id, payload, known_note_id, state, existing_ids FROM anki_submissions
         WHERE state NOT IN ('not_inserted', 'added', 'existing')",
    )?;
    let mut rows = statement.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let id: i64 = row.get(0)?;
    let payload: String = row.get(1)?;
    let note: Option<i64> = row.get(2)?;
    let state: String = row.get(3)?;
    let existing: Option<String> = row.get(4)?;
    if id <= 0
        || existing.is_some()
        || !matches!(state.as_str(), "pending" | "uncertain")
        || (state == "pending" && note.is_some())
    {
        return Err(AnkiJournalError::InvalidRecord);
    }
    let recovery = AnkiRecovery {
        id: AnkiSubmissionId(id),
        submission: serde_json::from_str(&payload).map_err(|_| AnkiJournalError::InvalidRecord)?,
        known_note_id: note
            .map(AnkiNoteId::try_from)
            .transpose()
            .map_err(|_| AnkiJournalError::InvalidRecord)?,
    };
    if rows.next()?.is_some() {
        return Err(AnkiJournalError::InvalidRecord);
    }
    Ok(Some(recovery))
}
