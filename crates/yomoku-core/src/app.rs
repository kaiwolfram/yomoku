//! Pure application state. Hosts execute effects; the core performs no I/O.

use crate::typography::ReaderTypography;
use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

mod effective;

use crate::{
    active_time::{
        ActiveReadingTime, ReaderClockAction, ReaderClockSample, ReadingTimeUpdate,
        SavedReadingTime,
    },
    completion::{
        AnalyzedCompletionOutcome, CompletionNotice, PageCompletion, PageCompletionIntent,
    },
    document::{CompletionCommandId, DocumentId, LibraryDocument, ReaderDocument, TextAnchor},
    effective_session::{
        EffectiveOverrideAction, EffectivePageIdentity, EffectiveReaderOverrideState,
        EffectiveReaderPage, EffectiveReaderRequest, EffectiveReaderSelection,
        EffectiveReaderSession, EffectiveReaderWordState, EffectiveStatusState,
    },
    imports::{EpubImport, SrtImport, TxtImport},
    library::{valid_document_title, LibraryRenameState, LibrarySort, LibraryWordDistribution},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::{
        ReaderAnalysis, ReaderChapterAnalysis, ReaderPage, ReaderStatusChange, ReaderTokenKey,
        ReaderTokenSnapshot,
    },
    reader::{ReaderWordDetails, ReaderWordSelection},
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
    library_title_query: String,
    library_sort: LibrarySort,
    library_word_distributions: Option<Vec<LibraryWordDistribution>>,
    library_word_error: Option<DocumentError>,
    loading_library_words: bool,
    library_delete_target: Option<LibraryDocument>,
    library_delete_error: Option<DocumentError>,
    pending_document_delete: Option<DocumentId>,
    library_selection: Option<Vec<DocumentId>>,
    library_bulk_delete_targets: Option<Vec<LibraryDocument>>,
    pending_documents_delete: Option<Vec<DocumentId>>,
    library_rename: Option<LibraryRenameState>,
    pending_document_rename: Option<(DocumentId, String)>,
    loaded_document: Option<ReaderDocument>,
    storage_busy: bool,
    document_error: Option<DocumentError>,
    content_route: ContentRoute,
    reader: Option<ReaderPage>,
    reader_analysis: Option<ReaderAnalysis>,
    pending_reader: Option<DocumentId>,
    reader_document: Option<ReaderDocument>,
    reader_analyses: Vec<ReaderChapterAnalysis>,
    page_length: Option<PageLength>,
    reader_typography: Option<ReaderTypography>,
    import_summary: Option<ImportSummary>,
    reader_time: Option<SavedReadingTime>,
    restoring_reader_time: bool,
    pending_time_checkpoint: Option<StorageOperation>,
    queued_time_checkpoint: Option<ReadingTimeUpdate>,
    deferred_reader_event: Option<Event>,
    pending_completion: Option<StorageOperation>,
    completion_notice: Option<CompletionNotice>,
    book_completion_notice: Option<CompletionCommandId>,
    reader_clock: Option<ActiveReadingTime>,
    time_checkpoint_can_wait: bool,
    reader_tokens: Option<ReaderTokenSnapshot>,
    pending_token_load: Option<ReaderTokenKey>,
    pending_status_change: Option<ReaderStatusChange>,
    pending_word_load: Option<ReaderWordSelection>,
    pending_custom_change: Option<ReaderWordSelection>,
    reader_custom_entry_error: Option<DocumentError>,
    reader_word_selection: Option<ReaderWordSelection>,
    reader_word: Option<ReaderWordDetails>,
    reader_word_error: Option<DocumentError>,
    effective_reader: Option<EffectiveReaderSession>,
    effective_request: Option<EffectiveReaderRequest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum Event {
    SelectDestination(Destination),
    OpenSettings,
    CloseSettings,
    RefreshLibrary,
    /// Read-only lazy refresh when Library is visible; retries are explicit.
    LoadLibraryWordDistributions,
    /// Literal, Unicode-lowercase substring search over Library titles only.
    SearchLibraryTitles(String),
    SortLibrary(LibrarySort),
    /// Freeze one visible Library entry for explicit confirmation.
    RequestDocumentDeletion(DocumentId),
    CancelDocumentDeletion,
    ConfirmDocumentDeletion(DocumentId),
    /// Long press starts selection; subsequent row taps toggle visible IDs.
    ToggleLibraryDocumentSelection(DocumentId),
    /// Add all matching titles, including rows outside the rendered viewport.
    SelectAllLibraryDocuments,
    CancelLibrarySelection,
    RequestSelectedDocumentDeletion,
    /// Confirm only the exact frozen IDs shown by the retained prompt.
    ConfirmSelectedDocumentDeletion(Vec<DocumentId>),
    RequestDocumentRename(DocumentId),
    EditDocumentTitle {
        document_id: DocumentId,
        title: String,
    },
    CancelDocumentRename,
    SaveDocumentTitle(DocumentId),
    CreatePaste(String),
    LoadPaste(DocumentId),
    OpenPaste,
    OpenReader(DocumentId),
    CloseDocument,
    /// Position-only persistence hook; waits for accepted time checkpoints.
    /// Page completion uses a separate command.
    SaveReadingPosition(ReadingPosition),
    SetPageLength(PageLength),
    SetReaderTypography(ReaderTypography),
    DismissImportSummary,
    /// Explicit preparation for the platform timer; opening itself stays read-only.
    RestoreReadingTime,
    /// Coalesce newer absolute durations while a checkpoint is unresolved.
    CheckpointReadingTime(ReadingTimeUpdate),
    /// Retry the frozen absolute checkpoint, including its original revision.
    RetryReadingTimeCheckpoint,
    /// Anchors reject late duplicate callbacks after moving to another page.
    /// Caller pauses accounting and submits its final checkpoint first.
    CompleteReaderPage {
        document_id: DocumentId,
        page: TextAnchor,
        position: ReadingPosition,
        completed_at_unix_ms: i64,
    },
    /// Retry preparation or the exact frozen analyzed command after a failure.
    RetryReaderCompletion,
    DismissCompletionNotice,
    /// Restore first, then supply Android's monotonic clock. Pause before any
    /// departure/completion; Checkpoint and Pause save absolute durations.
    ObserveReaderClock(ReaderClockSample),
    /// Position-only backward navigation anchored to the visible page.
    PreviousReaderPage {
        document_id: DocumentId,
        page: TextAnchor,
        position: ReadingPosition,
    },
    /// Consume before starting the transient celebration; stale acknowledgments ignore.
    DismissBookCompletionNotice(CompletionCommandId),
    /// Explicit refresh; stale UI callbacks cannot request another page's tokens.
    LoadReaderTokens(ReaderTokenKey),
    /// Explicit manual choice; only existing vocabulary on the loaded page.
    SetReaderStatus(ReaderStatusChange),
    /// Explicit selection/refresh; rejected until current page tokens are loaded.
    LoadReaderWord(ReaderWordSelection),
    DismissReaderWord,
    /// Explicit create/edit/delete for the currently loaded missing-match selection.
    /// Failures retain the committed entry and require another explicit submission.
    SaveReaderCustomEntry {
        selection: ReaderWordSelection,
        english_meaning: String,
        reading: String,
    },
    DeleteReaderCustomEntry(ReaderWordSelection),
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
    SaveEffectiveReaderCustomEntry {
        selection: EffectiveReaderSelection,
        english_meaning: String,
        reading: String,
    },
    DeleteEffectiveReaderCustomEntry(EffectiveReaderSelection),
    /// Retry frozen literal values, or explicitly release a failed write.
    RetryEffectiveReaderCustomEntry,
    CancelEffectiveReaderCustomEntry,
    SetEffectiveReaderStatus {
        selection: EffectiveReaderSelection,
        vocabulary_id: crate::document::VocabularyId,
        status: crate::vocabulary::VocabularyStatus,
        occurred_at_unix_ms: i64,
    },
    RetryEffectiveReaderStatus,
    CancelEffectiveReaderStatus,
    EffectiveReaderOverride(EffectiveOverrideAction),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    EffectiveStorageFinished(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Box<EffectiveReaderRequest>,
        StorageResult,
    ),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StorageFinished(StorageResult),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ViewModel {
    pub destination: Destination,
    pub settings_open: bool,
    /// Matching titles in the selected Library order; an empty query shows all.
    pub documents: Vec<LibraryDocument>,
    /// Session-local query; the model retains the complete unfiltered Library.
    pub library_title_query: String,
    /// Session-local selection; read recency itself is durable.
    pub library_sort: LibrarySort,
    /// Unfiltered count distinguishes an empty Library from an empty search.
    pub library_document_count: u64,
    pub library_word_distributions: Option<Vec<LibraryWordDistribution>>,
    pub library_word_error: Option<DocumentError>,
    pub library_delete_target: Option<LibraryDocument>,
    pub library_delete_error: Option<DocumentError>,
    /// None outside selection mode; Some(empty) keeps selection controls visible.
    pub library_selection: Option<Vec<DocumentId>>,
    pub library_bulk_delete_targets: Option<Vec<LibraryDocument>>,
    pub library_rename: Option<LibraryRenameState>,
    pub loaded_document: Option<ReaderDocument>,
    pub storage_busy: bool,
    pub document_error: Option<DocumentError>,
    pub content_route: ContentRoute,
    pub reader: Option<ReaderPage>,
    /// Present only for an opened reader; Missing identifies a legacy document.
    pub reader_analysis: Option<ReaderAnalysis>,
    /// None until persisted settings have been loaded successfully.
    pub page_length: Option<PageLength>,
    pub reader_typography: Option<ReaderTypography>,
    pub import_summary: Option<ImportSummary>,
    /// Only the committed duration at the current exact reading offset.
    pub reader_time: Option<ReadingTimeUpdate>,
    /// Remains true after a failed checkpoint until its exact retry succeeds.
    pub reading_time_save_pending: bool,
    pub completion_save_pending: bool,
    pub completion_notice: Option<CompletionNotice>,
    /// One newly recorded final-page command, independent of permanent finish state.
    pub book_completion_notice: Option<CompletionCommandId>,
    /// None until explicitly loaded for the current document/page/base analysis.
    pub reader_tokens: Option<ReaderTokenSnapshot>,
    /// Native opt-in integration path; production activation waits for all consumers.
    pub effective_reader_tokens: Option<EffectiveReaderPage>,
    pub effective_reader_word: Option<EffectiveReaderWordState>,
    pub effective_reader_status: Option<EffectiveStatusState>,
    pub effective_reader_override: Option<EffectiveReaderOverrideState>,
    /// Enabled source-preferred readings, in page-local UTF-8 ranges. No I/O.
    pub reader_furigana: Vec<crate::reader_ruby::ReaderRuby>,
    pub reader_word_selection: Option<ReaderWordSelection>,
    pub reader_word: Option<ReaderWordDetails>,
    /// Separate from save errors so timer checkpoints cannot erase a failed lookup.
    pub reader_word_error: Option<DocumentError>,
    /// A failed explicit custom save/delete; preserves the last committed entry.
    pub reader_custom_entry_error: Option<DocumentError>,
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
        if let Event::EffectiveStorageFinished(request, result) = event {
            return self.finish_effective(&request, result, model);
        }
        if let Some(command) = self.update_effective(&event, model) {
            return command;
        }
        if let Event::PreviousReaderPage {
            document_id,
            page,
            position,
        } = event
        {
            if model.content_route != ContentRoute::Reader
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
            let Some(previous) = pages
                .page_at(position)
                .ok()
                .and_then(|index| index.checked_sub(1))
                .map(|index| pages.pages()[index])
            else {
                return Command::done();
            };
            return self.update(
                Event::SaveReadingPosition(ReadingPosition {
                    source_id: previous.source_id,
                    chapter_id: previous.chapter_id,
                    offset: previous.range.start,
                }),
                model,
            );
        }
        if let Event::ObserveReaderClock(sample) = event {
            let Some(update) = model.observe_reader_clock(sample) else {
                return Command::done();
            };
            return self.update(Event::CheckpointReadingTime(update), model);
        }
        if (model.pending_time_checkpoint.is_some()
            || model.queued_time_checkpoint.is_some()
            || model.completion_pending()
            || model.effective_mutation_pending())
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
        if let Some(StorageOperation::SaveReadingTime { checkpoint, .. }) =
            model.pending_time_checkpoint.as_ref()
        {
            if let Event::CheckpointReadingTime(update) = &event {
                // Coalesce without replacing the frozen in-flight retry payload.
                let newest = model.queued_time_checkpoint.as_ref().unwrap_or(checkpoint);
                if !model.is_current_time(*update)
                    || update.pending.milliseconds() <= newest.pending.milliseconds()
                {
                    return Command::done();
                }
                model.queued_time_checkpoint = Some(*update);
                return render::render();
            }
        }
        // An unrelated operation may already own the FIFO (e.g. settings reflow).
        // Keep the greatest absolute checkpoint until its callback, rather than
        // dropping a platform final save under the normal busy-event policy.
        if model.storage_busy {
            if let Event::CheckpointReadingTime(update) = event {
                if !model.time_checkpoint_can_wait
                    || model.completion_pending()
                    || !model.is_current_time(update)
                {
                    return Command::done();
                }
                let Some(saved) = model.reader_time else {
                    return Command::done();
                };
                let newest = model.queued_time_checkpoint.unwrap_or(saved.value);
                if update.pending.milliseconds() <= newest.pending.milliseconds() {
                    return Command::done();
                }
                model.queued_time_checkpoint = Some(update);
                return render::render();
            }
        }
        let operation = match &event {
            Event::CompleteReaderPage {
                document_id,
                page,
                position,
                completed_at_unix_ms,
            } => {
                if model.completion_pending()
                    || model.deferred_reader_event.is_some()
                    || (model.storage_busy
                        && model.pending_time_checkpoint.is_none()
                        && model.queued_time_checkpoint.is_none())
                    || model.content_route != ContentRoute::Reader
                    || model.settings_open
                    || *completed_at_unix_ms < 0
                    || !model.reader.as_ref().is_some_and(|reader| {
                        reader.position.offset != reader.anchor.range.end
                            && reader.document_id == *document_id
                            && reader.anchor == *page
                            && reader.position == *position
                    })
                {
                    return Command::done();
                }
                let Some(ReaderAnalysis::Available(analysis_id)) = model.reader_analysis else {
                    model.document_error = Some(DocumentError::MissingAnalysis);
                    return render::render();
                };
                let operation = StorageOperation::PreparePageCompletion(PageCompletionIntent {
                    document_id: *document_id,
                    page: *page,
                    page_length: model.page_length.expect("opened reader settings"),
                    departing_position: *position,
                    completed_at_unix_ms: *completed_at_unix_ms,
                    analysis_id,
                });
                model.pending_completion = Some(operation.clone());
                model.completion_notice = None;
                model.book_completion_notice = None;
                if model.pending_time_checkpoint.is_some() || model.queued_time_checkpoint.is_some()
                {
                    return render::render();
                }
                Some(operation)
            }
            Event::RetryReaderCompletion => {
                if model.pending_time_checkpoint.is_some() || model.queued_time_checkpoint.is_some()
                {
                    return Command::done();
                }
                let Some(operation) = model.pending_completion.clone() else {
                    return Command::done();
                };
                Some(operation)
            }
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
                    || model.queued_time_checkpoint.is_some_and(|queued| {
                        checkpoint.pending.milliseconds() < queued.pending.milliseconds()
                    })
                {
                    return Command::done();
                }
                Some(StorageOperation::SaveReadingTime {
                    previous,
                    checkpoint: *checkpoint,
                })
            }
            Event::RetryReadingTimeCheckpoint => {
                if let Some(operation) = model.pending_time_checkpoint.clone() {
                    Some(operation)
                } else if let Some(checkpoint) = model.queued_time_checkpoint {
                    return self.update(Event::CheckpointReadingTime(checkpoint), model);
                } else {
                    return Command::done();
                }
            }
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
            Event::ConfirmSelectedDocumentDeletion(ids) => {
                if !model.library_visible()
                    || ids.is_empty()
                    || !model
                        .library_bulk_delete_targets
                        .as_ref()
                        .is_some_and(|targets| {
                            targets
                                .iter()
                                .map(|target| target.document_id)
                                .eq(ids.iter().copied())
                        })
                {
                    return Command::done();
                }
                Some(StorageOperation::DeleteDocuments(ids.clone()))
            }
            Event::LoadLibraryWordDistributions => {
                if model.destination != Destination::Library
                    || model.content_route != ContentRoute::Main
                    || model.settings_open
                {
                    return Command::done();
                }
                Some(if model.effective_reader.is_some() {
                    StorageOperation::LoadEffectiveLibraryWordDistributions
                } else {
                    StorageOperation::LoadLibraryWordDistributions
                })
            }
            Event::CreatePaste(text) => Some(StorageOperation::CreatePaste(text.clone())),
            Event::ImportTxt(input) => Some(StorageOperation::ImportTxt(input.clone())),
            Event::ImportSrt(input) => Some(StorageOperation::ImportSrt(input.clone())),
            Event::ImportEpub(input) => Some(StorageOperation::ImportEpub(input.clone())),
            Event::LoadPaste(id) => Some(StorageOperation::LoadPaste(*id)),
            Event::OpenReader(id) => Some(StorageOperation::OpenReader(*id)),
            Event::LoadReaderTokens(key) => {
                if model.settings_open || model.current_token_key() != Some(*key) {
                    return Command::done();
                }
                Some(StorageOperation::LoadReaderTokens(*key))
            }
            Event::LoadReaderWord(selection) => {
                if model.settings_open
                    || model.current_token_key() != Some(selection.key)
                    || !model.reader_tokens.as_ref().is_some_and(|snapshot| {
                        snapshot.key() == selection.key
                            && snapshot
                                .tokens
                                .iter()
                                .any(|token| token.token_index == selection.token_index)
                    })
                {
                    return Command::done();
                }
                Some(StorageOperation::LoadReaderWord(*selection))
            }
            Event::SaveReaderCustomEntry {
                selection,
                english_meaning,
                reading,
            } => {
                if !model.can_edit_custom_entry(*selection) {
                    return Command::done();
                }
                Some(StorageOperation::SaveReaderCustomEntry {
                    selection: *selection,
                    english_meaning: english_meaning.clone(),
                    reading: reading.clone(),
                })
            }
            Event::DeleteReaderCustomEntry(selection) => {
                if !model.can_edit_custom_entry(*selection) {
                    return Command::done();
                }
                Some(StorageOperation::DeleteReaderCustomEntry(*selection))
            }
            Event::SetReaderStatus(change) => {
                if model.settings_open
                    || change.occurred_at_unix_ms < 0
                    || model.current_token_key() != Some(change.key)
                    || !model.reader_tokens.as_ref().is_some_and(|snapshot| {
                        snapshot.key() == change.key
                            && snapshot.tokens.iter().any(|token| {
                                token
                                    .vocabulary
                                    .is_some_and(|entry| entry.id == change.vocabulary_id)
                            })
                    })
                {
                    return Command::done();
                }
                Some(StorageOperation::SetReaderStatus(*change))
            }
            Event::SetReaderTypography(typography) => {
                if !model.settings_open
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
            let draining_completion_time = model.completion_pending()
                && matches!(event, Event::CheckpointReadingTime(checkpoint)
                if model.queued_time_checkpoint == Some(checkpoint));
            // One in-flight command per session prevents duplicate submissions and
            // stale load/list callbacks from replacing a newer snapshot.
            if model.storage_busy
                || (model.effective_mutation_pending()
                    && !matches!(
                        event,
                        Event::CheckpointReadingTime(_) | Event::RetryReadingTimeCheckpoint
                    ))
                || (model.queued_time_checkpoint.is_some()
                    && !matches!(
                        event,
                        Event::CheckpointReadingTime(_) | Event::RetryReadingTimeCheckpoint
                    ))
                || (model.pending_time_checkpoint.is_some()
                    && !matches!(event, Event::RetryReadingTimeCheckpoint))
                || (model.completion_pending()
                    && !matches!(
                        event,
                        Event::CompleteReaderPage { .. }
                            | Event::RetryReaderCompletion
                            | Event::RetryReadingTimeCheckpoint
                    )
                    && !draining_completion_time)
            {
                return Command::done();
            }
            if let StorageOperation::SaveReadingTime { checkpoint, .. } = &operation {
                model.pending_time_checkpoint = Some(operation.clone());
                if model.queued_time_checkpoint.is_some_and(|queued| {
                    queued.pending.milliseconds() <= checkpoint.pending.milliseconds()
                }) {
                    model.queued_time_checkpoint = None;
                }
            }
            model.restoring_reader_time =
                matches!(operation, StorageOperation::LoadReadingTime { .. });
            model.time_checkpoint_can_wait = match &operation {
                StorageOperation::List
                | StorageOperation::LoadLibraryWordDistributions
                | StorageOperation::LoadEffectiveLibraryWordDistributions
                | StorageOperation::LoadPaste(_)
                | StorageOperation::SavePageLength(_)
                | StorageOperation::SaveReaderTypography(_)
                | StorageOperation::LoadReadingTime { .. }
                | StorageOperation::LoadReaderWord(_)
                | StorageOperation::SaveReaderCustomEntry { .. }
                | StorageOperation::DeleteReaderCustomEntry(_)
                | StorageOperation::LoadReaderTokens(_)
                | StorageOperation::SetReaderStatus(_)
                | StorageOperation::SaveReadingTime { .. } => true,
                StorageOperation::SaveReadingPosition { position, .. } => model
                    .reader
                    .as_ref()
                    .is_some_and(|reader| reader.position == *position),
                _ => false,
            };
            model.storage_busy = true;
            if let StorageOperation::RenameDocument { document_id, title } = &operation {
                model.pending_document_rename = Some((*document_id, title.clone()));
                if let Some(draft) = model.library_rename.as_mut() {
                    draft.error = None;
                }
            } else if !operation.loads_library_words() {
                model.library_rename = None;
            }
            match &operation {
                StorageOperation::DeleteDocument(id) => {
                    model.pending_document_delete = Some(*id);
                    model.library_delete_error = None;
                }
                StorageOperation::DeleteDocuments(ids) => {
                    model.pending_documents_delete = Some(ids.clone());
                    model.library_delete_error = None;
                }
                StorageOperation::LoadLibraryWordDistributions
                | StorageOperation::LoadEffectiveLibraryWordDistributions => {}
                _ => {
                    model.clear_library_delete();
                    model.library_selection = None;
                }
            }
            if operation.loads_library_words() {
                model.loading_library_words = true;
                model.library_word_distributions = None;
                model.library_word_error = None;
            }
            if !matches!(
                operation,
                StorageOperation::LoadLibraryWordDistributions
                    | StorageOperation::LoadEffectiveLibraryWordDistributions
                    | StorageOperation::DeleteDocument(_)
                    | StorageOperation::DeleteDocuments(_)
                    | StorageOperation::RenameDocument { .. }
            ) {
                model.document_error = None;
            }
            if let StorageOperation::LoadReaderWord(selection) = operation {
                model.reader_word_selection = Some(selection);
                model.pending_word_load = Some(selection);
                model.reader_word = None;
                model.reader_word_error = None;
                model.reader_custom_entry_error = None;
            }
            match &operation {
                StorageOperation::SaveReaderCustomEntry { selection, .. }
                | StorageOperation::DeleteReaderCustomEntry(selection) => {
                    model.pending_custom_change = Some(*selection);
                    model.reader_custom_entry_error = None;
                }
                _ => {}
            }
            // Every accepted operation that invalidates page tokens also retires
            // its selected-word result, even if the resulting anchor is unchanged.
            if matches!(
                operation,
                StorageOperation::LoadReaderTokens(_)
                    | StorageOperation::List
                    | StorageOperation::LoadPaste(_)
                    | StorageOperation::OpenReader(_)
                    | StorageOperation::CreatePaste(_)
                    | StorageOperation::ImportTxt(_)
                    | StorageOperation::ImportSrt(_)
                    | StorageOperation::ImportEpub(_)
                    | StorageOperation::SavePageLength(_)
                    | StorageOperation::SaveReadingPosition { .. }
                    | StorageOperation::PreparePageCompletion(_)
            ) {
                model.invalidate_effective_page();
                model.clear_reader_word();
            }
            if let StorageOperation::LoadReaderTokens(key) = operation {
                model.pending_token_load = Some(key);
                model.invalidate_effective_page();
                model.reader_tokens = None;
                model.clear_reader_word();
            }
            if let StorageOperation::SetReaderStatus(change) = operation {
                model.pending_status_change = Some(change);
            }
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
            if matches!(
                event,
                Event::CreatePaste(_)
                    | Event::ImportTxt(_)
                    | Event::ImportSrt(_)
                    | Event::ImportEpub(_)
                    | Event::LoadPaste(_)
                    | Event::OpenReader(_)
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
                model.invalidate_effective_page();
                model.reader_tokens = None;
                model.clear_reader_word();
                model.reader_document = None;
                model.reader_analyses.clear();
                model.clear_reader_time();
            }
            return render::render()
                .and(Command::request_from_shell(operation).then_send(Event::StorageFinished));
        }
        match event {
            Event::ToggleLibraryDocumentSelection(id) => {
                if !model.can_select_library_documents()
                    || !model
                        .library_sort
                        .project(&model.documents, &model.library_title_query)
                        .iter()
                        .any(|entry| entry.document_id == id)
                {
                    return Command::done();
                }
                let selection = model.library_selection.get_or_insert_with(Vec::new);
                match selection.binary_search(&id) {
                    Ok(index) => {
                        selection.remove(index);
                    }
                    Err(index) => selection.insert(index, id),
                }
            }
            Event::SelectAllLibraryDocuments => {
                if !model.can_select_library_documents() || model.library_selection.is_none() {
                    return Command::done();
                }
                let matching = model
                    .library_sort
                    .project(&model.documents, &model.library_title_query);
                let selection = model.library_selection.as_mut().expect("selection checked");
                selection.extend(matching.iter().map(|entry| entry.document_id));
                selection.sort_unstable();
                selection.dedup();
            }
            Event::CancelLibrarySelection => {
                if model.library_selection.is_none() || model.pending_documents_delete.is_some() {
                    return Command::done();
                }
                model.library_selection = None;
                model.clear_library_delete();
            }
            Event::RequestSelectedDocumentDeletion => {
                if !model.can_select_library_documents() {
                    return Command::done();
                }
                let Some(selection) = model
                    .library_selection
                    .as_ref()
                    .filter(|ids| !ids.is_empty())
                else {
                    return Command::done();
                };
                let targets: Vec<_> = model
                    .documents
                    .iter()
                    .filter(|entry| selection.binary_search(&entry.document_id).is_ok())
                    .cloned()
                    .collect();
                if targets.len() != selection.len() {
                    return Command::done();
                }
                model.library_bulk_delete_targets = Some(targets);
                model.library_delete_error = None;
            }
            Event::RequestDocumentRename(id) => {
                if !model.library_visible()
                    || model.library_selection.is_some()
                    || model.storage_busy
                    || model.pending_time_checkpoint.is_some()
                    || model.queued_time_checkpoint.is_some()
                    || model.completion_pending()
                {
                    return Command::done();
                }
                let Some(target) = model
                    .library_sort
                    .project(&model.documents, &model.library_title_query)
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
                if model.storage_busy || !model.library_visible() {
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
                if model.pending_document_rename.is_some() {
                    return Command::done();
                }
                model.library_rename = None;
            }
            Event::RequestDocumentDeletion(id) => {
                if !model.library_visible()
                    || model.library_selection.is_some()
                    || model.storage_busy
                    || model.pending_time_checkpoint.is_some()
                    || model.queued_time_checkpoint.is_some()
                    || model.completion_pending()
                {
                    return Command::done();
                }
                let Some(target) = model
                    .library_sort
                    .project(&model.documents, &model.library_title_query)
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
                if model.pending_document_delete.is_some()
                    || model.pending_documents_delete.is_some()
                {
                    return Command::done();
                }
                model.clear_library_delete();
            }
            Event::SearchLibraryTitles(query) => {
                if model.destination != Destination::Library
                    || model.content_route != ContentRoute::Main
                    || model.settings_open
                {
                    return Command::done();
                }
                model.library_title_query = query;
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
                model.library_selection = None;
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
                model.reader = None;
                model.reader_analysis = None;
                model.invalidate_effective_page();
                model.reader_tokens = None;
                model.clear_reader_word();
                model.reader_document = None;
                model.reader_analyses.clear();
                model.pending_reader = None;
                model.clear_reader_time();
            }
            Event::OpenPaste => {
                model.library_selection = None;
                model.library_rename = None;
                model.clear_library_delete();
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.import_summary = None;
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Paste;
                model.reader = None;
                model.reader_analysis = None;
                model.invalidate_effective_page();
                model.reader_tokens = None;
                model.clear_reader_word();
                model.reader_document = None;
                model.reader_analyses.clear();
                model.pending_reader = None;
                model.clear_reader_time();
                model.document_error = None;
            }
            Event::CloseDocument => model.close_document(),
            Event::OpenSettings => {
                model.library_selection = None;
                model.library_rename = None;
                model.clear_library_delete();
                model.settings_open = true;
                model.clear_reader_word();
            }
            Event::DismissReaderWord => model.clear_reader_word(),
            Event::CloseSettings => model.settings_open = false,
            Event::DismissImportSummary => model.import_summary = None,
            Event::DismissCompletionNotice => model.completion_notice = None,
            Event::DismissBookCompletionNotice(id) => {
                if model.book_completion_notice == Some(id) {
                    model.book_completion_notice = None;
                }
            }
            Event::StorageFinished(result) => {
                if model.effective_request.is_some() {
                    return Command::done();
                }
                let failed = matches!(result, StorageResult::Failed(_));
                model.storage_busy = false;
                model.time_checkpoint_can_wait = false;
                let restoring_time = std::mem::take(&mut model.restoring_reader_time);
                let loading_library_words = std::mem::take(&mut model.loading_library_words);
                let pending_reader = model.pending_reader.take();
                let token_load = model.pending_token_load.take();
                let status_change = model.pending_status_change.take();
                let word_load = model.pending_word_load.take();
                let custom_change = model.pending_custom_change.take();
                let document_delete = model.pending_document_delete.take();
                let documents_delete = model.pending_documents_delete.take();
                let document_rename = model.pending_document_rename.take();
                let import_outcome = match &result {
                    StorageResult::Created(_) => Some(ImportOutcome::Created),
                    StorageResult::Existing(_) => Some(ImportOutcome::Existing),
                    _ => None,
                };
                match result {
                    StorageResult::DocumentRenamed { document_id, title } => {
                        if document_rename
                            .as_ref()
                            .is_some_and(|(id, expected)| *id == document_id && *expected == title)
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
                            if model
                                .loaded_document
                                .as_ref()
                                .is_some_and(|document| document.document_id == document_id)
                            {
                                model.loaded_document = None;
                            }
                            model.library_rename = None;
                        }
                    }
                    StorageResult::DocumentDeleted(id) => {
                        if document_delete == Some(id) {
                            model.apply_document_deletion(&[id]);
                        }
                    }
                    StorageResult::DocumentsDeleted(ids) => {
                        if documents_delete.as_ref() == Some(&ids) {
                            model.apply_document_deletion(&ids);
                        }
                    }
                    // Effective results must arrive through their captured request,
                    // never through this uncaptured legacy callback envelope.
                    StorageResult::Effective(_) => {
                        model.document_error = Some(DocumentError::InvalidDocument);
                    }
                    StorageResult::ImportCancelled => {}
                    StorageResult::ReaderWordLoaded(details) => {
                        model.invalidate_library_words();
                        if word_load == Some(details.selection())
                            && model.reader_word_selection == word_load
                            && model.current_token_key() == Some(details.key)
                            && model.reader_tokens.as_ref().is_some_and(|snapshot| {
                                snapshot.key() == details.key
                                    && snapshot.tokens.iter().any(|token| {
                                        let mut expected = token.clone();
                                        expected.vocabulary = details.token.vocabulary;
                                        expected == details.token
                                    })
                            })
                        {
                            // Status can have changed in another session since page load.
                            if let Some(snapshot) = model.reader_tokens.as_mut() {
                                for token in &mut snapshot.tokens {
                                    if token.surface == details.token.surface {
                                        token.vocabulary = details.token.vocabulary;
                                    }
                                }
                            }
                            model.reader_word = Some(*details);
                        }
                    }
                    StorageResult::ReaderCustomEntryChanged(selection, entry) => {
                        if custom_change == Some(selection)
                            && model.can_edit_custom_entry(selection)
                            && entry.as_ref().is_none_or(|entry| {
                                model.reader_word.as_ref().is_some_and(|details| {
                                    entry.surface == details.token.surface
                                        && !entry.english_meaning.trim().is_empty()
                                        && !entry.reading.trim().is_empty()
                                })
                            })
                        {
                            model
                                .reader_word
                                .as_mut()
                                .expect("validated selection")
                                .custom_entry = entry;
                        }
                    }
                    StorageResult::ReaderStatusChanged(change, entry) => {
                        model.invalidate_library_words();
                        if status_change == Some(change)
                            && model.current_token_key() == Some(change.key)
                            && entry.is_none_or(|entry| {
                                entry.id == change.vocabulary_id && entry.status == change.status
                            })
                        {
                            if let Some(details) = model.reader_word.as_mut().filter(|details| {
                                details.key == change.key
                                    && details
                                        .token
                                        .vocabulary
                                        .is_some_and(|value| value.id == change.vocabulary_id)
                            }) {
                                details.token.vocabulary = entry;
                                if entry.is_none() {
                                    details.encountered = 0;
                                }
                            }
                            if let Some(snapshot) = model
                                .reader_tokens
                                .as_mut()
                                .filter(|snapshot| snapshot.key() == change.key)
                            {
                                for token in &mut snapshot.tokens {
                                    if token
                                        .vocabulary
                                        .is_some_and(|entry| entry.id == change.vocabulary_id)
                                    {
                                        token.vocabulary = entry;
                                    }
                                }
                            }
                        }
                    }
                    StorageResult::ReaderTokensLoaded(snapshot) => {
                        if token_load == Some(snapshot.key())
                            && model.current_token_key() == Some(snapshot.key())
                        {
                            model.reader_tokens = Some(snapshot);
                        }
                    }
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
                            // Completion is accepted but has not frozen its total:
                            // drain this slot before preparing the command.
                            let operation = StorageOperation::SaveReadingTime {
                                previous: saved,
                                checkpoint,
                            };
                            model.pending_time_checkpoint = Some(operation.clone());
                            model.storage_busy = true;
                            return render::render().and(
                                Command::request_from_shell(operation)
                                    .then_send(Event::StorageFinished),
                            );
                        }
                        if model.completion_pending() {
                            return self.update(Event::RetryReaderCompletion, model);
                        }
                        if !model.effective_mutation_pending() {
                            if let Some(event) = model.deferred_reader_event.take() {
                                return self.update(event, model);
                            }
                        }
                    }
                    StorageResult::PageCompletionPrepared(prepared) => {
                        model.pending_completion = Some(StorageOperation::CompletePage(prepared));
                        return self.update(Event::RetryReaderCompletion, model);
                    }
                    StorageResult::PageCompleted(document, outcome) => {
                        model.invalidate_library_words();
                        let Some(StorageOperation::CompletePage(prepared)) =
                            model.pending_completion.take()
                        else {
                            return render::render();
                        };
                        model.apply_completion(prepared.completion, document, outcome);
                        if let Some(event) = model.deferred_reader_event.take() {
                            return self.update(event, model);
                        }
                    }
                    StorageResult::Created(document) | StorageResult::Existing(document) => {
                        model.invalidate_library_words();
                        model.import_summary = Some(ImportSummary {
                            document_id: document.document_id,
                            title: document.title.clone(),
                            outcome: import_outcome.expect("import result"),
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
                        model.reader = None;
                        model.reader_analysis = None;
                        model.invalidate_effective_page();
                        model.reader_tokens = None;
                        model.clear_reader_word();
                        model.reader_document = None;
                        model.reader_analyses.clear();
                        model.clear_reader_time();
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
                        model.loaded_document = Some(document);
                    }
                    StorageResult::PositionSaved(document) => model.apply_position(document),
                    StorageResult::Failed(error) => {
                        let key = token_load.or(status_change.map(|change| change.key));
                        if let Some((id, _)) = document_rename {
                            if let Some(draft) = model
                                .library_rename
                                .as_mut()
                                .filter(|draft| draft.document_id == id)
                            {
                                draft.error = Some(error);
                            }
                        } else if documents_delete.is_some() {
                            if model.library_bulk_delete_targets.is_some() {
                                model.library_delete_error = Some(error);
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
                        } else if let Some(selection) = custom_change {
                            if model.can_edit_custom_entry(selection) {
                                model.reader_custom_entry_error = Some(error);
                            }
                        } else if let Some(selection) = word_load {
                            if model.reader_word_selection == Some(selection)
                                && model.current_token_key() == Some(selection.key)
                            {
                                model.reader_word_error = Some(error);
                            }
                        } else if key.is_none() || key == model.current_token_key() {
                            model.document_error = Some(error);
                        }
                    }
                }
                // Reads and atomic manual choices have no frozen retry obligation.
                // Their failures must still drain final time and deferred departures.
                if (!failed
                    || token_load.is_some()
                    || status_change.is_some()
                    || word_load.is_some()
                    || custom_change.is_some())
                    && model.pending_time_checkpoint.is_none()
                {
                    if let Some(checkpoint) = model.queued_time_checkpoint {
                        return self.update(Event::CheckpointReadingTime(checkpoint), model);
                    }
                }
            }
            Event::RefreshLibrary
            | Event::SaveDocumentTitle(_)
            | Event::ConfirmDocumentDeletion(_)
            | Event::ConfirmSelectedDocumentDeletion(_)
            | Event::LoadLibraryWordDistributions
            | Event::CreatePaste(_)
            | Event::ImportTxt(_)
            | Event::ImportSrt(_)
            | Event::ImportEpub(_)
            | Event::LoadPaste(_)
            | Event::OpenReader(_)
            | Event::LoadReaderWord(_)
            | Event::SaveReaderCustomEntry { .. }
            | Event::DeleteReaderCustomEntry(_)
            | Event::LoadReaderTokens(_)
            | Event::SetReaderStatus(_)
            | Event::SetReaderTypography(_)
            | Event::SetPageLength(_)
            | Event::SaveReadingPosition(_) => unreachable!(),
            Event::RestoreReadingTime
            | Event::CheckpointReadingTime(_)
            | Event::RetryReadingTimeCheckpoint
            | Event::CompleteReaderPage { .. }
            | Event::RetryReaderCompletion
            | Event::PreviousReaderPage { .. }
            | Event::ObserveReaderClock(_)
            | Event::CompleteEffectiveReaderPage { .. }
            | Event::LoadEffectiveReaderWord(_)
            | Event::SaveEffectiveReaderCustomEntry { .. }
            | Event::DeleteEffectiveReaderCustomEntry(_)
            | Event::RetryEffectiveReaderCustomEntry
            | Event::CancelEffectiveReaderCustomEntry
            | Event::SetEffectiveReaderStatus { .. }
            | Event::RetryEffectiveReaderStatus
            | Event::CancelEffectiveReaderStatus
            | Event::EffectiveReaderOverride(_)
            | Event::EffectiveStorageFinished(..) => unreachable!(),
        }
        render::render()
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            destination: model.destination,
            settings_open: model.settings_open,
            documents: model
                .library_sort
                .project(&model.documents, &model.library_title_query),
            library_title_query: model.library_title_query.clone(),
            library_sort: model.library_sort,
            library_word_distributions: model.library_word_distributions.clone(),
            library_word_error: model.library_word_error,
            library_delete_target: model.library_delete_target.clone(),
            library_delete_error: model.library_delete_error,
            library_selection: model.library_selection.clone(),
            library_bulk_delete_targets: model.library_bulk_delete_targets.clone(),
            library_rename: model.library_rename.clone(),
            library_document_count: model.documents.len() as u64,
            loaded_document: model.loaded_document.clone(),
            storage_busy: model.storage_busy,
            document_error: model.document_error,
            content_route: model.content_route,
            reader: model.reader.clone(),
            reader_analysis: model.reader_analysis,
            page_length: model.page_length,
            reader_typography: model.reader_typography,
            import_summary: model.import_summary.clone(),
            reader_time: model.reader_time.map(|saved| saved.value),
            reading_time_save_pending: model.pending_time_checkpoint.is_some()
                || model.queued_time_checkpoint.is_some(),
            completion_save_pending: model.completion_pending(),
            completion_notice: model.completion_notice,
            book_completion_notice: model.book_completion_notice,
            reader_tokens: model.reader_tokens.clone(),
            effective_reader_tokens: model
                .effective_reader
                .as_ref()
                .and_then(|session| session.page().cloned()),
            effective_reader_word: model
                .effective_reader
                .as_ref()
                .map(|session| session.word_state()),
            effective_reader_status: model
                .effective_reader
                .as_ref()
                .map(|session| session.status_state()),
            effective_reader_override: model
                .effective_reader
                .as_ref()
                .map(|session| session.override_view()),
            reader_furigana: model
                .reader_typography
                .filter(|typography| typography.furigana)
                .and(model.reader.as_ref())
                .and_then(|page| {
                    let analysis = model.reader_analysis.unwrap_or(ReaderAnalysis::Missing);
                    if let Some(session) = &model.effective_reader {
                        session.furigana(page, analysis).ok()
                    } else {
                        page.furigana(analysis, model.reader_tokens.as_ref()).ok()
                    }
                })
                .unwrap_or_default(),
            reader_word_selection: model.reader_word_selection,
            reader_word: model.reader_word.clone(),
            reader_word_error: model.reader_word_error,
            reader_custom_entry_error: model.reader_custom_entry_error,
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
        self.library_bulk_delete_targets = None;
        self.library_delete_error = None;
    }

    fn can_select_library_documents(&self) -> bool {
        self.library_visible()
            && !self.storage_busy
            && self.pending_time_checkpoint.is_none()
            && self.queued_time_checkpoint.is_none()
            && !self.completion_pending()
            && self.library_delete_target.is_none()
            && self.library_bulk_delete_targets.is_none()
            && self.library_rename.is_none()
    }

    fn apply_document_deletion(&mut self, ids: &[DocumentId]) {
        let ids: std::collections::BTreeSet<_> = ids.iter().copied().collect();
        self.documents
            .retain(|entry| !ids.contains(&entry.document_id));
        self.invalidate_library_words();
        self.clear_library_delete();
        self.library_selection = None;
        if self
            .import_summary
            .as_ref()
            .is_some_and(|summary| ids.contains(&summary.document_id))
        {
            self.import_summary = None;
        }
        if self
            .loaded_document
            .as_ref()
            .is_some_and(|document| ids.contains(&document.document_id))
        {
            self.loaded_document = None;
        }
    }

    fn invalidate_library_words(&mut self) {
        self.library_word_distributions = None;
        self.library_word_error = None;
    }

    fn can_edit_custom_entry(&self, selection: ReaderWordSelection) -> bool {
        !self.settings_open
            && self.current_token_key() == Some(selection.key)
            && self.reader_word_selection == Some(selection)
            && self.reader_word.as_ref().is_some_and(|details| {
                details.selection() == selection
                    && details.candidates.words.is_empty()
                    && details.candidates.names.is_empty()
            })
    }

    fn clear_reader_word(&mut self) {
        if let Some(session) = &mut self.effective_reader {
            session.dismiss_word();
        }
        self.reader_word_selection = None;
        self.reader_word = None;
        self.reader_word_error = None;
        self.reader_custom_entry_error = None;
        // Keep the in-flight identity until its callback for clock-save draining.
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

    fn close_document(&mut self) {
        self.library_selection = None;
        self.library_rename = None;
        self.clear_library_delete();
        self.invalidate_library_words();
        self.completion_notice = None;
        self.book_completion_notice = None;
        self.content_route = ContentRoute::Main;
        self.reader = None;
        self.reader_analysis = None;
        self.invalidate_effective_page();
        self.reader_tokens = None;
        self.clear_reader_word();
        self.reader_document = None;
        self.reader_analyses.clear();
        self.pending_reader = None;
        self.clear_reader_time();
        self.document_error = None;
    }

    fn observe_reader_clock(&mut self, sample: ReaderClockSample) -> Option<ReadingTimeUpdate> {
        let saved = self.reader_time?;
        if sample.monotonic_ms < 0
            || sample.document_id != saved.value.document_id
            || sample.position != saved.position()
            || !self.is_current_time(saved.value)
            || self.completion_pending()
            || self.deferred_reader_event.is_some()
            || (self.storage_busy
                && self.pending_time_checkpoint.is_none()
                && !self.time_checkpoint_can_wait)
            || (self.settings_open && sample.action != ReaderClockAction::Pause)
        {
            return None;
        }
        let clock = self
            .reader_clock
            .get_or_insert_with(|| ActiveReadingTime::from_pending(saved.pending()));
        // Explicit absolute checkpoints may coexist with the clock driver. A
        // higher externally restored duration starts a fresh, paused accumulator.
        if clock.pending().milliseconds() < saved.pending().milliseconds() {
            *clock = ActiveReadingTime::from_pending(saved.pending());
        }
        let now = sample.monotonic_ms as u64;
        let pending = match sample.action {
            ReaderClockAction::Resume => clock.resume(now),
            ReaderClockAction::Interact => clock.interact(now),
            ReaderClockAction::Checkpoint => clock.checkpoint(now),
            ReaderClockAction::Pause => clock.pause(now),
        }
        .ok()?;
        if !matches!(
            sample.action,
            ReaderClockAction::Checkpoint | ReaderClockAction::Pause
        ) || pending.milliseconds() <= saved.pending().milliseconds()
        {
            return None;
        }
        Some(ReadingTimeUpdate {
            pending,
            ..saved.value
        })
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
                    self.reader_tokens = None;
                    self.clear_reader_word();
                    if self.reader_time.is_some_and(|saved| {
                        saved.value.document_id != reader.document_id
                            || saved.position() != reader.position
                    }) {
                        self.reader_time = None;
                        self.reader_clock = None;
                    }
                    self.reader_analysis = self
                        .reader_analyses
                        .iter()
                        .find(|entry| entry.chapter_id == reader.anchor.chapter_id)
                        .map(|entry| entry.analysis);
                    self.reader = Some(reader);
                    self.reader_document = Some(document.clone());
                    self.loaded_document = Some(document);
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

    fn is_current_time(&self, value: ReadingTimeUpdate) -> bool {
        self.content_route == ContentRoute::Reader
            && self.reader.as_ref().is_some_and(|reader| {
                reader.document_id == value.document_id && reader.position == value.position
            })
    }

    fn clear_reader_time(&mut self) {
        self.reader_time = None;
        self.reader_clock = None;
        self.restoring_reader_time = false;
        // Checkpoint obligations drain before reader departure reaches here.
    }

    fn apply_page_length(&mut self, length: PageLength) {
        self.page_length = Some(length);
        if let (Some(reader), Some(document)) = (&self.reader, &self.reader_document) {
            match ReaderPage::at_document(document, reader.position, length) {
                Ok(page) => {
                    if page.anchor != reader.anchor {
                        self.invalidate_effective_page();
                        self.reader_tokens = None;
                        self.clear_reader_word();
                    }
                    self.reader = Some(page);
                }
                Err(_) => self.document_error = Some(DocumentError::InvalidDocument),
            }
        }
    }
}
