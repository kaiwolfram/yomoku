use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    active_time::PendingReadingTime,
    document::{DocumentId, PastedDocument, TextAnchor, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

use crate::{documents::load_paste_in, Storage, StorageError};

/// A durable duration and optimistic checkpoint token for one exact text offset.
/// Load a fresh token after completion; old tokens cannot resurrect consumed time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SavedReadingTime {
    document_id: DocumentId,
    position: ReadingPosition,
    pending: PendingReadingTime,
    revision: i64,
}

impl SavedReadingTime {
    pub fn pending(self) -> PendingReadingTime {
        self.pending
    }

    pub fn position(self) -> ReadingPosition {
        self.position
    }
}

impl Storage {
    /// Restore this anchor's accumulator, initially paused with a fresh clock
    /// epoch. A zero row reserves a revision even before the first checkpoint.
    pub fn load_pending_reading_time(
        &mut self,
        document_id: DocumentId,
        position: ReadingPosition,
    ) -> Result<SavedReadingTime, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document =
            load_paste_in(&transaction, document_id)?.ok_or(StorageError::InvalidReadingTime)?;
        position
            .validate(&document.chapter)
            .map_err(|_| StorageError::InvalidReadingTime)?;
        transaction.execute(
            "INSERT INTO pending_reading_time
             (document_id, chapter_id, byte_offset, milliseconds, revision)
             VALUES (?1, ?2, ?3, 0, 1) ON CONFLICT DO NOTHING",
            (
                i64::from(document_id),
                i64::from(position.chapter_id),
                i64::from(position.offset),
            ),
        )?;
        let saved = read_anchor_in(&transaction, document_id, position)?
            .ok_or(StorageError::InvalidReadingTime)?;
        transaction.commit()?;
        Ok(saved)
    }

    /// Save an absolute checkpoint, not a delta. Duplicate delivery is harmless;
    /// an older/conflicting token is rejected instead of losing or reviving time.
    /// Checkpoint the departing anchor before any position change on the FIFO.
    pub fn save_pending_reading_time(
        &mut self,
        previous: SavedReadingTime,
        pending: PendingReadingTime,
    ) -> Result<SavedReadingTime, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = load_paste_in(&transaction, previous.document_id)?
            .ok_or(StorageError::InvalidReadingTime)?;
        let current = document.reading_position.unwrap_or(ReadingPosition {
            source_id: document.chapter.source_id,
            chapter_id: document.chapter.chapter_id,
            offset: TextOffset::START,
        });
        if current != previous.position || pending.milliseconds() < previous.pending.milliseconds()
        {
            return Err(StorageError::InvalidReadingTime);
        }
        let stored = read_anchor_in(&transaction, previous.document_id, previous.position)?
            .ok_or(StorageError::InvalidReadingTime)?;
        let next_revision = previous
            .revision
            .checked_add(1)
            .ok_or(StorageError::InvalidReadingTime)?;
        let saved = SavedReadingTime {
            pending,
            revision: next_revision,
            ..previous
        };
        if stored == saved {
            // Exact retry after a committed checkpoint; never write again.
            transaction.commit()?;
            return Ok(saved);
        }
        if stored != previous {
            return Err(StorageError::InvalidReadingTime);
        }
        transaction.execute(
            "UPDATE pending_reading_time SET milliseconds = ?4, revision = ?5
             WHERE document_id = ?1 AND chapter_id = ?2 AND byte_offset = ?3",
            (
                i64::from(previous.document_id),
                i64::from(previous.position.chapter_id),
                i64::from(previous.position.offset),
                pending.milliseconds(),
                next_revision,
            ),
        )?;
        transaction.commit()?;
        Ok(saved)
    }

    /// Freeze this total with a PageCompletion after saving its final checkpoint.
    /// Reflow only changes which page contains each stable offset; time is never
    /// copied or proportionally split. A boundary belongs to the following page,
    /// except chapter EOF, which belongs to the final page.
    pub fn pending_page_time(
        &mut self,
        document_id: DocumentId,
        page: TextAnchor,
        page_length: PageLength,
    ) -> Result<PendingReadingTime, StorageError> {
        let transaction = self.connection.transaction()?;
        let document =
            load_paste_in(&transaction, document_id)?.ok_or(StorageError::InvalidReadingTime)?;
        let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length)
            .map_err(|_| StorageError::InvalidReadingTime)?;
        if !pages.pages().contains(&page) {
            return Err(StorageError::InvalidReadingTime);
        }
        let (total, _) = page_time_in(&transaction, &document, page)?;
        transaction.commit()?;
        Ok(total)
    }
}

fn read_anchor_in(
    connection: &Connection,
    document_id: DocumentId,
    position: ReadingPosition,
) -> Result<Option<SavedReadingTime>, StorageError> {
    connection
        .query_row(
            "SELECT milliseconds, revision FROM pending_reading_time
         WHERE document_id = ?1 AND chapter_id = ?2 AND byte_offset = ?3",
            (
                i64::from(document_id),
                i64::from(position.chapter_id),
                i64::from(position.offset),
            ),
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .map(|(milliseconds, revision)| {
            if revision <= 0 {
                return Err(StorageError::InvalidReadingTime);
            }
            Ok(SavedReadingTime {
                document_id,
                position,
                revision,
                pending: PendingReadingTime::from_millis(milliseconds)
                    .map_err(|_| StorageError::InvalidReadingTime)?,
            })
        })
        .transpose()
}

pub(crate) fn page_time_in(
    connection: &Connection,
    document: &PastedDocument,
    page: TextAnchor,
) -> Result<(PendingReadingTime, Vec<SavedReadingTime>), StorageError> {
    let eof =
        i64::try_from(document.chapter.text.len()).map_err(|_| StorageError::InvalidReadingTime)?;
    let mut statement = connection.prepare(
        "SELECT byte_offset, milliseconds, revision FROM pending_reading_time
         WHERE document_id = ?1 AND chapter_id = ?2 AND byte_offset >= ?3
           AND (byte_offset < ?4 OR (byte_offset = ?4 AND byte_offset = ?5))",
    )?;
    let rows = statement.query_map(
        (
            i64::from(document.document_id),
            i64::from(page.chapter_id),
            i64::from(page.range.start),
            i64::from(page.range.end),
            eof,
        ),
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    let mut total = 0_i64;
    let mut anchors = Vec::new();
    for row in rows {
        let (offset, milliseconds, revision) = row?;
        let offset = TextOffset::try_from(offset).map_err(|_| StorageError::InvalidReadingTime)?;
        offset
            .byte_index(&document.chapter.text)
            .map_err(|_| StorageError::InvalidReadingTime)?;
        let pending = PendingReadingTime::from_millis(milliseconds)
            .map_err(|_| StorageError::InvalidReadingTime)?;
        if revision <= 0 {
            return Err(StorageError::InvalidReadingTime);
        }
        total = total
            .checked_add(milliseconds)
            .ok_or(StorageError::InvalidReadingTime)?;
        anchors.push(SavedReadingTime {
            document_id: document.document_id,
            position: ReadingPosition {
                source_id: page.source_id,
                chapter_id: page.chapter_id,
                offset,
            },
            pending,
            revision,
        });
    }
    Ok((
        PendingReadingTime::from_millis(total).map_err(|_| StorageError::InvalidReadingTime)?,
        anchors,
    ))
}

pub(crate) fn consume_time_in(
    connection: &Connection,
    anchors: &[SavedReadingTime],
) -> Result<(), StorageError> {
    for saved in anchors {
        // Skip the revision a checkpoint retry would expect. Even a zero-time
        // checkpoint must not mistake consumption for its own successful save.
        let revision = saved
            .revision
            .checked_add(2)
            .ok_or(StorageError::InvalidReadingTime)?;
        connection.execute(
            "UPDATE pending_reading_time SET milliseconds = 0, revision = ?4
             WHERE document_id = ?1 AND chapter_id = ?2 AND byte_offset = ?3",
            (
                i64::from(saved.document_id),
                i64::from(saved.position.chapter_id),
                i64::from(saved.position.offset),
                revision,
            ),
        )?;
    }
    Ok(())
}
