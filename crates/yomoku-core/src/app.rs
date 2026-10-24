//! Pure application state. Hosts execute effects; the core performs no I/O.

use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

use crate::{
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
    /// Position-only persistence hook; page completion uses a separate command.
    SaveReadingPosition(ReadingPosition),
    SetPageLength(PageLength),
    DismissImportSummary,
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
        let operation = match &event {
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
            if model.storage_busy {
                return Command::done();
            }
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
                model.document_error = None;
            }
            Event::CloseDocument => {
                model.content_route = ContentRoute::Main;
                model.reader = None;
                model.reader_analysis = None;
                model.reader_document = None;
                model.pending_reader = None;
                model.document_error = None;
            }
            Event::OpenSettings => model.settings_open = true,
            Event::CloseSettings => model.settings_open = false,
            Event::DismissImportSummary => model.import_summary = None,
            Event::StorageFinished(result) => {
                model.storage_busy = false;
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
        }
    }
}

impl Model {
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
