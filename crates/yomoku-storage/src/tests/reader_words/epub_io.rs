use super::*;
use crate::{
    tests::{epub_fixture, reader_work},
    AnalyzedEpub, EpubImportOutcome,
};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use sha2::{Digest, Sha256};
use yomoku_core::{
    completion::PageCompletionIntent, pagination::ReadingPosition, reader::ReaderPage,
};

fn epub(storage: &mut Storage, size: usize) -> AnalyzedEpub {
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>試験語 試験語 別語</p>"),
            "本.epub",
            1,
            fixture,
        )
        .unwrap()
    else {
        panic!("expected EPUB")
    };
    // Only chapter one references this image. Chapter two has identical text at
    // both sizes; no book/image bytes are checked into the repository.
    let bytes = vec![42; size];
    storage
        .connection
        .execute(
            "UPDATE epub_assets SET bytes = ?1, sha256 = ?2 WHERE path = 'OPS/images/挿絵.png'",
            (&bytes, Sha256::digest(&bytes).as_slice()),
        )
        .unwrap();
    *imported
}

fn page_key(storage: &mut Storage, imported: &AnalyzedEpub) -> EffectiveReaderTokenKey {
    let id = imported.document.document_id;
    let snapshot = storage.load_document_reader_snapshot(id).unwrap().unwrap();
    let chapter = &snapshot.document.chapters[1];
    let page = ReaderPage::at_document(
        &snapshot.document,
        ReadingPosition {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            offset: TextOffset::START,
        },
        snapshot.page_length,
    )
    .unwrap();
    storage
        .load_effective_reader_tokens(id, page.anchor)
        .unwrap()
        .unwrap()
        .key()
        .unwrap()
}

fn deny_binaries(connection: &mut Connection, annotations: bool) {
    // Observe the actual SQL boundary, independently of helper/counter names.
    connection
        .authorizer(Some(move |context: AuthContext<'_>| match context.action {
            AuthAction::Read {
                table_name,
                column_name,
            } if (table_name == "imported_files" && column_name == "original_bytes")
                || (table_name == "epub_assets" && column_name == "bytes")
                || (annotations && matches!(table_name, "epub_spans" | "epub_inline_images")) =>
            {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }))
        .unwrap();
}

fn allow_reads(connection: &mut Connection) {
    connection
        .authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
        .unwrap();
}

#[test]
fn repeated_pages_words_and_lazy_chapter_reads_never_read_binaries_or_other_annotations() {
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    for size in [1024, 16 * 1024 * 1024] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = epub(&mut storage, size);
        let key = page_key(&mut storage, &imported);
        deny_binaries(&mut storage.connection, true);
        reader_work::take();
        for _ in 0..3 {
            assert_eq!(
                storage
                    .load_effective_reader_tokens(key.document_id, key.page)
                    .unwrap()
                    .unwrap()
                    .key()
                    .unwrap(),
                key
            );
            assert_eq!(
                storage
                    .load_reader_chapter(key.document_id, key.page.chapter_id)
                    .unwrap()
                    .unwrap(),
                imported.document.chapters[1].chapter
            );
            let word = storage
                .load_effective_reader_word(&dictionary, &key, 0)
                .unwrap()
                .unwrap();
            assert_eq!(word.token.surface, "試験語");
            assert_eq!(word.candidates.words.len(), 1);
        }
        assert_eq!(
            reader_work::take().0,
            0,
            "only the requested chapter should load"
        );
    }
}

#[test]
fn status_position_completion_retry_and_finished_reopen_do_not_revalidate_unchanged_binaries() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = epub(&mut storage, 1024);
    let key = page_key(&mut storage, &imported);
    deny_binaries(&mut storage.connection, false);
    let position = ReadingPosition {
        source_id: key.page.source_id,
        chapter_id: key.page.chapter_id,
        offset: TextOffset::START,
    };
    for status in [VocabularyStatus::Learning, VocabularyStatus::New] {
        let word = storage
            .load_effective_reader_word(&dictionary, &key, 0)
            .unwrap()
            .unwrap();
        storage
            .set_effective_reader_status(&key, 0, word.token.vocabulary.unwrap().id, status, 10)
            .unwrap()
            .unwrap();
        storage
            .save_document_reading_position(key.document_id, position)
            .unwrap()
            .unwrap();
        storage
            .load_effective_reader_tokens(key.document_id, key.page)
            .unwrap()
            .unwrap();
    }
    let command = storage
        .prepare_effective_page_completion(
            PageCompletionIntent {
                document_id: key.document_id,
                page: key.page,
                page_length: PageLength::default(),
                departing_position: position,
                completed_at_unix_ms: 100,
                analysis_id: key.segmentation.original_analysis_id,
            },
            &key,
        )
        .unwrap();
    assert!(matches!(
        storage.record_effective_page_completion(&command).unwrap(),
        AnalyzedCompletionOutcome::Recorded { .. }
    ));
    // This is the runtime's completion follow-up read and exact retry sequence.
    assert!(
        storage
            .load_document_reader_snapshot(key.document_id)
            .unwrap()
            .unwrap()
            .document
            .finished_at_least_once
    );
    assert_eq!(
        storage.record_effective_page_completion(&command).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .load_effective_reader_word(&dictionary, &key, 0)
            .unwrap()
            .unwrap()
            .encountered,
        2
    );
    let reopened = storage
        .open_document_reader_snapshot(key.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        reopened.document.reading_position.unwrap().chapter_id,
        imported.document.chapters[0].chapter.chapter_id
    );
}

#[test]
fn cached_integrity_rejects_local_external_and_triggered_corruption_on_the_next_page_or_word() {
    for journal in ["DELETE", "WAL"] {
        for external in [false, true] {
            for sql in [
                "UPDATE imported_files SET original_bytes = X'00'",
                "UPDATE epub_assets SET bytes = X'00'",
                "UPDATE imported_files SET sha256 = zeroblob(32)",
                "UPDATE epub_assets SET sha256 = zeroblob(32)",
                "UPDATE epub_assets SET media_type = 'text/html'",
                "UPDATE chapter_texts SET text = '破損' WHERE chapter_id = (SELECT MIN(chapter_id) FROM chapter_texts)",
                "UPDATE epub_inline_images SET start_byte = start_byte + 1",
                "UPDATE epub_spans SET span_index = span_index + 100",
                "UPDATE epub_chapters SET document_id = 999",
                "DELETE FROM imported_files",
                "DELETE FROM epub_assets",
                "DELETE FROM documents",
            ] {
                let database = TestDatabase::new();
                let index = dictionary();
                let dictionary = Dictionary::open(index.path()).unwrap();
                let mut storage = Storage::open(database.path()).unwrap();
                storage.connection.pragma_update(None, "journal_mode", journal).unwrap();
                let imported = epub(&mut storage, 16);
                let key = page_key(&mut storage, &imported);
                let word = storage.load_effective_reader_word(&dictionary, &key, 0).unwrap().unwrap();
                let vocabulary_id = word.token.vocabulary.unwrap().id;
                let writer = Connection::open(database.path()).unwrap();
                // Deliberately bypass FKs to test reader ownership checks too.
                let connection = if external { &writer } else { &storage.connection };
                connection.pragma_update(None, "foreign_keys", false).unwrap();
                connection.execute_batch(sql).unwrap();
                // A previously validated status selection must also be discarded.
                assert!(storage.set_effective_reader_status(&key, 0, vocabulary_id, VocabularyStatus::Known, 10).is_err(), "{journal}, external={external}: {sql}");
                let result = storage.load_effective_reader_tokens(key.document_id, key.page);
                assert!(!matches!(result, Ok(Some(_))), "{journal}, external={external}: {sql}");
                let result = storage.load_effective_reader_word(&dictionary, &key, 0);
                assert!(!matches!(result, Ok(Some(_))), "{journal}, external={external}: {sql}");
            }
        }
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = epub(&mut storage, 16);
    storage.connection.execute_batch("CREATE TRIGGER corrupt_image AFTER UPDATE ON vocabulary BEGIN UPDATE epub_assets SET bytes = X'00'; END;").unwrap();
    let key = page_key(&mut storage, &imported);
    let tokens = storage
        .load_effective_reader_tokens(key.document_id, key.page)
        .unwrap()
        .unwrap();
    let id = tokens.tokens[0].vocabulary.unwrap().id;
    storage
        .set_effective_reader_status(&key, 0, id, VocabularyStatus::Learning, 10)
        .unwrap()
        .unwrap();
    assert!(storage
        .load_effective_reader_tokens(key.document_id, key.page)
        .is_err());
}

#[test]
fn rollback_schema_and_external_revisions_require_binary_validation_but_assets_always_verify_bytes()
{
    for change in ["rollback", "schema", "external"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = epub(&mut storage, 16);
        let key = page_key(&mut storage, &imported);
        match change {
            "rollback" => storage
                .connection
                .execute_batch("BEGIN; UPDATE epub_assets SET bytes = X'00'; ROLLBACK;")
                .unwrap(),
            "schema" => storage
                .connection
                .execute_batch("CREATE TABLE revision_test (id INTEGER)")
                .unwrap(),
            _ => Connection::open(database.path())
                .unwrap()
                .execute_batch("UPDATE vocabulary SET status = 'learning'")
                .unwrap(),
        }
        deny_binaries(&mut storage.connection, false);
        assert!(
            storage
                .load_effective_reader_tokens(key.document_id, key.page)
                .is_err(),
            "{change}"
        );
        allow_reads(&mut storage.connection);
        storage
            .load_effective_reader_tokens(key.document_id, key.page)
            .unwrap()
            .unwrap();
        let asset = yomoku_core::reader_images::ReaderImageAsset {
            path: "OPS/images/挿絵.png".into(),
            media_type: "image/png".into(),
        };
        // Explicit asset reads never reuse the reader's prior binary validation.
        deny_binaries(&mut storage.connection, false);
        assert!(storage
            .load_epub_image_asset(key.document_id, &asset)
            .is_err());
        allow_reads(&mut storage.connection);
        assert_eq!(
            storage
                .load_epub_image_asset(key.document_id, &asset)
                .unwrap()
                .unwrap()
                .bytes,
            vec![42; 16]
        );
    }
}

#[test]
fn binary_validation_against_uncommitted_bytes_cannot_survive_rollback() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = epub(&mut storage, 16);
    let key = page_key(&mut storage, &imported);
    // Corrupt committed bytes, then temporarily replace them with valid bytes.
    storage
        .connection
        .execute_batch("UPDATE epub_assets SET bytes = X'00'")
        .unwrap();
    let transaction = storage.connection.transaction().unwrap();
    let bytes = vec![7; 16];
    transaction
        .execute(
            "UPDATE epub_assets SET bytes = ?1, sha256 = ?2",
            (&bytes, Sha256::digest(&bytes).as_slice()),
        )
        .unwrap();
    crate::reader_documents::load_reader_document_in(
        &transaction,
        key.document_id,
        &storage.reader_integrity,
    )
    .unwrap()
    .unwrap();
    transaction.rollback().unwrap();
    assert!(storage
        .load_effective_reader_tokens(key.document_id, key.page)
        .is_err());
}

#[test]
fn wal_commit_after_a_read_snapshot_cannot_attach_old_validation_to_new_bytes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    let imported = epub(&mut storage, 16);
    let key = page_key(&mut storage, &imported);
    let transaction = storage.connection.transaction().unwrap();
    // Establish this reader's snapshot before another connection commits.
    let revision = storage.reader_integrity.revision(&transaction).unwrap();
    Connection::open(database.path())
        .unwrap()
        .execute_batch("UPDATE epub_assets SET bytes = X'00'")
        .unwrap();
    crate::reader_documents::load_reader_document_in(
        &transaction,
        key.document_id,
        &storage.reader_integrity,
    )
    .unwrap()
    .unwrap();
    assert!(storage
        .reader_integrity
        .validated_source(key.document_id, revision)
        .is_some());
    transaction.commit().unwrap();
    assert!(storage
        .load_effective_reader_tokens(key.document_id, key.page)
        .is_err());
}
