//! Pure application state. Hosts execute effects; the core performs no I/O.

use crate::typography::ReaderTypography;
use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

mod anki;
mod effective;
mod pending_storage;
use pending_storage::{DocumentRequest, PendingStorage};
mod dictionary_lookup;
#[cfg(test)]
mod file_import_tests;
mod statistics;
mod statistics_preference;
mod vocabulary;
mod vocabulary_word;

use crate::{
    completion::{
        AnalyzedCompletionOutcome, CompletionNotice, PageCompletion, PageCompletionIntent,
    },
    document::{CompletionCommandId, DocumentId, LibraryDocument, ReaderDocument, TextAnchor},
    effective_session::{
        EffectivePageIdentity, EffectiveReaderPage, EffectiveReaderRequest,
        EffectiveReaderSelection, EffectiveReaderSession, EffectiveReaderWordState,
        EffectiveStatusState,
    },
    imports::{EpubImport, SrtImport, TxtImport},
    library::{valid_document_title, LibraryRenameState, LibrarySort, LibraryWordDistribution},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::{ReaderAnalysis, ReaderChapterAnalysis, ReaderPage, ReaderTokenKey},
    storage::{DocumentError, StorageOperation, StorageResult},
};

/// The four top-level destinations; Settings is a separate toolbar flow.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum Destination {
    #[default]
    Library,
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
    anki_settings: crate::anki::AnkiSettingsView,
    anki_entries: Vec<anki::entries::EntryState>,
    anki_setup: anki::entries::SetupState,
    anki_settings_sequence: u64,
    anki_settings_request: Option<(u64, anki::settings::SettingsRequest)>,
    prepared_anki_export: Option<crate::anki::AnkiExport>,
    anki_preparation_sequence: u64,
    anki_live: Option<crate::anki::AnkiLiveState>,
    anki_added_notice: Option<u64>,
    anki_notice_sequence: u64,
    anki_export_preflight: Option<crate::anki::AnkiPreflight>,
    vocabulary: crate::vocabulary::VocabularyState,
    vocabulary_request_sequence: u64,
    vocabulary_failed_offset: Option<u64>,
    vocabulary_refresh: Option<vocabulary::VocabularyRefresh>,
    vocabulary_status_changes_membership: bool,
    statistics: crate::statistics::StatisticsState,
    statistics_request_sequence: u64,
    statistics_preference_request: Option<(u64, Option<crate::statistics::StatisticsTimeframe>)>,
    statistics_preference_sequence: u64,
    statistics_preference_loaded: bool,
    statistics_preference_dirty: bool,
    statistics_preference_error: Option<DocumentError>,
    destination: Destination,
    settings_open: bool,
    documents: Vec<LibraryDocument>,
    library_sort: LibrarySort,
    library_word_distributions: Option<Vec<LibraryWordDistribution>>,
    library_word_error: Option<DocumentError>,
    library_delete_target: Option<LibraryDocument>,
    library_delete_error: Option<DocumentError>,
    library_rename: Option<LibraryRenameState>,
    pending_storage: PendingStorage,
    storage_sequence: u64,
    document_error: Option<DocumentError>,
    file_import_error: Option<DocumentError>,
    content_route: ContentRoute,
    reader: Option<ReaderPage>,
    reader_analysis: Option<ReaderAnalysis>,
    reader_document: Option<ReaderDocument>,
    reader_analyses: Vec<ReaderChapterAnalysis>,
    page_length: Option<PageLength>,
    reader_typography: Option<ReaderTypography>,
    import_summary: Option<ImportSummary>,
    deferred_reader_event: Option<Event>,
    completion_notice: Option<CompletionNotice>,
    book_completion_notice: Option<CompletionCommandId>,
    effective_reader: EffectiveReaderSession,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum Event {
    SelectDestination(Destination),
    /// Lazy read for the retained timeframe. Repeated identical loads coalesce.
    LoadStatistics {
        as_of_unix_ms: i64,
    },
    SetStatisticsTimeframe {
        timeframe: crate::statistics::StatisticsTimeframe,
        as_of_unix_ms: i64,
    },
    /// Retry the exact failed query, retaining its original as-of instant.
    RetryStatistics,
    /// Lazy first page for the retained literal query; failures require Retry.
    LoadVocabulary,
    SetVocabularyQuery(crate::vocabulary::VocabularyQuery),
    LoadMoreVocabulary,
    RetryVocabulary,
    OpenVocabularyWord(crate::document::VocabularyId),
    DismissVocabularyWord,
    RetryVocabularyWord,
    SetVocabularyWordStatus(crate::vocabulary::VocabularyWordStatusCommand),
    RetryVocabularyWordStatus,
    CancelVocabularyWordStatus,
    OpenSettings,
    CloseSettings,
    RefreshLibrary,
    /// Read-only lazy refresh when Library is visible; retries are explicit.
    LoadLibraryWordDistributions,
    SortLibrary(LibrarySort),
    /// Freeze one visible Library entry for explicit confirmation.
    RequestDocumentDeletion(DocumentId),
    CancelDocumentDeletion,
    ConfirmDocumentDeletion(DocumentId),
    RequestDocumentRename(DocumentId),
    EditDocumentTitle {
        document_id: DocumentId,
        title: String,
    },
    CancelDocumentRename,
    SaveDocumentTitle(DocumentId),
    CreatePaste(String),

    OpenPaste,
    OpenReader(DocumentId),
    CloseDocument,
    /// Position-only persistence hook.
    /// Page completion uses a separate command.
    SaveReadingPosition(ReadingPosition),
    SetPageLength(PageLength),
    SetReaderTypography(ReaderTypography),
    DismissImportSummary,
    /// Retry preparation or the exact frozen analyzed command after a failure.
    RetryReaderCompletion,
    DismissCompletionNotice,
    DismissAnkiAddedNotice(u64),
    /// Position-only backward navigation anchored to the visible page.
    PreviousReaderPage {
        document_id: DocumentId,
        page: TextAnchor,
        position: ReadingPosition,
    },
    /// Position-only forward navigation; never completes the departing page.
    SkipReaderPage {
        document_id: DocumentId,
        page: TextAnchor,
        position: ReadingPosition,
    },
    /// Consume before starting the transient celebration; stale acknowledgments ignore.
    DismissBookCompletionNotice(CompletionCommandId),
    /// Explicit refresh; stale UI callbacks cannot request another page's tokens.
    LoadReaderTokens(ReaderTokenKey),

    DismissReaderWord,

    /// Per-file atomic import; success returns to Library with a summary.
    ImportTxt(TxtImport),
    ImportSrt(SrtImport),
    ImportEpub(EpubImport),
    /// Compact rendered identity; full effective provenance stays in the model.
    CompleteEffectiveReaderPage {
        identity: EffectivePageIdentity,
        position: ReadingPosition,
        completed_at_unix_ms: i64,
    },
    LoadEffectiveReaderWord(EffectiveReaderSelection),

    SetEffectiveReaderStatus {
        selection: EffectiveReaderSelection,
        vocabulary_id: crate::document::VocabularyId,
        status: crate::vocabulary::VocabularyStatus,
        occurred_at_unix_ms: i64,
    },
    RetryEffectiveReaderStatus,
    CancelEffectiveReaderStatus,
    // Keep shell variants before skipped native callbacks: Serde's serializer
    // counts skipped variants while its deserializer/typegen omit them.
    /// Read-only dispatch for the native frozen export; no mutation authorization.
    BeginAnkiExportPreflight,
    /// Explicitly abandon a lost read or old observation and capture a fresh handle.
    RetryAnkiExportPreflight,

    /// Consume an in-memory native invocation once; cannot prepare or begin a write.
    DispatchAnkiLive,
    /// Freeze the current loaded dictionary selection with the acknowledged configured deck.
    /// Unready/unconfigured settings and invalid selections do nothing; no I/O is started.
    PrepareReaderAnkiExport {
        selection: crate::anki::AnkiSelection,
    },
    /// Begin only the exact frozen intent and preflight phase the shell presented.
    BeginPreparedAnkiExport {
        preparation_id: u64,
        expected_phase: crate::anki::AnkiExportPhase,
    },
    /// Dismiss only the matching frozen intent; active-operation gates still apply.
    ClearAnkiExport {
        preparation_id: u64,
    },
    /// Read local preferences at startup or in Settings; never queries the provider.
    LoadAnkiSettings,
    LoadStatisticsTimeframe,
    /// Save a positive signed-64 deck preference, or explicitly clear it.
    SetAnkiDefaultDeck(Option<i64>),
    /// Read back authoritative local settings after failure or a lost response.
    RetryAnkiSettings,
    /// Read-only entry availability for the current loaded selection.
    LoadReaderAnkiEntries(crate::anki::AnkiSelection),
    /// One explicit tap authorizes only this current dictionary entry.
    AddReaderAnkiEntry {
        selection: crate::anki::AnkiSelection,
        entry: crate::anki::AnkiEntryKey,
    },
    CheckAnkiSetup(crate::anki::AnkiSetupRequest),
    ChooseDictionaryPart {
        selection: crate::anki::AnkiSelection,
        part: Option<u64>,
    },
    RetryDictionaryPart(crate::anki::AnkiSelection),
    SetDocumentRead {
        document_id: DocumentId,
        is_read: bool,
    },
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiEntryChecked(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        Box<crate::anki::AnkiPreflightRequest>,
        crate::anki::AnkiPreflightResult,
    ),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiSetupChecked(u64, i64, crate::anki::AnkiPreflightResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiSettingsFinished(u64, StorageResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    EffectiveStorageFinished(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Box<EffectiveReaderRequest>,
        StorageResult,
    ),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StorageFinished(u64, StorageResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StatisticsFinished(u64, StorageResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyFinished(u64, StorageResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyWordFinished(u64, StorageResult),
    /// Native preparation only. No provider dispatch or live callback token.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    PrepareAnkiExport {
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        selection: crate::anki::AnkiSelection,
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        deck: crate::anki::AnkiDeckId,
    },
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    ClearPreparedAnkiExport,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiExportPreflightProviderFinished(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        Box<crate::anki::AnkiPreflightRequest>,
        crate::anki::AnkiPreflightResult,
    ),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiExportPreflightFinished(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        Box<crate::anki::AnkiPreflightRequest>,
        #[cfg_attr(feature = "facet_typegen", facet(opaque))]
        Result<crate::anki::AnkiPreflightObservation, crate::anki::AnkiExportError>,
    ),
    /// Native durable begin; preparation and preflight remain required.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    BeginAnkiLive,

    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    FinishAnkiLive(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Box<crate::anki::AnkiLiveCallback>,
    ),

    /// Invalidate a lost native callback; explicit Retry must restore read-only.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AbandonAnkiLive,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StatisticsTimeframeFinished(u64, StorageResult),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    DictionaryLookupFinished(crate::anki::AnkiSelection, StorageResult),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ViewModel {
    pub anki_settings: crate::anki::AnkiSettingsView,
    pub anki_setup: crate::anki::AnkiSetupPhase,
    pub anki_entries: Vec<crate::anki::AnkiEntryView>,
    /// Transient, confirmed provider insertion; consume before displaying.
    pub anki_added_notice: Option<u64>,
    /// Session-owned frozen export status; observed IDs carry no callback authority.
    pub anki_export: Option<crate::anki::AnkiExportView>,
    pub vocabulary: crate::vocabulary::VocabularyState,
    pub statistics: crate::statistics::StatisticsState,
    pub destination: Destination,
    pub settings_open: bool,
    /// All documents in the selected Library order.
    pub documents: Vec<LibraryDocument>,
    /// Session-local selection; read recency itself is durable.
    pub library_sort: LibrarySort,
    pub library_word_distributions: Option<Vec<LibraryWordDistribution>>,
    pub library_word_error: Option<DocumentError>,
    pub library_delete_target: Option<LibraryDocument>,
    pub library_delete_error: Option<DocumentError>,
    pub library_rename: Option<LibraryRenameState>,
    pub storage_busy: bool,
    pub document_error: Option<DocumentError>,
    pub content_route: ContentRoute,
    pub reader: Option<ReaderPage>,
    /// Present only for an opened reader; Missing identifies unavailable stored analysis.
    pub reader_analysis: Option<ReaderAnalysis>,
    /// None until persisted settings have been loaded successfully.
    pub page_length: Option<PageLength>,
    pub reader_typography: Option<ReaderTypography>,
    pub import_summary: Option<ImportSummary>,
    pub completion_save_pending: bool,
    pub completion_notice: Option<CompletionNotice>,
    /// One newly recorded final-page command, independent of permanent finish state.
    pub book_completion_notice: Option<CompletionCommandId>,
    /// None until explicitly loaded for the current document/page/base analysis.
    pub effective_reader_tokens: Option<EffectiveReaderPage>,
    pub effective_reader_word: Option<EffectiveReaderWordState>,
    pub effective_reader_status: Option<EffectiveStatusState>,
}

#[effect]
pub enum Effect {
    Render(RenderOperation),
    Storage(StorageOperation),

    AnkiExportPreflight(crate::anki::AnkiPreflightOperation),
    AnkiLive(crate::anki::AnkiLiveOperation),
}

#[derive(Default)]
pub struct Yomoku;

impl App for Yomoku {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        if let Some(command) = self.update_anki_settings(&event, model) {
            return command;
        }
        let anki_before = (
            model.anki_export_view(),
            model.anki_entries_view(),
            model.anki_setup.phase.clone(),
            model.anki_added_notice,
        );
        if let Some(command) = self.update_anki(&event, model) {
            if anki_before
                != (
                    model.anki_export_view(),
                    model.anki_entries_view(),
                    model.anki_setup.phase.clone(),
                    model.anki_added_notice,
                )
            {
                return command.and(render::render());
            }
            return command;
        }
        if let Event::VocabularyWordFinished(request_id, result) = event {
            return self.finish_vocabulary_word(request_id, result, model);
        }
        if let Event::DictionaryLookupFinished(selection, result) = event {
            return self.finish_dictionary_lookup(selection, result, model);
        }
        if let Some(command) = self.update_dictionary_lookup(&event, model) {
            return command;
        }
        if let Some(command) = self.update_vocabulary_word(&event, model) {
            return command;
        }
        if let Event::VocabularyFinished(request_id, result) = event {
            return self.finish_vocabulary(request_id, result, model);
        }
        if let Some(command) = self.update_vocabulary(&event, model) {
            return command;
        }
        if let Event::StatisticsFinished(request_id, result) = event {
            return self.finish_statistics(request_id, result, model);
        }
        if let Some(command) = self.update_statistics_preference(&event, model) {
            return command;
        }
        if let Some(command) = self.update_statistics(&event, model) {
            return command;
        }
        if let Event::EffectiveStorageFinished(request, result) = event {
            return self.finish_effective(&request, result, model);
        }
        if let Some(command) = self.update_effective(&event, model) {
            return command;
        }
        let mut skip_position = None;
        if let Event::PreviousReaderPage {
            document_id,
            page,
            position,
        }
        | Event::SkipReaderPage {
            document_id,
            page,
            position,
        } = event
        {
            let skip = matches!(event, Event::SkipReaderPage { .. });
            if model.storage_busy()
                || model.effective_mutation_pending()
                || model.deferred_reader_event.is_some()
                || model.content_route != ContentRoute::Reader
                || model.settings_open
                || model.completion_pending()
                || !model.reader.as_ref().is_some_and(|reader| {
                    reader.document_id == document_id
                        && reader.anchor == page
                        && reader.position == position
                })
            {
                return Command::done();
            }
            let Some(document) = model.reader_document.as_ref() else {
                return Command::done();
            };
            let Ok(pages) =
                LogicalPages::new(&document.chapters, model.page_length.unwrap_or_default())
            else {
                return Command::done();
            };
            let Some(destination) = pages
                .page_at(position)
                .ok()
                .and_then(|index| {
                    if skip {
                        index.checked_add(1)
                    } else {
                        index.checked_sub(1)
                    }
                })
                .and_then(|index| pages.pages().get(index).copied())
            else {
                return Command::done();
            };
            let target = ReadingPosition {
                source_id: destination.source_id,
                chapter_id: destination.chapter_id,
                offset: destination.range.start,
            };
            if skip {
                skip_position = Some(target);
            } else {
                return self.update(Event::SaveReadingPosition(target), model);
            }
        }

        if (model.completion_pending() || model.effective_mutation_pending())
            && matches!(
                event,
                Event::CloseDocument
                    | Event::OpenPaste
                    | Event::SelectDestination(_)
                    | Event::SaveReadingPosition(_)
            )
        {
            if model.deferred_reader_event.is_some() {
                return Command::done();
            }
            model.deferred_reader_event = Some(event);
            model.invalidate_effective_page();
            return render::render();
        }
        let operation = match &event {
            Event::RefreshLibrary => Some(StorageOperation::List),
            Event::SaveDocumentTitle(id) => {
                let Some(draft) = model.library_rename.as_ref() else {
                    return Command::done();
                };
                if !model.library_visible()
                    || draft.document_id != *id
                    || !valid_document_title(&draft.title)
                {
                    return Command::done();
                }
                Some(StorageOperation::RenameDocument {
                    document_id: *id,
                    title: draft.title.clone(),
                })
            }
            Event::SetDocumentRead {
                document_id,
                is_read,
            } => {
                if !model.library_visible()
                    || !model
                        .documents
                        .iter()
                        .any(|entry| entry.document_id == *document_id && entry.is_read != *is_read)
                {
                    return Command::done();
                }
                Some(StorageOperation::SetDocumentRead {
                    document_id: *document_id,
                    is_read: *is_read,
                })
            }
            Event::ConfirmDocumentDeletion(id) => {
                if !model.library_visible()
                    || !model
                        .library_delete_target
                        .as_ref()
                        .is_some_and(|target| target.document_id == *id)
                {
                    return Command::done();
                }
                Some(StorageOperation::DeleteDocument(*id))
            }
            Event::LoadLibraryWordDistributions => {
                if model.destination != Destination::Library
                    || model.content_route != ContentRoute::Main
                    || model.settings_open
                {
                    return Command::done();
                }
                Some(StorageOperation::LoadEffectiveLibraryWordDistributions)
            }
            Event::CreatePaste(text) => Some(StorageOperation::CreatePaste(text.clone())),
            Event::ImportTxt(input) => Some(StorageOperation::ImportTxt(input.clone())),
            Event::ImportSrt(input) => Some(StorageOperation::ImportSrt(input.clone())),
            Event::ImportEpub(input) => Some(StorageOperation::ImportEpub(input.clone())),

            Event::OpenReader(id) => Some(StorageOperation::OpenReader(*id)),

            Event::SetReaderTypography(typography) => {
                let active_reader =
                    model.content_route == ContentRoute::Reader && model.reader.is_some();
                if (!model.settings_open && !active_reader)
                    || model.reader_typography.is_none()
                    || model.reader_typography == Some(*typography)
                {
                    return Command::done();
                }
                Some(StorageOperation::SaveReaderTypography(*typography))
            }
            Event::SetPageLength(length) => {
                if !model.settings_open
                    || model.page_length.is_none()
                    || model.page_length == Some(*length)
                {
                    return Command::done();
                }
                Some(StorageOperation::SavePageLength(*length))
            }
            Event::SkipReaderPage { document_id, .. } => {
                Some(StorageOperation::SaveReadingPosition {
                    document_id: *document_id,
                    position: skip_position.expect("validated skip destination"),
                })
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
            if model.storage_busy()
                || model.effective_mutation_pending()
                || model.completion_pending()
            {
                return Command::done();
            }
            let Some(attempt) = model.storage_sequence.checked_add(1) else {
                model.document_error = Some(DocumentError::Unavailable);
                return render::render();
            };
            model.storage_sequence = attempt;
            let request = DocumentRequest::capture(&operation);
            if !matches!(request, DocumentRequest::LibraryWords) {
                model.file_import_error = None;
            }
            if !matches!(
                request,
                DocumentRequest::FileImport
                    | DocumentRequest::LibraryWords
                    | DocumentRequest::Delete(_)
                    | DocumentRequest::Rename(..)
            ) {
                model.document_error = None;
            }
            model.pending_storage = PendingStorage::Document { attempt, request };
            if let StorageOperation::RenameDocument { .. } = &operation {
                if let Some(draft) = model.library_rename.as_mut() {
                    draft.error = None;
                }
            } else if !operation.loads_library_words() {
                model.library_rename = None;
            }
            match &operation {
                StorageOperation::DeleteDocument(_) => {
                    model.library_delete_error = None;
                }
                StorageOperation::LoadEffectiveLibraryWordDistributions => {}
                _ => {
                    model.clear_library_delete();
                }
            }
            if operation.loads_library_words() {
                model.library_word_distributions = None;
                model.library_word_error = None;
            }

            // Every accepted operation that invalidates page tokens also retires
            // its selected-word result, even if the resulting anchor is unchanged.
            if matches!(
                operation,
                |StorageOperation::List| StorageOperation::OpenReader(_)
                    | StorageOperation::CreatePaste(_)
                    | StorageOperation::ImportTxt(_)
                    | StorageOperation::ImportSrt(_)
                    | StorageOperation::ImportEpub(_)
                    | StorageOperation::SavePageLength(_)
                    | StorageOperation::SaveReadingPosition { .. }
            ) && !matches!(event, Event::SkipReaderPage { .. })
            {
                model.invalidate_effective_page();
                model.clear_reader_word();
            }
            // Skip retains the current projection until the position save succeeds,
            // so a failed save can retry the same menu action without reloading words.

            if matches!(
                event,
                Event::CreatePaste(_)
                    | Event::ImportTxt(_)
                    | Event::ImportSrt(_)
                    | Event::ImportEpub(_)
                    | Event::OpenReader(_)
            ) {
                model.import_summary = None;
                model.completion_notice = None;
                model.book_completion_notice = None;
            }

            if let Event::OpenReader(_) = event {
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Reader;
                model.clear_reader_presentation();
            }
            return render::render().and(
                Command::request_from_shell(operation)
                    .then_send(move |result| Event::StorageFinished(attempt, result)),
            );
        }
        match event {
            Event::RequestDocumentRename(id) => {
                if !model.library_visible() || model.storage_busy() || model.completion_pending() {
                    return Command::done();
                }
                let Some(target) = model
                    .library_sort
                    .project(&model.documents)
                    .into_iter()
                    .find(|entry| entry.document_id == id)
                else {
                    return Command::done();
                };
                model.clear_library_delete();
                model.library_rename = Some(LibraryRenameState {
                    document_id: id,
                    title: target.title,
                    error: None,
                });
            }
            Event::EditDocumentTitle { document_id, title } => {
                if model.storage_busy() || !model.library_visible() {
                    return Command::done();
                }
                let Some(draft) = model.library_rename.as_mut() else {
                    return Command::done();
                };
                if draft.document_id != document_id {
                    return Command::done();
                }
                draft.title = title;
                draft.error = None;
            }
            Event::CancelDocumentRename => {
                if matches!(
                    model.pending_storage,
                    PendingStorage::Document {
                        request: DocumentRequest::Rename(..),
                        ..
                    }
                ) {
                    return Command::done();
                }
                model.library_rename = None;
            }
            Event::RequestDocumentDeletion(id) => {
                if !model.library_visible() || model.storage_busy() || model.completion_pending() {
                    return Command::done();
                }
                let Some(target) = model
                    .library_sort
                    .project(&model.documents)
                    .into_iter()
                    .find(|entry| entry.document_id == id)
                else {
                    return Command::done();
                };
                model.library_delete_target = Some(target);
                model.library_delete_error = None;
                model.library_rename = None;
            }
            Event::CancelDocumentDeletion => {
                if matches!(
                    model.pending_storage,
                    PendingStorage::Document {
                        request: DocumentRequest::Delete(_),
                        ..
                    }
                ) {
                    return Command::done();
                }
                model.clear_library_delete();
            }
            Event::SortLibrary(sort) => {
                if model.destination != Destination::Library
                    || model.content_route != ContentRoute::Main
                    || model.settings_open
                {
                    return Command::done();
                }
                model.library_sort = sort;
            }
            Event::SelectDestination(destination) => {
                if destination == Destination::Statistics
                    && model.destination != Destination::Statistics
                {
                    model.invalidate_statistics();
                }
                if destination == Destination::Vocabulary
                    && model.destination != Destination::Vocabulary
                {
                    model.invalidate_vocabulary();
                }
                model.library_rename = None;
                model.clear_library_delete();
                if destination == Destination::Library {
                    model.invalidate_library_words();
                }
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.import_summary = None;
                model.destination = destination;
                model.settings_open = false;
                model.content_route = ContentRoute::Main;
                model.clear_reader_presentation();
            }
            Event::OpenPaste => {
                model.library_rename = None;
                model.clear_library_delete();
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.import_summary = None;
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Paste;
                model.clear_reader_presentation();

                model.document_error = None;
            }
            Event::CloseDocument => model.close_document(),
            // Settings intentionally overlays the retained route: closing it
            // returns to the same reader/paste/main presentation.
            Event::OpenSettings => {
                model.library_rename = None;
                model.clear_library_delete();
                model.settings_open = true;
                model.clear_reader_word();
            }
            Event::DismissReaderWord => model.clear_reader_word(),
            Event::CloseSettings => model.settings_open = false,
            Event::DismissImportSummary => {
                model.import_summary = None;
                model.file_import_error = None;
            }
            Event::DismissAnkiAddedNotice(id) => {
                if model.anki_added_notice == Some(id) {
                    model.anki_added_notice = None;
                }
            }
            Event::DismissCompletionNotice => model.completion_notice = None,
            Event::DismissBookCompletionNotice(id) => {
                if model.book_completion_notice == Some(id) {
                    model.book_completion_notice = None;
                }
            }
            Event::StorageFinished(attempt, result) => {
                if !matches!(model.pending_storage, PendingStorage::Document { attempt: current, .. } if current == attempt)
                {
                    return Command::done();
                }
                let PendingStorage::Document { request, .. } =
                    std::mem::take(&mut model.pending_storage)
                else {
                    unreachable!()
                };
                let loading_library_words = matches!(request, DocumentRequest::LibraryWords);
                let pending_reader = match request {
                    DocumentRequest::Reader(id) => Some(id),
                    _ => None,
                };
                let document_delete = match request {
                    DocumentRequest::Delete(id) => Some(id),
                    _ => None,
                };
                let document_rename = match &request {
                    DocumentRequest::Rename(id, title) => Some((*id, title)),
                    _ => None,
                };
                let import_outcome = match &result {
                    StorageResult::Created(_) => Some(ImportOutcome::Created),
                    StorageResult::Existing(_) => Some(ImportOutcome::Existing),
                    _ => None,
                };
                if matches!(result, StorageResult::Created(_)) {
                    model.invalidate_statistics();
                    model.invalidate_vocabulary();
                }
                match result {
                    StorageResult::DocumentRenamed { document_id, title } => {
                        if document_rename
                            .is_some_and(|(id, expected)| id == document_id && expected == &title)
                        {
                            if let Some(document) = model
                                .documents
                                .iter_mut()
                                .find(|entry| entry.document_id == document_id)
                            {
                                document.title.clone_from(&title);
                            }
                            if let Some(summary) = model
                                .import_summary
                                .as_mut()
                                .filter(|summary| summary.document_id == document_id)
                            {
                                summary.title.clone_from(&title);
                            }
                            // A cached read must not present the old title later.

                            model.library_rename = None;
                        }
                    }
                    StorageResult::DocumentDeleted(id) => {
                        if document_delete == Some(id) {
                            model.apply_document_deletion(id);
                        }
                    }
                    StorageResult::DocumentReadSet {
                        document_id,
                        is_read,
                    } => {
                        if matches!(request, DocumentRequest::SetRead(id, read)
                            if id == document_id && read == is_read)
                        {
                            if let Some(document) = model
                                .documents
                                .iter_mut()
                                .find(|entry| entry.document_id == document_id)
                            {
                                document.is_read = is_read;
                            }
                        }
                    }
                    // Effective results must arrive through their captured request,
                    // never through this uncaptured callback envelope.
                    StorageResult::Effective(_)
                    | StorageResult::StatisticsLoaded(_)
                    | StorageResult::StatisticsTimeframeLoaded(_)
                    | StorageResult::StatisticsTimeframeSaved(_)
                    | StorageResult::VocabularyWordLoaded(_)
                    | StorageResult::DictionaryLoaded(_)
                    | StorageResult::VocabularyWordStatusChanged(_)
                    | StorageResult::VocabularyLoaded(_)
                    | StorageResult::AnkiSettingsLoaded(_)
                    | StorageResult::AnkiDefaultDeckSaved(_) => {
                        model.document_error = Some(DocumentError::InvalidDocument);
                    }
                    StorageResult::ImportCancelled => {}

                    StorageResult::Listed(documents, length, typography) => {
                        model.invalidate_library_words();
                        model.reader_typography = Some(typography);
                        model.documents = documents;
                        model.apply_page_length(length);
                    }
                    StorageResult::LibraryWordDistributionsLoaded(distributions) => {
                        if loading_library_words {
                            model.library_word_distributions = Some(distributions);
                        }
                    }
                    StorageResult::ReaderTypographySaved(typography) => {
                        model.reader_typography = Some(typography);
                    }
                    StorageResult::PageLengthSaved(length) => model.apply_page_length(length),
                    StorageResult::Created(document) | StorageResult::Existing(document) => {
                        let outcome = import_outcome.expect("import result");
                        if outcome == ImportOutcome::Created
                            || !model
                                .documents
                                .iter()
                                .any(|entry| entry.document_id == document.document_id)
                        {
                            model.invalidate_library_words();
                        }
                        model.import_summary = Some(ImportSummary {
                            document_id: document.document_id,
                            title: document.title.clone(),
                            outcome,
                        });
                        if let Some(entry) = model
                            .documents
                            .iter_mut()
                            .find(|entry| entry.document_id == document.document_id)
                        {
                            *entry = document;
                        } else {
                            model.documents.push(document);
                            model.documents.sort_by_key(|entry| entry.document_id);
                        }
                        model.destination = Destination::Library;
                        model.settings_open = false;
                        model.content_route = ContentRoute::Main;
                        model.clear_reader_presentation();
                    }
                    StorageResult::Loaded(snapshot) => {
                        if snapshot.validate().is_err() {
                            model.document_error = Some(DocumentError::InvalidDocument);
                            return render::render();
                        }
                        model.reader_typography = Some(snapshot.typography);
                        let document = snapshot.document;
                        model.update_library(&document);
                        if let Some(entry) = model
                            .documents
                            .iter_mut()
                            .find(|entry| entry.document_id == document.document_id)
                        {
                            entry.last_read_sequence = snapshot.last_read_sequence;
                        }
                        model.apply_page_length(snapshot.page_length);
                        if pending_reader == Some(document.document_id)
                            && model.content_route == ContentRoute::Reader
                        {
                            match ReaderPage::open_document(&document, snapshot.page_length) {
                                Ok(reader) => {
                                    model.reader_analysis = snapshot
                                        .analyses
                                        .iter()
                                        .find(|entry| entry.chapter_id == reader.anchor.chapter_id)
                                        .map(|entry| entry.analysis);
                                    model.reader_analyses = snapshot.analyses;
                                    model.reader = Some(reader);
                                    model.reader_document = Some(document.clone());
                                }
                                Err(_) => {
                                    model.document_error = Some(DocumentError::InvalidDocument)
                                }
                            }
                        }
                    }
                    StorageResult::PositionSaved(document) => model.apply_position(document),
                    StorageResult::Failed(error) => {
                        if let Some((id, _)) = document_rename {
                            if let Some(draft) = model
                                .library_rename
                                .as_mut()
                                .filter(|draft| draft.document_id == id)
                            {
                                draft.error = Some(error);
                            }
                        } else if let Some(id) = document_delete {
                            if model
                                .library_delete_target
                                .as_ref()
                                .is_some_and(|target| target.document_id == id)
                            {
                                model.library_delete_error = Some(error);
                            }
                        } else if loading_library_words {
                            model.library_word_error = Some(error);
                        } else if matches!(request, DocumentRequest::FileImport) {
                            model.file_import_error = Some(error);
                        } else {
                            model.document_error = Some(error);
                        }
                    }
                }
            }
            Event::RefreshLibrary
            | Event::SaveDocumentTitle(_)
            | Event::SetDocumentRead { .. }
            | Event::ConfirmDocumentDeletion(_)
            | Event::LoadLibraryWordDistributions
            | Event::CreatePaste(_)
            | Event::ImportTxt(_)
            | Event::ImportSrt(_)
            | Event::ImportEpub(_)
            | Event::OpenReader(_)
            | Event::LoadReaderTokens(_)
            | Event::SetReaderTypography(_)
            | Event::SetPageLength(_)
            | Event::SaveReadingPosition(_) => unreachable!(),
            Event::RetryReaderCompletion
            | Event::PreviousReaderPage { .. }
            | Event::SkipReaderPage { .. }
            | Event::CompleteEffectiveReaderPage { .. }
            | Event::LoadEffectiveReaderWord(_)
            | Event::SetEffectiveReaderStatus { .. }
            | Event::RetryEffectiveReaderStatus
            | Event::CancelEffectiveReaderStatus
            | Event::EffectiveStorageFinished(..) => unreachable!(),
            Event::LoadVocabulary
            | Event::SetVocabularyQuery(_)
            | Event::LoadMoreVocabulary
            | Event::RetryVocabulary
            | Event::VocabularyFinished(..)
            | Event::VocabularyWordFinished(..)
            | Event::ChooseDictionaryPart { .. }
            | Event::RetryDictionaryPart(_)
            | Event::DictionaryLookupFinished(..)
            | Event::OpenVocabularyWord(_)
            | Event::DismissVocabularyWord
            | Event::RetryVocabularyWord
            | Event::SetVocabularyWordStatus(_)
            | Event::RetryVocabularyWordStatus
            | Event::CancelVocabularyWordStatus
            | Event::LoadStatistics { .. }
            | Event::LoadStatisticsTimeframe
            | Event::StatisticsTimeframeFinished(..)
            | Event::SetStatisticsTimeframe { .. }
            | Event::RetryStatistics
            | Event::StatisticsFinished(..)
            | Event::CheckAnkiSetup(_)
            | Event::LoadReaderAnkiEntries(_)
            | Event::AddReaderAnkiEntry { .. }
            | Event::AnkiEntryChecked(..)
            | Event::AnkiSetupChecked(..)
            | Event::LoadAnkiSettings
            | Event::SetAnkiDefaultDeck(_)
            | Event::RetryAnkiSettings
            | Event::AnkiSettingsFinished(..)
            | Event::BeginAnkiLive
            | Event::DispatchAnkiLive
            | Event::PrepareReaderAnkiExport { .. }
            | Event::BeginPreparedAnkiExport { .. }
            | Event::ClearAnkiExport { .. }
            | Event::FinishAnkiLive(..)
            | Event::AbandonAnkiLive
            | Event::PrepareAnkiExport { .. }
            | Event::ClearPreparedAnkiExport
            | Event::BeginAnkiExportPreflight
            | Event::RetryAnkiExportPreflight
            | Event::AnkiExportPreflightFinished(..)
            | Event::AnkiExportPreflightProviderFinished(..) => unreachable!(),
        }
        render::render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            anki_added_notice: model.anki_added_notice,
            anki_settings: model.anki_settings.clone(),
            anki_setup: model.anki_setup.phase.clone(),
            anki_entries: model.anki_entries_view(),
            anki_export: model.anki_export_view(),
            vocabulary: model.vocabulary.clone(),
            statistics: {
                let mut state = model.statistics.clone();
                state.error = state.error.or(model.statistics_preference_error);
                state
            },
            destination: model.destination,
            settings_open: model.settings_open,
            documents: model.library_sort.project(&model.documents),
            library_sort: model.library_sort,
            library_word_distributions: model.library_word_distributions.clone(),
            library_word_error: model.library_word_error,
            library_delete_target: model.library_delete_target.clone(),
            library_delete_error: model.library_delete_error,
            library_rename: model.library_rename.clone(),
            storage_busy: model.storage_busy(),
            document_error: model.file_import_error.or(model.document_error),
            content_route: model.content_route,
            reader: model.reader.clone(),
            reader_analysis: model.reader_analysis,
            page_length: model.page_length,
            reader_typography: model.reader_typography,
            import_summary: model.import_summary.clone(),
            completion_save_pending: model.completion_pending(),
            completion_notice: model.completion_notice,
            book_completion_notice: model.book_completion_notice,
            effective_reader_tokens: model.effective_reader.page().cloned(),
            effective_reader_word: Some(model.effective_reader.word_state()),
            effective_reader_status: Some(model.effective_reader.status_state()),
        }
    }
}

impl Model {
    fn library_visible(&self) -> bool {
        self.destination == Destination::Library
            && self.content_route == ContentRoute::Main
            && !self.settings_open
    }

    fn clear_library_delete(&mut self) {
        self.library_delete_target = None;
        self.library_delete_error = None;
    }

    fn apply_document_deletion(&mut self, id: DocumentId) {
        self.documents.retain(|entry| entry.document_id != id);
        self.invalidate_library_words();
        self.clear_library_delete();
        if self
            .import_summary
            .as_ref()
            .is_some_and(|summary| summary.document_id == id)
        {
            self.import_summary = None;
        }
    }

    fn invalidate_library_words(&mut self) {
        self.library_word_distributions = None;
        self.library_word_error = None;
    }

    fn clear_reader_word(&mut self) {
        self.effective_reader.dismiss_word();
    }

    fn current_token_key(&self) -> Option<ReaderTokenKey> {
        if self.content_route != ContentRoute::Reader {
            return None;
        }
        let reader = self.reader.as_ref()?;
        Some(ReaderTokenKey {
            document_id: reader.document_id,
            page: reader.anchor,
            analysis: self.reader_analysis?,
        })
    }

    /// Retire visible reader data and stale shell selections. Accepted mutations
    /// and completion retries remain owned by the session and shared storage slot.
    fn clear_reader_presentation(&mut self) {
        self.reader = None;
        self.reader_analysis = None;
        self.invalidate_effective_page();
        self.clear_reader_word();
        self.reader_document = None;
        self.reader_analyses.clear();
    }

    fn close_document(&mut self) {
        self.library_rename = None;
        self.clear_library_delete();
        self.invalidate_library_words();
        self.completion_notice = None;
        self.book_completion_notice = None;
        self.content_route = ContentRoute::Main;
        self.clear_reader_presentation();

        self.document_error = None;
    }

    fn apply_position(&mut self, document: ReaderDocument) {
        self.update_library(&document);
        if self.content_route == ContentRoute::Reader
            && self
                .reader
                .as_ref()
                .is_some_and(|reader| reader.document_id == document.document_id)
        {
            match ReaderPage::open_document(&document, self.page_length.unwrap_or_default()) {
                Ok(reader) => {
                    self.invalidate_effective_page();
                    self.clear_reader_word();
                    self.reader_analysis = self
                        .reader_analyses
                        .iter()
                        .find(|entry| entry.chapter_id == reader.anchor.chapter_id)
                        .map(|entry| entry.analysis);
                    self.reader = Some(reader);
                    self.reader_document = Some(document.clone());
                }
                Err(_) => self.document_error = Some(DocumentError::InvalidDocument),
            }
        }
    }

    fn update_library(&mut self, document: &ReaderDocument) {
        match document.library_summary() {
            Ok(summary) => {
                if let Some(entry) = self
                    .documents
                    .iter_mut()
                    .find(|entry| entry.document_id == document.document_id)
                {
                    let last_read_sequence = entry.last_read_sequence;
                    *entry = summary;
                    entry.last_read_sequence = last_read_sequence;
                }
            }
            Err(_) => self.document_error = Some(DocumentError::InvalidDocument),
        }
    }

    fn apply_page_length(&mut self, length: PageLength) {
        self.page_length = Some(length);
        if let (Some(reader), Some(document)) = (&self.reader, &self.reader_document) {
            match ReaderPage::at_document(document, reader.position, length) {
                Ok(page) => {
                    if page.anchor != reader.anchor {
                        self.invalidate_effective_page();
                        self.clear_reader_word();
                    }
                    self.reader = Some(page);
                }
                Err(_) => self.document_error = Some(DocumentError::InvalidDocument),
            }
        }
    }
}
