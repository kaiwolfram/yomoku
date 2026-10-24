use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    document::VocabularyId,
    vocabulary::{Surface, VocabularyEntry, VocabularyStatus},
};

use crate::{Storage, StorageError};

impl Storage {
    /// Register analyzer-supplied Japanese surfaces, preserving existing entries.
    /// Returns the number created, not token frequency or reading encounters.
    pub fn register_vocabulary(
        &mut self,
        surfaces: &[Surface],
        registered_at_unix_ms: i64,
    ) -> Result<usize, StorageError> {
        validate_timestamp(registered_at_unix_ms)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let created = register_vocabulary_in(&transaction, surfaces.iter(), registered_at_unix_ms)?;
        transaction.commit()?;
        Ok(created)
    }

    /// Lookup never registers entries or changes their status/history.
    pub fn vocabulary_entry(
        &self,
        surface: &Surface,
    ) -> Result<Option<VocabularyEntry>, StorageError> {
        read_entry_by_surface(&self.connection, surface)
    }

    /// Commit the status and its dated event together. Re-selecting the current
    /// status is a no-op; an absent (e.g. deleted) ID never recreates an entry.
    pub fn set_vocabulary_status(
        &mut self,
        id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    ) -> Result<Option<VocabularyEntry>, StorageError> {
        validate_timestamp(occurred_at_unix_ms)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored = transaction
            .query_row(
                "SELECT id, surface, status, registered_at_unix_ms FROM vocabulary WHERE id = ?1",
                [i64::from(id)],
                entry_columns,
            )
            .optional()?;
        let Some(mut entry) = stored.map(validate_entry).transpose()? else {
            return Ok(None);
        };
        if entry.status != status {
            transaction.execute(
                "UPDATE vocabulary SET status = ?1 WHERE id = ?2",
                (encode_status(status), i64::from(id)),
            )?;
            transaction.execute(
                "INSERT INTO vocabulary_status_events
                 (semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms)
                 VALUES (1, ?1, ?2, ?3, 'manual', ?4)",
                (i64::from(id), encode_status(entry.status), encode_status(status), occurred_at_unix_ms),
            )?;
            entry.status = status;
        }
        transaction.commit()?;
        Ok(Some(entry))
    }
}

fn validate_timestamp(timestamp: i64) -> Result<(), StorageError> {
    if timestamp < 0 {
        return Err(StorageError::InvalidVocabulary);
    }
    Ok(())
}

pub(crate) fn encode_status(status: VocabularyStatus) -> &'static str {
    match status {
        VocabularyStatus::New => "new",
        VocabularyStatus::Learning => "learning",
        VocabularyStatus::Familiar => "familiar",
        VocabularyStatus::Known => "known",
        VocabularyStatus::Ignored => "ignored",
    }
}

type EntryColumns = (i64, String, String, i64);

fn entry_columns(row: &rusqlite::Row<'_>) -> rusqlite::Result<EntryColumns> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn validate_entry(
    (id, surface, status, timestamp): EntryColumns,
) -> Result<VocabularyEntry, StorageError> {
    validate_timestamp(timestamp)?;
    let status = match status.as_str() {
        "new" => VocabularyStatus::New,
        "learning" => VocabularyStatus::Learning,
        "familiar" => VocabularyStatus::Familiar,
        "known" => VocabularyStatus::Known,
        "ignored" => VocabularyStatus::Ignored,
        _ => return Err(StorageError::InvalidVocabulary),
    };
    Ok(VocabularyEntry {
        id: VocabularyId::try_from(id).map_err(|_| StorageError::InvalidVocabulary)?,
        surface: Surface::new(surface).map_err(|_| StorageError::InvalidVocabulary)?,
        status,
        registered_at_unix_ms: timestamp,
    })
}

pub(crate) fn read_entry_by_surface(
    connection: &Connection,
    surface: &Surface,
) -> Result<Option<VocabularyEntry>, StorageError> {
    connection
        .query_row(
            "SELECT id, surface, status, registered_at_unix_ms FROM vocabulary WHERE surface = ?1",
            [surface.as_str()],
            entry_columns,
        )
        .optional()?
        .map(validate_entry)
        .transpose()
}

pub(crate) fn register_vocabulary_in<'a>(
    connection: &Connection,
    surfaces: impl IntoIterator<Item = &'a Surface>,
    registered_at_unix_ms: i64,
) -> Result<usize, StorageError> {
    validate_timestamp(registered_at_unix_ms)?;
    let mut created = 0;
    for surface in surfaces {
        if read_entry_by_surface(connection, surface)?.is_some() {
            continue;
        }
        connection.execute(
            "INSERT INTO vocabulary (surface, status, registered_at_unix_ms) VALUES (?1, 'new', ?2)",
            (surface.as_str(), registered_at_unix_ms),
        )?;
        let id = VocabularyId::try_from(connection.last_insert_rowid())
            .map_err(|_| StorageError::InvalidVocabulary)?;
        connection.execute(
            "INSERT INTO vocabulary_status_events
             (semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms)
             VALUES (1, ?1, NULL, 'new', 'registration', ?2)",
            (i64::from(id), registered_at_unix_ms),
        )?;
        created += 1;
    }
    Ok(created)
}
