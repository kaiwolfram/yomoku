use crux_core::{
    bridge::{BincodeFfiFormat, FfiFormat},
    Core,
};
use std::{error::Error, fs, path::Path};
use yomoku_core::{
    app::{Event, ViewModel, Yomoku},
    storage::DocumentError,
    vocabulary::{
        VocabularyBulkAction, VocabularyBulkCommand, VocabularyQuery, VocabularyRow,
        VocabularySort, VocabularyState, VocabularyStatus,
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
    let ids = vec![
        1.try_into()?,
        9_007_199_254_740_993_i64.try_into()?,
        i64::MAX.try_into()?,
    ];
    let actions = VocabularyQuery::default()
        .statuses
        .into_iter()
        .map(|status| VocabularyBulkAction::SetStatus {
            status,
            occurred_at_unix_ms: 9_007_199_254_740_993,
        })
        .chain([VocabularyBulkAction::Delete]);
    for (index, action) in actions.enumerate() {
        let command = VocabularyBulkCommand {
            ids: ids.clone(),
            action,
        };
        let mut view = Core::<Yomoku>::new().view();
        view.vocabulary.selection = Some(ids.clone());
        view.vocabulary.confirmation = Some(command.clone());
        view.vocabulary.bulk_error = Some(DocumentError::Unavailable);
        let events = [
            Event::RequestVocabularyBulkAction(action),
            Event::ConfirmVocabularyBulkAction(command),
        ];
        if writing {
            fs::write(
                dir.join(format!("vocabulary-bulk-view-{index}.bin")),
                super::encode(&view),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-bulk-view-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
        }
        for (kind, event) in events.into_iter().enumerate() {
            let name = format!("vocabulary-bulk-event-{index}-{kind}");
            if writing {
                fs::write(dir.join(format!("{name}.bin")), super::encode(&event))?;
            } else {
                let returned: Event = BincodeFfiFormat::deserialize(&fs::read(
                    dir.join(format!("{name}-return.bin")),
                )?)?;
                assert_eq!(returned, event);
            }
        }
    }
    for (index, event) in [
        Event::ToggleVocabularySelection(i64::MAX.try_into()?),
        Event::SelectAllVocabulary,
        Event::RetryVocabularySelection,
        Event::CancelVocabularySelection,
        Event::RetryVocabularyBulkAction,
        Event::CancelVocabularyBulkAction,
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = Core::<Yomoku>::new().view();
        view.vocabulary.selection = Some(vec![]);
        view.vocabulary.selecting_all = index % 2 == 0;
        view.vocabulary.bulk_error = (index % 2 != 0).then_some(DocumentError::Unavailable);
        if writing {
            fs::write(
                dir.join(format!("vocabulary-selection-view-{index}.bin")),
                super::encode(&view),
            )?;
            fs::write(
                dir.join(format!("vocabulary-selection-event-{index}.bin")),
                super::encode(&event),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-selection-view-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
            let returned: Event = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("vocabulary-selection-event-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, event);
        }
    }
    Ok(())
}
