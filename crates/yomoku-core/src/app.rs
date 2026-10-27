//! Pure application state. Hosts execute effects; the core performs no I/O.

use crate::typography::ReaderTypography;
use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

use crate::{
    active_time::{
        ActiveReadingTime, ReaderClockAction, ReaderClockSample, ReadingTimeUpdate,
        SavedReadingTime,
    },
    completion::{AnalyzedCompletionOutcome, CompletionNotice, PageCompletionIntent},
    document::{CompletionCommandId, DocumentId, LibraryDocument, ReaderDocument, TextAnchor},
    imports::{EpubImport, SrtImport, TxtImport},
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
    reader_word_selection: Option<ReaderWordSelection>,
    reader_word: Option<ReaderWordDetails>,
    reader_word_error: Option<DocumentError>,
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
    /// Per-file atomic import; success returns to Library with a summary.
    ImportTxt(TxtImport),
    ImportSrt(SrtImport),
    ImportEpub(EpubImport),
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
    /// Enabled source-preferred readings, in page-local UTF-8 ranges. No I/O.
    pub reader_furigana: Vec<crate::reader_ruby::ReaderRuby>,
    pub reader_word_selection: Option<ReaderWordSelection>,
    pub reader_word: Option<ReaderWordDetails>,
    /// Separate from save errors so timer checkpoints cannot erase a failed lookup.
    pub reader_word_error: Option<DocumentError>,
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
        if let Event::PreviousReaderPage {
            document_id,
            page,
            position,
        } = event
        {
            if model.content_route != ContentRoute::Reader
                || model.settings_open
                || model.pending_completion.is_some()
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
            || model.pending_completion.is_some())
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
                    || model.pending_completion.is_some()
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
                if model.pending_completion.is_some()
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
            let draining_completion_time = matches!(
                model.pending_completion,
                Some(StorageOperation::PreparePageCompletion(_))
            ) && matches!(event, Event::CheckpointReadingTime(checkpoint)
                if model.queued_time_checkpoint == Some(checkpoint));
            // One in-flight command per session prevents duplicate submissions and
            // stale load/list callbacks from replacing a newer snapshot.
            if model.storage_busy
                || (model.queued_time_checkpoint.is_some()
                    && !matches!(
                        event,
                        Event::CheckpointReadingTime(_) | Event::RetryReadingTimeCheckpoint
                    ))
                || (model.pending_time_checkpoint.is_some()
                    && !matches!(event, Event::RetryReadingTimeCheckpoint))
                || (model.pending_completion.is_some()
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
                | StorageOperation::LoadPaste(_)
                | StorageOperation::SavePageLength(_)
                | StorageOperation::SaveReaderTypography(_)
                | StorageOperation::LoadReadingTime { .. }
                | StorageOperation::LoadReaderWord(_)
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
            model.document_error = None;
            if let StorageOperation::LoadReaderWord(selection) = operation {
                model.reader_word_selection = Some(selection);
                model.pending_word_load = Some(selection);
                model.reader_word = None;
                model.reader_word_error = None;
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
                model.clear_reader_word();
            }
            if let StorageOperation::LoadReaderTokens(key) = operation {
                model.pending_token_load = Some(key);
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
            Event::SelectDestination(destination) => {
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.import_summary = None;
                model.destination = destination;
                model.settings_open = false;
                model.content_route = ContentRoute::Main;
                model.reader = None;
                model.reader_analysis = None;
                model.reader_tokens = None;
                model.clear_reader_word();
                model.reader_document = None;
                model.reader_analyses.clear();
                model.pending_reader = None;
                model.clear_reader_time();
            }
            Event::OpenPaste => {
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.import_summary = None;
                model.destination = Destination::Library;
                model.settings_open = false;
                model.content_route = ContentRoute::Paste;
                model.reader = None;
                model.reader_analysis = None;
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
                let failed = matches!(result, StorageResult::Failed(_));
                model.storage_busy = false;
                model.time_checkpoint_can_wait = false;
                let restoring_time = std::mem::take(&mut model.restoring_reader_time);
                let pending_reader = model.pending_reader.take();
                let token_load = model.pending_token_load.take();
                let status_change = model.pending_status_change.take();
                let word_load = model.pending_word_load.take();
                let import_outcome = match &result {
                    StorageResult::Created(_) => Some(ImportOutcome::Created),
                    StorageResult::Existing(_) => Some(ImportOutcome::Existing),
                    _ => None,
                };
                match result {
                    StorageResult::ImportCancelled => {}
                    StorageResult::ReaderWordLoaded(details) => {
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
                    StorageResult::ReaderStatusChanged(change, entry) => {
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
                        model.reader_typography = Some(typography);
                        model.documents = documents;
                        model.apply_page_length(length);
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
                        if model.pending_completion.is_some() {
                            return self.update(Event::RetryReaderCompletion, model);
                        }
                        if let Some(event) = model.deferred_reader_event.take() {
                            return self.update(event, model);
                        }
                    }
                    StorageResult::PageCompletionPrepared(prepared) => {
                        model.pending_completion = Some(StorageOperation::CompletePage(prepared));
                        return self.update(Event::RetryReaderCompletion, model);
                    }
                    StorageResult::PageCompleted(document, outcome) => {
                        let Some(StorageOperation::CompletePage(prepared)) =
                            model.pending_completion.take()
                        else {
                            return render::render();
                        };
                        // The frozen page identifies this completion, not the permanent
                        // marker (which is already true on every deliberate reread).
                        let final_page = document.chapters.last().is_some_and(|chapter| {
                            prepared.completion.page.source_id == chapter.source_id
                                && prepared.completion.page.chapter_id == chapter.chapter_id
                                && prepared.completion.page.range.end.byte_index(&chapter.text)
                                    == Ok(chapter.text.len())
                        });
                        model.completion_notice = match outcome {
                            AnalyzedCompletionOutcome::Recorded { newly_known }
                                if newly_known > 0 =>
                            {
                                Some(CompletionNotice {
                                    command_id: prepared.completion.command_id,
                                    newly_known,
                                })
                            }
                            _ => None,
                        };
                        // Completion consumed every bucket on the page, even if
                        // its resulting position equals the departing EOF.
                        model.reader_time = None;
                        model.reader_clock = None;
                        model.apply_position(document);
                        if final_page {
                            // Successful Finish Book always returns to Library after
                            // the same final-clock drain and atomic completion.
                            model.close_document();
                            model.destination = Destination::Library;
                            model.settings_open = false;
                            model.deferred_reader_event = None;
                            if let AnalyzedCompletionOutcome::Recorded { newly_known } = outcome {
                                model.book_completion_notice = Some(prepared.completion.command_id);
                                model.completion_notice =
                                    (newly_known > 0).then_some(CompletionNotice {
                                        command_id: prepared.completion.command_id,
                                        newly_known,
                                    });
                            }
                        } else if let Some(event) = model.deferred_reader_event.take() {
                            return self.update(event, model);
                        }
                    }
                    StorageResult::Created(document) | StorageResult::Existing(document) => {
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
                        if let Some(selection) = word_load {
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
                    || word_load.is_some())
                    && model.pending_time_checkpoint.is_none()
                {
                    if let Some(checkpoint) = model.queued_time_checkpoint {
                        return self.update(Event::CheckpointReadingTime(checkpoint), model);
                    }
                }
            }
            Event::RefreshLibrary
            | Event::CreatePaste(_)
            | Event::ImportTxt(_)
            | Event::ImportSrt(_)
            | Event::ImportEpub(_)
            | Event::LoadPaste(_)
            | Event::OpenReader(_)
            | Event::LoadReaderWord(_)
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
            | Event::ObserveReaderClock(_) => unreachable!(),
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
            reader_typography: model.reader_typography,
            import_summary: model.import_summary.clone(),
            reader_time: model.reader_time.map(|saved| saved.value),
            reading_time_save_pending: model.pending_time_checkpoint.is_some()
                || model.queued_time_checkpoint.is_some(),
            completion_save_pending: model.pending_completion.is_some(),
            completion_notice: model.completion_notice,
            book_completion_notice: model.book_completion_notice,
            reader_tokens: model.reader_tokens.clone(),
            reader_furigana: model
                .reader_typography
                .filter(|typography| typography.furigana)
                .and(model.reader.as_ref())
                .and_then(|page| {
                    page.furigana(
                        model.reader_analysis.unwrap_or(ReaderAnalysis::Missing),
                        model.reader_tokens.as_ref(),
                    )
                    .ok()
                })
                .unwrap_or_default(),
            reader_word_selection: model.reader_word_selection,
            reader_word: model.reader_word.clone(),
            reader_word_error: model.reader_word_error,
        }
    }
}

impl Model {
    fn clear_reader_word(&mut self) {
        self.reader_word_selection = None;
        self.reader_word = None;
        self.reader_word_error = None;
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
        self.completion_notice = None;
        self.book_completion_notice = None;
        self.content_route = ContentRoute::Main;
        self.reader = None;
        self.reader_analysis = None;
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
            || self.pending_completion.is_some()
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
                    *entry = summary;
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
