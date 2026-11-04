//! Exact saved file identity and text annotations; never import or allocate IDs.

use rusqlite::{types::ValueRef, Connection, Row};
use sha2::{Digest, Sha256};
use yomoku_import::{
    srt,
    txt::{self, SourceRuby, TextEncoding},
};

use super::{analyses::slice, check_cancelled, CandidateError};

pub(super) fn check(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    check_pastes(connection, cancelled)?;

    check_files(connection, cancelled)?;
    check_ruby(connection, cancelled)?;

    // Each SRT's full cue list is compared below. Reject foreign ownership
    // separately so an otherwise valid cue on a TXT/paste/EPUB cannot hide.
    let mut statement = connection.prepare(
        "SELECT s.kind FROM main.subtitle_cues q
             JOIN main.chapters c ON c.id = q.chapter_id
             JOIN main.sources s ON s.id = c.source_id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        require(text(row, 0)? == "srt")?;
    }

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
        "SELECT s.kind, f.document_id, f.sha256, f.original_bytes, f.text_encoding,
                c.id, t.text
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
        let bytes = blob(row, 3)?;
        check_hash(bytes, blob(row, 2)?, cancelled)?;
        let encoding = text(row, 4)?;
        if kind == "epub" {
            require(encoding == "epub")?;
            // Container/asset/chapter metadata validation is a separate group.
            continue;
        }
        let encoding = match encoding {
            "utf8" => TextEncoding::Utf8,
            "shift_jis" => TextEncoding::ShiftJis,
            _ => return Err(CandidateError::InvalidData),
        };
        let chapter = row.get::<_, i64>(5)?;
        let canonical = text(row, 6)?;
        check_cancelled(cancelled)?;
        // Pure adapters derive expected content using the saved explicit choice.
        // No dedup, analyzer, identity allocation, writes or external resolution.
        match kind {
            "txt" => {
                let parsed =
                    txt::parse(bytes, Some(encoding)).map_err(|_| CandidateError::InvalidData)?;
                check_cancelled(cancelled)?;
                require(parsed.text == canonical)?;
                match_ruby(connection, chapter, &parsed.ruby, cancelled)?;
            }
            "srt" => {
                let (_, decoded) =
                    txt::decode(bytes, Some(encoding)).map_err(|_| CandidateError::InvalidData)?;
                check_cancelled(cancelled)?;
                let parsed = srt::parse(&decoded).map_err(|_| CandidateError::InvalidData)?;
                check_cancelled(cancelled)?;
                require(parsed.text == canonical)?;
                let mut cues = connection.prepare(
                    "SELECT cue_index, start_byte, end_byte FROM main.subtitle_cues
                     WHERE chapter_id = ?1 ORDER BY cue_index",
                )?;
                let mut cues = cues.query([chapter])?;
                for (index, expected) in parsed.cues.iter().enumerate() {
                    check_cancelled(cancelled)?;
                    let cue = cues.next()?.ok_or(CandidateError::InvalidData)?;
                    require(
                        cue.get::<_, i64>(0)? == to_i64(index)?
                            && cue.get::<_, i64>(1)? == to_i64(expected.start)?
                            && cue.get::<_, i64>(2)? == to_i64(expected.end)?,
                    )?;
                }
                require(cues.next()?.is_none())?;
            }
            _ => return Err(CandidateError::InvalidData),
        }
    }
    Ok(())
}

fn match_ruby(
    connection: &Connection,
    chapter: i64,
    expected: &[SourceRuby],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT ruby_index, start_byte, end_byte, reading FROM main.source_ruby
         WHERE chapter_id = ?1 ORDER BY ruby_index",
    )?;
    let mut rows = statement.query([chapter])?;
    for (index, expected) in expected.iter().enumerate() {
        check_cancelled(cancelled)?;
        let row = rows.next()?.ok_or(CandidateError::InvalidData)?;
        require(
            row.get::<_, i64>(0)? == to_i64(index)?
                && row.get::<_, i64>(1)? == to_i64(expected.range.start)?
                && row.get::<_, i64>(2)? == to_i64(expected.range.end)?
                && text(row, 3)? == expected.reading,
        )?;
    }
    require(rows.next()?.is_none())
}

fn check_ruby(
    connection: &Connection,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), CandidateError> {
    let mut statement = connection.prepare(
        "SELECT r.chapter_id, s.kind, t.text, r.ruby_index, r.start_byte, r.end_byte, r.reading
         FROM main.source_ruby r JOIN main.chapters c ON c.id = r.chapter_id
         JOIN main.sources s ON s.id = c.source_id
         JOIN main.chapter_texts t ON t.chapter_id = c.id ORDER BY r.chapter_id, r.ruby_index",
    )?;
    let mut rows = statement.query([])?;
    let mut chapter = None;
    let mut index = 0_i64;
    let mut previous_end = 0;
    while let Some(row) = rows.next()? {
        check_cancelled(cancelled)?;
        let id = row.get::<_, i64>(0)?;
        if chapter != Some(id) {
            chapter = Some(id);
            index = 0;
            previous_end = 0;
        }
        let start = row.get::<_, i64>(4)?;
        let end = row.get::<_, i64>(5)?;
        // Existing validate_ruby permits literal whitespace/NUL readings.
        require(
            matches!(text(row, 1)?, "txt" | "epub")
                && row.get::<_, i64>(3)? == index
                && start >= previous_end
                && end > start
                && !slice(text(row, 2)?, start, end)?.is_empty()
                && !text(row, 6)?.is_empty(),
        )?;
        index = index.checked_add(1).ok_or(CandidateError::InvalidData)?;
        previous_end = end;
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

fn to_i64(value: usize) -> Result<i64, CandidateError> {
    i64::try_from(value).map_err(|_| CandidateError::InvalidData)
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
