//! Guarded boundary-edit repositories. Contexts are native-only and database scoped.

use std::{collections::BTreeSet, sync::Arc};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    analysis::{AnalysisProfile, ChapterAnalysis, OverrideScope, SegmentationOverride, SplitMode},
    document::{AnalysisId, ChapterText, OverrideId, TextAnchor},
    effective_overrides::OverrideRuleRevision,
    effective_reader::EffectiveReaderTokenKey,
    segmentation::segmentation_from_snapshot,
    vocabulary::Surface,
};

use crate::{
    analysis::{
        read_analysis, read_chapter, read_override, save_registered_override_in,
        store_chapter_analysis_in,
    },
    reader_tokens::load_effective_reader_tokens_in,
    Storage, StorageError,
};

#[derive(Debug)]
pub enum EffectiveOverrideError {
    Storage(StorageError),
    InvalidSelection,
    /// Reload explicitly. Never replay an old edit against a newer context.
    StaleContext,
}

impl From<StorageError> for EffectiveOverrideError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<rusqlite::Error> for EffectiveOverrideError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.into())
    }
}

impl std::fmt::Display for EffectiveOverrideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Storage(_) => "boundary edit storage operation failed",
            Self::InvalidSelection => "invalid boundary edit selection",
            Self::StaleContext => "boundary edit context changed; reload explicitly",
        })
    }
}

impl std::error::Error for EffectiveOverrideError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

pub use yomoku_core::effective_overrides::{EffectiveOverrideContext, StoredBoundaryChoice};

#[derive(Debug)]
pub enum EffectiveOverrideAnalysisError<E> {
    Context(EffectiveOverrideError),
    Analysis(E),
}

impl<E> From<EffectiveOverrideError> for EffectiveOverrideAnalysisError<E> {
    fn from(error: EffectiveOverrideError) -> Self {
        Self::Context(error)
    }
}

impl<E> From<StorageError> for EffectiveOverrideAnalysisError<E> {
    fn from(error: StorageError) -> Self {
        Self::Context(error.into())
    }
}

impl<E> From<rusqlite::Error> for EffectiveOverrideAnalysisError<E> {
    fn from(error: rusqlite::Error) -> Self {
        Self::Context(error.into())
    }
}

impl Storage {
    /// Explicit on-demand cache fill, never part of a page/prompt read. Reuse
    /// stored same-profile A/C before invoking the analyzer; compute against the
    /// WHOLE canonical chapter outside the transaction/write lock. Revalidate
    /// the frozen key and rule revision under the final immediate transaction.
    /// No rules, registration, history, time or progress are changed. Reload the
    /// page/context after success: new exact metadata may retire the old key.
    pub fn generate_effective_override_analysis<E>(
        &mut self,
        context: &EffectiveOverrideContext,
        mode: SplitMode,
        analyze: impl FnOnce(&ChapterText, SplitMode, &AnalysisProfile) -> Result<ChapterAnalysis, E>,
    ) -> Result<AnalysisId, EffectiveOverrideAnalysisError<E>> {
        if mode == SplitMode::B {
            return Err(EffectiveOverrideError::InvalidSelection.into());
        }
        let transaction = self.connection.transaction()?;
        validate_context_in(&transaction, context)?;
        let chapter = read_chapter(&transaction, context.anchor().chapter_id)?;
        let original = read_analysis(
            &transaction,
            context.key().segmentation.original_analysis_id,
        )?
        .ok_or(StorageError::InvalidAnalysis)?;
        let base = segmentation_from_snapshot(
            &chapter,
            &original,
            &original,
            context.anchor(),
            OverrideScope::default(),
        )
        .map_err(|_| EffectiveOverrideError::InvalidSelection)?;
        if base.surface != *context.surface() {
            return Err(EffectiveOverrideError::InvalidSelection.into());
        }
        let existing: Option<i64> = transaction
            .query_row(
                "SELECT id FROM chapter_analyses WHERE chapter_id = ?1
                 AND engine_revision = ?2 AND dictionary_revision = ?3 AND split_mode = ?4",
                (
                    i64::from(chapter.chapter_id),
                    &original.profile.engine_revision,
                    &original.profile.dictionary_revision,
                    if mode == SplitMode::A { "a" } else { "c" },
                ),
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            let id = id.try_into().map_err(|_| StorageError::InvalidAnalysis)?;
            read_analysis(&transaction, id)?.ok_or(StorageError::InvalidAnalysis)?;
            transaction.commit()?;
            return Ok(id);
        }
        transaction.commit()?;

        let analysis = analyze(&chapter, mode, &original.profile)
            .map_err(EffectiveOverrideAnalysisError::Analysis)?;
        if analysis.mode != mode || analysis.profile != original.profile {
            return Err(StorageError::InvalidAnalysis.into());
        }
        analysis
            .validate(&chapter)
            .map_err(|_| StorageError::InvalidAnalysis)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_context_in(&transaction, context)?;
        let id = store_chapter_analysis_in(&transaction, chapter.chapter_id, &analysis)?;
        transaction.commit()?;
        Ok(id)
    }

    /// Read choices and an intervening-edit guard from the rendered full key in
    /// one read-only transaction. Both span ends cover complete contiguous JP B
    /// tokens that intersect the page; a crossing edge token stays whole.
    /// No analyzer, registration, position/time or history writes.
    pub fn load_effective_override_context(
        &mut self,
        key: Arc<EffectiveReaderTokenKey>,
        anchor: TextAnchor,
    ) -> Result<EffectiveOverrideContext, EffectiveOverrideError> {
        let transaction = self.connection.transaction()?;
        validate_key_in(&transaction, &key)?;
        if anchor.source_id != key.page.source_id || anchor.chapter_id != key.page.chapter_id {
            return Err(EffectiveOverrideError::InvalidSelection);
        }
        let chapter = read_chapter(&transaction, anchor.chapter_id)?;
        let original = read_analysis(&transaction, key.segmentation.original_analysis_id)?
            .ok_or(EffectiveOverrideError::InvalidSelection)?;
        let base = segmentation_from_snapshot(
            &chapter,
            &original,
            &original,
            anchor,
            OverrideScope::default(),
        )
        .map_err(|_| EffectiveOverrideError::InvalidSelection)?;
        // Do not accept a span containing wholly off-page original tokens.
        if original.tokens.iter().any(|token| {
            token.anchor.range.start >= anchor.range.start
                && token.anchor.range.end <= anchor.range.end
                && (token.anchor.range.end <= key.page.range.start
                    || token.anchor.range.start >= key.page.range.end)
        }) {
            return Err(EffectiveOverrideError::InvalidSelection);
        }
        let mut choices = Vec::new();
        for (mode, encoded) in [
            (SplitMode::A, "a"),
            (SplitMode::B, "b"),
            (SplitMode::C, "c"),
        ] {
            if mode == SplitMode::B {
                choices.push(StoredBoundaryChoice {
                    mode,
                    parts: base.parts.clone(),
                });
                continue;
            }
            let mut statement = transaction.prepare(
                "SELECT id FROM chapter_analyses WHERE chapter_id = ?1
                 AND engine_revision = ?2 AND dictionary_revision = ?3 AND split_mode = ?4",
            )?;
            let ids = statement
                .query_map(
                    (
                        i64::from(anchor.chapter_id),
                        &original.profile.engine_revision,
                        &original.profile.dictionary_revision,
                        encoded,
                    ),
                    |row| row.get::<_, i64>(0),
                )?
                .collect::<Result<Vec<_>, _>>()?;
            for id in ids {
                let alternate = read_analysis(
                    &transaction,
                    id.try_into().map_err(|_| StorageError::InvalidAnalysis)?,
                )?
                .ok_or(StorageError::InvalidAnalysis)?;
                if let Ok(edit) = segmentation_from_snapshot(
                    &chapter,
                    &original,
                    &alternate,
                    anchor,
                    OverrideScope::default(),
                ) {
                    choices.push(StoredBoundaryChoice {
                        mode,
                        parts: edit.parts,
                    });
                }
            }
        }
        let ids: BTreeSet<_> = key
            .segmentation
            .segments
            .iter()
            .filter(|segment| {
                segment.boundary.anchor.range.start < anchor.range.end
                    && segment.boundary.anchor.range.end > anchor.range.start
            })
            .filter_map(|segment| segment.boundary.override_id)
            .collect();
        let rules = ids
            .into_iter()
            .map(|id| read_override(&transaction, id)?.ok_or(StorageError::InvalidAnalysis))
            .collect::<Result<Vec<_>, _>>()?;
        let context = EffectiveOverrideContext::from_snapshot(
            key,
            anchor,
            base.surface,
            rule_revision_in(&transaction)?,
            choices,
            rules,
        );
        transaction.commit()?;
        Ok(context)
    }

    /// Guard and save/register in the SAME immediate transaction. Failed writes
    /// may retry this context after rollback. Successful mutations consume its
    /// before-image: even a lost-success response requires explicit reload, so a
    /// retry cannot recreate deleted vocabulary or overwrite an intervening edit.
    pub fn save_effective_override(
        &mut self,
        context: &EffectiveOverrideContext,
        scope: OverrideScope,
        parts: Vec<Surface>,
        registered_at_unix_ms: i64,
    ) -> Result<(OverrideId, usize), EffectiveOverrideError> {
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_context_in(&transaction, context)?;
        let edit = SegmentationOverride {
            scope,
            anchor: context.anchor(),
            surface: context.surface().clone(),
            parts,
        };
        let result = save_registered_override_in(&transaction, &edit, registered_at_unix_ms)?;
        transaction.commit()?;
        Ok(result)
    }

    /// Reverse only an exact winning rule shown by this prompt. Registration,
    /// time, progress and past events remain intact; superseded rules stay inactive.
    pub fn reverse_effective_override(
        &mut self,
        context: &EffectiveOverrideContext,
        id: OverrideId,
    ) -> Result<(), EffectiveOverrideError> {
        if !context
            .rules()
            .iter()
            .any(|rule| rule.id == id && rule.active)
        {
            return Err(EffectiveOverrideError::InvalidSelection);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_context_in(&transaction, context)?;
        let changed = transaction.execute(
            "UPDATE segmentation_overrides SET active = 0 WHERE id = ?1 AND active = 1",
            [i64::from(id)],
        )?;
        if changed != 1 {
            return Err(EffectiveOverrideError::StaleContext);
        }
        transaction.commit()?;
        Ok(())
    }
}

fn rule_revision_in(connection: &Connection) -> Result<OverrideRuleRevision, StorageError> {
    Ok(connection.query_row(
        "SELECT COALESCE(MAX(id), 0), COALESCE(SUM(CASE WHEN active = 0 THEN 1 ELSE 0 END), 0)
         FROM segmentation_overrides",
        [],
        |row| {
            Ok(OverrideRuleRevision {
                newest_id: row.get(0)?,
                inactive: row.get(1)?,
            })
        },
    )?)
}

fn validate_key_in(
    connection: &Connection,
    key: &EffectiveReaderTokenKey,
) -> Result<(), EffectiveOverrideError> {
    let snapshot = load_effective_reader_tokens_in(connection, key.document_id, key.page)?
        .ok_or(EffectiveOverrideError::InvalidSelection)?;
    if snapshot.segmentation.as_ref() != Some(&key.segmentation) {
        return Err(EffectiveOverrideError::StaleContext);
    }
    Ok(())
}

fn validate_context_in(
    connection: &Connection,
    context: &EffectiveOverrideContext,
) -> Result<(), EffectiveOverrideError> {
    if rule_revision_in(connection)? != context.rule_revision() {
        return Err(EffectiveOverrideError::StaleContext);
    }
    validate_key_in(connection, context.key())
}
