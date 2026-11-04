//! Partial application preflight. Runs only after exact schema/integrity/FKs,
//! before and after private migration. Never creates Storage or live Anki tokens.

use std::collections::BTreeMap;

use rusqlite::{types::ValueRef, Connection, Row};

use super::{check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    reference: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    check_sequences(connection, reference, cancelled)?;
    for table in ["reader_settings", "library_read_clock", "anki_settings"] {
        check_cancelled(cancelled)?;

        // These identifiers are bundled constants. CHECK(id = 1) and value
        // domains were already checked; SQL constraints do not require a row.
        let count: i64 = connection.query_row(
            &format!("SELECT COUNT(*) FROM main.{table} WHERE id = 1"),
            [],
            |row| row.get(0),
        )?;
        if count != 1 {
            return Err(CandidateError::InvalidData);
        }
    }

    let clock: i64 = connection.query_row(
        "SELECT sequence FROM main.library_read_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let mut statement = connection.prepare(
        "SELECT last_read_sequence FROM main.documents
             WHERE last_read_sequence IS NOT NULL ORDER BY last_read_sequence",
    )?;
    let mut rows = statement.query([])?;
    let mut previous = 0;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let sequence: i64 = row.get(0)?;
        if sequence <= previous || sequence > clock {
            return Err(CandidateError::InvalidData);
        }
        previous = sequence;
    }

    check_cancelled(cancelled)
}

fn check_sequences(
    connection: &Connection,
    reference: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    // Derive identifiers ONLY from the trusted bundled reference. No imported
    // table names become SQL, and candidate name/sequence values stay borrowed.
    let mut statement = reference.prepare(
        "SELECT name FROM main.sqlite_schema WHERE type = 'table' AND sql LIKE '%AUTOINCREMENT%'",
    )?;
    let mut rows = statement.query([])?;
    let mut required = BTreeMap::new();
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let name: String = row.get(0)?;
        let maximum: Option<i64> = connection.query_row(
            &format!("SELECT MAX(id) FROM main.\"{}\"", name.replace('"', "\"\"")),
            [],
            |row| row.get(0),
        )?;
        required.insert(name, (maximum.unwrap_or(0), false));
    }
    let mut statement = connection.prepare("SELECT name, seq FROM main.sqlite_sequence")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let name = text(row, 0)?;
        let ValueRef::Integer(sequence) = row.get_ref(1)? else {
            return Err(CandidateError::InvalidData);
        };
        let (minimum, seen) = required.get_mut(name).ok_or(CandidateError::InvalidData)?;
        if *seen || sequence < 0 || sequence < *minimum {
            return Err(CandidateError::InvalidData);
        }
        *seen = true;
    }
    if required
        .values()
        .any(|(minimum, seen)| *minimum > 0 && !seen)
    {
        return Err(CandidateError::InvalidData);
    }
    // Empty tables may lack an entry or retain a deleted-ID high-water mark.
    // i64::MAX is valid exhausted state; never repair/reset allocation history.
    Ok(())
}

fn text<'row>(row: &'row Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}
