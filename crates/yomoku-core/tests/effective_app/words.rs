use super::*;
use yomoku_core::{
    dictionary::DictionaryCandidates,
    effective_reader::EffectiveReaderWordDetails,
    effective_session::EffectiveReaderSelection,
    reader::{ReaderCustomEntry, ReaderVocabulary},
    vocabulary::VocabularyStatus,
};

fn selection(f: &Fixture) -> EffectiveReaderSelection {
    let page = f.core.view().effective_reader_tokens.unwrap();
    EffectiveReaderSelection {
        page: page.identity,
        segment_index: page.tokens[0].segment_index,
    }
}

fn details(f: &Fixture, selection: EffectiveReaderSelection) -> EffectiveReaderWordDetails {
    let snapshot = f.snapshot();
    let token = snapshot
        .tokens
        .into_iter()
        .find(|token| token.segment_index == selection.segment_index)
        .unwrap();
    EffectiveReaderWordDetails {
        key: f.snapshot().key().unwrap(),
        candidates: DictionaryCandidates {
            surface: token.surface.clone(),
            dictionary_form: token
                .metadata
                .as_ref()
                .and_then(|m| m.dictionary_form.as_ref().map(|f| f.as_str().to_owned())),
            words: vec![],
            names: vec![],
        },
        token,
        encountered: 7,
        custom_entry: None,
    }
}

fn load_word(
    f: &Fixture,
    selection: EffectiveReaderSelection,
    vocabulary: Option<ReaderVocabulary>,
) {
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selection));
    assert!(matches!(
        operation(&effects),
        StorageOperation::Effective(Op::LoadWord(_))
    ));
    let mut details = details(f, selection);
    details.token.vocabulary = vocabulary;
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::WordLoaded(selection, Box::new(details))),
    );
}

fn status_event(selection: EffectiveReaderSelection, status: VocabularyStatus) -> Event {
    Event::SetEffectiveReaderStatus {
        selection,
        vocabulary_id: 55.try_into().unwrap(),
        status,
        occurred_at_unix_ms: 600,
    }
}

fn custom_event(selection: EffectiveReaderSelection) -> Event {
    Event::SaveEffectiveReaderCustomEntry {
        selection,
        english_meaning: " literal\0meaning 📖 ".into(),
        reading: " よみか\u{3099}\0 ".into(),
    }
}

fn entry() -> ReaderCustomEntry {
    ReaderCustomEntry {
        surface: "猫".into(),
        english_meaning: " literal\0meaning 📖 ".into(),
        reading: " よみか\u{3099}\0 ".into(),
    }
}

#[test]
fn compact_lookup_requires_current_visible_selection_and_explicit_retry_after_bad_results() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);
    for invalid in [
        EffectiveReaderSelection {
            segment_index: u64::MAX,
            ..selected
        },
        EffectiveReaderSelection {
            page: EffectivePageIdentity {
                revision: selected.page.revision + 1,
                ..selected.page
            },
            ..selected
        },
    ] {
        assert!(f
            .core
            .process_event(Event::LoadEffectiveReaderWord(invalid))
            .is_empty());
    }
    f.core.process_event(Event::OpenSettings);
    assert!(f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected))
        .is_empty());
    f.core.process_event(Event::CloseSettings);
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    assert!(f.core.view().effective_reader_word.unwrap().busy);
    assert!(f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected))
        .is_empty());
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let failed = f.core.view().effective_reader_word.unwrap();
    assert_eq!(failed.error, Some(DocumentError::Unavailable));
    assert!(failed.details.is_none() && !failed.busy);
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    let mut bad = details(&f, selected);
    bad.token.surface = "別".into();
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::WordLoaded(selected, Box::new(bad))),
    );
    assert_eq!(
        f.core.view().effective_reader_word.unwrap().error,
        Some(DocumentError::InvalidDocument)
    );
    load_word(&f, selected, None);
    let word = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    assert_eq!(word.token.surface, "猫");
    assert!(word.token.reading.is_some() && word.token.dictionary_form.is_some());
    assert_eq!(word.encountered, 7);
    assert!(f.core.view().reader_word.is_none());
    f.load();
    assert!(f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .selection
        .is_none());
    assert!(f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected))
        .is_empty());
}

#[test]
fn all_five_status_choices_update_repeated_spellings_and_word_details_including_deletion() {
    let f = Fixture::new(true);
    f.load();
    let selected = selection(&f);
    let id = 55.try_into().unwrap();
    load_word(
        &f,
        selected,
        Some(ReaderVocabulary {
            id,
            status: VocabularyStatus::New,
        }),
    );
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        let mut effects = f.core.process_event(status_event(selected, status));
        assert!(f.core.view().effective_reader_status.unwrap().busy);
        resolve(
            &f.core,
            &mut effects,
            StorageResult::Effective(Output::StatusChanged(
                selected,
                Some(ReaderVocabulary { id, status }),
            )),
        );
        let view = f.core.view();
        let tokens = view.effective_reader_tokens.unwrap().tokens;
        assert!(tokens.iter().filter(|token| token.surface == "猫").count() > 1);
        assert!(tokens
            .iter()
            .filter(|token| token.surface == "猫")
            .all(|token| token.vocabulary == Some(ReaderVocabulary { id, status })));
        let word = view.effective_reader_word.unwrap().details.unwrap();
        assert_eq!(word.token.vocabulary.unwrap().status, status);
        assert_eq!(word.encountered, 7);
        assert!(!view.effective_reader_status.unwrap().pending);
    }
    let mut effects = f
        .core
        .process_event(status_event(selected, VocabularyStatus::Known));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::StatusChanged(selected, None)),
    );
    let word = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    assert!(word.token.vocabulary.is_none());
    assert_eq!(word.encountered, 0);
    assert!(f
        .core
        .process_event(status_event(selected, VocabularyStatus::New))
        .is_empty());
    assert!(f.core.view().completion_notice.is_none());
}

#[test]
fn status_failure_drains_time_but_defers_departure_until_exact_retry_or_cancel() {
    for cancel in [false, true] {
        let f = Fixture::new(false);
        f.load();
        let selected = selection(&f);
        load_word(
            &f,
            selected,
            Some(ReaderVocabulary {
                id: 55.try_into().unwrap(),
                status: VocabularyStatus::New,
            }),
        );
        restore(&f);
        f.core
            .process_event(clock(&f, 0, ReaderClockAction::Resume));
        let mut effects = f
            .core
            .process_event(status_event(selected, VocabularyStatus::Learning));
        let frozen = operation(&effects);
        let StorageOperation::Effective(Op::SetStatus {
            command,
            vocabulary_id,
            status,
            occurred_at_unix_ms,
        }) = frozen.clone()
        else {
            panic!("status")
        };
        assert_eq!(occurred_at_unix_ms, 600);
        let old = f.core.view();
        f.core
            .process_event(clock(&f, 250, ReaderClockAction::Pause));
        f.core.process_event(Event::CloseDocument);
        assert!(f.core.view().effective_reader_tokens.is_none());
        assert!(f
            .core
            .process_event(Event::CancelEffectiveReaderStatus)
            .is_empty());
        let mut checkpoint = resolve(
            &f.core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(f.core.view().effective_reader_status.unwrap().pending);
        assert_eq!(
            f.core.view().effective_reader_status.unwrap().error,
            Some(DocumentError::Unavailable)
        );
        assert!(f
            .core
            .process_event(Event::RetryEffectiveReaderStatus)
            .is_empty());
        save_time(&f, &mut checkpoint, 250);
        assert_eq!(f.core.view().content_route, ContentRoute::Reader);
        assert!(!f.core.view().storage_busy && !f.core.view().reading_time_save_pending);
        for event in [
            Event::LoadReaderTokens(f.base()),
            Event::OpenReader(f.document.document_id),
            Event::CreatePaste("other".into()),
            f.complete(selected.page),
        ] {
            assert!(f.core.process_event(event).is_empty());
        }
        if cancel {
            f.core.process_event(Event::CancelEffectiveReaderStatus);
        } else {
            let mut retry = f.core.process_event(Event::RetryEffectiveReaderStatus);
            assert_eq!(operation(&retry), frozen);
            // Wrong ID is a malformed result, not a committed status observation.
            resolve(
                &f.core,
                &mut retry,
                StorageResult::Effective(Output::StatusChanged(
                    command.selection(),
                    Some(ReaderVocabulary {
                        id: 99.try_into().unwrap(),
                        status,
                    }),
                )),
            );
            assert_eq!(
                f.core.view().effective_reader_status.unwrap().error,
                Some(DocumentError::InvalidDocument)
            );
            let mut retry = f.core.process_event(Event::RetryEffectiveReaderStatus);
            assert_eq!(operation(&retry), frozen);
            resolve(
                &f.core,
                &mut retry,
                StorageResult::Effective(Output::StatusChanged(
                    command.selection(),
                    Some(ReaderVocabulary {
                        id: vocabulary_id,
                        status,
                    }),
                )),
            );
        }
        assert_eq!(f.core.view().content_route, ContentRoute::Main);
        assert!(!f.core.view().effective_reader_status.unwrap().pending);
        assert_eq!(
            old.reader.unwrap().position,
            f.document.reading_position.unwrap()
        );
    }
}

#[test]
fn custom_save_delete_retain_literal_payload_and_committed_entry_through_failure_and_cancel() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);
    load_word(&f, selected, None);
    let mut effects = f.core.process_event(custom_event(selected));
    let frozen = operation(&effects);
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(
        f.core
            .view()
            .effective_reader_word
            .unwrap()
            .custom_entry_pending
    );
    assert!(f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected))
        .is_empty());
    assert!(f.core.process_event(f.complete(selected.page)).is_empty());
    let mut retry = f.core.process_event(Event::RetryEffectiveReaderCustomEntry);
    assert_eq!(operation(&retry), frozen);
    resolve(
        &f.core,
        &mut retry,
        StorageResult::Effective(Output::CustomEntryChanged(selected, Some(entry()))),
    );
    assert_eq!(
        f.core
            .view()
            .effective_reader_word
            .unwrap()
            .details
            .unwrap()
            .custom_entry,
        Some(entry())
    );
    let mut effects = f
        .core
        .process_event(Event::DeleteEffectiveReaderCustomEntry(selected));
    let frozen = operation(&effects);
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(
        f.core
            .view()
            .effective_reader_word
            .unwrap()
            .details
            .unwrap()
            .custom_entry,
        Some(entry())
    );
    f.core
        .process_event(Event::CancelEffectiveReaderCustomEntry);
    assert_eq!(
        f.core
            .view()
            .effective_reader_word
            .unwrap()
            .details
            .unwrap()
            .custom_entry,
        Some(entry())
    );
    let mut effects = f
        .core
        .process_event(Event::DeleteEffectiveReaderCustomEntry(selected));
    assert_eq!(operation(&effects), frozen);
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::CustomEntryChanged(selected, None)),
    );
    assert!(f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap()
        .custom_entry
        .is_none());
    assert!(f
        .core
        .process_event(Event::DeleteEffectiveReaderCustomEntry(selected))
        .is_empty());
}

#[test]
fn dismissed_custom_failure_stays_retryable_and_cancel_releases_deferred_position() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);
    load_word(&f, selected, None);
    let mut effects = f.core.process_event(custom_event(selected));
    f.core.process_event(Event::DismissReaderWord);
    f.core.process_event(Event::SaveReadingPosition(
        f.document.reading_position.unwrap(),
    ));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let word = f.core.view().effective_reader_word.unwrap();
    assert!(word.selection.is_none() && word.details.is_none() && word.custom_entry_pending);
    assert_eq!(word.custom_entry_error, Some(DocumentError::Unavailable));
    f.core.process_event(Event::OpenSettings);
    assert!(f
        .core
        .process_event(Event::SetPageLength(PageLength::Characters250))
        .is_empty());
    f.core.process_event(Event::CloseSettings);
    assert_eq!(
        f.core
            .view()
            .effective_reader_word
            .unwrap()
            .custom_entry_error,
        Some(DocumentError::Unavailable)
    );
    let mut save = f
        .core
        .process_event(Event::CancelEffectiveReaderCustomEntry);
    assert!(matches!(
        operation(&save),
        StorageOperation::SaveReadingPosition { .. }
    ));
    resolve(
        &f.core,
        &mut save,
        StorageResult::PositionSaved(f.document.clone()),
    );
    assert!(!f.core.view().storage_busy);
}

#[test]
fn dismissed_word_callback_does_not_reopen_panel_or_release_newer_checkpoint() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);
    restore(&f);
    f.core
        .process_event(clock(&f, 0, ReaderClockAction::Resume));
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    let StorageOperation::Effective(Op::LoadWord(command)) = operation(&effects) else {
        panic!("word")
    };
    let mut retained = yomoku_core::effective_session::EffectiveReaderSession::default();
    let tokens = retained.begin_load(f.base()).unwrap();
    assert!(retained.accept(tokens, f.snapshot()));
    let captured = EffectiveReaderRequest::Word(retained.begin_word_load(selected).unwrap());
    assert_eq!(
        captured.operation(),
        StorageOperation::Effective(Op::LoadWord(command.clone()))
    );
    f.core
        .process_event(clock(&f, 300, ReaderClockAction::Pause));
    f.core.process_event(Event::DismissReaderWord);
    let mut checkpoint = resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::WordLoaded(
            command.selection(),
            Box::new(details(&f, selected)),
        )),
    );
    assert!(f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .is_none());
    f.core.process_event(Event::EffectiveStorageFinished(
        Box::new(captured),
        StorageResult::Failed(DocumentError::Unavailable),
    ));
    assert!(f.core.view().storage_busy);
    assert!(f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected))
        .is_empty());
    save_time(&f, &mut checkpoint, 300);
    assert!(!f.core.view().storage_busy);
}

#[test]
fn compact_events_fail_closed_in_production_model() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);
    let core = Core::<Yomoku>::new();
    for event in [
        Event::LoadEffectiveReaderWord(selected),
        custom_event(selected),
        Event::DeleteEffectiveReaderCustomEntry(selected),
        Event::RetryEffectiveReaderCustomEntry,
        Event::CancelEffectiveReaderCustomEntry,
        status_event(selected, VocabularyStatus::Known),
        Event::RetryEffectiveReaderStatus,
        Event::CancelEffectiveReaderStatus,
    ] {
        assert!(core.process_event(event).is_empty());
    }
    assert!(
        core.view().effective_reader_word.is_none()
            && core.view().effective_reader_status.is_none()
    );
}
