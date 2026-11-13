use super::{imports::fixture, *};
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletion},
    document::PasteText,
    effective_reader::EffectiveReaderChapter,
};

#[test]
fn reservation_authenticates_every_completion_field_before_and_after_recording_or_deletion() {
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
            // Invalid anchors fail at construction; all other payload changes
            // must fail reservation authentication without touching persisted state.
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
    }
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
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
