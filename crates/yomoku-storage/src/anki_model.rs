//! Durable model-creation journal. Restored records authorize read-only layout
//! comparison, never creation, template repair, renaming or deletion.

use rusqlite::{params, Connection, TransactionBehavior};
use yomoku_core::anki::{AnkiModelCreation, AnkiModelId, AnkiModelLayout};

use crate::{anki::AnkiJournalError, Storage};

pub use yomoku_core::anki::{AnkiModelCreationId, AnkiModelRecovery};

/// Only newly committed preparation produces a live callback token. There is
/// no serialization, cloning or reconstruction from a recovery ID.
#[derive(Debug)]
pub struct AnkiLiveModelCreation {
    id: AnkiModelCreationId,
}

impl AnkiLiveModelCreation {
    pub fn id(&self) -> AnkiModelCreationId {
        self.id
    }
}

/// `NotCreated` requires knowledge that no mutating provider call was submitted.
/// Everything after the initial submission is uncertain unless full readback
/// verified creation, including lost insert/update responses and partial models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiModelCreationOutcome {
    NotCreated,
    Created(AnkiModelId),
    Uncertain { known_model_id: Option<AnkiModelId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiModelReconciliation {
    /// Full normal/sort-zero/single-card provider readback, not name-only
    /// discovery. A matching layout establishes usability, never ownership.
    Observed {
        id: AnkiModelId,
        layout: Box<AnkiModelLayout>,
    },
    /// Empty, failed, partial, ambiguous or incompatible provider reads.
    Unresolved,
}

impl Storage {
    /// Commit before one live model-creator invocation. A lost commit response
    /// must restore read-only; this method does not invoke or replay a provider.
    /// Both journals share a mutation gate under the same immediate transaction.
    pub fn begin_anki_model_creation(
        &mut self,
        creation: &AnkiModelCreation,
    ) -> Result<AnkiLiveModelCreation, AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if read_recovery(&transaction)?.is_some() {
            return Err(AnkiJournalError::UnresolvedModelCreation);
        }
        if crate::anki::read_recovery(&transaction)?.is_some() {
            return Err(AnkiJournalError::UnresolvedSubmission);
        }
        let payload =
            serde_json::to_string(creation).map_err(|_| AnkiJournalError::InvalidRecord)?;
        let changed = transaction.execute(
            "INSERT INTO anki_model_creations (payload, state) VALUES (?1, 'pending')",
            [payload],
        )?;
        let id = AnkiModelCreationId::try_from(transaction.last_insert_rowid())
            .map_err(|_| AnkiJournalError::InvalidRecord)?;
        if changed != 1 {
            return Err(AnkiJournalError::InvalidRecord);
        }
        transaction.commit()?;
        Ok(AnkiLiveModelCreation { id })
    }

    /// Pending and uncertain restore identically. Errors must gate live work;
    /// never treat a failed read as an empty journal or retry creation on restart.
    pub fn anki_model_recovery(&self) -> Result<Option<AnkiModelRecovery>, AnkiJournalError> {
        read_recovery(&self.connection)
    }

    /// Only the exact still-pending live callback can classify creation. Failed
    /// outcome commits leave the obligation intact for read-only reconciliation.
    pub fn finish_anki_model_creation(
        &mut self,
        live: &AnkiLiveModelCreation,
        outcome: AnkiModelCreationOutcome,
    ) -> Result<(), AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recovery = read_recovery(&transaction)?.ok_or(AnkiJournalError::StaleCallback)?;
        if recovery.id != live.id {
            return Err(AnkiJournalError::StaleCallback);
        }
        let (state, model) = match outcome {
            AnkiModelCreationOutcome::NotCreated => ("not_created", None),
            AnkiModelCreationOutcome::Created(id) => ("created", Some(id)),
            AnkiModelCreationOutcome::Uncertain { known_model_id } => ("uncertain", known_model_id),
        };
        let changed = transaction.execute(
            "UPDATE anki_model_creations SET state = ?2, known_model_id = ?3
             WHERE id = ?1 AND state = 'pending'",
            params![live.id.value(), state, model.map(i64::from)],
        )?;
        if changed != 1 {
            return Err(AnkiJournalError::StaleCallback);
        }
        transaction.commit()?;
        Ok(())
    }

    /// Require the frozen full layout and, when known, the original returned ID.
    /// Missing/partial/incompatible models retain the obligation indefinitely;
    /// never repair or repeat creation. Preserve terminal payloads and IDs.
    pub fn reconcile_anki_model_creation(
        &mut self,
        id: AnkiModelCreationId,
        observation: AnkiModelReconciliation,
    ) -> Result<(), AnkiJournalError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let recovery = read_recovery(&transaction)?.ok_or(AnkiJournalError::StaleCallback)?;
        if recovery.id != id {
            return Err(AnkiJournalError::StaleCallback);
        }
        let existing = match observation {
            AnkiModelReconciliation::Observed { id, layout }
                if layout.as_ref() == recovery.creation.layout()
                    && recovery.known_model_id.is_none_or(|known| known == id) =>
            {
                Some(id)
            }
            _ => None,
        };
        let state = if existing.is_some() {
            "existing"
        } else {
            "uncertain"
        };
        let changed = transaction.execute(
            "UPDATE anki_model_creations SET state = ?2, existing_model_id = ?3 WHERE id = ?1",
            params![id.value(), state, existing.map(i64::from)],
        )?;
        if changed != 1 {
            return Err(AnkiJournalError::StaleCallback);
        }
        transaction.commit()?;
        Ok(())
    }
}

pub(crate) fn read_recovery(
    connection: &Connection,
) -> Result<Option<AnkiModelRecovery>, AnkiJournalError> {
    let mut statement = connection.prepare(
        "SELECT id, payload, known_model_id, state, existing_model_id FROM anki_model_creations
         WHERE state NOT IN ('not_created', 'created', 'existing')",
    )?;
    let mut rows = statement.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let id: i64 = row.get(0)?;
    let payload: String = row.get(1)?;
    let model: Option<i64> = row.get(2)?;
    let state: String = row.get(3)?;
    let existing: Option<i64> = row.get(4)?;
    if id <= 0
        || existing.is_some()
        || !matches!(state.as_str(), "pending" | "uncertain")
        || (state == "pending" && model.is_some())
    {
        return Err(AnkiJournalError::InvalidRecord);
    }
    let recovery = AnkiModelRecovery {
        id: AnkiModelCreationId::try_from(id).map_err(|_| AnkiJournalError::InvalidRecord)?,
        creation: serde_json::from_str(&payload).map_err(|_| AnkiJournalError::InvalidRecord)?,
        known_model_id: model
            .map(AnkiModelId::try_from)
            .transpose()
            .map_err(|_| AnkiJournalError::InvalidRecord)?,
    };
    if rows.next()?.is_some() {
        return Err(AnkiJournalError::InvalidRecord);
    }
    Ok(Some(recovery))
}
