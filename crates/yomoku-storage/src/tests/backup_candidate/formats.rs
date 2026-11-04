use sha2::{Digest, Sha256};
use yomoku_core::document::PasteText;
use yomoku_import::txt::TextEncoding;

use super::*;
use crate::tests::imports::fixture;

const TXT: &str = "\u{feff}｜𠮷《よし》 か\u{3099}\0\r\n｜猫《ねこ》［＃改ページ］";
const SRT: &str = "\u{feff}1\r\n00:01,0 --> 00:02,0\r\n<i>𠮷</i> &amp; 猫\r\n\r\n2\r\n00:03 --> 00:04\r\n｜犬《いぬ》\0";

fn setup() -> (TestDatabase, Storage) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, MIGRATIONS).unwrap();
    let mut storage = Storage { connection };
    for _ in 0..2 {
        storage
            .create_paste(&PasteText::new("\n か\u{3099}𠮷\0\r\n".into()).unwrap())
            .unwrap();
    }

    storage
        .import_analyzed_txt(
            TXT.as_bytes(),
            "原本.txt",
            Some(TextEncoding::Utf8),
            1,
            fixture,
        )
        .unwrap();

    storage
        .import_analyzed_srt(
            SRT.as_bytes(),
            "原本.srt",
            Some(TextEncoding::Utf8),
            1,
            fixture,
        )
        .unwrap();

    storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p><ruby>猫<rt>ねこ</rt></ruby> <em>犬</em></p>"),
            "原本.epub",
            1,
            fixture,
        )
        .unwrap();

    (database, storage)
}

fn accept_prefix(database: &TestDatabase, version: usize) {
    let bytes = fs::read(database.path()).unwrap();
    let expected_path = database.0.join("expected.sqlite3");
    fs::write(&expected_path, &bytes).unwrap();
    let expected = Storage::open(&expected_path).unwrap();
    let expected = super::super::backup_snapshot::state(&expected.connection);
    let directory = database.0.join("accepted");
    let checked = candidate(&directory, &bytes)
        .check_schema(version as u32, bytes.len() as u64, || false)
        .unwrap_or_else(|error| panic!("schema {version}: {error}"));
    let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
    assert_eq!(super::super::backup_snapshot::state(&copy), expected);
    drop(copy);
    checked.discard().unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn backup_candidate_formats_preserve_exact_files_annotations_and_paste_hashes() {
    {
        let version = 1;
        let (database, storage) = setup();
        drop(storage);
        accept_prefix(&database, version);
    }
}

#[test]
fn backup_candidate_formats_reject_wrong_paste_hashes_and_file_hash_ownership() {
    {
        let version = 1;
        for sql in [
            "UPDATE paste_content_hashes SET sha256 = zeroblob(32)",
            "UPDATE chapter_texts SET text = 'different literal bytes' WHERE chapter_id = 1",
        ] {
            let (database, storage) = setup();
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
    for document in [3, 4, 5] {
        let (database, storage) = setup();
        storage
            .connection
            .execute(
                "INSERT INTO paste_content_hashes VALUES (?1, zeroblob(32))",
                [document],
            )
            .unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_formats_require_file_records_matching_kind_encoding_and_exact_digest() {
    {
        let version = 1;
        for sql in [
            "DELETE FROM imported_files WHERE document_id = 3",
            "UPDATE imported_files SET sha256 = zeroblob(32) WHERE document_id = 3",
            "UPDATE imported_files SET original_bytes = X'ff' WHERE document_id = 3",
            "UPDATE imported_files SET document_id = 1 WHERE document_id = 3",
            "UPDATE imported_files SET text_encoding = 'shift_jis' WHERE document_id = 3",
        ] {
            let (database, storage) = setup();
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
    for sql in [
        "UPDATE imported_files SET text_encoding = 'epub' WHERE document_id = 3",
        "UPDATE imported_files SET text_encoding = 'utf8' WHERE document_id = 5",
        "DELETE FROM imported_files WHERE document_id = 5",
        "UPDATE imported_files SET sha256 = zeroblob(32) WHERE document_id = 5",
        "UPDATE imported_files SET original_bytes = X'00' WHERE document_id = 5",
        "DELETE FROM epub_documents WHERE document_id = 5",
        "UPDATE epub_documents SET document_id = 1 WHERE document_id = 5",
    ] {
        let (database, storage) = setup();
        // Clear dependent EPUB rows to keep FKs valid while testing header ownership.
        if sql.contains("epub_documents") {
            storage.connection.execute_batch("DELETE FROM epub_inline_images; DELETE FROM epub_spans; DELETE FROM epub_chapters; DELETE FROM epub_assets;").unwrap();
        }
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_formats_reject_undecodable_or_different_text_even_with_updated_hash() {
    for (document, bytes) in [
        (3, b"different TXT".as_slice()),
        (3, &[0xff]),
        (4, b"different SRT".as_slice()),
        (4, &[0xff]),
        (4, b"  ".as_slice()),
    ] {
        let (database, storage) = setup();
        storage
            .connection
            .execute(
                "UPDATE imported_files SET original_bytes = ?1, sha256 = ?2 WHERE document_id = ?3",
                (bytes, Sha256::digest(bytes).as_slice(), document),
            )
            .unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_formats_reject_missing_extra_changed_or_invalid_txt_ruby() {
    {
        let version = 1;
        for sql in [
            "DELETE FROM source_ruby WHERE chapter_id = 3",
            "DELETE FROM source_ruby WHERE chapter_id = 3 AND ruby_index = 1",
            "UPDATE source_ruby SET ruby_index = ruby_index + 10 WHERE chapter_id = 3",
            "UPDATE source_ruby SET start_byte = 1 WHERE chapter_id = 3 AND ruby_index = 0",
            "UPDATE source_ruby SET end_byte = 9999 WHERE chapter_id = 3 AND ruby_index = 0",
            "UPDATE source_ruby SET start_byte = 0 WHERE chapter_id = 3 AND ruby_index = 1",
            "UPDATE source_ruby SET reading = 'changed' WHERE chapter_id = 3",
            "UPDATE source_ruby SET reading = CAST(X'ff' AS TEXT) WHERE chapter_id = 3",
            "INSERT INTO source_ruby VALUES (3, 2, 0, 4, 'extra')",
            "INSERT INTO source_ruby VALUES (1, 0, 2, 5, 'foreign')",
        ] {
            let (database, storage) = setup();
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
    let (database, storage) = setup();
    storage
        .connection
        .execute_batch("INSERT INTO source_ruby VALUES (4, 0, 0, 4, 'foreign')")
        .unwrap();
    check_rejected(&database, 1, CandidateError::InvalidData);
}

#[test]
fn backup_candidate_formats_reject_missing_extra_changed_or_foreign_srt_cues() {
    {
        let version = 1;
        for sql in [
            "DELETE FROM subtitle_cues",
            "DELETE FROM subtitle_cues WHERE cue_index = 1",
            "UPDATE subtitle_cues SET cue_index = cue_index + 10",
            "UPDATE subtitle_cues SET start_byte = 1 WHERE cue_index = 0",
            "UPDATE subtitle_cues SET end_byte = 9999 WHERE cue_index = 0",
            "UPDATE subtitle_cues SET start_byte = 0 WHERE cue_index = 1",
            "INSERT INTO subtitle_cues VALUES (4, 2, 0, 4)",
            "INSERT INTO subtitle_cues VALUES (1, 0, 2, 5)",
            "INSERT INTO subtitle_cues VALUES (3, 0, 0, 4)",
        ] {
            let (database, storage) = setup();
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
    let (database, storage) = setup();
    storage
        .connection
        .execute_batch("INSERT INTO subtitle_cues VALUES (5, 0, 0, 3)")
        .unwrap();
    check_rejected(&database, 1, CandidateError::InvalidData);
}

#[test]
fn backup_candidate_formats_validate_epub_ruby_ranges_without_reinterpreting_readings() {
    for sql in [
        "UPDATE source_ruby SET ruby_index = 10 WHERE chapter_id = 5",
        "UPDATE source_ruby SET start_byte = 1 WHERE chapter_id = 5",
        "UPDATE source_ruby SET end_byte = 9999 WHERE chapter_id = 5",
        "UPDATE source_ruby SET reading = CAST(X'ff' AS TEXT) WHERE chapter_id = 5",
        "INSERT INTO source_ruby VALUES (5, 1, 0, 3, 'overlap')",
    ] {
        let (database, storage) = setup();
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
    let (database, storage) = setup();
    storage
        .connection
        .execute(
            "UPDATE source_ruby SET reading = ?1 WHERE chapter_id = 5",
            ["　\0\n"],
        )
        .unwrap();
    accept_unchanged(&database, 1);
}

#[test]
fn backup_candidate_formats_preserve_explicit_shift_jis_renames_and_deleted_file_provenance() {
    let (database, mut storage) = setup();
    for (bytes, srt) in [
        ([0x94, 0x4c].as_slice(), false),
        ([0x8c, 0xa2].as_slice(), true),
        // Valid under both encodings: retain the explicit saved choice.
        (b"ASCII TXT".as_slice(), false),
        (b"ASCII SRT".as_slice(), true),
    ] {
        if srt {
            storage
                .import_analyzed_srt(bytes, "shift.srt", Some(TextEncoding::ShiftJis), 0, fixture)
                .unwrap();
        } else {
            storage
                .import_analyzed_txt(bytes, "shift.txt", Some(TextEncoding::ShiftJis), 0, fixture)
                .unwrap();
        }
    }
    storage
        .rename_document(3.try_into().unwrap(), " renamed\0.txt ")
        .unwrap();
    // Deletion removes format bytes/annotations, retaining source/chapter history.
    for id in [4, 5] {
        storage.delete_document(id.try_into().unwrap()).unwrap();
    }
    accept_unchanged(&database, 1);
}

#[test]
fn backup_candidate_formats_cancel_all_populated_validation_and_hash_chunk_boundaries() {
    {
        let version = 1;
        let (database, storage) = setup();
        // Multiple hash chunks, with no decoder allocation for a paste.
        let large = "か".repeat(710000);
        storage
            .connection
            .execute(
                "UPDATE chapter_texts SET text = ?1 WHERE chapter_id = 1",
                [&large],
            )
            .unwrap();
        storage
            .connection
            .execute(
                "UPDATE paste_content_hashes SET sha256 = ?1 WHERE document_id = 1",
                [Sha256::digest(large.as_bytes()).as_slice()],
            )
            .unwrap();
        drop(storage);
        let bytes = fs::read(database.path()).unwrap();
        let mut polls = 0;
        candidate(&database.0.join("count"), &bytes)
            .check_schema(version as u32, bytes.len() as u64, || {
                polls += 1;
                false
            })
            .unwrap()
            .discard()
            .unwrap();
        for stop in 1..=polls {
            let directory = database.0.join("cancel");
            let mut calls = 0;
            assert!(matches!(
                candidate(&directory, &bytes).check_schema(
                    version as u32,
                    bytes.len() as u64,
                    || {
                        calls += 1;
                        calls == stop
                    }
                ),
                Err(CandidateError::Cancelled)
            ));
            assert!(!directory.exists());
        }
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}
