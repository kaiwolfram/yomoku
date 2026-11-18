use super::*;

fn update(model: &mut Model, event: Event) {
    drop(Yomoku.update(event, model));
}

fn finish_file(model: &mut Model, result: StorageResult) {
    update(
        model,
        Event::ImportEpub(crate::imports::EpubImport {
            original_bytes: vec![1],
            filename: "fixture.epub".into(),
        }),
    );
    update(
        model,
        Event::StorageFinished(model.storage_sequence, result),
    );
}

#[test]
fn acknowledging_an_import_clears_only_its_error_and_never_reads_the_library() {
    let mut model = Model::default();
    model.document_error = Some(DocumentError::Unavailable);
    finish_file(
        &mut model,
        StorageResult::Failed(DocumentError::InvalidEpub),
    );
    assert_eq!(
        Yomoku.view(&model).document_error,
        Some(DocumentError::InvalidEpub)
    );
    let mut effects = Yomoku.update(Event::DismissImportSummary, &mut model);
    assert!(effects
        .effects()
        .all(|effect| !matches!(effect, Effect::Storage(_))));
    assert_eq!(
        Yomoku.view(&model).document_error,
        Some(DocumentError::Unavailable)
    );
}

#[test]
fn only_new_library_membership_invalidates_distributions_on_import() {
    let row = LibraryDocument {
        document_id: DocumentId::try_from(1).unwrap(),
        title: "fixture".into(),
        progress_percent: 0,
        is_read: false,
        last_read_sequence: None,
    };
    let mut model = Model::default();
    model.library_word_distributions = Some(vec![]);
    finish_file(&mut model, StorageResult::Existing(row.clone()));
    assert_eq!(model.documents, [row.clone()]);
    assert!(model.library_word_distributions.is_none());
    model.library_word_distributions = Some(vec![]);
    finish_file(&mut model, StorageResult::Existing(row.clone()));
    assert_eq!(model.library_word_distributions, Some(vec![]));
    finish_file(&mut model, StorageResult::Created(row));
    assert!(model.library_word_distributions.is_none());
}
