//! Canonical ownership and stable live anchors, without format parsing or repair.

use rusqlite::{types::ValueRef, Connection, Row};
use yomoku_core::{
    active_time::PendingReadingTime,
    document::{ChapterId, DocumentId, SourceId, TextOffset},
};

use super::{check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    version: usize,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    if version < 2 {
        return Ok(());
    }
    check_sources(connection, cancelled)?;
    if version >= 3 {
        check_positions(connection, cancelled)?;
    }
    if version >= 9 {
        check_pending_time(connection, cancelled)?;
    }
    check_cancelled(cancelled)
}

fn check_sources(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut sources = connection.prepare(
        "SELECT s.id, s.kind, s.original_title, d.id, d.title
         FROM main.sources s LEFT JOIN main.documents d ON d.source_id = s.id ORDER BY s.id",
    )?;
    let mut chapters = connection.prepare(
        "SELECT c.id, c.chapter_index, t.text FROM main.chapters c
         LEFT JOIN main.chapter_texts t ON t.chapter_id = c.id
         WHERE c.source_id = ?1 ORDER BY c.chapter_index",
    )?;
    let mut rows = sources.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let source =
            SourceId::try_from(row.get::<_, i64>(0)?).map_err(|_| CandidateError::InvalidData)?;
        let single = match text(row, 1)? {
            "paste" | "txt" | "srt" => true,
            "epub" => false,
            _ => return Err(CandidateError::InvalidData),
        };
        // A paste's literal first line can be empty/blank; renames are independent.
        // Do not apply the newer rename-input restrictions to persisted titles.
        text(row, 2)?;
        let live = row.get_ref(3)? != ValueRef::Null;
        if live {
            DocumentId::try_from(row.get::<_, i64>(3)?).map_err(|_| CandidateError::InvalidData)?;
            text(row, 4)?;
        }
        let mut units = chapters.query([i64::from(source)])?;
        let mut count = 0_i64;
        let mut total_bytes = 0_i64;
        while let Some(unit) = units.next()? {
            check_cancelled(cancelled)?;
            ChapterId::try_from(unit.get::<_, i64>(0)?).map_err(|_| CandidateError::InvalidData)?;
            if unit.get::<_, i64>(1)? != count {
                return Err(CandidateError::InvalidData);
            }
            count = count.checked_add(1).ok_or(CandidateError::InvalidData)?;
            if live {
                let canonical = text(unit, 2)?;
                // All existing import adapters omit empty text units, and the
                // legacy single-text reader requires nonblank PasteText.
                if canonical.trim().is_empty() {
                    return Err(CandidateError::InvalidData);
                }
                let bytes =
                    i64::try_from(canonical.len()).map_err(|_| CandidateError::InvalidData)?;
                total_bytes = total_bytes
                    .checked_add(bytes)
                    .ok_or(CandidateError::InvalidData)?;
            } else if unit.get_ref(2)? != ValueRef::Null {
                // Deletion retains chapter/source identities but removes text.
                return Err(CandidateError::InvalidData);
            }
        }
        if count == 0 || single && count != 1 {
            return Err(CandidateError::InvalidData);
        }
    }
    Ok(())
}

fn check_positions(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT d.source_id, c.source_id, t.text, p.byte_offset
         FROM main.reading_positions p JOIN main.documents d ON d.id = p.document_id
         JOIN main.chapters c ON c.id = p.chapter_id
         LEFT JOIN main.chapter_texts t ON t.chapter_id = c.id ORDER BY p.document_id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        check_anchor(row)?;
    }
    // Missing rows are valid for unopened and historical documents. Never add a
    // default row, move an EOF position or reinterpret it as a completion.
    Ok(())
}

fn check_pending_time(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT d.source_id, c.source_id, t.text, p.byte_offset, p.milliseconds, p.revision
         FROM main.pending_reading_time p JOIN main.documents d ON d.id = p.document_id
         JOIN main.chapters c ON c.id = p.chapter_id
         LEFT JOIN main.chapter_texts t ON t.chapter_id = c.id
         ORDER BY p.document_id, p.chapter_id, p.byte_offset",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        check_anchor(row)?;
        PendingReadingTime::from_millis(row.get(4)?).map_err(|_| CandidateError::InvalidData)?;
        if row.get::<_, i64>(5)? <= 0 {
            return Err(CandidateError::InvalidData);
        }
        // Keep all anchors across movement/reflow, including zero-duration
        // revision tombstones and exhausted revisions. Do not replay checkpoints.
    }
    Ok(())
}

fn check_anchor(row: &Row<'_>) -> Result<(), CandidateError> {
    if row.get::<_, i64>(0)? != row.get::<_, i64>(1)? {
        return Err(CandidateError::InvalidData);
    }
    let canonical = text(row, 2)?;
    let offset =
        TextOffset::try_from(row.get::<_, i64>(3)?).map_err(|_| CandidateError::InvalidData)?;
    offset
        .byte_index(canonical)
        .map_err(|_| CandidateError::InvalidData)?;
    Ok(())
}

fn text<'row>(row: &'row Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}
