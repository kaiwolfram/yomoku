use super::*;
use yomoku_core::{
    dictionary::DictionaryCandidates, effective_reader::EffectiveReaderWordDetails,
    effective_session::EffectiveReaderSelection, reader::ReaderVocabulary,
    vocabulary::VocabularyStatus,
};

fn selection(f: &Fixture) -> EffectiveReaderSelection {
    let page = f.core.view().effective_reader_tokens.unwrap();
    EffectiveReaderSelection {
        page: page.identity,
        segment_index: page.tokens[0].segment_index,
    }
}

#[test]
fn dictionary_choices_keep_word_identity_validate_revisions_and_retry_the_same_part() {
    use yomoku_core::{anki::AnkiSelection, dictionary::DictionaryPart};
    let f = Fixture::new(false);
    f.load();
    let page = f.core.view().effective_reader_tokens.unwrap();
    let selected = EffectiveReaderSelection {
        page: page.identity,
        segment_index: page.tokens[2].segment_index,
    };
    let mut original = details(&f, selected);
    original.parts = vec![DictionaryPart {
        surface: "𠮷".into(),
        dictionary_form: None,
        range: range(0, 4),
        primary: true,
    }];
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::WordLoaded(selected, Box::new(original))),
    );
    let before = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    let owner = AnkiSelection::Effective {
        selection: selected,
        lookup_revision: before.lookup.revision,
    };
    assert!(f
        .core
        .process_event(Event::ChooseDictionaryPart {
            selection: owner,
            part: None
        })
        .is_empty());
    assert!(f
        .core
        .process_event(Event::ChooseDictionaryPart {
            selection: owner,
            part: Some(99)
        })
        .is_empty());
    let mut effects = f.core.process_event(Event::ChooseDictionaryPart {
        selection: owner,
        part: Some(0),
    });
    let query = operation(&effects);
    let StorageOperation::LookupDictionary(query) = query else {
        panic!("dictionary read only")
    };
    assert_eq!(query.surface, "𠮷");
    let current = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    let current_owner = AnkiSelection::Effective {
        selection: selected,
        lookup_revision: current.lookup.revision,
    };
    assert!(current.lookup.loading && current.lookup.revision > before.lookup.revision);
    assert!(f
        .core
        .process_event(Event::ChooseDictionaryPart {
            selection: current_owner,
            part: None
        })
        .is_empty());
    f.core.process_event(Event::DictionaryLookupFinished(
        owner,
        StorageResult::Failed(DocumentError::NotFound),
    ));
    assert!(f.core.view().storage_busy);
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let mut retry = f
        .core
        .process_event(Event::RetryDictionaryPart(current_owner));
    assert_eq!(
        operation(&retry),
        StorageOperation::LookupDictionary(query.clone())
    );
    let candidates = DictionaryCandidates {
        surface: query.surface,
        dictionary_form: query.dictionary_form,
        words: vec![],
        names: vec![],
    };
    resolve(
        &f.core,
        &mut retry,
        StorageResult::DictionaryLoaded(candidates.clone()),
    );
    let current = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    assert_eq!(current.candidates, candidates);
    assert_eq!(current.token, before.token);
    assert_eq!(current.encountered, before.encountered);
    assert_eq!(current.lookup.active, Some(0));
    assert!(f
        .core
        .process_event(Event::ChooseDictionaryPart {
            selection: owner,
            part: None
        })
        .is_empty());
    let owner = AnkiSelection::Effective {
        selection: selected,
        lookup_revision: current.lookup.revision,
    };
    let mut whole = f.core.process_event(Event::ChooseDictionaryPart {
        selection: owner,
        part: None,
    });
    assert_eq!(
        operation(&whole),
        StorageOperation::LookupDictionary(before.lookup.whole)
    );
    f.core.process_event(Event::DismissReaderWord);
    resolve(
        &f.core,
        &mut whole,
        StorageResult::DictionaryLoaded(before.candidates),
    );
    assert!(f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .is_none());
    load_word(&f, selected, None);
    let reopened = f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
    assert_eq!(reopened.lookup.active, None);
    assert!(reopened.lookup.revision > current.lookup.revision);
}

fn details(f: &Fixture, selection: EffectiveReaderSelection) -> EffectiveReaderWordDetails {
    let snapshot = f.snapshot();
    let token = snapshot
        .tokens
        .into_iter()
        .find(|token| token.segment_index == selection.segment_index)
        .unwrap();
    EffectiveReaderWordDetails {
        parts: Vec::new(),
        key: f.snapshot().key().unwrap(),
        candidates: DictionaryCandidates {
            surface: token.surface.clone(),
            dictionary_form: token
                .metadata
                .dictionary_form
                .as_ref()
                .map(|f| f.as_str().to_owned()),
            words: vec![],
            names: vec![],
        },
        token,
        encountered: 7,
        last_seen_unix_ms: None,
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
    f.core.process_event(Event::CloseSettings);
    let mut effects = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    assert!(f.core.view().effective_reader_word.unwrap().busy);
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
    assert_eq!(word.encountered, 8);
    f.load();
    assert!(f
        .core
        .view()
        .effective_reader_word
        .unwrap()
        .selection
        .is_none());
}

#[test]
fn all_five_status_choices_update_repeated_spellings_and_word_details_including_deletion() {
    let f = Fixture::new(true);
    f.load();
    let selected = selection(&f);
    let visible_occurrences = f
        .snapshot()
        .tokens
        .iter()
        .filter(|token| token.surface == "猫")
        .count() as u64;
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
        assert_eq!(word.encountered, 7 + visible_occurrences);
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
    assert_eq!(word.encountered, visible_occurrences);
    assert!(f
        .core
        .process_event(status_event(selected, VocabularyStatus::New))
        .is_empty());
    assert!(f.core.view().completion_notice.is_none());
}

#[test]
fn status_failure_defers_departure_until_exact_retry_or_cancel() {
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

        f.core.process_event(Event::CloseDocument);
        assert!(f.core.view().effective_reader_tokens.is_none());
        assert!(f
            .core
            .process_event(Event::CancelEffectiveReaderStatus)
            .is_empty());
        resolve(
            &f.core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(f.core.view().effective_reader_status.unwrap().pending);
        assert_eq!(
            f.core.view().effective_reader_status.unwrap().error,
            Some(DocumentError::Unavailable)
        );
        assert_eq!(f.core.view().content_route, ContentRoute::Reader);

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
fn dismissed_word_callback_does_not_reopen_panel_or_release_newer_work() {
    let f = Fixture::new(false);
    f.load();
    let selected = selection(&f);

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

    f.core.process_event(Event::DismissReaderWord);
    resolve(
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
    let mut next = f
        .core
        .process_event(Event::LoadEffectiveReaderWord(selected));
    f.core.process_event(Event::EffectiveStorageFinished(
        Box::new(captured),
        StorageResult::Failed(DocumentError::Unavailable),
    ));
    assert!(f.core.view().storage_busy);
    resolve(
        &f.core,
        &mut next,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(!f.core.view().storage_busy);
}
