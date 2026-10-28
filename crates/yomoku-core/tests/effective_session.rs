use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode, StoredOverride,
    },
    document::{ChapterText, TextAnchor, TextRange},
    effective_reader::{
        EffectiveReaderChapter, EffectiveReaderTokenSnapshot, EffectiveTokenEligibility,
    },
    effective_session::{EffectiveReaderSelection, EffectiveReaderSession},
    reader::{ReaderAnalysis, ReaderTokenKey, ReaderVocabulary},
    vocabulary::{Surface, VocabularyStatus},
};

#[path = "effective_session/completions.rs"]
mod completions;
#[path = "effective_session/overrides.rs"]
mod overrides;
#[path = "effective_session/ruby.rs"]
mod ruby;
#[path = "effective_session/statuses.rs"]
mod statuses;
#[path = "effective_session/words.rs"]
mod words;

fn snapshot() -> EffectiveReaderTokenSnapshot {
    snapshot_with_manual_split(false)
}

fn snapshot_with_manual_split(manual: bool) -> EffectiveReaderTokenSnapshot {
    let chapter = ChapterText {
        source_id: 2.try_into().unwrap(),
        chapter_id: 3.try_into().unwrap(),
        text: "𠮷か\u{3099}\0。猫犬".into(),
    };
    let mut start = 0;
    let original = ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::B,
        tokens: ["𠮷か\u{3099}\0", "。", "猫犬"]
            .into_iter()
            .map(|surface| {
                let end = start + surface.len();
                let token = AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: range(start, end),
                    },
                    surface: surface.into(),
                    is_japanese: surface != "。",
                    reading: Some(format!("読み{surface}")),
                    dictionary_form: Some(Surface::new(surface).unwrap()),
                };
                start = end;
                token
            })
            .collect(),
    };
    let rules = if manual {
        vec![StoredOverride {
            id: 5.try_into().unwrap(),
            active: true,
            edit: SegmentationOverride {
                scope: OverrideScope::AllOccurrences,
                anchor: original.tokens[2].anchor,
                surface: Surface::new("猫犬").unwrap(),
                parts: vec![Surface::new("猫").unwrap(), Surface::new("犬").unwrap()],
            },
        }]
    } else {
        Vec::new()
    };
    let resolved =
        EffectiveReaderChapter::resolve(&chapter, 4.try_into().unwrap(), &original, &[], &rules)
            .unwrap();
    // Clip both ends; selectable occurrences retain their complete identity.
    let page = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: range(4, chapter.text.len() - 3),
    };
    EffectiveReaderTokenSnapshot {
        document_id: 1.try_into().unwrap(),
        page,
        tokens: resolved.tokens_on_page(page).unwrap(),
        segmentation: Some(resolved.into_segmentation()),
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: (start as i64).try_into().unwrap(),
        end: (end as i64).try_into().unwrap(),
    }
}

fn base(snapshot: &EffectiveReaderTokenSnapshot) -> ReaderTokenKey {
    ReaderTokenKey {
        document_id: snapshot.document_id,
        page: snapshot.page,
        analysis: snapshot
            .segmentation
            .as_ref()
            .map_or(ReaderAnalysis::Missing, |segmentation| {
                ReaderAnalysis::Available(segmentation.original_analysis_id)
            }),
    }
}

#[test]
fn compact_page_preserves_literal_visible_data_and_authenticates_only_visible_segments() {
    let snapshot = snapshot();
    let full_key = snapshot.key().unwrap();
    let expected = snapshot.tokens.clone();
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base(&snapshot)).unwrap();
    assert_eq!(request.base(), base(&snapshot));
    assert!(session.accept(request, snapshot));
    let page = session.page().unwrap();
    assert_eq!(session.resolve(page.identity), Some(&full_key));
    assert_eq!(page.tokens.len(), 2);
    for (token, expected) in page.tokens.iter().zip(&expected) {
        assert_eq!(token.segment_index, expected.segment_index);
        assert_eq!(token.anchor, expected.anchor);
        assert_eq!(token.display_range, expected.display_range);
        assert_eq!(token.surface, expected.surface);
        assert_eq!(token.eligibility, expected.eligibility);
        let metadata = expected.metadata.as_ref().unwrap();
        assert_eq!(token.reading, metadata.reading);
        assert_eq!(
            token.dictionary_form.as_deref(),
            metadata.dictionary_form.as_ref().map(Surface::as_str)
        );
    }
    for (index, valid) in [
        (0, true),
        (1, false),
        (2, true),
        (3, false),
        (u64::MAX, false),
    ] {
        assert_eq!(
            session
                .resolve_selection(EffectiveReaderSelection {
                    page: page.identity,
                    segment_index: index,
                })
                .is_some(),
            valid
        );
    }
    for field in 0..8 {
        let mut foreign = page.identity;
        match field {
            0 => foreign.revision = 0,
            1 => foreign.revision += 1,
            2 => foreign.base.document_id = 99.try_into().unwrap(),
            3 => foreign.base.page.source_id = 99.try_into().unwrap(),
            4 => foreign.base.page.chapter_id = 99.try_into().unwrap(),
            5 => foreign.base.page.range.start = 0.try_into().unwrap(),
            6 => foreign.base.page.range.end = 99.try_into().unwrap(),
            _ => foreign.base.analysis = ReaderAnalysis::Missing,
        }
        assert!(session.resolve(foreign).is_none());
    }
}

#[test]
fn late_duplicate_failed_and_invalidated_loads_cannot_resurrect_a_page() {
    let snapshot = snapshot();
    let base = base(&snapshot);
    let mut session = EffectiveReaderSession::default();
    let first = session.begin_load(base).unwrap();
    let second = session.begin_load(base).unwrap();
    assert_ne!(first, second);
    assert!(!session.accept(first, snapshot.clone()));
    assert!(!session.fail(first));
    assert!(session.page().is_none());
    assert!(session.accept(second, snapshot.clone()));
    let old_identity = session.page().unwrap().identity;
    assert!(!session.accept(second, snapshot.clone()));
    let third = session.begin_load(base).unwrap();
    assert!(session.page().is_none());
    assert!(session.resolve(old_identity).is_none());
    assert!(session.fail(third));
    assert!(!session.accept(third, snapshot.clone()));
    let fourth = session.begin_load(base).unwrap();
    session.invalidate();
    assert!(!session.accept(fourth, snapshot.clone()));
    let fifth = session.begin_load(base).unwrap();
    assert!(session.accept(fifth, snapshot));
    assert_ne!(session.page().unwrap().identity, old_identity);
    assert!(session.resolve(old_identity).is_none());
}

#[test]
fn foreign_results_do_not_consume_current_attempt() {
    let snapshot = snapshot();
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base(&snapshot)).unwrap();
    for field in 0..5 {
        let mut foreign = snapshot.clone();
        match field {
            0 => foreign.document_id = 99.try_into().unwrap(),
            1 => foreign.page.source_id = 99.try_into().unwrap(),
            2 => foreign.page.range = range(0, 3),
            3 => foreign.segmentation = None,
            _ => {
                foreign.segmentation.as_mut().unwrap().original_analysis_id = 99.try_into().unwrap()
            }
        }
        assert!(!session.accept(request, foreign));
        assert!(session.page().is_none());
    }
    assert!(session.accept(request, snapshot));
}

#[test]
fn missing_analysis_is_plain_and_unselectable_without_fallback() {
    let mut snapshot = snapshot();
    let mut session = EffectiveReaderSession::default();
    snapshot.segmentation = None;
    let request = session.begin_load(base(&snapshot)).unwrap();
    // A missing analysis cannot smuggle selectable tokens into the plain page.
    assert!(!session.accept(request, snapshot.clone()));
    snapshot.tokens.clear();
    assert!(session.accept(request, snapshot));
    let page = session.page().unwrap();
    assert!(page.tokens.is_empty());
    assert!(session.resolve(page.identity).is_none());
    assert!(session
        .resolve_selection(EffectiveReaderSelection {
            page: page.identity,
            segment_index: 0,
        })
        .is_none());
}

#[test]
fn vocabulary_refresh_preserves_full_provenance_but_retires_old_callbacks() {
    let mut snapshot = snapshot();
    let mut session = EffectiveReaderSession::default();
    let base = base(&snapshot);
    let request = session.begin_load(base).unwrap();
    assert!(session.accept(request, snapshot.clone()));
    let old_identity = session.page().unwrap().identity;
    let full_key = session.resolve(old_identity).unwrap().clone();
    for status in [
        Some(VocabularyStatus::New),
        Some(VocabularyStatus::Learning),
        Some(VocabularyStatus::Familiar),
        Some(VocabularyStatus::Known),
        Some(VocabularyStatus::Ignored),
        None,
    ] {
        snapshot.tokens[0].vocabulary = status.map(|status| ReaderVocabulary {
            id: 5.try_into().unwrap(),
            status,
        });
        let request = session.begin_load(base).unwrap();
        assert!(session.accept(request, snapshot.clone()));
        let page = session.page().unwrap();
        assert_eq!(page.tokens[0].vocabulary, snapshot.tokens[0].vocabulary);
        assert_eq!(session.resolve(page.identity), Some(&full_key));
        assert!(session.resolve(old_identity).is_none());
    }
}

#[test]
fn accepted_command_key_survives_page_invalidation_and_later_metadata_changes() {
    let mut snapshot = snapshot();
    let mut session = EffectiveReaderSession::default();
    let base = base(&snapshot);
    let request = session.begin_load(base).unwrap();
    assert!(session.accept(request, snapshot.clone()));
    let old_identity = session.page().unwrap().identity;
    // A future pending command owns this independently across drain/write retries.
    let frozen = session.resolve(old_identity).unwrap().clone();
    session.invalidate();
    assert!(session.resolve(old_identity).is_none());
    let metadata = snapshot.segmentation.as_mut().unwrap().segments[0]
        .metadata
        .as_mut()
        .unwrap();
    metadata.reading = Some("新しい読み𠮷\0".into());
    snapshot.tokens[0].metadata = Some(metadata.clone());
    let request = session.begin_load(base).unwrap();
    assert!(session.accept(request, snapshot));
    let current = session.resolve(session.page().unwrap().identity).unwrap();
    assert_ne!(current, &frozen);
    assert_eq!(
        frozen.segmentation.segments[0]
            .metadata
            .as_ref()
            .unwrap()
            .reading
            .as_deref(),
        Some("読み𠮷か\u{3099}\0")
    );
}

#[test]
fn unresolved_manual_parts_keep_eligibility_without_borrowing_original_b_metadata() {
    let snapshot = snapshot_with_manual_split(true);
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base(&snapshot)).unwrap();
    assert!(session.accept(request, snapshot));
    let page = session.page().unwrap();
    let manual = &page.tokens[1];
    assert_eq!(manual.surface, "猫");
    assert_eq!(
        manual.eligibility,
        EffectiveTokenEligibility::ManualJapaneseSpan
    );
    assert!(manual.reading.is_none());
    assert!(manual.dictionary_form.is_none());
    assert!(manual.vocabulary.is_none());
    assert!(session
        .resolve_selection(EffectiveReaderSelection {
            page: page.identity,
            segment_index: manual.segment_index,
        })
        .is_some());
    // The other manual part is outside this clipped page.
    assert!(session
        .resolve_selection(EffectiveReaderSelection {
            page: page.identity,
            segment_index: manual.segment_index + 1,
        })
        .is_none());
}

#[test]
fn accepted_commands_share_one_key_and_survive_invalidation_without_authorizing_new_callbacks() {
    let snapshot = snapshot();
    let base = base(&snapshot);
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base).unwrap();
    assert!(session.accept(request, snapshot));
    let identity = session.page().unwrap().identity;
    let selection = EffectiveReaderSelection {
        page: identity,
        segment_index: 0,
    };
    let page = session.freeze_page(identity).unwrap();
    let word = session.freeze_selection(selection).unwrap();
    let retry = word.clone();
    assert!(std::ptr::eq(page.key(), word.key()));
    assert!(std::ptr::eq(word.key(), retry.key()));
    assert!(std::ptr::eq(session.resolve(identity).unwrap(), page.key()));
    assert_eq!(word.selection(), selection);
    assert_eq!(page.identity(), identity);
    for index in [1, 3, u64::MAX] {
        assert!(session
            .freeze_selection(EffectiveReaderSelection {
                segment_index: index,
                ..selection
            })
            .is_none());
    }
    session.invalidate();
    assert!(session.freeze_page(identity).is_none());
    assert!(session.freeze_selection(selection).is_none());
    assert_eq!(word.key(), page.key());
    assert!(std::ptr::eq(word.key(), retry.key()));
    let request = session.begin_load(base).unwrap();
    assert!(session.accept(request, snapshot_with_manual_split(true)));
    assert!(session.freeze_page(identity).is_none());
    assert!(session.freeze_selection(selection).is_none());
    assert_ne!(
        session.resolve(session.page().unwrap().identity).unwrap(),
        page.key()
    );
}
