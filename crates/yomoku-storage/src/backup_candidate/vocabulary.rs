//! Validate saved spelling/entry domains without live content or dictionaries.

use rusqlite::Connection;

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

    check_cancelled(cancelled)
}
