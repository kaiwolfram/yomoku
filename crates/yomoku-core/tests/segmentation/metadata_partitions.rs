//! Compare provenance with a deliberately simple full-scan oracle. These keys
//! authenticate durable commands, so optimization must preserve every field.

use super::*;
use yomoku_core::{document::AnalysisId, segmentation::SegmentMetadata};

fn parts(mask: usize) -> Vec<String> {
    let units = ["𠮷", "か\u{3099}", "\0", "猫"];
    let mut parts = Vec::new();
    let mut part = String::new();
    for (index, unit) in units.iter().enumerate() {
        part.push_str(unit);
        if index == units.len() - 1 || mask & (1 << index) != 0 {
            parts.push(std::mem::take(&mut part));
        }
    }
    parts
}

fn snapshot(chapter: &ChapterText, mask: usize, mode: SplitMode) -> ChapterAnalysis {
    let parts = parts(mask);
    let tokens = parts
        .iter()
        .map(|part| (part.as_str(), true))
        .chain([("。", false)])
        .chain(parts.iter().map(|part| (part.as_str(), true)))
        .collect::<Vec<_>>();
    let mut snapshot = analysis(chapter, &tokens);
    snapshot.mode = mode;
    for (index, token) in snapshot.tokens.iter_mut().enumerate() {
        // Exact spans may disagree between modes and repeated occurrences.
        // Some resolved Japanese tokens intentionally lack reading/lemma.
        if mode != SplitMode::B {
            token.is_japanese = index % 3 != 0;
        }
        token.reading = (index % 2 == 0).then(|| format!("{mode:?}-{index}"));
        token.dictionary_form =
            (index % 2 != 0).then(|| Surface::new(format!("辞書{mode:?}-{index}")).unwrap());
    }
    snapshot
}

fn reference_metadata(
    anchor: TextAnchor,
    snapshots: &[(AnalysisId, &ChapterAnalysis)],
) -> Option<SegmentMetadata> {
    for (id, snapshot) in snapshots {
        for (index, token) in snapshot.tokens.iter().enumerate() {
            if token.anchor == anchor {
                return Some(SegmentMetadata {
                    analysis_id: *id,
                    mode: snapshot.mode,
                    token_index: index,
                    is_japanese: token.is_japanese,
                    reading: token.reading.clone(),
                    dictionary_form: token.dictionary_form.clone(),
                });
            }
        }
    }
    None
}

#[test]
fn effective_metadata_matches_exact_span_oracle_for_every_four_unit_partition() {
    let surface = "𠮷か\u{3099}\0猫";
    let chapter = chapter(&format!("{surface}。 {surface}"));
    let original_id = 10.try_into().unwrap();
    let a_id = 11.try_into().unwrap();
    let c_id = 12.try_into().unwrap();
    let mut cases = 0;
    for b_mask in 0..8 {
        let original = snapshot(&chapter, b_mask, SplitMode::B);
        for a_mask in 0..8 {
            let a = snapshot(&chapter, a_mask, SplitMode::A);
            for c_mask in 0..8 {
                let c = snapshot(&chapter, c_mask, SplitMode::C);
                // C arrives first, but exact A must still win over C. Matching
                // starts without matching ends must fall through to the next mode.
                let alternates = [(c_id, c.clone()), (a_id, a.clone())];
                for manual_mask in 0..8 {
                    let parts = parts(manual_mask);
                    let parts = parts.iter().map(String::as_str).collect::<Vec<_>>();
                    let rules = [rule(
                        &chapter,
                        1,
                        0,
                        surface,
                        &parts,
                        OverrideScope::AllOccurrences,
                    )];
                    let resolved = resolve_effective_segmentation(
                        &chapter,
                        original_id,
                        &original,
                        &alternates,
                        &rules,
                    )
                    .unwrap();
                    let boundaries = resolve_segmentation(&chapter, &original, &rules).unwrap();
                    assert_eq!(resolved.segments.len(), boundaries.len());
                    for (segment, boundary) in resolved.segments.iter().zip(boundaries) {
                        assert_eq!(segment.boundary, boundary);
                        assert_eq!(
                            segment.metadata,
                            reference_metadata(
                                boundary.anchor,
                                &[(original_id, &original), (a_id, &a), (c_id, &c)],
                            ),
                            "B={b_mask} A={a_mask} C={c_mask} manual={manual_mask}"
                        );
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 4096);
}
