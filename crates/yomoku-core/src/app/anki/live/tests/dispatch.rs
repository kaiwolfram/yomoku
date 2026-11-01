use super::*;
use crate::anki::{AnkiLiveOperation, AnkiLiveResult};
use crate::app::anki::test_support::reader_view;

fn dispatch(model: &mut Model, result: AnkiLiveResult) -> Event {
    let frozen = model.prepared_anki_export().unwrap().clone();
    let mut command = Yomoku.update(Event::DispatchAnkiLive, model);
    let Some(Effect::AnkiLive(mut effect)) = command.effects().next() else {
        panic!("one owning live provider request")
    };
    assert!(command
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
    match &effect.operation {
        AnkiLiveOperation::InsertNote {
            surface,
            fields,
            deck_id,
            model_id,
            layout,
        } => {
            assert_eq!(surface, frozen.note().surface().as_str());
            assert_eq!(fields, frozen.note().fields());
            assert_eq!(*deck_id, i64::MAX);
            assert_eq!(*model_id, 9007199254740993);
            assert_eq!(layout.as_ref(), &frozen.layout().into());
        }
        AnkiLiveOperation::CreateModel {
            surface,
            deck_id,
            layout,
        } => {
            assert_eq!(surface, frozen.note().surface().as_str());
            assert_eq!(*deck_id, i64::MAX);
            assert_eq!(layout.as_ref(), &frozen.layout().into());
        }
    }
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Invoking));
    assert!(model.take_anki_live_invocation().is_none());
    update(model, Event::DispatchAnkiLive);
    effect.resolve(result).unwrap();
    let event = command.events().next().unwrap();
    assert!(matches!(event, Event::FinishAnkiLive(_)));
    event
}

#[test]
fn anki_live_dispatch_freezes_both_selections_and_kinds_and_routes_every_outcome_to_storage() {
    for effective in [false, true] {
        for result in [
            AnkiLiveResult::NoSubmission,
            AnkiLiveResult::NoteAdded(i64::MAX),
            AnkiLiveResult::NoteExisting(vec![9007199254740993, i64::MAX]),
            AnkiLiveResult::NoteUncertain(None),
            AnkiLiveResult::NoteUncertain(Some(i64::MAX)),
            AnkiLiveResult::ModelCreated(i64::MAX),
            AnkiLiveResult::ModelUncertain(None),
            AnkiLiveResult::ModelUncertain(Some(9007199254740993)),
        ] {
            let create = matches!(
                result,
                AnkiLiveResult::ModelCreated(_) | AnkiLiveResult::ModelUncertain(_)
            );
            let mut model = prepared(effective, create);
            let request = ready(&mut model);
            let before = reader_view(&model);
            let event = dispatch(&mut model, result.clone());
            let Event::FinishAnkiLive(callback) = &event else {
                unreachable!()
            };
            let expected = callback.expected_result(&snapshot(&request)).unwrap();
            match result {
                AnkiLiveResult::NoteUncertain(id) => assert_eq!(
                    expected.note.as_ref().unwrap().known_note_id.map(i64::from),
                    id
                ),
                AnkiLiveResult::ModelUncertain(id) => assert_eq!(
                    expected
                        .model
                        .as_ref()
                        .unwrap()
                        .known_model_id
                        .map(i64::from),
                    id
                ),
                _ => assert_eq!(*expected, AnkiRecoverySnapshot::default()),
            }
            assert_eq!(reader_view(&model), before);
            let mut saving = Yomoku.update(event.clone(), &mut model);
            assert!(
                matches!(saving.effects().next(), Some(Effect::Storage(effect))
                if matches!(effect.operation, StorageOperation::FinishAnkiLive(_)))
            );
            update(&mut model, event);
            update(
                &mut model,
                Event::AnkiLiveOutcomeFinished(
                    Box::new(request),
                    StorageResult::AnkiLiveFinished(expected),
                ),
            );
            assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Finished));
            update(&mut model, Event::DispatchAnkiLive);
            assert_eq!(reader_view(&model), before);
        }
    }
}

#[test]
fn anki_live_dispatch_invalid_ids_empty_duplicates_and_wrong_kinds_stay_uncertain() {
    for (create, result) in [
        (false, AnkiLiveResult::NoteAdded(0)),
        (false, AnkiLiveResult::NoteAdded(-1)),
        (false, AnkiLiveResult::NoteExisting(vec![])),
        (false, AnkiLiveResult::NoteExisting(vec![i64::MAX, 0])),
        (false, AnkiLiveResult::NoteUncertain(Some(-1))),
        (false, AnkiLiveResult::ModelCreated(i64::MAX)),
        (true, AnkiLiveResult::ModelCreated(0)),
        (true, AnkiLiveResult::ModelUncertain(Some(-1))),
        (true, AnkiLiveResult::NoteAdded(i64::MAX)),
        (true, AnkiLiveResult::NoteExisting(vec![i64::MAX])),
    ] {
        let mut model = prepared(false, create);
        let request = ready(&mut model);
        let event = dispatch(&mut model, result);
        let Event::FinishAnkiLive(callback) = &event else {
            unreachable!()
        };
        assert_eq!(
            callback.expected_result(&snapshot(&request)),
            Some(snapshot(&request))
        );
        let _saving = Yomoku.update(event, &mut model);
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Saving));
        assert!(!model.anki_recovery().allows_preparation());
    }
}

#[test]
fn anki_live_dispatch_requires_ready_and_rechecks_exact_journals() {
    update(&mut Model::default(), Event::DispatchAnkiLive);
    for gate in 0..5 {
        let mut model = prepared(false, false);
        ready(&mut model);
        match gate {
            0 => model.anki_recovery_commit_error = Some(DocumentError::Unavailable),
            1 => model.anki_recovery_read_error = Some(AnkiExportError::ProviderFailure),
            2 => model.anki_recovery = AnkiRecoveryState::Unrestored,
            3 => model.anki_recovery = AnkiRecoveryState::Restored(Box::default()),
            4 => update(&mut model, Event::AbandonAnkiLive),
            _ => unreachable!(),
        }
        update(&mut model, Event::DispatchAnkiLive);
    }
}

#[test]
fn anki_live_dispatch_retry_abandon_and_other_sessions_reject_late_provider_results() {
    for abandon in [false, true] {
        let mut model = prepared(true, false);
        ready(&mut model);
        let event = dispatch(&mut model, AnkiLiveResult::NoteAdded(i64::MAX));
        let mut other = prepared(true, false);
        ready(&mut other);
        let _other_permit = other.take_anki_live_invocation().unwrap();
        update(&mut other, event.clone());
        update(&mut model, Event::AbandonAnkiLive);
        if !abandon {
            let _restore = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
        }
        update(&mut model, event);
        assert!(!model.anki_recovery().allows_preparation());
        update(&mut model, Event::DispatchAnkiLive);
    }
}

#[test]
fn anki_live_dispatch_bincode_bridge_retains_callback_owner_and_rejects_duplicate_resolution() {
    use crux_core::{
        bridge::{BincodeFfiFormat, FfiFormat, Requests},
        middleware::{Bridge, Layer},
        Core,
    };
    #[crux_core::macros::effect]
    enum LiveWire {
        Invoke(AnkiLiveOperation),
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum LiveWireFfi {
        Invoke(AnkiLiveOperation),
    }
    impl crux_core::EffectFFI for LiveWire {
        type Ffi = LiveWireFfi;
        fn serialize<T: FfiFormat>(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized<T>) {
            let Self::Invoke(request) = self;
            request.serialize(LiveWireFfi::Invoke)
        }
    }
    impl From<Effect> for LiveWire {
        fn from(effect: Effect) -> Self {
            let Effect::AnkiLive(request) = effect else {
                panic!("only one live effect")
            };
            Self::Invoke(request)
        }
    }
    #[derive(Default)]
    struct Probe;
    impl App for Probe {
        type Event = Event;
        type Model = Model;
        type Effect = Effect;
        type ViewModel = bool;
        fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
            let finish = matches!(event, Event::FinishAnkiLive(_));
            // This narrow wire harness maps only invocation effects.
            let mut command = Yomoku.update_anki_live(&event, model).unwrap();
            if finish {
                // The owning closure must produce the normal durable outcome command.
                assert!(
                    matches!(command.effects().next(), Some(Effect::Storage(effect))
                    if matches!(effect.operation, StorageOperation::FinishAnkiLive(_)))
                );
                return Command::done();
            }
            command
        }
        fn view(&self, model: &Model) -> bool {
            model.anki_live_progress() == Some(AnkiLiveProgress::Saving)
        }
    }
    fn encode(value: &impl serde::Serialize) -> Vec<u8> {
        let mut bytes = vec![];
        BincodeFfiFormat::serialize(&mut bytes, value).unwrap();
        bytes
    }
    for create in [false, true] {
        let mut model = prepared(true, create);
        ready(&mut model);
        let bridge = Bridge::<_, BincodeFfiFormat>::new(
            Core::new_with(Probe, model).map_effect::<LiveWire>(),
            |_| panic!("no asynchronous effects"),
        );
        let mut bytes = vec![];
        bridge
            .update(&encode(&Event::DispatchAnkiLive), &mut bytes)
            .unwrap();
        let request: Requests<LiveWireFfi> = BincodeFfiFormat::deserialize(&bytes).unwrap();
        assert_eq!(request.0.len(), 1);
        assert!(
            matches!(
                &request.0[0].effect,
                LiveWireFfi::Invoke(AnkiLiveOperation::CreateModel { .. })
            ) == create
        );
        bytes.clear();
        bridge
            .update(&encode(&Event::DispatchAnkiLive), &mut bytes)
            .unwrap();
        assert!(
            BincodeFfiFormat::deserialize::<Requests<LiveWireFfi>>(&bytes)
                .unwrap()
                .0
                .is_empty()
        );
        let result = encode(&if create {
            AnkiLiveResult::ModelUncertain(Some(i64::MAX))
        } else {
            AnkiLiveResult::NoteUncertain(Some(9007199254740993))
        });
        bytes.clear();
        bridge
            .resolve(request.0[0].id, &result, &mut bytes)
            .unwrap();
        assert!(
            BincodeFfiFormat::deserialize::<Requests<LiveWireFfi>>(&bytes)
                .unwrap()
                .0
                .is_empty()
        );
        let mut view = vec![];
        bridge.view(&mut view).unwrap();
        assert!(BincodeFfiFormat::deserialize::<bool>(&view).unwrap());
        assert!(bridge
            .resolve(request.0[0].id, &result, &mut vec![])
            .is_err());
    }
}
