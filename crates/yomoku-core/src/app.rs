//! Pure application state. Hosts execute effects; the core performs no I/O.

use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

use crate::{
    active_time::{ReadingTimeUpdate, SavedReadingTime},
    document::{DocumentId, LibraryDocument, PastedDocument},
    pagination::{PageLength, ReadingPosition},
    reader::{ReaderAnalysis, ReaderPage},
    storage::{DocumentError, StorageOperation, StorageResult},
};

/// The four top-level destinations; Settings is a separate toolbar flow.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum Destination {
    #[default]
    Library,
    Nostr,
    Statistics,
    Vocabulary,
}

/// Document flows overlay the selected top-level destination.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ContentRoute {
    #[default]
    Main,
    Paste,
    Reader,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ImportOutcome {
    Created,
    Existing,
}

/// Transient confirmation of a committed import; never restores or opens a reader.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ImportSummary {
    pub document_id: DocumentId,
    pub title: String,
    pub outcome: ImportOutcome,
}

#[derive(Default)]
pub struct Model {
    destination: Destination,
    settings_open: bool,
    documents: Vec<LibraryDocument>,
    loaded_document: Option<PastedDocument>,
    storage_busy: bool,
    document_error: Option<DocumentError>,
    content_route: ContentRoute,
    reader: Option<ReaderPage>,
    reader_analysis: Option<ReaderAnalysis>,
    pending_reader: Option<DocumentId>,
    reader_document: Option<PastedDocument>,
    page_length: Option<PageLength>,
    import_summary: Option<ImportSummary>,
    reader_time: Option<SavedReadingTime>,
    restoring_reader_time: bool,
    pending_time_checkpoint: Option<StorageOperation>,
    queued_time_checkpoint: Option<ReadingTimeUpdate>,
    deferred_reader_event: Option<Event>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum Event {
    SelectDestination(Destination),
    OpenSettings,
    CloseSettings,
    RefreshLibrary,
    CreatePaste(String),
    LoadPaste(DocumentId),
    OpenPaste,
    OpenReader(DocumentId),
    CloseDocument,
    /// Position-only persistence hook; waits for accepted time checkpoints.
    /// Page completion uses a separate command.
    SaveReadingPosition(ReadingPosition),
    SetPageLength(PageLength),
    DismissImportSummary,
    /// Explicit preparation for the platform timer; opening itself stays read-only.
    RestoreReadingTime,
    /// Coalesce newer absolute durations while a checkpoint is unresolved.
    CheckpointReadingTime(ReadingTimeUpdate),
    /// Retry the frozen absolute checkpoint, including its original revision.
    RetryReadingTimeCheckpoint,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StorageFinished(StorageResult),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ViewModel {
    pub destination: Destination,
    pub settings_open: bool,
    pub documents: Vec<LibraryDocument>,
    pub loaded_document: Option<PastedDocument>,
    pub storage_busy: bool,
    pub document_error: Option<DocumentError>,
    pub content_route: ContentRoute,
    pub reader: Option<ReaderPage>,
    /// Present only for an opened reader; Missing identifies a legacy paste.
    pub reader_analysis: Option<ReaderAnalysis>,
    /// None until persisted settings have been loaded successfully.
    pub page_length: Option<PageLength>,
    pub import_summary: Option<ImportSummary>,
    /// Only the committed duration at the current exact reading offset.
    pub reader_time: Option<ReadingTimeUpdate>,
    /// Remains true after a failed checkpoint until its exact retry succeeds.
    pub reading_time_save_pending: bool,
}

#[effect]
pub enum Effect {
    Render(RenderOperation),
    Storage(StorageOperation),
}

#[derive(Default)]
pub struct Yomoku;

impl App for Yomoku {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        if let Some(StorageOperation::SaveReadingTime { checkpoint, .. }) =
            model.pending_time_checkpoint.as_ref()
        {
            match &event {
                Event::CheckpointReadingTime(update) => {
                    // Keep the newest absolute duration without changing the frozen
                    // in-flight payload/revision, including after a failed save.
                    let newest = model.queued_time_checkpoint.as_ref().unwrap_or(checkpoint);
                    if !model.is_current_time(*update)
                        || update.pending.milliseconds() <= newest.pending.milliseconds()
                    {
                        return Command::done();
                    }
                    model.queued_time_checkpoint = Some(*update);
                    return render::render();
                }
                Event::CloseDocument
                | Event::OpenPaste
                | Event::SelectDestination(_)
                | Event::SaveReadingPosition(_) => {
                    // Accept one departure, then drain all accepted time first.
                    // Repeated callbacks cannot replace the first destination.
                    if model.deferred_reader_event.is_some() {
                        return Command::done();
                    }
                    model.deferred_reader_event = Some(event);
                    return render::render();
                }
                _ => {}
            }
        }
        let operation = match &event {
            Event::RestoreReadingTime => {
                let Some(reader) = model.reader.as_ref() else {
                    return Command::done();
                };
                if model.reader_time.is_some() {
                    return Command::done();
                }
                Some(StorageOperation::LoadReadingTime {
                    document_id: reader.document_id,
                    position: reader.position,
                })
            }
            Event::CheckpointReadingTime(checkpoint) => {
                let Some(previous) = model.reader_time else {
                    return Command::done();
                };
                if !model.is_current_time(*checkpoint)
                    || checkpoint.pending.milliseconds() < previous.pending().milliseconds()
                {
                    return Command::done();
                }
                Some(StorageOperation::SaveReadingTime {
                    previous,
                    checkpoint: *checkpoint,
                })
            }
            Event::RetryReadingTimeCheckpoint => {
                let Some(operation) = model.pending_time_checkpoint.clone() else {
                    return Command::done();
                };
                Some(operation)
            }
            Event::RefreshLibrary => Some(StorageOperation::List),
            Event::CreatePaste(text) => Some(StorageOperation::CreatePaste(text.clone())),
            Event::LoadPaste(id) | Event::OpenReader(id) => Some(StorageOperation::LoadPaste(*id)),
            Event::SetPageLength(length) => {
                if !model.settings_open
                    || model.page_length.is_none()
                    || model.page_length == Some(*length)
                {
                    return Command::done();
                }
                Some(StorageOperation::SavePageLength(*length))
            }
            Event::SaveReadingPosition(position) => {
                let Some(reader) = model
                    .reader
                    .as_ref()
                    .filter(|_| model.content_route == ContentRoute::Reader)
                else {
                    return Command::done();
                };
                Some(StorageOperation::SaveReadingPosition {
                    document_id: reader.document_id,
                    position: *position,
                })
            }
            _ => None,
        };
        if let Some(operation) = operation {
            // One in-flight command per session prevents duplicate submissions and
            // stale load/list callbacks from replacing a newer snapshot.
            if model.storage_busy
                || (model.pending_time_checkpoint.is_some()
                    && !matches!(event, Event::RetryReadingTimeCheckpoint))
            {
                return Command::done();
            }
            if matches!(operation, StorageOperation::SaveReadingTime { .. }) {
                model.pending_time_checkpoint = Some(operation.clone());
            }
            model.restoring_reader_time =
                matches!(operation, StorageOperation::LoadReadingTime { .. });
            model.storage_busy = true;
            model.document_error = None;
            if matches!(event, Event::CreatePaste(_) | Event::OpenReader(_)) {
                model.import_summary = None;
            }
            if matches!(
                event,
                Event::CreatePaste(_) | Event::LoadPaste(_) | Event::OpenReader(_)
            ) {
                model.loaded_document = None;
            }
            if let Event::OpenReader(id) = event {
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Reader;
                model.pending_reader = Some(id);
                model.reader = None;
                model.reader_analysis = None;
                model.reader_document = None;
                model.reader_time = None;
            }
            return render::render()
                .and(Command::request_from_shell(operation).then_send(Event::StorageFinished));
        }
        match event {
            Event::SelectDestination(destination) => {
                model.import_summary = None;
                model.destination = destination;
                model.settings_open = false;
                model.content_route = ContentRoute::Main;
                model.reader = None;
                model.reader_analysis = None;
                model.reader_document = None;
                model.pending_reader = None;
                model.clear_reader_time();
            }
            Event::OpenPaste => {
                model.import_summary = None;
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Paste;
                model.reader = None;
                model.reader_analysis = None;
                model.reader_document = None;
                model.pending_reader = None;
                model.clear_reader_time();
                model.document_error = None;
            }
            Event::CloseDocument => {
                model.content_route = ContentRoute::Main;
                model.reader = None;
                model.reader_analysis = None;
                model.reader_document = None;
                model.pending_reader = None;
                model.clear_reader_time();
                model.document_error = None;
            }
            Event::OpenSettings => model.settings_open = true,
            Event::CloseSettings => model.settings_open = false,
            Event::DismissImportSummary => model.import_summary = None,
            Event::StorageFinished(result) => {
                model.storage_busy = false;
                let restoring_time = std::mem::take(&mut model.restoring_reader_time);
                let pending_reader = model.pending_reader.take();
                let import_outcome = match &result {
                    StorageResult::Created(_) => Some(ImportOutcome::Created),
                    StorageResult::Existing(_) => Some(ImportOutcome::Existing),
                    _ => None,
                };
                match result {
                    StorageResult::Listed(documents, length) => {
                        model.documents = documents;
                        model.apply_page_length(length);
                    }
                    StorageResult::PageLengthSaved(length) => model.apply_page_length(length),
                    StorageResult::ReadingTimeLoaded(saved) => {
                        if restoring_time && model.is_current_time(saved.value) {
                            model.reader_time = Some(saved);
                        }
                    }
                    StorageResult::ReadingTimeSaved(saved) => {
                        model.pending_time_checkpoint = None;
                        if model.is_current_time(saved.value) {
                            model.reader_time = Some(saved);
                        }
                        if let Some(checkpoint) = model.queued_time_checkpoint.take() {
                            return self.update(Event::CheckpointReadingTime(checkpoint), model);
                        }
                        if let Some(event) = model.deferred_reader_event.take() {
                            return self.update(event, model);
                        }
                    }
                    StorageResult::Created(document) | StorageResult::Existing(document) => {
                        match document.library_summary() {
                            Ok(summary) => {
                                model.import_summary = Some(ImportSummary {
                                    document_id: summary.document_id,
                                    title: summary.title.clone(),
                                    outcome: import_outcome.expect("import result"),
                                });
                                if let Some(entry) = model
                                    .documents
                                    .iter_mut()
                                    .find(|entry| entry.document_id == summary.document_id)
                                {
                                    *entry = summary;
                                } else {
                                    model.documents.push(summary);
                                    model.documents.sort_by_key(|entry| entry.document_id);
                                }
                            }
                            Err(_) => model.document_error = Some(DocumentError::InvalidDocument),
                        }
                        model.destination = Destination::Library;
                        model.settings_open = false;
                        model.content_route = ContentRoute::Main;
                        model.reader = None;
                        model.reader_analysis = None;
                        model.reader_document = None;
                        model.clear_reader_time();
                    }
                    StorageResult::Loaded(snapshot) => {
                        let document = snapshot.document;
                        model.apply_page_length(snapshot.page_length);
                        if pending_reader == Some(document.document_id)
                            && model.content_route == ContentRoute::Reader
                        {
                            match ReaderPage::open(&document, snapshot.page_length) {
                                Ok(reader) => {
                                    model.reader = Some(reader);
                                    model.reader_analysis = Some(snapshot.analysis);
                                    model.reader_document = Some(document.clone());
                                }
                                Err(_) => {
                                    model.document_error = Some(DocumentError::InvalidDocument)
                                }
                            }
                        }
                        model.loaded_document = Some(document);
                    }
                    StorageResult::PositionSaved(document) => {
                        match document.library_summary() {
                            Ok(summary) => {
                                if let Some(entry) = model
                                    .documents
                                    .iter_mut()
                                    .find(|entry| entry.document_id == document.document_id)
                                {
                                    *entry = summary;
                                }
                            }
                            Err(_) => model.document_error = Some(DocumentError::InvalidDocument),
                        }
                        if model.content_route == ContentRoute::Reader
                            && model
                                .reader
                                .as_ref()
                                .is_some_and(|reader| reader.document_id == document.document_id)
                        {
                            match ReaderPage::open(&document, model.page_length.unwrap_or_default())
                            {
                                Ok(reader) => {
                                    if model.reader_time.is_some_and(|saved| {
                                        saved.value.document_id != reader.document_id
                                            || saved.position() != reader.position
                                    }) {
                                        model.reader_time = None;
                                    }
                                    model.reader = Some(reader);
                                    model.reader_document = Some(document.clone());
                                    model.loaded_document = Some(document);
                                }
                                Err(_) => {
                                    model.document_error = Some(DocumentError::InvalidDocument)
                                }
                            }
                        }
                    }
                    StorageResult::Failed(error) => model.document_error = Some(error),
                }
            }
            Event::RefreshLibrary
            | Event::CreatePaste(_)
            | Event::LoadPaste(_)
            | Event::OpenReader(_)
            | Event::SetPageLength(_)
            | Event::SaveReadingPosition(_) => unreachable!(),
            Event::RestoreReadingTime
            | Event::CheckpointReadingTime(_)
            | Event::RetryReadingTimeCheckpoint => unreachable!(),
        }
        render::render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            destination: model.destination,
            settings_open: model.settings_open,
            documents: model.documents.clone(),
            loaded_document: model.loaded_document.clone(),
            storage_busy: model.storage_busy,
            document_error: model.document_error,
            content_route: model.content_route,
            reader: model.reader.clone(),
            reader_analysis: model.reader_analysis,
            page_length: model.page_length,
            import_summary: model.import_summary.clone(),
            reader_time: model.reader_time.map(|saved| saved.value),
            reading_time_save_pending: model.pending_time_checkpoint.is_some(),
        }
    }
}

impl Model {
    fn is_current_time(&self, value: ReadingTimeUpdate) -> bool {
        self.content_route == ContentRoute::Reader
            && self.reader.as_ref().is_some_and(|reader| {
                reader.document_id == value.document_id && reader.position == value.position
            })
    }

    fn clear_reader_time(&mut self) {
        self.reader_time = None;
        self.restoring_reader_time = false;
        // Checkpoint obligations drain before reader departure reaches here.
    }

    fn apply_page_length(&mut self, length: PageLength) {
        self.page_length = Some(length);
        if let (Some(reader), Some(document)) = (&self.reader, &self.reader_document) {
            match ReaderPage::at(document, reader.position, length) {
                Ok(page) => self.reader = Some(page),
                Err(_) => self.document_error = Some(DocumentError::InvalidDocument),
            }
        }
    }
}
