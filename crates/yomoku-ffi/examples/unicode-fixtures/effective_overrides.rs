use std::{error::Error, fs, path::Path};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    analysis::{OverrideScope, SplitMode},
    document::{TextAnchor, TextRange},
    effective_overrides::EffectiveOverrideFailure as Failure,
    effective_session::{
        EffectiveBoundaryChoice, EffectiveOverrideAction as Action, EffectiveOverrideIdentity,
        EffectiveOverridePrompt, EffectivePageIdentity, EffectiveReaderOverrideState,
        EffectiveWinningRule,
    },
    storage::DocumentError,
};

pub fn exchange(
    dir: &Path,
    writing: bool,
    page: EffectivePageIdentity,
) -> Result<(), Box<dyn Error>> {
    let identity = EffectiveOverrideIdentity {
        page,
        revision: u64::MAX,
    };
    let anchor = TextAnchor {
        range: TextRange {
            start: 4.try_into()?,
            end: 15.try_into()?,
        },
        ..page.base.page
    };
    let surface = "𠮷か\u{3099}\0".to_owned();
    let parts = vec!["𠮷".to_owned(), "か\u{3099}\0".to_owned()];
    let prompt = EffectiveOverridePrompt {
        identity,
        anchor,
        surface: surface.clone(),
        choices: [SplitMode::A, SplitMode::B, SplitMode::C]
            .into_iter()
            .map(|mode| EffectiveBoundaryChoice {
                mode,
                parts: if mode == SplitMode::B {
                    vec![surface.clone()]
                } else {
                    parts.clone()
                },
            })
            .collect(),
        winning_rules: [OverrideScope::AllOccurrences, OverrideScope::ThisOccurrence]
            .into_iter()
            .enumerate()
            .map(|(i, scope)| EffectiveWinningRule {
                id: (i64::MAX - i as i64).try_into().unwrap(),
                scope,
                surface: surface.clone(),
                parts: parts.clone(),
            })
            .collect(),
        default_scope: OverrideScope::default(),
    };
    let ready = EffectiveReaderOverrideState {
        identity: Some(identity),
        prompt: Some(prompt),
        pending: false,
        busy: false,
        error: None,
        reload_required: false,
    };
    let mut states = vec![ready.clone()];
    states.push(EffectiveReaderOverrideState {
        prompt: None,
        pending: true,
        busy: true,
        ..ready.clone()
    });
    for error in [
        Failure::Storage(DocumentError::Unavailable),
        Failure::InvalidSelection,
        Failure::StaleContext,
    ] {
        let pending = matches!(error, Failure::Storage(_));
        states.push(EffectiveReaderOverrideState {
            identity: pending.then_some(identity),
            prompt: None,
            pending,
            busy: false,
            error: Some(error),
            reload_required: !pending,
        });
    }
    for (i, state) in states.into_iter().enumerate() {
        exchange_value(
            dir,
            writing,
            &format!("effective-override-state-{i}"),
            &state,
        )?;
        let mut view = crux_core::Core::<yomoku_core::app::Yomoku>::new().view();
        view.effective_reader_override = Some(state);
        exchange_value(
            dir,
            writing,
            &format!("effective-override-app-view-{i}"),
            &view,
        )?;
    }
    let mut actions = vec![Action::Open { page, anchor }];
    for scope in [OverrideScope::AllOccurrences, OverrideScope::ThisOccurrence] {
        for mode in [SplitMode::A, SplitMode::B, SplitMode::C] {
            actions.push(Action::Choose {
                identity,
                mode,
                scope,
                registered_at_unix_ms: i64::MAX,
            });
        }
        actions.push(Action::Save {
            identity,
            scope,
            parts: parts.clone(),
            registered_at_unix_ms: i64::MAX,
        });
    }
    for mode in [SplitMode::A, SplitMode::C] {
        actions.push(Action::Generate { identity, mode });
    }
    actions.extend([
        Action::Reverse {
            identity,
            id: i64::MAX.try_into()?,
        },
        Action::Retry { identity },
        Action::Cancel { identity },
        Action::Dismiss { identity },
    ]);
    for (i, action) in actions.into_iter().enumerate() {
        exchange_value(
            dir,
            writing,
            &format!("effective-override-action-{i}"),
            &action,
        )?;
        exchange_value(
            dir,
            writing,
            &format!("effective-override-app-event-{i}"),
            &yomoku_core::app::Event::EffectiveReaderOverride(action),
        )?;
    }
    Ok(())
}

fn exchange_value<
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
>(
    dir: &Path,
    writing: bool,
    name: &str,
    value: &T,
) -> Result<(), Box<dyn Error>> {
    if writing {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, value)?;
        fs::write(dir.join(format!("{name}.bin")), bytes)?;
    } else {
        let returned: T =
            BincodeFfiFormat::deserialize(&fs::read(dir.join(format!("{name}-return.bin")))?)?;
        assert_eq!(&returned, value);
    }
    Ok(())
}
