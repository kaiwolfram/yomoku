//! Validate saved spelling/entry domains without live content or dictionaries.

use rusqlite::Connection;
use yomoku_core::{dictionary::CustomDictionaryEntry, vocabulary::Surface};

use super::{check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    version: usize,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    if version >= 6 {
        let mut statement = connection.prepare(
            "SELECT id, surface, status, registered_at_unix_ms FROM main.vocabulary ORDER BY id",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            check_cancelled(cancelled)?;
            // Reuse the ordinary repository's typed ID, exact Surface, status
            // and date checks. In particular SQL's nonempty test permits blanks.
            let columns =
                crate::vocabulary::entry_columns(row).map_err(|_| CandidateError::InvalidData)?;
            crate::vocabulary::validate_entry(columns).map_err(|_| CandidateError::InvalidData)?;
        }
    }
    if version >= 17 {
        let mut statement = connection.prepare(
            "SELECT surface, semantics_version, english_meaning, reading
             FROM main.custom_dictionary_entries ORDER BY surface",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            check_cancelled(cancelled)?;
            if row.get::<_, i64>(1)? != 1 {
                return Err(CandidateError::InvalidData);
            }
            let surface = Surface::new(text(row, 0)?).map_err(|_| CandidateError::InvalidData)?;
            CustomDictionaryEntry::new(surface, text(row, 2)?.into(), text(row, 3)?.into())
                .map_err(|_| CandidateError::InvalidData)?;
            // No vocabulary/content FK or dictionary lookup: user entries can
            // survive vocabulary/document deletion and dictionary asset changes.
        }
    }
    check_cancelled(cancelled)
}

fn text<'row>(row: &'row rusqlite::Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}
