use yomoku_core::document::PasteText;

use super::*;

fn prefix() -> (TestDatabase, Connection) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, MIGRATIONS).unwrap();
    for text in ["\n 𠮷か\u{3099}。\r\n", "二つ目"] {
        crate::documents::create_paste_in(&connection, &PasteText::new(text.into()).unwrap())
            .unwrap();
    }
    (database, connection)
}

#[test]
fn backup_candidate_documents_require_complete_live_text_and_deleted_provenance() {
    {
        let version = 1;
        for sql in [
            "DELETE FROM chapter_texts WHERE chapter_id = 1",
            "DELETE FROM chapter_texts WHERE chapter_id = 1; DELETE FROM chapters WHERE id = 1",
            "DELETE FROM documents WHERE id = 1", // Unowned retained text.
            "UPDATE chapter_texts SET text = '' WHERE chapter_id = 1",
            "UPDATE chapter_texts SET text = '　 \n' WHERE chapter_id = 1",
            // A source allocated without any canonical chapter is incomplete,
            // even when there is no live document.
            "INSERT INTO sources (kind, original_title) VALUES ('paste', 'orphan')",
        ] {
            let (database, connection) = prefix();
            connection.execute_batch(sql).unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_documents_require_contiguous_and_format_appropriate_chapters() {
    {
        let version = 1;
        for sql in [
            "UPDATE chapters SET chapter_index = 2 WHERE id = 1",
            "INSERT INTO chapters (source_id, chapter_index) VALUES (1, 1);
             INSERT INTO chapter_texts VALUES (last_insert_rowid(), 'extra')",
            // Deleted sources retain the same complete chapter ordering.
            "DELETE FROM chapter_texts WHERE chapter_id = 1; DELETE FROM documents WHERE id = 1;
             UPDATE chapters SET chapter_index = 1 WHERE id = 1",
        ] {
            let (database, connection) = prefix();
            connection.execute_batch(sql).unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
    // Multi-chapter EPUB provenance remains after deletion without live metadata.
    for sql in [
        "UPDATE chapters SET chapter_index = 2 WHERE source_id = 1 AND chapter_index = 1",
        "DELETE FROM chapters WHERE source_id = 1 AND chapter_index = 0",
    ] {
        let (database, connection) = prefix();
        connection
            .execute_batch(
                "UPDATE sources SET kind = 'epub' WHERE id = 1;
             INSERT INTO chapters (source_id, chapter_index) VALUES (1, 1);
             DELETE FROM chapter_texts WHERE chapter_id = 1;
             DELETE FROM documents WHERE id = 1;",
            )
            .unwrap();
        connection.execute_batch(sql).unwrap();
        drop(connection);
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_documents_reject_invalid_utf8_and_nontext_canonical_fields() {
    {
        let version = 1;
        for sql in [
            "UPDATE sources SET original_title = CAST(X'ff' AS TEXT) WHERE id = 1",
            "UPDATE documents SET title = CAST(X'ff' AS TEXT) WHERE id = 1",
            "UPDATE chapter_texts SET text = CAST(X'ff' AS TEXT) WHERE chapter_id = 1",
            // These original tables are not STRICT; affinity does not reject BLOBs.
            "UPDATE sources SET original_title = X'ff' WHERE id = 1",
            "UPDATE documents SET title = X'ff' WHERE id = 1",
            "UPDATE chapter_texts SET text = X'ff' WHERE chapter_id = 1",
            // Deleted provenance still needs valid UTF-8 metadata.
            "DELETE FROM chapter_texts WHERE chapter_id = 1; DELETE FROM documents WHERE id = 1;
             UPDATE sources SET original_title = CAST(X'ff' AS TEXT) WHERE id = 1",
        ] {
            let (database, connection) = prefix();
            connection.execute_batch(sql).unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_documents_check_position_ownership_range_and_scalar_boundaries() {
    {
        let version = 1;
        for (chapter, offset) in [(2, 0), (1, 3), (1, 5), (1, 999), (1, i64::MAX)] {
            let (database, connection) = prefix();
            connection
                .execute(
                    "INSERT INTO reading_positions VALUES (1, ?1, ?2)",
                    (chapter, offset),
                )
                .unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_documents_migrate_all_populated_prefixes_without_repairing_legacy_positions() {
    {
        let version = 1;
        let (database, connection) = prefix();
        let literal = "\n 𠮷か\u{3099}。\r\n";

        connection
            .execute(
                "INSERT INTO reading_positions VALUES (1, 1, ?1)",
                [literal.len() as i64],
            )
            .unwrap();

        drop(connection);
        let before = fs::read(database.path()).unwrap();
        let directory = database.0.join("upgrade");
        let checked = candidate(&directory, &before)
            .check_schema(version as u32, before.len() as u64, || false)
            .unwrap_or_else(|error| panic!("version {version}: {error}"));
        let copy = Storage::open(directory.join("snapshot.sqlite3")).unwrap();
        let document = crate::documents::load_paste_in(&copy.connection, 1.try_into().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(document.title, ""); // Literal empty first line is valid.
        assert_eq!(document.chapter.text, literal);
        assert_eq!(
            document.reading_position.map(|p| i64::from(p.offset)),
            true.then_some(literal.len() as i64)
        );
        let absent: i64 = copy
            .connection
            .query_row(
                "SELECT COUNT(*) FROM reading_positions WHERE document_id = 2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(absent, 0);

        drop(copy);
        checked.discard().unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn backup_candidate_documents_preserve_renames_literal_text_and_deleted_sources() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let literal = "\n　か\u{3099}𠮷\0<&>\r\n ";
    let document = storage
        .create_paste(&PasteText::new(literal.into()).unwrap())
        .unwrap();
    storage
        .rename_document(document.document_id, " renamed\0 title ")
        .unwrap();
    let erased = storage
        .create_paste(&PasteText::new("erased".into()).unwrap())
        .unwrap();
    storage.delete_document(erased.document_id).unwrap();
    // Retained multi-chapter EPUB provenance needs no live text/format records.
    storage
        .connection
        .execute_batch(
            "UPDATE sources SET kind = 'epub' WHERE id = 2;
         INSERT INTO chapters (source_id, chapter_index) VALUES (2, 1);",
        )
        .unwrap();
    accept_unchanged(&database, 1);
}

#[test]
fn backup_candidate_documents_accept_every_scalar_position_including_eof() {
    for offset in "\n 𠮷か\u{3099}。\r\n"
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once("\n 𠮷か\u{3099}。\r\n".len()))
    {
        let (database, connection) = prefix();
        connection
            .execute(
                "INSERT INTO reading_positions VALUES (1, 1, ?1)",
                [offset as i64],
            )
            .unwrap();

        drop(connection);
        accept_unchanged(&database, 1);
    }
}

#[test]
fn backup_candidate_documents_preserve_full_i64_identities() {
    for id in [9007199254740993_i64, i64::MAX] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        for table in ["sources", "chapters", "documents"] {
            storage
                .connection
                .execute(
                    "INSERT INTO sqlite_sequence (name, seq) VALUES (?1, ?2)",
                    (table, id - 1),
                )
                .unwrap();
        }
        let document = storage
            .create_paste(&PasteText::new("𠮷".into()).unwrap())
            .unwrap();
        assert_eq!(i64::from(document.document_id), id);
        assert_eq!(i64::from(document.chapter.source_id), id);
        assert_eq!(i64::from(document.chapter.chapter_id), id);
        storage
            .connection
            .execute("INSERT INTO reading_positions VALUES (?1, ?1, 4)", [id])
            .unwrap();

        accept_unchanged(&database, 1);
    }
}

#[test]
fn backup_candidate_documents_cancel_every_populated_validation_poll() {
    {
        let version = 1;
        let (database, connection) = prefix();
        connection
            .execute_batch("INSERT INTO reading_positions VALUES (1, 1, 0)")
            .unwrap();

        drop(connection);
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
                    },
                ),
                Err(CandidateError::Cancelled)
            ));
            assert!(!directory.exists());
            assert_eq!(fs::read(database.path()).unwrap(), bytes);
        }
    }
}
