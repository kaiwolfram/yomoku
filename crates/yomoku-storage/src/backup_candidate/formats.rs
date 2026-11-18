//! Saved file identity metadata and canonical text integrity.

use rusqlite::{types::ValueRef, Connection, Row};
use sha2::{Digest, Sha256};

use super::{check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    check_pastes(connection, cancelled)?;

    check_files(connection, cancelled)?;

    let mut statement = connection.prepare(
        "SELECT s.kind, e.document_id FROM main.documents d
             JOIN main.sources s ON s.id = d.source_id
             LEFT JOIN main.epub_documents e ON e.document_id = d.id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        require((text(row, 0)? == "epub") == (row.get_ref(1)? != ValueRef::Null))?;
    }

    check_cancelled(cancelled)
}

fn check_pastes(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT s.kind, t.text, h.sha256 FROM main.paste_content_hashes h
         JOIN main.documents d ON d.id = h.document_id
         JOIN main.sources s ON s.id = d.source_id
         JOIN main.chapters c ON c.source_id = s.id AND c.chapter_index = 0
         JOIN main.chapter_texts t ON t.chapter_id = c.id ORDER BY h.document_id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        require(text(row, 0)? == "paste")?;
        check_hash(text(row, 1)?.as_bytes(), blob(row, 2)?, cancelled)?;
    }
    let missing: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM main.documents d JOIN main.sources s ON s.id = d.source_id
         LEFT JOIN main.paste_content_hashes h ON h.document_id = d.id
         WHERE s.kind = 'paste' AND h.document_id IS NULL)",
        [],
        |row| row.get(0),
    )?;
    require(!missing)?;
    Ok(())
}

fn check_files(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT s.kind, f.document_id, f.sha256, f.text_encoding, f.canonical_text_sha256,
                t.text
         FROM main.documents d JOIN main.sources s ON s.id = d.source_id
         LEFT JOIN main.imported_files f ON f.document_id = d.id
         JOIN main.chapters c ON c.source_id = s.id AND c.chapter_index = 0
         JOIN main.chapter_texts t ON t.chapter_id = c.id ORDER BY d.id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let kind = text(row, 0)?;
        if kind == "paste" {
            require(row.get_ref(1)? == ValueRef::Null)?;
            continue;
        }
        require(row.get_ref(1)? != ValueRef::Null)?;
        // The input digest identifies duplicates. Only canonical content is retained.
        require(blob(row, 2)?.len() == 32)?;
        match (kind, text(row, 3)?) {
            ("epub", "epub") => require(row.get_ref(4)? == ValueRef::Null)?,
            ("txt" | "srt", "utf8" | "shift_jis") => {
                check_hash(text(row, 5)?.as_bytes(), blob(row, 4)?, cancelled)?;
            }
            _ => return Err(CandidateError::InvalidData),
        }
    }
    Ok(())
}

fn check_hash(
    bytes: &[u8],
    expected: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut digest = Sha256::new();
    for chunk in bytes.chunks(1024 * 1024) {
        check_cancelled(cancelled)?;
        digest.update(chunk);
    }
    require(digest.finalize().as_slice() == expected)
}

fn text<'row>(row: &'row Row<'_>, column: usize) -> Result<&'row str, CandidateError> {
    row.get_ref(column)?
        .as_str()
        .map_err(|_| CandidateError::InvalidData)
}

fn blob<'row>(row: &'row Row<'_>, column: usize) -> Result<&'row [u8], CandidateError> {
    row.get_ref(column)?
        .as_blob()
        .map_err(|_| CandidateError::InvalidData)
}

fn require(valid: bool) -> Result<(), CandidateError> {
    if valid {
        Ok(())
    } else {
        Err(CandidateError::InvalidData)
    }
}
