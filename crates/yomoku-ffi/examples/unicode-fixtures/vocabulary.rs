use crux_core::{
    bridge::{BincodeFfiFormat, FfiFormat},
    Core,
};
use std::{error::Error, fs, path::Path};
use yomoku_core::{
    app::{Event, ViewModel, Yomoku},
    storage::DocumentError,
    vocabulary::{
        VocabularyQuery, VocabularyRow, VocabularySort, VocabularyState, VocabularyStatus,
    },
};

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    if writing {
        fs::create_dir_all(dir)?;
    }
    for (index, sort) in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
        VocabularySort::Oldest,
        VocabularySort::FrequencyAscending,
        VocabularySort::LastSeenAscending,
        VocabularySort::AlphabeticalDescending,
    ]
    .into_iter()
    .enumerate()
    {
        let query = VocabularyQuery {
            prefix: " 𠮷e\u{301}\0_%".into(),
            statuses: vec![VocabularyStatus::New, VocabularyStatus::Ignored],
            sort,
        };
        let mut view = Core::<Yomoku>::new().view();
        view.vocabulary = VocabularyState {
            query: query.clone(),
            matching_count: Some(u64::MAX),
            rows: vec![
                VocabularyRow {
                    id: i64::MAX.try_into()?,
                    surface: format!("{}猫", query.prefix),
                    status: VocabularyStatus::Ignored,
                    registered_at_unix_ms: i64::MAX,
                    encountered_count: u64::MAX,
                    last_seen_unix_ms: Some(9_007_199_254_740_993),
                },
                VocabularyRow {
                    id: 1.try_into()?,
                    surface: format!("{}犬", query.prefix),
                    status: VocabularyStatus::New,
                    registered_at_unix_ms: 0,
                    encountered_count: 0,
                    last_seen_unix_ms: None,
                },
            ],
            next_offset: Some(9_007_199_254_740_993),
            loading: false,
            error: None,
            ..Default::default()
        };
        let event = Event::SetVocabularyQuery(query);
        if writing {
            fs::write(
                dir.join(format!("vocabulary-view-{index}.bin")),
                super::encode(&view),
            )?;
            fs::write(
                dir.join(format!("vocabulary-event-{index}.bin")),
                super::encode(&event),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-view-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
            let returned: Event = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-event-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, event);
        }
    }
    for (index, state) in [
        VocabularyState::default(),
        VocabularyState {
            loading: true,
            ..Default::default()
        },
        VocabularyState {
            error: Some(DocumentError::InvalidDocument),
            ..Default::default()
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = Core::<Yomoku>::new().view();
        view.vocabulary = state;
        if writing {
            fs::write(
                dir.join(format!("vocabulary-state-{index}.bin")),
                super::encode(&view),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-state-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
        }
    }
    for (index, event) in [
        Event::LoadVocabulary,
        Event::LoadMoreVocabulary,
        Event::RetryVocabulary,
    ]
    .into_iter()
    .enumerate()
    {
        if writing {
            fs::write(
                dir.join(format!("vocabulary-action-{index}.bin")),
                super::encode(&event),
            )?;
        } else {
            let returned: Event = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-action-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, event);
        }
    }
    let command = yomoku_core::vocabulary::VocabularyWordStatusCommand {
        id: i64::MAX.try_into()?,
        status: yomoku_core::vocabulary::VocabularyStatus::Learning,
        occurred_at_unix_ms: 9_007_199_254_740_993,
    };
    for (index, event) in [
        Event::OpenVocabularyWord(command.id),
        Event::DismissVocabularyWord,
        Event::RetryVocabularyWord,
        Event::SetVocabularyWordStatus(command.clone()),
        Event::RetryVocabularyWordStatus,
        Event::CancelVocabularyWordStatus,
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = Core::<Yomoku>::new().view();
        view.vocabulary.word = Some(yomoku_core::vocabulary::VocabularyWordState {
            lookup: Default::default(),
            selection: yomoku_core::anki::VocabularyAnkiSelection {
                vocabulary_id: i64::MAX.try_into().unwrap(),
                generation: u64::MAX,
            },
            row: yomoku_core::vocabulary::VocabularyRow {
                id: command.id,
                surface: "𠮷か\u{3099}\0".into(),
                status: command.status,
                registered_at_unix_ms: 123,
                encountered_count: u64::MAX,
                last_seen_unix_ms: Some(456),
            },
            candidates: (index % 2 == 0).then(|| yomoku_core::dictionary::DictionaryCandidates {
                surface: "𠮷か\u{3099}\0".into(),
                dictionary_form: None,
                words: vec![],
                names: vec![],
            }),
            error: (index % 2 != 0).then_some(DocumentError::Unavailable),
        });
        view.vocabulary.word_status = Some(command.clone());
        view.vocabulary.word_status_error = Some(DocumentError::Unavailable);
        for (kind, bytes) in [
            ("view", super::encode(&view)),
            ("event", super::encode(&event)),
        ] {
            let name = format!("vocabulary-word-{kind}-{index}");
            if writing {
                fs::write(dir.join(format!("{name}.bin")), bytes)?;
            } else {
                let returned = fs::read(dir.join(format!("{name}-return.bin")))?;
                if kind == "view" {
                    assert_eq!(BincodeFfiFormat::deserialize::<ViewModel>(&returned)?, view);
                } else {
                    assert_eq!(BincodeFfiFormat::deserialize::<Event>(&returned)?, event);
                }
            }
        }
    }
    Ok(())
}
