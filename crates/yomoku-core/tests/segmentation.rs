use yomoku_core::{
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode, StoredOverride,
    },
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    segmentation::{resolve_segmentation, segmentation_from_snapshot},
    vocabulary::Surface,
};

fn chapter(text: &str) -> ChapterText {
    ChapterText {
        source_id: 1.try_into().unwrap(),
        chapter_id: 2.try_into().unwrap(),
        text: text.into(),
    }
}

fn anchor(chapter: &ChapterText, start: usize, end: usize) -> TextAnchor {
    TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: TextRange {
            start: TextOffset::at(&chapter.text, start).unwrap(),
            end: TextOffset::at(&chapter.text, end).unwrap(),
        },
    }
}

fn analysis(chapter: &ChapterText, parts: &[(&str, bool)]) -> ChapterAnalysis {
    let mut offset = 0;
    let tokens = parts
        .iter()
        .map(|(surface, japanese)| {
            let start = offset + chapter.text[offset..].find(surface).unwrap();
            offset = start + surface.len();
            AnalyzedToken {
                anchor: anchor(chapter, start, offset),
                surface: (*surface).into(),
                is_japanese: *japanese,
                reading: japanese.then(|| "ヨミ".into()),
                dictionary_form: japanese.then(|| Surface::new(*surface).unwrap()),
            }
        })
        .collect();
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::B,
        tokens,
    }
}

fn rule(
    chapter: &ChapterText,
    id: i64,
    start: usize,
    surface: &str,
    parts: &[&str],
    scope: OverrideScope,
) -> StoredOverride {
    StoredOverride {
        id: id.try_into().unwrap(),
        active: true,
        edit: SegmentationOverride {
            anchor: anchor(chapter, start, start + surface.len()),
            scope,
            surface: Surface::new(surface).unwrap(),
            parts: parts
                .iter()
                .map(|part| Surface::new(*part).unwrap())
                .collect(),
        },
    }
}

#[test]
fn global_split_and_manual_conjugation_merge_preserve_exact_anchors_and_original_metadata() {
    let chapter = chapter("国家公務員。国家公務員 食べた");
    let original = analysis(
        &chapter,
        &[
            ("国家", true),
            ("公務員", true),
            ("。", false),
            ("国家", true),
            ("公務員", true),
            ("食べ", true),
            ("た", true),
        ],
    );
    let before = original.clone();
    let rules = [
        rule(
            &chapter,
            1,
            0,
            "国家公務員",
            &["国家", "公務", "員"],
            OverrideScope::AllOccurrences,
        ),
        rule(
            &chapter,
            2,
            chapter.text.find("食べ").unwrap(),
            "食べた",
            &["食べた"],
            OverrideScope::AllOccurrences,
        ),
    ];
    let resolved = resolve_segmentation(&chapter, &original, &rules).unwrap();
    assert_eq!(
        resolved
            .iter()
            .map(|segment| segment.surface.as_str())
            .collect::<Vec<_>>(),
        ["国家", "公務", "員", "。", "国家", "公務", "員", "食べた"]
    );
    assert_eq!(resolved[0].original_token_index, Some(0));
    assert_eq!(resolved[1].original_token_index, None);
    assert_eq!(resolved[3].original_token_index, Some(2));
    assert_eq!(resolved[3].override_id, None);
    assert_eq!(resolved[7].original_token_index, None);
    for segment in resolved {
        assert_eq!(segment.anchor.slice(&chapter).unwrap(), segment.surface);
    }
    assert_eq!(original, before);
}

#[test]
fn local_wins_global_and_newest_wins_overlaps_independently_of_input_order() {
    let chapter = chapter("猫犬鳥 猫犬鳥");
    let original = analysis(
        &chapter,
        &[
            ("猫", true),
            ("犬", true),
            ("鳥", true),
            ("猫", true),
            ("犬", true),
            ("鳥", true),
        ],
    );
    let mut rules = vec![
        rule(
            &chapter,
            2,
            0,
            "猫犬鳥",
            &["猫犬鳥"],
            OverrideScope::AllOccurrences,
        ),
        rule(
            &chapter,
            3,
            3,
            "犬鳥",
            &["犬鳥"],
            OverrideScope::AllOccurrences,
        ),
        rule(
            &chapter,
            1,
            0,
            "猫犬",
            &["猫犬"],
            OverrideScope::ThisOccurrence,
        ),
    ];
    let resolved = resolve_segmentation(&chapter, &original, &rules).unwrap();
    assert_eq!(
        resolved
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["猫犬", "鳥", "猫", "犬鳥"]
    );
    rules.reverse();
    assert_eq!(
        resolve_segmentation(&chapter, &original, &rules).unwrap(),
        resolved
    );
    rules
        .iter_mut()
        .find(|r| i64::from(r.id) == 1)
        .unwrap()
        .active = false;
    let reversed = resolve_segmentation(&chapter, &original, &rules).unwrap();
    assert_eq!(
        reversed
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["猫", "犬鳥", "猫", "犬鳥"]
    );
    rules
        .iter_mut()
        .find(|r| i64::from(r.id) == 3)
        .unwrap()
        .active = false;
    let reversed = resolve_segmentation(&chapter, &original, &rules).unwrap();
    assert_eq!(
        reversed
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["猫犬鳥", "猫犬鳥"]
    );
}

#[test]
fn self_overlapping_global_matches_choose_leftmost_without_rewriting_twice() {
    let chapter = chapter("猫猫猫");
    let original = analysis(&chapter, &[("猫", true), ("猫", true), ("猫", true)]);
    let edit = rule(
        &chapter,
        1,
        0,
        "猫猫",
        &["猫猫"],
        OverrideScope::AllOccurrences,
    );
    let resolved = resolve_segmentation(&chapter, &original, &[edit]).unwrap();
    assert_eq!(
        resolved
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["猫猫", "猫"]
    );
}

#[test]
fn global_matches_cross_documents_but_not_substrings_non_japanese_or_gaps() {
    let origin = chapter("猫犬");
    let mut target = chapter("猫犬 猫犬 猫 犬 猫。犬 猫犬鳥");
    target.source_id = 10.try_into().unwrap();
    target.chapter_id = 11.try_into().unwrap();
    let original = analysis(
        &target,
        &[
            ("猫", true),
            ("犬", true),
            ("猫犬", false),
            ("猫", true),
            ("犬", true),
            ("猫", true),
            ("。", false),
            ("犬", true),
            ("猫犬鳥", true),
        ],
    );
    let global = rule(
        &origin,
        1,
        0,
        "猫犬",
        &["猫犬"],
        OverrideScope::AllOccurrences,
    );
    let local = rule(
        &origin,
        2,
        0,
        "猫犬",
        &["猫", "犬"],
        OverrideScope::ThisOccurrence,
    );
    let resolved = resolve_segmentation(&target, &original, &[global, local]).unwrap();
    assert_eq!(
        resolved.iter().filter(|s| s.override_id.is_some()).count(),
        1
    );
    assert_eq!(resolved[0].surface, "猫犬");
    assert_eq!(resolved.last().unwrap().surface, "猫犬鳥");
}

#[test]
fn literal_supplementary_combining_and_nul_surfaces_never_normalize() {
    let chapter = chapter("𠮷か\u{3099}\0 が 𠮷か\u{3099}\0");
    let original = analysis(
        &chapter,
        &[
            ("𠮷か\u{3099}\0", true),
            ("が", true),
            ("𠮷か\u{3099}\0", true),
        ],
    );
    let edit = rule(
        &chapter,
        1,
        0,
        "𠮷か\u{3099}\0",
        &["𠮷", "か\u{3099}\0"],
        OverrideScope::AllOccurrences,
    );
    let resolved = resolve_segmentation(&chapter, &original, &[edit]).unwrap();
    assert_eq!(
        resolved
            .iter()
            .map(|s| s.surface.as_str())
            .collect::<Vec<_>>(),
        ["𠮷", "か\u{3099}\0", "が", "𠮷", "か\u{3099}\0"]
    );
    for segment in resolved {
        assert_eq!(segment.anchor.slice(&chapter).unwrap(), segment.surface);
    }
}

#[test]
fn snapshot_choices_require_same_profile_complete_japanese_spans_and_original_b() {
    let chapter = chapter("国家公務員。食べた");
    let original = analysis(
        &chapter,
        &[
            ("国家", true),
            ("公務員", true),
            ("。", false),
            ("食べ", true),
            ("た", true),
        ],
    );
    let mut alternate = analysis(
        &chapter,
        &[
            ("国家", true),
            ("公務", true),
            ("員", true),
            ("。", false),
            ("食べ", true),
            ("た", true),
        ],
    );
    alternate.mode = SplitMode::A;
    let selected = anchor(&chapter, 0, "国家公務員".len());
    let split = segmentation_from_snapshot(
        &chapter,
        &original,
        &alternate,
        selected,
        OverrideScope::default(),
    )
    .unwrap();
    assert_eq!(
        split.parts.iter().map(Surface::as_str).collect::<Vec<_>>(),
        ["国家", "公務", "員"]
    );
    alternate.mode = SplitMode::C;
    alternate.tokens.splice(
        0..3,
        [AnalyzedToken {
            anchor: selected,
            surface: "国家公務員".into(),
            is_japanese: true,
            reading: Some("コッカコウムイン".into()),
            dictionary_form: Some(Surface::new("国家公務員").unwrap()),
        }],
    );
    let merged = segmentation_from_snapshot(
        &chapter,
        &original,
        &alternate,
        selected,
        OverrideScope::ThisOccurrence,
    )
    .unwrap();
    assert_eq!(merged.parts, std::slice::from_ref(&merged.surface));
    assert!(segmentation_from_snapshot(
        &chapter,
        &original,
        &alternate,
        anchor(&chapter, 0, 6),
        OverrideScope::default()
    )
    .is_err());
    assert!(segmentation_from_snapshot(
        &chapter,
        &original,
        &alternate,
        anchor(&chapter, 0, chapter.text.len()),
        OverrideScope::default()
    )
    .is_err());
    alternate.profile.dictionary_revision = "new-full@2".into();
    assert!(segmentation_from_snapshot(
        &chapter,
        &original,
        &alternate,
        selected,
        OverrideScope::default()
    )
    .is_err());
    let back = segmentation_from_snapshot(
        &chapter,
        &original,
        &original,
        selected,
        OverrideScope::default(),
    )
    .unwrap();
    assert_eq!(
        back.parts.iter().map(Surface::as_str).collect::<Vec<_>>(),
        ["国家", "公務員"]
    );
}

#[test]
fn invalid_original_or_active_rules_fail_instead_of_partially_applying() {
    let chapter = chapter("猫犬");
    let original = analysis(&chapter, &[("猫", true), ("犬", true)]);
    let edit = rule(
        &chapter,
        1,
        0,
        "猫犬",
        &["猫犬"],
        OverrideScope::ThisOccurrence,
    );
    let mut bad = edit.clone();
    bad.edit.parts = vec![Surface::new("犬猫").unwrap()];
    assert!(resolve_segmentation(&chapter, &original, &[bad]).is_err());
    let mut bad = edit.clone();
    bad.edit.anchor.source_id = 99.try_into().unwrap();
    assert!(resolve_segmentation(&chapter, &original, &[bad]).is_err());
    let mut bad = edit.clone();
    bad.edit.anchor.range.start = 1.try_into().unwrap();
    assert!(resolve_segmentation(&chapter, &original, &[bad]).is_err());
    assert!(resolve_segmentation(&chapter, &original, &[edit.clone(), edit.clone()]).is_err());
    let mut duplicate = edit.clone();
    duplicate.id = 2.try_into().unwrap();
    assert!(resolve_segmentation(&chapter, &original, &[edit.clone(), duplicate]).is_err());
    let mut not_b = original.clone();
    not_b.mode = SplitMode::A;
    assert!(resolve_segmentation(&chapter, &not_b, &[]).is_err());
    let mut bad_original = original;
    bad_original.tokens[0].surface = "鳥".into();
    assert!(resolve_segmentation(&chapter, &bad_original, &[edit]).is_err());
}

#[test]
fn empty_and_whitespace_only_chapters_remain_empty() {
    for text in ["", " \r\n\t"] {
        let chapter = chapter(text);
        let original = analysis(&chapter, &[]);
        assert!(resolve_segmentation(&chapter, &original, &[])
            .unwrap()
            .is_empty());
    }
}
