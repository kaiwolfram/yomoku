use super::{imports::fixture, *};
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletion, PageCompletionIntent},
    document::PasteText,
    effective_reader::EffectiveReaderChapter,
    pagination::{PageLayout, PageLength},
    reader::ReaderPage,
};

#[test]
fn warm_forward_turn_loads_one_document_without_building_layout_or_decoding_analysis() {
    let text = "猫 犬 鳥 を 読む 。\n".repeat(2_000);
    for format in ["txt", "epub"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let id = if format == "txt" {
            let TxtImportOutcome::Created(value) = storage
                .import_analyzed_txt(
                    text.as_bytes(),
                    "long.txt",
                    Some(yomoku_import::txt::TextEncoding::Utf8),
                    1,
                    fixture,
                )
                .unwrap()
            else {
                unreachable!()
            };
            value.document.document_id
        } else {
            let EpubImportOutcome::Created(value) = storage
                .import_analyzed_epub(
                    &epub_fixture::book(true, &format!("<pre>{text}</pre>")),
                    "chapters.epub",
                    1,
                    fixture,
                )
                .unwrap()
            else {
                unreachable!()
            };
            value.document.document_id
        };
        let snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        let layout = PageLayout::new(&snapshot.document.chapters, snapshot.page_length).unwrap();
        let mut document = snapshot.document;
        for turn in 0..3 {
            let page = ReaderPage::open_document(&document, snapshot.page_length).unwrap();
            let key = storage
                .load_effective_reader_tokens(id, page.anchor)
                .unwrap()
                .unwrap()
                .key()
                .unwrap();
            reader_work::take();
            reader_work::LAYOUTS.set(0);
            reader_work::ANALYSES.set(0);
            let completion = storage
                .prepare_effective_page_completion(
                    PageCompletionIntent {
                        document_id: id,
                        page: page.anchor,
                        page_length: snapshot.page_length,
                        departing_position: page.position,
                        completed_at_unix_ms: 2 + turn,
                        analysis_id: key.segmentation.original_analysis_id,
                    },
                    &key,
                )
                .unwrap();
            let (_, updated) = storage
                .commit_effective_page_completion(&completion)
                .unwrap();
            document = updated.expect("new commit reuses the validated document");
            let expected = layout.pages()[page.page_number as usize];
            let position = document.reading_position.unwrap();
            assert_eq!(
                (position.chapter_id, position.offset),
                (expected.chapter_id, expected.range.start)
            );
            if format == "epub" && turn == 0 {
                assert_ne!(position.chapter_id, page.position.chapter_id);
            }
            let (documents, chapters, _) = reader_work::take();
            let layouts = reader_work::LAYOUTS.get();
            let analyses = reader_work::ANALYSES.get();
            eprintln!("{format} turn {turn}: layouts={layouts}, documents={documents}, chapter_reads={chapters}, analyses={analyses}");
            assert_eq!((layouts, documents, chapters, analyses), (0, 1, 0, 0));
        }

        // Reflow preserves exact offsets. The next reservation builds the new layout once.
        storage.save_page_length(PageLength::Characters500).unwrap();
        let reflowed = ReaderPage::open_document(&document, PageLength::Characters500).unwrap();
        assert_eq!(Some(reflowed.position), document.reading_position);
        let key = storage
            .load_effective_reader_tokens(id, reflowed.anchor)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        reader_work::LAYOUTS.set(0);
        storage
            .prepare_effective_page_completion(
                PageCompletionIntent {
                    document_id: id,
                    page: reflowed.anchor,
                    page_length: PageLength::Characters500,
                    departing_position: reflowed.position,
                    completed_at_unix_ms: 100,
                    analysis_id: key.segmentation.original_analysis_id,
                },
                &key,
            )
            .unwrap();
        assert_eq!(reader_work::LAYOUTS.get(), 1);
    }
}

#[test]
fn pending_command_and_committed_receipt_reject_mismatched_fields() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .analyzed_fixture(&PasteText::new("猫 猫 犬".into()).unwrap(), 0, fixture)
        .unwrap();
    let frozen = reserve_current(&mut storage, imported.document.document_id, 500);
    let effective = EffectiveReaderChapter::resolve(
        &imported.document.chapter,
        imported.analysis_id,
        &fixture(&imported.document.chapter).unwrap(),
    )
    .unwrap();
    let receipt = frozen.completion();
    for recorded in [false, true] {
        if recorded {
            storage.record_effective_page_completion(&frozen).unwrap();
            storage
                .delete_document(imported.document.document_id)
                .unwrap();
        }
        let before = fs::read(database.path()).unwrap();
        for field in 0..11 {
            let mut changed = receipt;
            match field {
                0 => changed.command_id = i64::MAX.try_into().unwrap(),
                1 => changed.document_id = i64::MAX.try_into().unwrap(),
                2 => changed.page.source_id = i64::MAX.try_into().unwrap(),
                3 => changed.page.chapter_id = i64::MAX.try_into().unwrap(),
                4 => changed.page.range.start = 3.try_into().unwrap(),
                5 => changed.page.range.end = 3.try_into().unwrap(),
                6 => changed.departing_position.source_id = i64::MAX.try_into().unwrap(),
                7 => changed.departing_position.chapter_id = i64::MAX.try_into().unwrap(),
                8 => changed.departing_position.offset = 3.try_into().unwrap(),
                9 => changed.page_length = yomoku_core::pagination::PageLength::Characters500,
                _ => changed.completed_at_unix_ms += 1,
            }
            // Invalid anchors fail at construction. Reject other field changes
            // against the pending fingerprint or committed receipt.
            if let Ok(forged) = effective.freeze_completion(changed) {
                assert!(
                    matches!(
                        storage.record_effective_page_completion(&forged),
                        Err(StorageError::InvalidCompletion)
                    ),
                    "field {field}, recorded {recorded}"
                );
            }
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
        let original = fixture(&imported.document.chapter).unwrap();
        let mut different_engine = original.clone();
        different_engine.profile.engine_revision.push('2');
        let mut different_dictionary = original.clone();
        different_dictionary.profile.dictionary_revision.push('2');
        for (analysis_id, analysis) in [
            (i64::MAX.try_into().unwrap(), &original),
            (imported.analysis_id, &different_engine),
            (imported.analysis_id, &different_dictionary),
        ] {
            let forged =
                EffectiveReaderChapter::resolve(&imported.document.chapter, analysis_id, analysis)
                    .unwrap()
                    .freeze_completion(receipt)
                    .unwrap();
            assert!(matches!(
                storage.record_effective_page_completion(&forged),
                Err(StorageError::InvalidCompletion)
            ));
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
    }
    reader_work::take();
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(reader_work::take(), (0, 0, Vec::new()));
}

#[test]
fn commit_verifies_occurrences_then_clears_the_fingerprint() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .analyzed_fixture(&PasteText::new("猫".into()).unwrap(), 0, fixture)
        .unwrap();
    let frozen = reserve_current(&mut storage, imported.document.document_id, 500);
    let pending_fingerprint = |storage: &Storage| -> Option<[u8; 32]> {
        storage
            .connection
            .query_row(
                "SELECT pending_fingerprint FROM completion_commands",
                [],
                |row| row.get(0),
            )
            .unwrap()
    };
    let pending = pending_fingerprint(&storage).unwrap();
    let mut changed = fixture(&imported.document.chapter).unwrap();
    changed.tokens[0].reading = Some("ネコ".into());
    let changed =
        EffectiveReaderChapter::resolve(&imported.document.chapter, imported.analysis_id, &changed)
            .unwrap()
            .freeze_completion(frozen.completion())
            .unwrap();
    assert!(matches!(
        storage.record_effective_page_completion(&changed),
        Err(StorageError::InvalidCompletion)
    ));
    assert_eq!(pending_fingerprint(&storage), Some(pending));
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
    );
    assert_eq!(pending_fingerprint(&storage), None);
    let before = fs::read(database.path()).unwrap();
    drop(storage);

    // The response was lost. Reopening and retrying uses the receipt alone.
    let mut storage = Storage::open(database.path()).unwrap();
    reader_work::take();
    for command in [frozen, changed] {
        assert_eq!(
            storage.record_effective_page_completion(&command).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    }
    assert_eq!(reader_work::take(), (0, 0, Vec::new()));
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn bare_command_ids_cannot_complete_and_reservation_exhaustion_rolls_back() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .analyzed_fixture(&PasteText::new("猫".into()).unwrap(), 0, fixture)
        .unwrap();
    let frozen = reserve_current(&mut storage, imported.document.document_id, 500);
    storage
        .connection
        .execute("INSERT INTO completion_commands DEFAULT VALUES", [])
        .unwrap();
    let id = storage.connection.last_insert_rowid().try_into().unwrap();
    let effective = EffectiveReaderChapter::resolve(
        &imported.document.chapter,
        imported.analysis_id,
        &fixture(&imported.document.chapter).unwrap(),
    )
    .unwrap();
    let bare = effective
        .freeze_completion(PageCompletion {
            command_id: id,
            ..frozen.completion()
        })
        .unwrap();
    assert!(matches!(
        storage.record_effective_page_completion(&bare),
        Err(StorageError::InvalidCompletion)
    ));
    storage.connection.execute("UPDATE sqlite_sequence SET seq = 9223372036854775807 WHERE name = 'completion_commands'", []).unwrap();
    let before = fs::read(database.path()).unwrap();
    let intent = yomoku_core::completion::PageCompletionIntent {
        document_id: frozen.completion().document_id,
        page: frozen.completion().page,
        page_length: frozen.completion().page_length,
        departing_position: frozen.completion().departing_position,
        completed_at_unix_ms: 600,
        analysis_id: imported.analysis_id,
    };
    assert!(matches!(
        storage.prepare_effective_page_completion(intent, frozen.key()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert!(matches!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
    ));
}
