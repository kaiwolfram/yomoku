use yomoku_core::document::{DocumentId, PasteText, TextAnchor, TextOffset, TextRange};

use super::*;

fn paste(text: &str) -> PasteText {
    PasteText::new(text.to_owned()).unwrap()
}

fn row_counts(storage: &Storage) -> [i64; 4] {
    ["sources", "chapters", "documents", "chapter_texts"].map(|table| {
        storage
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    })
}

fn sequences(storage: &Storage) -> Vec<(String, i64)> {
    let mut statement = storage
        .connection
        .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn persists_exact_paste_title_ids_and_unicode_anchors_across_reopens() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut expected = Vec::new();
    for text in [
        "  題名𠮷📖  \r\n\n第一章\nか\u{3099} e\u{301} 👩‍👩‍👧‍👦\0終\n",
        "\n本文",
        "引用'\"; DROP TABLE documents;--\0\n次",
    ] {
        let input = paste(text);
        let document = storage.unanalyzed_fixture(&input).unwrap();
        assert_eq!(document.title, input.initial_title());
        assert_eq!(document.chapter.text, text);
        let original_title: String = storage
            .connection
            .query_row(
                "SELECT original_title FROM sources WHERE id = ?1",
                [i64::from(document.chapter.source_id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(original_title, document.title);
        expected.push(document);
    }
    assert_eq!(row_counts(&storage), [3; 4]);
    drop(storage);
    for _ in 0..2 {
        let mut storage = Storage::open(database.path()).unwrap();
        for document in &expected {
            let reopened = storage.load_paste(document.document_id).unwrap().unwrap();
            assert_eq!(&reopened, document);
            let anchor = TextAnchor {
                source_id: reopened.chapter.source_id,
                chapter_id: reopened.chapter.chapter_id,
                range: TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(&document.chapter.text, document.chapter.text.len())
                        .unwrap(),
                },
            };
            assert_eq!(
                anchor.slice(&reopened.chapter).unwrap(),
                document.chapter.text
            );
        }
        assert_eq!(row_counts(&storage), [3; 4]);
    }
}

#[test]
fn rolls_back_every_insert_stage_without_changing_existing_documents_or_sequences() {
    for table in ["sources", "chapters", "documents", "chapter_texts"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let existing = storage.unanalyzed_fixture(&paste("保持\n本文")).unwrap();
        let before_sequences = sequences(&storage);
        storage
            .connection
            .execute_batch(&format!(
                "CREATE TRIGGER fail_paste BEFORE INSERT ON {table}
                 BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END;"
            ))
            .unwrap();
        assert!(matches!(
            storage.unanalyzed_fixture(&paste("失敗\n𠮷")),
            Err(StorageError::Sqlite(_))
        ));
        assert_eq!(row_counts(&storage), [1; 4]);
        assert_eq!(sequences(&storage), before_sequences);
        assert_eq!(
            storage.load_paste(existing.document_id).unwrap(),
            Some(existing.clone())
        );
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(row_counts(&storage), [1; 4]);
        assert_eq!(
            storage.load_paste(existing.document_id).unwrap(),
            Some(existing)
        );
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_paste")
            .unwrap();
        storage.unanalyzed_fixture(&paste("再試行\n本文")).unwrap();
        assert_eq!(row_counts(&storage), [2; 4]);
    }
}

#[test]
fn commit_failure_also_rolls_back_all_content_and_id_allocations() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let existing = storage.unanalyzed_fixture(&paste("保存済み")).unwrap();
    let before_sequences = sequences(&storage);
    storage
        .connection
        .execute_batch(
            "CREATE TABLE deferred_failure (
             source_id INTEGER REFERENCES sources(id) DEFERRABLE INITIALLY DEFERRED
         );
         CREATE TRIGGER fail_commit AFTER INSERT ON chapter_texts
         BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
        )
        .unwrap();
    assert!(matches!(
        storage.unanalyzed_fixture(&paste("コミット失敗")),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(row_counts(&storage), [1; 4]);
    assert_eq!(sequences(&storage), before_sequences);
    assert_eq!(
        storage.load_paste(existing.document_id).unwrap(),
        Some(existing)
    );
    let failed_rows: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM deferred_failure", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(failed_rows, 0);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(row_counts(&storage), [1; 4]);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_commit")
        .unwrap();
    storage.unanalyzed_fixture(&paste("再試行")).unwrap();
    assert_eq!(row_counts(&storage), [2; 4]);
}

#[test]
fn deleted_committed_ids_are_not_reused_after_reopening() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let deleted = storage
        .unanalyzed_fixture(&paste("削除される本文"))
        .unwrap();
    // Simulate later deletion: remove live content/entry, retain anchor provenance.
    storage
        .connection
        .execute_batch("BEGIN; DELETE FROM chapter_texts; DELETE FROM documents; COMMIT;")
        .unwrap();
    assert_eq!(row_counts(&storage), [1, 1, 0, 0]);
    assert_eq!(storage.load_paste(deleted.document_id).unwrap(), None);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let reimported = storage
        .unanalyzed_fixture(&paste("削除される本文"))
        .unwrap();
    assert!(reimported.document_id > deleted.document_id);
    assert!(reimported.chapter.source_id > deleted.chapter.source_id);
    assert!(reimported.chapter.chapter_id > deleted.chapter.chapter_id);
    let previous_provenance: (String, String, i64) = storage
        .connection
        .query_row(
            "SELECT kind, original_title, chapter_index FROM sources
         JOIN chapters ON chapters.source_id = sources.id WHERE sources.id = ?1",
            [i64::from(deleted.chapter.source_id)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(previous_provenance, ("paste".into(), deleted.title, 0));
    // Even removing provenance through test-only SQL must not reset high-water marks.
    storage
        .connection
        .execute_batch(
            "BEGIN; DELETE FROM chapter_texts; DELETE FROM documents;
         DELETE FROM chapters; DELETE FROM sources; COMMIT;",
        )
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let next = storage.unanalyzed_fixture(&paste("新しい本文")).unwrap();
    assert!(next.document_id > reimported.document_id);
    assert!(next.chapter.source_id > reimported.chapter.source_id);
    assert!(next.chapter.chapter_id > reimported.chapter.chapter_id);
}

#[test]
fn missing_document_is_distinct_from_missing_or_invalid_canonical_content() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .load_paste(DocumentId::try_from(99).unwrap())
            .unwrap(),
        None
    );
    let document = storage.unanalyzed_fixture(&paste("本文")).unwrap();
    storage
        .connection
        .execute("UPDATE chapter_texts SET text = '  '", [])
        .unwrap();
    assert!(matches!(
        storage.load_paste(document.document_id),
        Err(StorageError::InvalidDocument)
    ));
    storage
        .connection
        .execute("DELETE FROM chapter_texts", [])
        .unwrap();
    assert!(matches!(
        storage.load_paste(document.document_id),
        Err(StorageError::InvalidDocument)
    ));
}

#[test]
fn live_title_is_separate_from_canonical_text_and_historical_title() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&paste("元の題名\n本文"))
        .unwrap();
    assert!(storage
        .rename_document(document.document_id, "新しい題名")
        .unwrap());
    let renamed = storage.load_paste(document.document_id).unwrap().unwrap();
    assert_eq!(renamed.title, "新しい題名");
    assert_eq!(renamed.chapter, document.chapter);
    let original_title: String = storage
        .connection
        .query_row("SELECT original_title FROM sources", [], |row| row.get(0))
        .unwrap();
    assert_eq!(original_title, document.title);
}

#[test]
fn concurrent_pastes_allocate_distinct_complete_documents() {
    let database = TestDatabase::new();
    let barrier = Arc::new(Barrier::new(4));
    let documents = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .unanalyzed_fixture(&paste(&format!("題名{index}\n𠮷")))
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(row_counts(&storage), [4; 4]);
    for document in &documents {
        assert_eq!(
            storage.load_paste(document.document_id).unwrap().as_ref(),
            Some(document)
        );
        assert_eq!(
            documents
                .iter()
                .filter(|other| other.document_id == document.document_id)
                .count(),
            1
        );
        assert_eq!(
            documents
                .iter()
                .filter(|other| other.chapter.source_id == document.chapter.source_id)
                .count(),
            1
        );
        assert_eq!(
            documents
                .iter()
                .filter(|other| other.chapter.chapter_id == document.chapter.chapter_id)
                .count(),
            1
        );
    }
}

#[test]
fn each_identity_uses_its_own_full_width_sqlite_allocation() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch(
            "INSERT INTO sqlite_sequence (name, seq) VALUES
         ('sources', 9007199254740992), ('chapters', 9223372036854775806), ('documents', 41);",
        )
        .unwrap();
    let document = storage.unanalyzed_fixture(&paste("大きなID𠮷")).unwrap();
    assert_eq!(i64::from(document.document_id), 42);
    assert_eq!(i64::from(document.chapter.source_id), (1 << 53) + 1);
    assert_eq!(i64::from(document.chapter.chapter_id), i64::MAX);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(document)
    );
}

#[test]
fn exhausted_document_ids_fail_atomically_without_reusing_ids() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let existing = storage.unanalyzed_fixture(&paste("最後の保存")).unwrap();
    storage
        .connection
        .execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'documents'",
            [i64::MAX],
        )
        .unwrap();
    let before_sequences = sequences(&storage);
    assert!(matches!(
        storage.unanalyzed_fixture(&paste("割当失敗")),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(row_counts(&storage), [1; 4]);
    assert_eq!(sequences(&storage), before_sequences);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.load_paste(existing.document_id).unwrap(),
        Some(existing)
    );
    assert!(matches!(
        storage.unanalyzed_fixture(&paste("再試行")),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(row_counts(&storage), [1; 4]);
}
