//! Pure application state. Hosts execute effects; the core performs no I/O.

use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

use crate::{
    document::{DocumentId, LibraryDocument, PastedDocument},
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

#[derive(Default)]
pub struct Model {
    destination: Destination,
    settings_open: bool,
    documents: Vec<LibraryDocument>,
    loaded_document: Option<PastedDocument>,
    storage_busy: bool,
    document_error: Option<DocumentError>,
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
            Event::LoadPaste(id) => Some(StorageOperation::LoadPaste(*id)),
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
            model.loaded_document = None;
            return render::render()
                .and(Command::request_from_shell(operation).then_send(Event::StorageFinished));
        }
        match event {
            Event::SelectDestination(destination) => {
                model.destination = destination;
                model.settings_open = false;
            }
            Event::OpenSettings => model.settings_open = true,
            Event::CloseSettings => model.settings_open = false,
            Event::StorageFinished(result) => {
                model.storage_busy = false;
                match result {
                    StorageResult::Listed(documents) => model.documents = documents,
                    StorageResult::Created(document) => {
                        model.documents.push(LibraryDocument {
                            document_id: document.document_id,
                            title: document.title,
                        });
                    }
                    StorageResult::Loaded(document) => model.loaded_document = Some(document),
                    StorageResult::Failed(error) => model.document_error = Some(error),
                }
            }
            Event::RefreshLibrary | Event::CreatePaste(_) | Event::LoadPaste(_) => unreachable!(),
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
        }
    }
}
