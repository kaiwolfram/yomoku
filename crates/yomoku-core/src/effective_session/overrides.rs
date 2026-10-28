//! Retained boundary prompts and exact native edit requests. No shell codec.

use std::sync::Arc;

use super::{EffectivePageCommand, EffectivePageIdentity, EffectiveReaderSession};
use crate::{
    analysis::{OverrideScope, SegmentationOverride, SplitMode},
    document::{OverrideId, TextAnchor},
    effective_overrides::{EffectiveOverrideContext, EffectiveOverrideFailure as Failure},
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    storage::DocumentError,
    vocabulary::Surface,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Load {
        command: EffectivePageCommand,
        anchor: TextAnchor,
    },
    Save {
        context: Arc<EffectiveOverrideContext>,
        scope: OverrideScope,
        parts: Vec<Surface>,
        registered_at_unix_ms: i64,
    },
    Reverse {
        context: Arc<EffectiveOverrideContext>,
        id: OverrideId,
    },
}

impl Action {
    fn is_write(&self) -> bool {
        !matches!(self, Self::Load { .. })
    }
}

/// Capture this native token in the callback. Retries share the context/key but
/// use a fresh attempt; a duplicate old callback cannot consume the new request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveOverrideRequest {
    attempt: u64,
    action: Action,
}

impl EffectiveOverrideRequest {
    pub fn operation(&self) -> Operation {
        match &self.action {
            Action::Load { command, anchor } => Operation::LoadOverrideContext {
                command: command.clone(),
                anchor: *anchor,
            },
            Action::Save {
                context,
                scope,
                parts,
                registered_at_unix_ms,
            } => Operation::SaveOverride {
                context: Arc::clone(context),
                scope: *scope,
                parts: parts.clone(),
                registered_at_unix_ms: *registered_at_unix_ms,
            },
            Action::Reverse { context, id } => Operation::ReverseOverride {
                context: Arc::clone(context),
                id: *id,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveOverrideState {
    pub pending: bool,
    pub busy: bool,
    pub error: Option<Failure>,
    /// Success or a retired before-image requires explicit page/context reload.
    pub reload_required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectiveOverrideProgress {
    Loaded,
    Saved { id: OverrideId, registered: usize },
    Reversed(OverrideId),
    Failed(Failure),
}

#[derive(Default)]
pub(super) struct OverrideState {
    attempt: u64,
    prompt: Option<(EffectivePageIdentity, TextAnchor)>,
    context: Option<(EffectivePageIdentity, Arc<EffectiveOverrideContext>)>,
    pending: Option<EffectiveOverrideRequest>,
    in_flight: bool,
    error: Option<Failure>,
    reload_required: bool,
}

impl OverrideState {
    pub(super) fn blocks_reader(&self) -> bool {
        self.in_flight || self.pending.as_ref().is_some_and(|r| r.action.is_write())
    }
}

impl EffectiveReaderSession {
    pub fn override_state(&self) -> EffectiveOverrideState {
        EffectiveOverrideState {
            pending: self.overrides.pending.is_some(),
            busy: self.overrides.in_flight,
            error: self.overrides.error,
            reload_required: self.overrides.reload_required,
        }
    }

    /// Native-only full context; a future shell view must compact choices/rules.
    pub fn override_context(&self) -> Option<&EffectiveOverrideContext> {
        self.overrides
            .context
            .as_ref()
            .map(|(_, context)| context.as_ref())
    }

    /// Retire presentation without abandoning accepted I/O or failed writes.
    pub fn dismiss_override(&mut self) {
        self.overrides.prompt = None;
        self.overrides.context = None;
    }

    /// Explicit open/retry after a read failure. Repository validation enforces
    /// contiguous original-B Japanese boundaries, including full crossing tokens.
    /// The live app must additionally apply shared time/FIFO/lifecycle gates.
    pub fn begin_override_load(
        &mut self,
        identity: EffectivePageIdentity,
        anchor: TextAnchor,
    ) -> Option<EffectiveOverrideRequest> {
        if self.overrides.blocks_reader() || !self.override_work_allowed() {
            return None;
        }
        let command = self.freeze_page(identity)?;
        if anchor.source_id != identity.base.page.source_id
            || anchor.chapter_id != identity.base.page.chapter_id
            || anchor.range.start >= anchor.range.end
            || anchor.range.end <= identity.base.page.range.start
            || anchor.range.start >= identity.base.page.range.end
        {
            return None;
        }
        let request = self.start_override(Action::Load { command, anchor })?;
        self.dismiss_override();
        self.overrides.prompt = Some((identity, anchor));
        self.overrides.reload_required = false;
        Some(request)
    }

    pub fn begin_override_choice(
        &mut self,
        mode: SplitMode,
        scope: OverrideScope,
        registered_at_unix_ms: i64,
    ) -> Option<EffectiveOverrideRequest> {
        let parts = self
            .override_context()?
            .choices()
            .iter()
            .find(|choice| choice.mode == mode)?
            .parts
            .clone();
        self.begin_override_save(scope, parts, registered_at_unix_ms)
    }

    /// Freeze literal parts, scope, timestamp and before-image. Immediately retire
    /// the rendered page so completion/selection cannot use its old partition.
    pub fn begin_override_save(
        &mut self,
        scope: OverrideScope,
        parts: Vec<Surface>,
        registered_at_unix_ms: i64,
    ) -> Option<EffectiveOverrideRequest> {
        let context = self.editable_override_context()?;
        if registered_at_unix_ms < 0 {
            return None;
        }
        SegmentationOverride {
            scope,
            anchor: context.anchor(),
            surface: context.surface().clone(),
            parts: parts.clone(),
        }
        .validate_shape()
        .ok()?;
        let request = self.start_override(Action::Save {
            context,
            scope,
            parts,
            registered_at_unix_ms,
        })?;
        self.invalidate();
        Some(request)
    }

    pub fn begin_override_reverse(&mut self, id: OverrideId) -> Option<EffectiveOverrideRequest> {
        let context = self.editable_override_context()?;
        context
            .rules()
            .iter()
            .find(|rule| rule.id == id && rule.active)?;
        let request = self.start_override(Action::Reverse { context, id })?;
        self.invalidate();
        Some(request)
    }

    /// Only rolled-back/unknown I/O failures retain a write. Stale contexts are
    /// discarded, never rebased onto a new page, even after a lost success.
    pub fn retry_override_write(&mut self) -> Option<EffectiveOverrideRequest> {
        if self.overrides.in_flight || !self.override_work_allowed() {
            return None;
        }
        let action = self.overrides.pending.as_ref()?.action.clone();
        if !action.is_write() {
            return None;
        }
        self.start_override(action)
    }

    pub fn cancel_override_retry(&mut self) -> bool {
        if self.overrides.in_flight || self.overrides.pending.is_none() {
            return false;
        }
        self.overrides.pending = None;
        self.overrides.error = None;
        // A failed response may hide a commit. Reload before any further edit.
        self.invalidate();
        self.overrides.reload_required = true;
        true
    }

    pub fn finish_override_request(
        &mut self,
        request: &EffectiveOverrideRequest,
        result: Result<Output, DocumentError>,
    ) -> Option<EffectiveOverrideProgress> {
        if !self.overrides.in_flight || self.overrides.pending.as_ref() != Some(request) {
            return None;
        }
        let result = match (&request.action, result) {
            (Action::Load { command, anchor }, Ok(Output::OverrideContextLoaded(context)))
                if context.key() == command.key() && context.anchor() == *anchor =>
            {
                if self.overrides.prompt == Some((command.identity(), *anchor))
                    && self.resolve(command.identity()).is_some()
                {
                    self.overrides.context = Some((command.identity(), context));
                }
                Ok(EffectiveOverrideProgress::Loaded)
            }
            (Action::Save { .. }, Ok(Output::OverrideSaved { id, registered })) => {
                Ok(EffectiveOverrideProgress::Saved { id, registered })
            }
            (Action::Reverse { id, .. }, Ok(Output::OverrideReversed(returned)))
                if *id == returned =>
            {
                Ok(EffectiveOverrideProgress::Reversed(returned))
            }
            (_, Ok(Output::OverrideFailed(error))) => Err(error),
            (_, Err(error)) => Err(Failure::Storage(error)),
            _ => Err(Failure::Storage(DocumentError::InvalidDocument)),
        };
        self.overrides.in_flight = false;
        match result {
            Ok(progress) => {
                self.overrides.pending = None;
                self.overrides.error = None;
                if request.action.is_write() {
                    // Even a newer page may have loaded before the write ended.
                    self.invalidate();
                    self.overrides.reload_required = true;
                }
                Some(progress)
            }
            Err(error) => {
                self.overrides.error = Some(error);
                if !matches!(error, Failure::Storage(_)) {
                    self.overrides.pending = None;
                    if request.action.is_write()
                        || (error == Failure::StaleContext
                            && matches!(&request.action, Action::Load { command, .. }
                                if self.resolve(command.identity()).is_some()))
                    {
                        self.invalidate();
                        self.overrides.reload_required = true;
                    }
                } else if !request.action.is_write() {
                    self.overrides.pending = None;
                }
                Some(EffectiveOverrideProgress::Failed(error))
            }
        }
    }

    fn override_work_allowed(&self) -> bool {
        !self.words.blocks_completion()
            && !self.status_state().pending
            && !self.completion_state().pending
    }

    fn editable_override_context(&self) -> Option<Arc<EffectiveOverrideContext>> {
        if self.overrides.blocks_reader() || !self.override_work_allowed() {
            return None;
        }
        let (identity, context) = self.overrides.context.as_ref()?;
        (self.resolve(*identity) == Some(context.key())).then(|| Arc::clone(context))
    }

    fn start_override(&mut self, action: Action) -> Option<EffectiveOverrideRequest> {
        let attempt = self.overrides.attempt.checked_add(1)?;
        let request = EffectiveOverrideRequest { attempt, action };
        self.overrides.attempt = attempt;
        self.overrides.pending = Some(request.clone());
        self.overrides.in_flight = true;
        self.overrides.error = None;
        Some(request)
    }
}
