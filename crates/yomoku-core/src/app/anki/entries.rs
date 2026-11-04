//! Read-only entry observations and explicit one-tap export orchestration.
use super::*;
use crate::anki::*;

pub(in crate::app) struct EntryState {
    request: AnkiPreflightRequest,
    result: Option<Result<AnkiPreflightPlan, AnkiExportError>>,
}

#[derive(Default)]
pub(in crate::app) struct SetupState {
    pub phase: AnkiSetupPhase,
    generation: u64,
    deck: Option<i64>,
    pub live_deck: Option<i64>,
}

impl Model {
    pub(super) fn complete_entry_action(&mut self) {
        let Some(view) = self.anki_export_view() else {
            return;
        };
        if self.anki_setup.live_deck.is_some() {
            match view.phase {
                AnkiExportPhase::ModelCreated(_) => {
                    if self.anki_setup.live_deck != self.anki_settings.default_deck_id {
                        self.anki_setup.phase = AnkiSetupPhase::Unchecked;
                    }
                }
                AnkiExportPhase::NotSubmitted => {
                    self.anki_setup.phase = AnkiSetupPhase::Failed(AnkiExportError::ProviderFailure)
                }
                _ => return,
            }
            self.anki_live = None;
            self.anki_setup.live_deck = None;
            return;
        }
        let Some(export) = self
            .prepared_anki_export
            .as_ref()
            .filter(|export| export.entry.is_some())
        else {
            return;
        };
        let result = match view.phase {
            AnkiExportPhase::NoteAdded(id) => Ok(AnkiPreflightPlan::Existing(
                AnkiExistingNotes::new(vec![id.try_into().unwrap()]).unwrap(),
            )),
            AnkiExportPhase::Existing(ids) => Ok(AnkiPreflightPlan::Existing(
                AnkiExistingNotes::new(ids.into_iter().map(|id| id.try_into().unwrap()).collect())
                    .unwrap(),
            )),
            AnkiExportPhase::NotSubmitted => Err(AnkiExportError::ProviderFailure),
            AnkiExportPhase::ProviderFailed(error) => Err(error),
            AnkiExportPhase::CreateModel => Err(AnkiExportError::IncompatibleModel),
            _ => return,
        };
        for state in &mut self.anki_entries {
            if state.request.export().note().surface() == export.note().surface() {
                state.result = Some(result.clone());
            }
        }
        self.anki_live = None;
        self.prepared_anki_export = None;
        self.anki_export_preflight = None;
    }

    fn entry_keys(&self, selection: AnkiSelection) -> Option<Vec<AnkiEntryKey>> {
        if self.settings_open || self.content_route != ContentRoute::Reader {
            return None;
        }
        let effective_word = self
            .effective_reader
            .as_ref()
            .map(|session| session.word_state());
        let candidates = match selection {
            AnkiSelection::Original(s) if self.reader_word_selection == Some(s) => {
                let details = self.reader_word.as_ref()?;
                &details.candidates
            }
            AnkiSelection::Effective(s) => {
                let word = effective_word.as_ref()?;
                if word.selection != Some(s) || word.busy || word.error.is_some() {
                    return None;
                }
                let details = word.details.as_ref()?;
                if details.selection != s {
                    return None;
                }
                &details.candidates
            }
            _ => return None,
        };
        Some(
            candidates
                .words
                .iter()
                .map(|e| AnkiEntryKey::Word(e.entry_id))
                .chain(
                    candidates
                        .names
                        .iter()
                        .map(|e| AnkiEntryKey::Name(e.entry_id)),
                )
                .collect(),
        )
    }

    pub(in crate::app) fn anki_entries_view(&self) -> Vec<AnkiEntryView> {
        if self.anki_setup.phase != AnkiSetupPhase::Ready
            || self.anki_settings.phase != AnkiSettingsPhase::Ready
        {
            return Vec::new();
        }
        let Some(owner) = self
            .anki_entries
            .first()
            .map(|state| state.request.export().selection())
        else {
            return Vec::new();
        };
        let Some(keys) = self.entry_keys(owner) else {
            return Vec::new();
        };
        self.anki_entries
            .iter()
            .filter_map(|state| {
                let export = state.request.export();
                let key = export.entry?;
                if export.selection() != owner || !keys.contains(&key) {
                    return None;
                }
                let phase = if let Some(active) = &self.prepared_anki_export {
                    if active.note().surface() == export.note().surface() {
                        self.anki_export_view().map(|view| match view.phase {
                            AnkiExportPhase::Existing(ids) => AnkiEntryPhase::Existing(ids),
                            AnkiExportPhase::NoteAdded(id) => AnkiEntryPhase::Existing(vec![id]),
                            phase => AnkiEntryPhase::Export(phase),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
                .unwrap_or_else(|| match &state.result {
                    None => AnkiEntryPhase::Checking,
                    Some(Ok(AnkiPreflightPlan::ReuseModel(_))) => {
                        if self.anki_export_gate_open() {
                            AnkiEntryPhase::Add
                        } else if self.anki_live.as_ref().is_some_and(|live| {
                            matches!(
                                live.progress,
                                AnkiLiveProgress::Beginning
                                    | AnkiLiveProgress::Ready
                                    | AnkiLiveProgress::Invoking
                                    | AnkiLiveProgress::Saving
                            )
                        }) {
                            AnkiEntryPhase::Busy
                        } else {
                            AnkiEntryPhase::RecoveryRequired
                        }
                    }
                    Some(Ok(AnkiPreflightPlan::Existing(ids))) => AnkiEntryPhase::Existing(
                        ids.ids().iter().copied().map(Into::into).collect(),
                    ),
                    Some(Ok(AnkiPreflightPlan::CreateModel)) => {
                        AnkiEntryPhase::Failed(AnkiExportError::IncompatibleModel)
                    }
                    Some(Err(error)) => AnkiEntryPhase::Failed(*error),
                });
                Some(AnkiEntryView {
                    selection: export.selection(),
                    key,
                    headword: export.note().surface().as_str().into(),
                    phase,
                })
            })
            .collect()
    }
}

impl Yomoku {
    pub(super) fn update_anki_entries(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::CheckAnkiSetup => {
                if model.anki_settings.phase != AnkiSettingsPhase::Ready
                    || model.anki_live_blocks_recovery()
                {
                    return Some(Command::done());
                }
                let Some(deck) = model.anki_settings.default_deck_id else {
                    model.anki_setup.phase = AnkiSetupPhase::Unchecked;
                    model.anki_entries.clear();
                    return Some(Command::done());
                };
                let Some(id) = model.anki_setup.generation.checked_add(1) else {
                    return Some(Command::done());
                };
                model.anki_setup.generation = id;
                model.anki_setup.deck = Some(deck);
                model.anki_setup.phase = AnkiSetupPhase::Checking;
                // A model check has no selected word and never checks empty-word duplicates.
                let operation = AnkiPreflightOperation {
                    surface: String::new(),
                    deck_id: deck,
                    layout: Box::new((&AnkiModelLayout::yomoku()).into()),
                };
                Some(
                    Command::request_from_shell(operation)
                        .then_send(move |result| Event::AnkiSetupChecked(id, deck, result)),
                )
            }
            Event::AnkiSetupChecked(id, deck, result) => {
                if *id != model.anki_setup.generation
                    || model.anki_settings.default_deck_id != Some(*deck)
                    || model.anki_setup.phase != AnkiSetupPhase::Checking
                {
                    return Some(Command::done());
                }
                model.anki_setup.phase = match result {
                    AnkiPreflightResult::Complete(observation) => match evaluate_layout(
                        &AnkiModelLayout::yomoku(),
                        *deck,
                        observation.clone(),
                    ) {
                        Ok(AnkiPreflightPlan::ReuseModel(_)) => AnkiSetupPhase::Ready,
                        Ok(AnkiPreflightPlan::CreateModel) => AnkiSetupPhase::Missing,
                        Ok(_) => AnkiSetupPhase::Failed(AnkiExportError::InvalidResponse),
                        Err(error) => AnkiSetupPhase::Failed(error),
                    },
                    AnkiPreflightResult::Failed(error) => AnkiSetupPhase::Failed(*error),
                };
                Some(Command::done())
            }
            Event::CreateAnkiModel => {
                if !model.settings_open
                    || model.anki_settings.phase != AnkiSettingsPhase::Ready
                    || model.anki_setup.phase != AnkiSetupPhase::Missing
                    || !model.anki_export_gate_open()
                    || model.anki_setup.deck != model.anki_settings.default_deck_id
                {
                    return Some(Command::done());
                }
                let Some(deck) = model.anki_setup.deck else {
                    return Some(Command::done());
                };
                let Some(id) = model.anki_preparation_sequence.checked_add(1) else {
                    return Some(Command::done());
                };
                model.anki_preparation_sequence = id;
                model.prepared_anki_export = None;
                model.anki_export_preflight = None;
                model.anki_setup.live_deck = Some(deck);
                let intent = AnkiLiveIntent::Model(Box::new(
                    AnkiModelCreation::new(AnkiModelLayout::yomoku()).ok()?,
                ));
                Some(self.begin_anki_intent(model, intent))
            }
            Event::LoadReaderAnkiEntries(selection) => {
                if model.anki_settings.phase != AnkiSettingsPhase::Ready
                    || model.anki_setup.phase != AnkiSetupPhase::Ready
                    || model.anki_setup.deck != model.anki_settings.default_deck_id
                {
                    return Some(Command::done());
                }
                let Some(deck) = model
                    .anki_settings
                    .default_deck_id
                    .and_then(|id| AnkiDeckId::try_from(id).ok())
                else {
                    return Some(Command::done());
                };
                let Some(keys) = model.entry_keys(*selection) else {
                    return Some(Command::done());
                };
                let exports: Vec<_> = keys
                    .into_iter()
                    .filter_map(|key| model.prepare_anki_export(*selection, deck, Some(key)))
                    .collect();
                model.anki_entries.clear();
                let mut command = Command::done();
                for export in exports {
                    let request = AnkiPreflightRequest::new(export);
                    model.anki_entries.push(EntryState {
                        request: request.clone(),
                        result: None,
                    });
                    command =
                        command.and(Command::request_from_shell(request.operation()).then_send(
                            move |result| Event::AnkiEntryChecked(Box::new(request), result),
                        ));
                }
                Some(command)
            }
            Event::AnkiEntryChecked(request, result) => {
                let Some(state) = model
                    .anki_entries
                    .iter_mut()
                    .find(|state| state.request == **request && state.result.is_none())
                else {
                    return Some(Command::done());
                };
                state.result = Some(match result {
                    AnkiPreflightResult::Complete(observation) => {
                        request.evaluate(observation.clone())
                    }
                    AnkiPreflightResult::Failed(error) => Err(*error),
                });
                Some(Command::done())
            }
            Event::AddReaderAnkiEntry { selection, entry } => {
                if model.anki_settings.phase != AnkiSettingsPhase::Ready
                    || model.anki_setup.phase != AnkiSetupPhase::Ready
                    || model.anki_setup.deck != model.anki_settings.default_deck_id
                    || !model.anki_export_gate_open()
                {
                    return Some(Command::done());
                }
                let Some(deck) = model
                    .anki_settings
                    .default_deck_id
                    .and_then(|id| AnkiDeckId::try_from(id).ok())
                else {
                    return Some(Command::done());
                };
                if model.prepared_anki_export.is_some() {
                    return Some(Command::done());
                }
                let Some(export) = model.prepare_anki_export(*selection, deck, Some(*entry)) else {
                    return Some(Command::done());
                };
                let checked = model.anki_entries.iter().any(|state| {
                    state.request.export() == &export
                        && matches!(state.result, Some(Ok(AnkiPreflightPlan::ReuseModel(_))))
                });
                if !checked {
                    return Some(Command::done());
                }
                if model.prepared_anki_export.as_ref().is_some_and(|active| {
                    active.entry.is_some() && active.note().surface() == export.note().surface()
                }) {
                    // Repeated taps cannot restart even a completed or uncertain operation.
                    return Some(Command::done());
                }
                let Some(id) = model.anki_preparation_sequence.checked_add(1) else {
                    return Some(Command::done());
                };
                model.anki_preparation_sequence = id;
                model.prepared_anki_export = Some(export);
                model.anki_export_preflight = None;
                model.anki_setup.live_deck = None;
                self.update_anki_preflight(&Event::BeginAnkiExportPreflight, model)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::anki::test_support::fixture;
    use crate::dictionary::{
        Reading, Sense, Spelling, Translation, WordCandidate, WordEntry, WordFormMatch,
    };

    fn candidate(id: i64, word: &str, reading: &str, meaning: &str) -> WordCandidate {
        let entry = WordEntry {
            kanji: vec![Spelling {
                text: word.into(),
                ke_inf: vec![],
                ke_pri: vec![],
            }],
            readings: vec![Reading {
                text: reading.into(),
                no_kanji: false,
                re_restr: vec![],
                re_inf: vec![],
                re_pri: vec![],
            }],
            senses: vec![Sense {
                stagk: vec![],
                stagr: vec![],
                pos: vec![],
                field: vec![],
                misc: vec![],
                s_inf: vec![],
                dial: vec![],
                xref: vec![],
                ant: vec![],
                loan_sources: vec![],
                glosses: vec![Translation {
                    text: meaning.into(),
                    lang: "eng".into(),
                    g_type: None,
                    g_gend: None,
                }],
            }],
        };
        WordCandidate {
            entry_id: id,
            forms: vec![WordFormMatch {
                form: word.into(),
                matches: entry.matches(word),
            }],
            entry,
        }
    }

    fn observation(ids: Vec<i64>, model: bool) -> AnkiPreflightResult {
        AnkiPreflightResult::Complete(AnkiPreflightObservation {
            decks: vec![AnkiPreflightDeck {
                id: 7,
                filtered: false,
            }],
            duplicate_note_ids: ids,
            models: if model {
                vec![AnkiPreflightModel {
                    id: 9,
                    name: MODEL_NAME.into(),
                    matches_layout: true,
                }]
            } else {
                vec![]
            },
        })
    }

    fn configured() -> (Model, AnkiSelection) {
        let (mut model, selection) = fixture(false);
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        model.anki_setup.phase = AnkiSetupPhase::Ready;
        model.anki_setup.deck = Some(7);
        model.reader_word.as_mut().unwrap().candidates.words = vec![
            candidate(1, "猫", "ねこ", "cat & feline"),
            candidate(2, "猫", "ねこ", "different entry"),
        ];
        (model, selection)
    }

    fn update(model: &mut Model, event: Event) {
        drop(Yomoku.update(event, model));
    }

    fn acknowledge_live(model: &mut Model, result: AnkiLiveResult) {
        let request = model.anki_live.as_ref().unwrap().request.clone();
        let mut snapshot = AnkiRecoverySnapshot::default();
        match request.intent() {
            AnkiLiveIntent::Model(creation) => {
                snapshot.model = Some(AnkiModelRecovery {
                    id: 1.try_into().unwrap(),
                    creation: *creation.clone(),
                    known_model_id: None,
                })
            }
            AnkiLiveIntent::Note(submission) => {
                snapshot.note = Some(AnkiRecovery {
                    id: 1.try_into().unwrap(),
                    submission: *submission.clone(),
                    known_note_id: None,
                })
            }
        }
        update(
            model,
            Event::AnkiLiveBeginFinished(
                Box::new(request.clone()),
                StorageResult::AnkiLiveBegun(Box::new(snapshot.clone())),
            ),
        );
        let mut command = Yomoku.update(Event::DispatchAnkiLive, model);
        let effect = command
            .effects()
            .find_map(|effect| match effect {
                Effect::AnkiLive(request) => Some(request),
                _ => None,
            })
            .unwrap();
        let mut effect = effect;
        effect.resolve(result).unwrap();
        let event = command.events().next().unwrap();
        let Event::FinishAnkiLive(callback) = &event else {
            panic!("live completion");
        };
        let expected = callback.expected_result(&snapshot).unwrap();
        update(model, event);
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Saving));
        update(
            model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(request),
                StorageResult::AnkiLiveFinished(expected),
            ),
        );
    }

    #[test]
    fn entry_checks_do_not_write_and_one_tap_rechecks_then_begins_once() {
        let (mut model, selection) = configured();
        let before = model.reader_word.clone();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        assert_eq!(model.anki_entries.len(), 2);
        assert_eq!(
            model.anki_entries[0].request.export().note().fields()[2],
            "<ol><li>cat &amp; feline</li></ol>"
        );
        assert_eq!(
            model.anki_entries[1].request.export().note().fields()[2],
            "<ol><li>different entry</li></ol>"
        );
        for index in 0..2 {
            let request = model.anki_entries[index].request.clone();
            update(
                &mut model,
                Event::AnkiEntryChecked(Box::new(request), observation(vec![], true)),
            );
        }
        assert!(model.anki_live.is_none());
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word(1),
            },
        );
        let prepared = model.prepared_anki_export.clone().unwrap();
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word(2),
            },
        );
        assert_eq!(model.prepared_anki_export.as_ref(), Some(&prepared));
        let request = model.pending_anki_export_preflight().unwrap().clone();
        update(
            &mut model,
            Event::AnkiExportPreflightProviderFinished(
                Box::new(request.clone()),
                observation(vec![], true),
            ),
        );
        assert_eq!(
            model.anki_live_progress(),
            Some(AnkiLiveProgress::Beginning)
        );
        let live = model.anki_live.as_ref().unwrap().request.clone();
        update(
            &mut model,
            Event::AnkiExportPreflightProviderFinished(
                Box::new(request),
                observation(vec![], true),
            ),
        );
        assert_eq!(model.anki_live.as_ref().unwrap().request, live);
        assert_eq!(model.reader_word, before);
        update(&mut model, Event::DismissReaderWord);
        assert!(model.prepared_anki_export.is_some());
        acknowledge_live(&mut model, AnkiLiveResult::NoteAdded(88));
        assert!(model.anki_export_gate_open());
        assert!(model.prepared_anki_export.is_none());
        assert!(model.anki_entries.iter().all(|entry| matches!(&entry.result, Some(Ok(AnkiPreflightPlan::Existing(ids))) if ids.ids()[0] == 88.try_into().unwrap())));
    }

    #[test]
    fn existing_and_failed_reads_never_offer_add_and_stale_callbacks_cannot_relabel_entries() {
        let (mut model, selection) = configured();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        let old = model.anki_entries[0].request.clone();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        update(
            &mut model,
            Event::AnkiEntryChecked(Box::new(old), observation(vec![22], true)),
        );
        assert_eq!(model.anki_entries_view()[0].phase, AnkiEntryPhase::Checking);
        let request = model.anki_entries[0].request.clone();
        update(
            &mut model,
            Event::AnkiEntryChecked(Box::new(request), observation(vec![22, 23], true)),
        );
        assert_eq!(
            model.anki_entries_view()[0].phase,
            AnkiEntryPhase::Existing(vec![22, 23])
        );
        let request = model.anki_entries[1].request.clone();
        update(
            &mut model,
            Event::AnkiEntryChecked(
                Box::new(request),
                AnkiPreflightResult::Failed(AnkiExportError::PermissionDenied),
            ),
        );
        assert_eq!(
            model.anki_entries_view()[1].phase,
            AnkiEntryPhase::Failed(AnkiExportError::PermissionDenied)
        );
        for entry in [
            AnkiEntryKey::Word(1),
            AnkiEntryKey::Word(2),
            AnkiEntryKey::Name(1),
        ] {
            update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        }
        assert!(model.prepared_anki_export.is_none());
        model.reader_word_selection = None;
        assert!(model.anki_entries_view().is_empty());
    }

    #[test]
    fn model_setup_is_settings_only_and_uses_durable_begin_without_a_selected_word() {
        let mut model = Model::default();
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        model.anki_recovery = AnkiRecoveryState::Restored(Box::default());
        update(&mut model, Event::CheckAnkiSetup);
        let generation = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], false)),
        );
        assert_eq!(model.anki_setup.phase, AnkiSetupPhase::Missing);
        update(&mut model, Event::CreateAnkiModel);
        assert!(model.anki_live.is_none());
        model.settings_open = true;
        update(&mut model, Event::CreateAnkiModel);
        assert_eq!(
            model.anki_live_progress(),
            Some(AnkiLiveProgress::Beginning)
        );
        assert!(model.prepared_anki_export.is_none());
        assert_eq!(model.anki_export_view().unwrap().deck_id, 7);
        assert!(matches!(
            model.anki_live.as_ref().unwrap().request.intent(),
            AnkiLiveIntent::Model(_)
        ));
        acknowledge_live(&mut model, AnkiLiveResult::ModelCreated(9));
        assert_eq!(model.anki_setup.phase, AnkiSetupPhase::Ready);
        assert!(model.anki_export_gate_open());
        assert!(model.anki_export_view().is_none());
    }

    #[test]
    fn reader_can_observe_existing_notes_while_recovery_blocks_new_writes() {
        let (mut model, selection) = configured();
        model.anki_recovery = AnkiRecoveryState::Unrestored;
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        assert_eq!(model.anki_entries.len(), 2);
        for index in 0..2 {
            let request = model.anki_entries[index].request.clone();
            update(
                &mut model,
                Event::AnkiEntryChecked(
                    Box::new(request),
                    observation(if index == 0 { vec![88] } else { vec![] }, true),
                ),
            );
        }
        assert_eq!(
            model.anki_entries_view()[0].phase,
            AnkiEntryPhase::Existing(vec![88])
        );
        assert_eq!(
            model.anki_entries_view()[1].phase,
            AnkiEntryPhase::RecoveryRequired
        );
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word(2),
            },
        );
        assert!(model.prepared_anki_export.is_none());
        model.anki_recovery = AnkiRecoveryState::Restored(Box::default());
        assert_eq!(model.anki_entries_view()[1].phase, AnkiEntryPhase::Add);
    }

    #[test]
    fn component_export_uses_analyzer_anchor_instead_of_searching_the_sentence() {
        let (model, _) = fixture(true);
        let session = model.effective_reader.as_ref().unwrap();
        let page = session.page().unwrap();
        let token = page.tokens[0].clone();
        let chapter = &model.reader_document.as_ref().unwrap().chapters[0];
        let start = chapter.text.find('𠮷').unwrap();
        let source = crate::dictionary::DictionarySource {
            form: "𠮷".into(),
            anchor: TextAnchor {
                chapter_id: chapter.chapter_id,
                source_id: chapter.source_id,
                range: crate::document::TextRange {
                    start: (start as i64).try_into().unwrap(),
                    end: ((start + "𠮷".len()) as i64).try_into().unwrap(),
                },
            },
        };
        let details = crate::effective_session::EffectiveWordDetails {
            selection: crate::effective_session::EffectiveReaderSelection {
                page: page.identity,
                segment_index: token.segment_index,
            },
            candidates: crate::dictionary::DictionaryCandidates {
                surface: token.surface.clone(),
                dictionary_form: token.dictionary_form.clone(),
                words: vec![candidate(1, "𠮷", "よし", "good fortune")],
                names: vec![],
            },
            token,
            encountered: 0,

            dictionary_sources: vec![source],
        };
        let note = AnkiNote::from_effective_entry(
            chapter,
            page,
            details.selection,
            &details,
            Some(AnkiEntryKey::Word(1)),
        )
        .unwrap();
        assert_eq!(note.fields()[0], "𠮷");
        assert_eq!(
            note.fields()[3],
            "猫と<strong class=\"yomoku-target\">𠮷</strong>と猫。"
        );
        let mut bad = details;
        bad.dictionary_sources.clear();
        assert!(AnkiNote::from_effective_entry(
            chapter,
            page,
            bad.selection,
            &bad,
            Some(AnkiEntryKey::Word(1))
        )
        .is_err());
    }

    #[test]
    fn dictionary_headword_can_differ_from_the_highlighted_inflection() {
        let note = AnkiNote::with_occurrence(
            &crate::vocabulary::Surface::new("食べる").unwrap(),
            "たべる",
            &["to eat"],
            "食べたと食べた。",
            crate::document::TextRange {
                start: 12.try_into().unwrap(),
                end: 21.try_into().unwrap(),
            },
        )
        .unwrap();
        assert_eq!(note.fields()[0], "食べる");
        assert_eq!(
            note.fields()[3],
            "食べたと<strong class=\"yomoku-target\">食べた</strong>。"
        );
    }
}
