//! Effective boundaries derived from immutable Mode B and reversible edits.
//!
//! These values are not analyzer snapshots or vocabulary identities. In
//! particular, a manual boundary never fabricates a reading or dictionary form.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    analysis::{
        ChapterAnalysis, InvalidAnalysis, OverrideScope, SegmentationOverride, SplitMode,
        StoredOverride,
    },
    document::{AnalysisId, ChapterText, OverrideId, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSegment {
    pub anchor: TextAnchor,
    pub surface: String,
    /// Present only when this segment exactly matches an original Mode B token.
    /// Consumers may retrieve metadata from that immutable token, never guess it.
    pub original_token_index: Option<usize>,
    pub override_id: Option<OverrideId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterSegmentation {
    pub original_analysis_id: AnalysisId,
    pub segments: Vec<ResolvedSegment>,
}

/// Build a boundary edit from A/B/C tokens in a validated, same-profile snapshot.
/// Both ends must be token boundaries; crossing tokens are never truncated.
pub fn segmentation_from_snapshot(
    chapter: &ChapterText,
    original: &ChapterAnalysis,
    alternate: &ChapterAnalysis,
    anchor: TextAnchor,
    scope: OverrideScope,
) -> Result<SegmentationOverride, InvalidAnalysis> {
    original.validate(chapter)?;
    alternate.validate(chapter)?;
    if original.mode != SplitMode::B || original.profile != alternate.profile {
        return Err(InvalidAnalysis);
    }
    japanese_span(original, anchor).ok_or(InvalidAnalysis)?;
    let (start, end) = japanese_span(alternate, anchor).ok_or(InvalidAnalysis)?;
    let edit = SegmentationOverride {
        scope,
        anchor,
        surface: Surface::new(anchor.slice(chapter).map_err(|_| InvalidAnalysis)?)
            .map_err(|_| InvalidAnalysis)?,
        parts: alternate.tokens[start..end]
            .iter()
            .map(|token| Surface::new(&token.surface).map_err(|_| InvalidAnalysis))
            .collect::<Result<_, _>>()?,
    };
    edit.replacement_anchors(chapter)?;
    Ok(edit)
}

/// Resolve once over the whole chapter, independently of pagination. Global
/// matches must cover complete, adjacent Japanese Mode B tokens, so a rule cannot
/// silently replace a substring of a different word, punctuation or whitespace.
/// Local edits win overlaps; within one scope the newest OverrideId wins. Losing
/// rules remain active and can become visible when a winning rule is reversed.
pub fn resolve_segmentation(
    chapter: &ChapterText,
    original: &ChapterAnalysis,
    rules: &[StoredOverride],
) -> Result<Vec<ResolvedSegment>, InvalidAnalysis> {
    original.validate(chapter)?;
    if original.mode != SplitMode::B {
        return Err(InvalidAnalysis);
    }
    let mut ids = BTreeSet::new();
    let mut global_keys = BTreeSet::new();
    let mut local_keys = BTreeSet::new();
    let mut candidates = Vec::new();
    for rule in rules.iter().filter(|rule| rule.active) {
        rule.edit.validate_shape()?;
        if !ids.insert(rule.id) {
            return Err(InvalidAnalysis);
        }
        if rule.edit.anchor.chapter_id == chapter.chapter_id {
            rule.edit.replacement_anchors(chapter)?;
        }
        match rule.edit.scope {
            OverrideScope::ThisOccurrence => {
                let key = (
                    rule.edit.anchor.source_id,
                    rule.edit.anchor.chapter_id,
                    rule.edit.anchor.range.start,
                    rule.edit.anchor.range.end,
                );
                if !local_keys.insert(key) {
                    return Err(InvalidAnalysis);
                }
                if rule.edit.anchor.chapter_id == chapter.chapter_id
                    && rule.edit.anchor.source_id == chapter.source_id
                {
                    let (start, end) =
                        japanese_span(original, rule.edit.anchor).ok_or(InvalidAnalysis)?;
                    candidates.push((start, end, rule));
                }
            }
            OverrideScope::AllOccurrences => {
                if !global_keys.insert(rule.edit.surface.as_str()) {
                    return Err(InvalidAnalysis);
                }
                for (start, token) in original.tokens.iter().enumerate() {
                    if !token.is_japanese {
                        continue;
                    }
                    let start_byte = token
                        .anchor
                        .range
                        .start
                        .byte_index(&chapter.text)
                        .map_err(|_| InvalidAnalysis)?;
                    let Some(end_byte) = start_byte.checked_add(rule.edit.surface.as_str().len())
                    else {
                        return Err(InvalidAnalysis);
                    };
                    if chapter.text.get(start_byte..end_byte) != Some(rule.edit.surface.as_str()) {
                        continue;
                    }
                    let anchor = TextAnchor {
                        range: TextRange {
                            start: token.anchor.range.start,
                            end: TextOffset::at(&chapter.text, end_byte)
                                .map_err(|_| InvalidAnalysis)?,
                        },
                        ..token.anchor
                    };
                    if let Some((_, end)) = japanese_span(original, anchor) {
                        candidates.push((start, end, rule));
                    }
                }
            }
        }
    }
    candidates.sort_by_key(|(start, _, rule)| {
        (
            std::cmp::Reverse(rule.edit.scope == OverrideScope::ThisOccurrence),
            std::cmp::Reverse(rule.id),
            *start,
        )
    });
    let mut selected = BTreeMap::new();
    let mut occupied = vec![false; original.tokens.len()];
    for (start, end, rule) in candidates {
        if occupied[start..end].iter().any(|occupied| *occupied) {
            continue;
        }
        occupied[start..end].fill(true);
        selected.insert(start, (end, rule));
    }
    let mut segments = Vec::new();
    let mut index = 0;
    while index < original.tokens.len() {
        if let Some((end, rule)) = selected.get(&index) {
            let anchor = TextAnchor {
                range: TextRange {
                    start: original.tokens[index].anchor.range.start,
                    end: original.tokens[end - 1].anchor.range.end,
                },
                ..original.tokens[index].anchor
            };
            let occurrence = SegmentationOverride {
                anchor,
                ..rule.edit.clone()
            };
            for anchor in occurrence.replacement_anchors(chapter)? {
                let original_token_index = original.tokens[index..*end]
                    .iter()
                    .position(|token| token.anchor == anchor)
                    .map(|offset| index + offset);
                segments.push(ResolvedSegment {
                    surface: anchor.slice(chapter).map_err(|_| InvalidAnalysis)?.into(),
                    anchor,
                    original_token_index,
                    override_id: Some(rule.id),
                });
            }
            index = *end;
        } else {
            let token = &original.tokens[index];
            segments.push(ResolvedSegment {
                anchor: token.anchor,
                surface: token.surface.clone(),
                original_token_index: Some(index),
                override_id: None,
            });
            index += 1;
        }
    }
    Ok(segments)
}

/// End is exclusive. Only contiguous Japanese tokens can form an editable span.
fn japanese_span(analysis: &ChapterAnalysis, anchor: TextAnchor) -> Option<(usize, usize)> {
    let start = analysis
        .tokens
        .binary_search_by_key(&anchor.range.start, |token| token.anchor.range.start)
        .ok()?;
    let mut next = anchor.range.start;
    for (index, token) in analysis.tokens.iter().enumerate().skip(start) {
        if !token.is_japanese
            || token.anchor.source_id != anchor.source_id
            || token.anchor.chapter_id != anchor.chapter_id
            || token.anchor.range.start != next
            || token.anchor.range.end > anchor.range.end
        {
            return None;
        }
        next = token.anchor.range.end;
        if next == anchor.range.end {
            return Some((start, index + 1));
        }
    }
    None
}
