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
    configure: bool,
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
                AnkiExportPhase::NotSubmitted | AnkiExportPhase::ModelUncertain(_) => {
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
            AnkiExportPhase::NotSubmitted
            | AnkiExportPhase::NoteUncertain(_)
            | AnkiExportPhase::ModelUncertain(_) => Err(AnkiExportError::ProviderFailure),
            AnkiExportPhase::ProviderFailed(error) => Err(error),
            AnkiExportPhase::CreateModel => Err(AnkiExportError::IncompatibleModel),
            _ => return,
        };
        for state in &mut self.anki_entries {
            if state.request.export().note().same_identity(export.note()) {
                state.result = Some(result.clone());
            }
        }
        self.anki_live = None;
        self.prepared_anki_export = None;
        self.anki_export_preflight = None;
    }

    pub(super) fn current_vocabulary_anki_word(
        &self,
        selection: VocabularyAnkiSelection,
    ) -> Option<&crate::vocabulary::VocabularyWordState> {
        if !self.vocabulary_visible() {
            return None;
        }
        self.vocabulary.word.as_ref().filter(|word| {
            word.selection == selection
                && word.row.id == selection.vocabulary_id
                && word.candidates.is_some()
                && word.error.is_none()
        })
    }

    fn current_anki_selection(&self) -> Option<AnkiSelection> {
        if self.vocabulary_visible() {
            self.vocabulary
                .word
                .as_ref()
                .map(|word| AnkiSelection::Vocabulary(word.selection))
        } else if !self.settings_open && self.content_route == ContentRoute::Reader {
            self.effective_reader
                .word_state()
                .selection
                .map(AnkiSelection::Effective)
        } else {
            None
        }
    }

    fn entry_keys(&self, selection: AnkiSelection) -> Option<Vec<AnkiEntryKey>> {
        if self.settings_open {
            return None;
        }
        let effective_word = self.effective_reader.word_state();
        let candidates = match selection {
            AnkiSelection::Vocabulary(s) => {
                self.current_vocabulary_anki_word(s)?.candidates.as_ref()?
            }
            AnkiSelection::Effective(s) => {
                if self.content_route != ContentRoute::Reader {
                    return None;
                }
                let word = &effective_word;
                if word.selection != Some(s) || word.busy || word.error.is_some() {
                    return None;
                }
                let details = word.details.as_ref()?;
                if details.selection != s {
                    return None;
                }
                &details.candidates
            }
        };
        Some(AnkiEntryKey::for_candidates(candidates))
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
                    if active.note().same_identity(export.note()) {
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
                        } else {
                            AnkiEntryPhase::Busy
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
                    reading: export.note().reading().into(),
                    phase,
                })
            })
            .collect()
    }
}

impl Yomoku {
    pub(in crate::app) fn check_anki_setup(
        &self,
        model: &mut Model,
        configure: bool,
    ) -> Command<Effect, Event> {
        if model.anki_settings.phase != AnkiSettingsPhase::Ready
            || model.anki_operation_busy()
            || (model.anki_setup.phase == AnkiSetupPhase::Checking
                && (!configure || model.anki_setup.configure))
        {
            return Command::done();
        }
        let Some(deck) = model.anki_settings.default_deck_id else {
            model.anki_setup.phase = AnkiSetupPhase::Unchecked;
            model.anki_entries.clear();
            return Command::done();
        };
        let Some(id) = model.anki_setup.generation.checked_add(1) else {
            return Command::done();
        };
        model.anki_setup.generation = id;
        model.anki_setup.configure = configure;
        model.anki_setup.deck = Some(deck);
        model.anki_setup.phase = AnkiSetupPhase::Checking;
        // Retire observations and callbacks before querying the external collection.
        model.anki_entries.clear();
        // A model check has no selected word and never checks empty-word duplicates.
        let operation = AnkiPreflightOperation {
            surface: String::new(),
            reading: String::new(),
            deck_id: deck,
            layout: Box::new((&AnkiModelLayout::yomoku()).into()),
        };
        Command::request_from_shell(operation)
            .then_send(move |result| Event::AnkiSetupChecked(id, deck, result))
    }

    fn create_anki_model(&self, model: &mut Model) -> Command<Effect, Event> {
        let Some(deck) = model.anki_setup.deck else {
            return Command::done();
        };
        let Some(id) = model.anki_preparation_sequence.checked_add(1) else {
            return Command::done();
        };
        model.anki_preparation_sequence = id;
        model.prepared_anki_export = None;
        model.anki_export_preflight = None;
        model.anki_setup.live_deck = Some(deck);
        let intent = AnkiLiveIntent::Model(Box::new(
            AnkiModelCreation::new(AnkiModelLayout::yomoku()).expect("built-in layout"),
        ));
        self.begin_anki_intent(model, intent)
    }

    pub(super) fn update_anki_entries(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::CheckAnkiSetup(request) => {
                if *request == AnkiSetupRequest::Configure && !model.settings_open {
                    return Some(Command::done());
                }
                Some(self.check_anki_setup(model, *request == AnkiSetupRequest::Configure))
            }
            Event::AnkiSetupChecked(id, deck, result) => {
                if *id != model.anki_setup.generation
                    || model.anki_settings.default_deck_id != Some(*deck)
                    || model.anki_setup.phase != AnkiSetupPhase::Checking
                    || model.anki_settings.phase != AnkiSettingsPhase::Ready
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
                let configure = std::mem::take(&mut model.anki_setup.configure);
                if configure
                    && model.anki_setup.phase == AnkiSetupPhase::Missing
                    && model.anki_settings.phase == AnkiSettingsPhase::Ready
                    && model.anki_export_gate_open()
                {
                    return Some(self.create_anki_model(model));
                }
                if model.anki_setup.phase == AnkiSetupPhase::Ready {
                    if let Some(selection) = model.current_anki_selection() {
                        // Refresh even when Compose coalesces Checking -> Ready into one frame.
                        return self
                            .update_anki_entries(&Event::LoadReaderAnkiEntries(selection), model);
                    }
                }
                Some(Command::done())
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
                    .filter_map(|key| model.project_anki_export(*selection, deck, Some(key)))
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
                if model.entry_keys(request.export().selection()).is_none() {
                    return Some(Command::done());
                }
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
                    active.entry.is_some() && active.note().same_identity(export.note())
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
        let (mut model, selection) = fixture();
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        model.anki_setup.phase = AnkiSetupPhase::Ready;
        model.anki_setup.deck = Some(7);
        let words = vec![
            candidate(1, "猫", "ねこ", "cat & feline"),
            candidate(2, "猫", "ねこ", "different entry"),
        ];
        super::test_support::set_candidates(&mut model, words);
        (model, selection)
    }

    fn update(model: &mut Model, event: Event) {
        drop(Yomoku.update(event, model));
    }

    fn acknowledge_live(model: &mut Model, result: AnkiLiveResult) {
        let mut command = Yomoku.update(Event::DispatchAnkiLive, model);
        let mut effect = command
            .effects()
            .find_map(|effect| match effect {
                Effect::AnkiLive(r) => Some(r),
                _ => None,
            })
            .unwrap();
        effect.resolve(result).unwrap();
        let event = command.events().next().unwrap();
        update(model, event);
    }
    fn vocabulary_configured() -> (Model, AnkiSelection) {
        let (mut model, _) = configured();
        let candidates = model
            .effective_reader
            .word_state()
            .details
            .unwrap()
            .candidates;
        model.destination = Destination::Vocabulary;
        model.content_route = ContentRoute::Main;
        model.reader_document = None;
        let row = crate::vocabulary::VocabularyRow {
            id: 42.try_into().unwrap(),
            surface: "猫".into(),
            status: crate::vocabulary::VocabularyStatus::New,
            registered_at_unix_ms: 0,
            encountered_count: 0,
            last_seen_unix_ms: None,
        };
        model.vocabulary.rows = vec![row.clone()];
        update(&mut model, Event::OpenVocabularyWord(row.id));
        let sequence = model.vocabulary_request_sequence;
        update(
            &mut model,
            Event::VocabularyWordFinished(
                sequence,
                StorageResult::VocabularyWordLoaded(Box::new(
                    crate::vocabulary::VocabularyWordDetails { row, candidates },
                )),
            ),
        );
        let selection =
            AnkiSelection::Vocabulary(model.vocabulary.word.as_ref().unwrap().selection);
        (model, selection)
    }

    fn settle_entries(model: &mut Model, result: AnkiPreflightResult) {
        let requests: Vec<_> = model
            .anki_entries
            .iter()
            .map(|state| state.request.clone())
            .collect();
        for request in requests {
            update(
                model,
                Event::AnkiEntryChecked(Box::new(request), result.clone()),
            );
        }
    }

    #[test]
    fn vocabulary_exports_without_book_recheck_once_and_report_success() {
        let (mut model, selection) = vocabulary_configured();
        let before = model.vocabulary.clone();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        assert_eq!(model.anki_entries.len(), 2);
        for state in &model.anki_entries {
            let note = state.request.export().note();
            assert_eq!(note.surface().as_str(), "猫");
            assert_eq!(note.reading(), "ねこ");
            assert!(note.fields()[2].contains("JMdict"));
            assert!(note.fields()[3].is_empty());
        }
        settle_entries(&mut model, observation(vec![], true));
        let entry = model.anki_entries_view()[0].key;
        for _ in 0..2 {
            update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        }
        let request = model.pending_anki_export_preflight().unwrap().clone();
        update(
            &mut model,
            Event::AnkiExportPreflightProviderFinished(
                Box::new(request),
                observation(vec![], true),
            ),
        );
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
        acknowledge_live(&mut model, AnkiLiveResult::NoteAdded(88));
        assert!(model
            .anki_entries_view()
            .iter()
            .all(|entry| entry.phase == AnkiEntryPhase::Existing(vec![88])));
        assert!(model.anki_added_notice.is_some());
        update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        assert!(model.anki_live.is_none());
        assert_eq!(model.vocabulary, before);
    }

    #[test]
    fn vocabulary_duplicate_failure_retry_and_return_refresh_share_reader_guards() {
        let (mut model, selection) = vocabulary_configured();
        for phase in [AnkiSetupPhase::Unchecked, AnkiSetupPhase::Missing] {
            model.anki_setup.phase = phase;
            update(&mut model, Event::LoadReaderAnkiEntries(selection));
            assert!(model.anki_entries_view().is_empty());
        }
        model.anki_setup.phase = AnkiSetupPhase::Ready;
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        settle_entries(
            &mut model,
            AnkiPreflightResult::Failed(AnkiExportError::PermissionDenied),
        );
        let entry = model.anki_entries_view()[0].key;
        update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        assert!(model.prepared_anki_export.is_none());
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        settle_entries(&mut model, observation(vec![22, 23], true));
        assert!(model
            .anki_entries_view()
            .iter()
            .all(|entry| entry.phase == AnkiEntryPhase::Existing(vec![22, 23])));
        update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        assert!(model.prepared_anki_export.is_none());
        let old = model.anki_entries[0].request.clone();
        update(&mut model, Event::CheckAnkiSetup(AnkiSetupRequest::Refresh));
        let generation = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], true)),
        );
        assert_eq!(model.anki_entries.len(), 2);
        update(
            &mut model,
            Event::AnkiEntryChecked(Box::new(old), observation(vec![88], true)),
        );
        assert!(model
            .anki_entries_view()
            .iter()
            .all(|entry| entry.phase == AnkiEntryPhase::Checking));
        settle_entries(&mut model, observation(vec![], true));
        assert!(model
            .anki_entries_view()
            .iter()
            .all(|entry| entry.phase == AnkiEntryPhase::Add));
    }

    #[test]
    fn vocabulary_dismissal_navigation_and_reopen_reject_old_actions_and_callbacks() {
        let (mut model, old_selection) = vocabulary_configured();
        update(&mut model, Event::LoadReaderAnkiEntries(old_selection));
        let old = model.anki_entries[0].request.clone();
        let entry = old.export().entry.unwrap();
        update(&mut model, Event::DismissVocabularyWord);
        assert!(model
            .project_anki_export(old_selection, 7.try_into().unwrap(), Some(entry))
            .is_none());
        update(
            &mut model,
            Event::OpenVocabularyWord(42.try_into().unwrap()),
        );
        let current = model.vocabulary.word.as_ref().unwrap().selection;
        assert_ne!(AnkiSelection::Vocabulary(current), old_selection);
        let sequence = model.vocabulary_request_sequence;
        let row = model.vocabulary.word.as_ref().unwrap().row.clone();
        let candidates = old.export().note().surface().as_str();
        assert_eq!(candidates, row.surface);
        let dictionary = configured()
            .0
            .effective_reader
            .word_state()
            .details
            .unwrap()
            .candidates;
        update(
            &mut model,
            Event::VocabularyWordFinished(
                sequence,
                StorageResult::VocabularyWordLoaded(Box::new(
                    crate::vocabulary::VocabularyWordDetails {
                        row,
                        candidates: dictionary,
                    },
                )),
            ),
        );
        update(
            &mut model,
            Event::LoadReaderAnkiEntries(AnkiSelection::Vocabulary(current)),
        );
        update(
            &mut model,
            Event::AnkiEntryChecked(Box::new(old), observation(vec![88], true)),
        );
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection: old_selection,
                entry,
            },
        );
        assert!(model.prepared_anki_export.is_none());
        assert!(model
            .anki_entries_view()
            .iter()
            .all(|entry| entry.phase == AnkiEntryPhase::Checking));
        update(&mut model, Event::SelectDestination(Destination::Library));
        assert!(model.anki_entries_view().is_empty());
        assert!(model
            .project_anki_export(
                AnkiSelection::Vocabulary(current),
                7.try_into().unwrap(),
                Some(entry)
            )
            .is_none());
    }

    #[test]
    fn vocabulary_status_pending_failure_retry_and_success_preserve_observations() {
        let (mut model, selection) = vocabulary_configured();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        settle_entries(&mut model, observation(vec![88], true));
        let entries = model.anki_entries_view();
        let command = crate::vocabulary::VocabularyWordStatusCommand {
            id: 42.try_into().unwrap(),
            status: crate::vocabulary::VocabularyStatus::Learning,
            occurred_at_unix_ms: 1,
        };
        update(&mut model, Event::SetVocabularyWordStatus(command.clone()));
        assert_eq!(model.anki_entries_view(), entries);
        let sequence = model.vocabulary_request_sequence;
        update(
            &mut model,
            Event::VocabularyWordFinished(
                sequence,
                StorageResult::Failed(DocumentError::Unavailable),
            ),
        );
        assert_eq!(model.anki_entries_view(), entries);
        update(&mut model, Event::RetryVocabularyWordStatus);
        assert_eq!(model.anki_entries_view(), entries);
        let sequence = model.vocabulary_request_sequence;
        update(
            &mut model,
            Event::VocabularyWordFinished(
                sequence,
                StorageResult::VocabularyWordStatusChanged(command),
            ),
        );
        assert_eq!(model.anki_entries_view(), entries);
        assert_eq!(
            AnkiSelection::Vocabulary(model.vocabulary.word.as_ref().unwrap().selection),
            selection
        );
    }

    #[test]
    fn return_queries_can_refresh_during_reader_io_but_cannot_start_a_write() {
        let (mut model, selection) = configured();
        model.pending_storage = PendingStorage::Document {
            attempt: 0,
            request: DocumentRequest::Library,
        };
        update(
            &mut model,
            Event::CheckAnkiSetup(crate::anki::AnkiSetupRequest::Refresh),
        );
        let generation = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], true)),
        );
        assert_eq!(model.anki_entries.len(), 2);
        let requests = model
            .anki_entries
            .iter()
            .map(|entry| entry.request.clone())
            .collect::<Vec<_>>();
        for request in requests {
            update(
                &mut model,
                Event::AnkiEntryChecked(Box::new(request), observation(vec![], true)),
            );
        }
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word {
                    entry_id: 1,
                    reading: crate::dictionary::ReadingMatch {
                        spelling_index: Some(0),
                        reading_index: 0,
                    },
                },
            },
        );
        assert!(model.prepared_anki_export.is_none() && model.anki_live.is_none());
        model.pending_storage = PendingStorage::Idle;
        assert!(model.prepared_anki_export.is_none());
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word {
                    entry_id: 1,
                    reading: crate::dictionary::ReadingMatch {
                        spelling_index: Some(0),
                        reading_index: 0,
                    },
                },
            },
        );
        assert!(model.prepared_anki_export.is_some());
        assert!(model.anki_live.is_none()); // A fresh pre-insert query still precedes invocation.
    }

    #[test]
    fn returning_to_app_replaces_entry_observations_without_a_shell_reload() {
        let (mut model, selection) = configured();
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        let old = model.anki_entries[0].request.clone();
        update(
            &mut model,
            Event::AnkiEntryChecked(Box::new(old.clone()), observation(vec![88], true)),
        );
        for ids in [vec![], vec![99]] {
            update(
                &mut model,
                Event::CheckAnkiSetup(crate::anki::AnkiSetupRequest::Refresh),
            );
            assert!(model.anki_entries.is_empty());
            let generation = model.anki_setup.generation;
            update(
                &mut model,
                Event::AnkiSetupChecked(generation, 7, observation(vec![], true)),
            );
            assert_eq!(model.anki_entries.len(), 2);
            update(
                &mut model,
                Event::AnkiEntryChecked(Box::new(old.clone()), observation(vec![88], true)),
            );
            assert!(model
                .anki_entries
                .iter()
                .all(|entry| entry.result.is_none()));
            let requests = model
                .anki_entries
                .iter()
                .map(|entry| entry.request.clone())
                .collect::<Vec<_>>();
            for request in requests {
                update(
                    &mut model,
                    Event::AnkiEntryChecked(Box::new(request), observation(ids.clone(), true)),
                );
            }
            assert!(model
                .anki_entries_view()
                .iter()
                .all(|entry| if ids.is_empty() {
                    entry.phase == AnkiEntryPhase::Add
                } else {
                    entry.phase == AnkiEntryPhase::Existing(ids.clone())
                }));
            assert!(model.anki_live.is_none());
        }
    }

    #[test]
    fn entry_checks_do_not_write_and_one_tap_rechecks_then_begins_once() {
        let (mut model, selection) = configured();
        let before = model.effective_reader.word_state().details;
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        assert_eq!(model.anki_entries.len(), 2);
        assert_eq!(
            model.anki_entries[0].request.export().note().fields()[2],
            "<div class=\"yomoku-dictionary\">JMdict</div><ol><li>cat &amp; feline</li></ol>"
        );
        assert_eq!(
            model.anki_entries[1].request.export().note().fields()[2],
            "<div class=\"yomoku-dictionary\">JMdict</div><ol><li>different entry</li></ol>"
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
                entry: AnkiEntryKey::Word {
                    entry_id: 1,
                    reading: crate::dictionary::ReadingMatch {
                        spelling_index: Some(0),
                        reading_index: 0,
                    },
                },
            },
        );
        let prepared = model.prepared_anki_export.clone().unwrap();
        update(
            &mut model,
            Event::AddReaderAnkiEntry {
                selection,
                entry: AnkiEntryKey::Word {
                    entry_id: 2,
                    reading: crate::dictionary::ReadingMatch {
                        spelling_index: Some(0),
                        reading_index: 0,
                    },
                },
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
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
        let live = model.anki_live.as_ref().unwrap().request.clone();
        update(
            &mut model,
            Event::AnkiExportPreflightProviderFinished(
                Box::new(request),
                observation(vec![], true),
            ),
        );
        assert_eq!(model.anki_live.as_ref().unwrap().request, live);
        assert_eq!(model.effective_reader.word_state().details, before);
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
            AnkiEntryKey::Word {
                entry_id: 1,
                reading: crate::dictionary::ReadingMatch {
                    spelling_index: Some(0),
                    reading_index: 0,
                },
            },
            AnkiEntryKey::Word {
                entry_id: 2,
                reading: crate::dictionary::ReadingMatch {
                    spelling_index: Some(0),
                    reading_index: 0,
                },
            },
            AnkiEntryKey::Name {
                entry_id: 1,
                reading: crate::dictionary::ReadingMatch {
                    spelling_index: Some(0),
                    reading_index: 0,
                },
            },
        ] {
            update(&mut model, Event::AddReaderAnkiEntry { selection, entry });
        }
        assert!(model.prepared_anki_export.is_none());
        model.effective_reader.dismiss_word();
        assert!(model.anki_entries_view().is_empty());
    }

    #[test]
    fn model_setup_is_settings_only_and_dispatches_without_a_selected_word() {
        let mut model = Model::default();
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        update(
            &mut model,
            Event::CheckAnkiSetup(crate::anki::AnkiSetupRequest::Refresh),
        );
        let generation = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], false)),
        );
        assert_eq!(model.anki_setup.phase, AnkiSetupPhase::Missing);
        update(
            &mut model,
            Event::CheckAnkiSetup(crate::anki::AnkiSetupRequest::Configure),
        );
        assert!(model.anki_live.is_none());
        model.settings_open = true;
        update(
            &mut model,
            Event::CheckAnkiSetup(crate::anki::AnkiSetupRequest::Configure),
        );
        let generation = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], false)),
        );
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
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
    fn configuration_authorization_is_one_request_and_deck_even_after_navigation() {
        let mut model = Model {
            settings_open: true,
            ..Model::default()
        };
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        update(&mut model, Event::CheckAnkiSetup(AnkiSetupRequest::Refresh));
        let old = model.anki_setup.generation;
        update(
            &mut model,
            Event::CheckAnkiSetup(AnkiSetupRequest::Configure),
        );
        let current = model.anki_setup.generation;
        assert_ne!(old, current);
        for request in [AnkiSetupRequest::Configure, AnkiSetupRequest::Refresh] {
            update(&mut model, Event::CheckAnkiSetup(request));
            assert_eq!(current, model.anki_setup.generation);
        }
        update(
            &mut model,
            Event::AnkiSetupChecked(old, 7, observation(vec![], false)),
        );
        assert!(model.anki_live.is_none());
        update(&mut model, Event::CloseSettings);
        update(
            &mut model,
            Event::AnkiSetupChecked(current, 7, observation(vec![], false)),
        );
        let request = model.anki_live.as_ref().unwrap().request.clone();
        update(
            &mut model,
            Event::AnkiSetupChecked(current, 7, observation(vec![], false)),
        );
        assert_eq!(request, model.anki_live.as_ref().unwrap().request);
        acknowledge_live(&mut model, AnkiLiveResult::ModelUncertain(Some(9)));
        assert!(matches!(model.anki_setup.phase, AnkiSetupPhase::Failed(_)));
        update(&mut model, Event::CheckAnkiSetup(AnkiSetupRequest::Refresh));
        let refresh = model.anki_setup.generation;
        update(
            &mut model,
            Event::AnkiSetupChecked(refresh, 7, observation(vec![], false)),
        );
        assert!(model.anki_live.is_none());
        assert!(!model.anki_setup.configure);
    }

    #[test]
    fn setup_reuses_compatible_models_and_rejects_partial_models_without_writing() {
        for compatible in [true, false] {
            let mut model = Model {
                settings_open: true,
                ..Model::default()
            };
            model.anki_settings.phase = AnkiSettingsPhase::Ready;
            model.anki_settings.default_deck_id = Some(7);
            update(
                &mut model,
                Event::CheckAnkiSetup(AnkiSetupRequest::Configure),
            );
            let generation = model.anki_setup.generation;
            let mut result = observation(vec![], true);
            if let AnkiPreflightResult::Complete(value) = &mut result {
                value.models[0].matches_layout = compatible;
            }
            update(&mut model, Event::AnkiSetupChecked(generation, 7, result));
            assert!(model.anki_live.is_none());
            assert_eq!(
                model.anki_setup.phase,
                if compatible {
                    AnkiSetupPhase::Ready
                } else {
                    AnkiSetupPhase::Failed(AnkiExportError::IncompatibleModel)
                }
            );
            assert!(!model.anki_setup.configure);
        }
    }

    #[test]
    fn saving_another_deck_invalidates_pending_creation_authorization() {
        let mut model = Model {
            settings_open: true,
            ..Model::default()
        };
        model.anki_settings.phase = AnkiSettingsPhase::Ready;
        model.anki_settings.default_deck_id = Some(7);
        update(
            &mut model,
            Event::CheckAnkiSetup(AnkiSetupRequest::Configure),
        );
        let generation = model.anki_setup.generation;
        update(&mut model, Event::SetAnkiDefaultDeck(Some(8)));
        update(
            &mut model,
            Event::AnkiSetupChecked(generation, 7, observation(vec![], false)),
        );
        assert!(model.anki_live.is_none());
        let sequence = model.anki_settings_sequence;
        update(
            &mut model,
            Event::AnkiSettingsFinished(
                sequence,
                StorageResult::Failed(DocumentError::Unavailable),
            ),
        );
        assert!(model.anki_live.is_none());
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Failed);
    }

    #[test]
    fn reading_specific_actions_filter_senses_and_only_share_identical_pairs() {
        let (mut model, selection) = configured();
        let mut word = candidate(1, "猫", "ねこ", "cat & feline");
        let mut other_reading = word.entry.readings[0].clone();
        other_reading.text = "びょう".into();
        word.entry.readings.push(other_reading);
        word.entry.senses[0].stagr = vec!["ねこ".into()];
        word.entry.senses[0].pos = vec!["noun <label>".into()];
        word.entry.senses[0].s_inf = vec!["literal [sound:cat.mp3] & note".into()];
        let mut other_sense = word.entry.senses[0].clone();
        other_sense.stagr = vec!["びょう".into()];
        other_sense.glosses[0].text = "other reading".into();
        word.entry.senses.push(other_sense);
        word.forms[0].matches = word.entry.matches("猫");
        // Repeated source/base matches must not create another button or sense.
        word.forms.push(word.forms[0].clone());
        super::test_support::set_candidates(
            &mut model,
            vec![
                word,
                candidate(2, "猫", "ねこ", "same pair in another entry"),
            ],
        );
        update(&mut model, Event::LoadReaderAnkiEntries(selection));
        assert_eq!(model.anki_entries.len(), 3);
        let exports: Vec<_> = model
            .anki_entries
            .iter()
            .map(|state| state.request.export().clone())
            .collect();
        let fields = exports[0].note().fields();
        assert_eq!(fields[1], "ねこ");
        assert!(fields[2]
            .contains("<i class=\"yomoku-labels\">(noun &lt;label&gt;)</i> cat &amp; feline"));
        assert!(fields[2].contains("literal &#91;sound:cat.mp3&#93; &amp; note"));
        assert!(!fields[2].contains("other reading"));
        assert_eq!(exports[1].note().reading(), "びょう");
        assert!(!exports[1].note().fields()[2].contains("cat &amp; feline"));
        assert!(exports[0].note().same_identity(exports[2].note()));
        assert!(!exports[0].note().same_identity(exports[1].note()));
        for index in 0..3 {
            let request = model.anki_entries[index].request.clone();
            assert_eq!(
                request.operation().reading,
                request.export().note().reading()
            );
            update(
                &mut model,
                Event::AnkiEntryChecked(Box::new(request), observation(vec![], true)),
            );
        }
        for (index, id) in [(1, 81), (0, 82)] {
            update(
                &mut model,
                Event::AddReaderAnkiEntry {
                    selection,
                    entry: exports[index].entry.unwrap(),
                },
            );
            let request = model.pending_anki_export_preflight().unwrap().clone();
            update(
                &mut model,
                Event::AnkiExportPreflightProviderFinished(
                    Box::new(request),
                    observation(vec![], true),
                ),
            );
            acknowledge_live(&mut model, AnkiLiveResult::NoteAdded(id));
            let views = model.anki_entries_view();
            assert_eq!(views[1].phase, AnkiEntryPhase::Existing(vec![81]));
            if index == 1 {
                assert_eq!(views[0].phase, AnkiEntryPhase::Add);
                assert_eq!(views[2].phase, AnkiEntryPhase::Add);
            } else {
                assert_eq!(views[0].phase, AnkiEntryPhase::Existing(vec![82]));
                assert_eq!(views[2].phase, AnkiEntryPhase::Existing(vec![82]));
            }
        }
        let invalid = AnkiEntryKey::Word {
            entry_id: 1,
            reading: crate::dictionary::ReadingMatch {
                spelling_index: Some(99),
                reading_index: 0,
            },
        };
        assert!(model
            .project_anki_export(selection, 7.try_into().unwrap(), Some(invalid))
            .is_none());
    }

    #[test]
    fn component_export_uses_analyzer_anchor_instead_of_searching_the_sentence() {
        let (model, _) = fixture();
        let session = &model.effective_reader;
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
            Some(AnkiEntryKey::Word {
                entry_id: 1,
                reading: crate::dictionary::ReadingMatch {
                    spelling_index: Some(0),
                    reading_index: 0,
                },
            }),
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
            Some(AnkiEntryKey::Word {
                entry_id: 1,
                reading: crate::dictionary::ReadingMatch {
                    spelling_index: Some(0),
                    reading_index: 0
                }
            })
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
