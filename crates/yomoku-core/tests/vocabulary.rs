use std::collections::HashSet;
use yomoku_core::vocabulary::{
    register, status_after_completion, Registration, Surface, VocabularyStatus,
};

#[test]
fn inflections_are_distinct_but_identical_spellings_share_identity() {
    let surfaces: HashSet<_> = ["食べる", "食べた", "食べます", "食べる"]
        .into_iter()
        .map(|text| Surface::new(text).unwrap())
        .collect();
    assert_eq!(surfaces.len(), 3);
    // Different readings of 生 must still use this same surface identity.
    assert_eq!(Surface::new("生"), Surface::new("生"));
}

#[test]
fn surface_preserves_exact_input_without_unicode_normalization() {
    let composed = Surface::new("が").unwrap();
    let decomposed = Surface::new("か\u{3099}").unwrap();
    assert_ne!(composed, decomposed);
    assert_eq!(Surface::new(" 食べる ").unwrap().as_str(), " 食べる ");
    assert_eq!(composed.as_str(), "が");
}

#[test]
fn empty_and_whitespace_only_surfaces_are_rejected() {
    for text in ["", " ", "\n\t", "\u{3000}"] {
        assert!(Surface::new(text).is_err());
    }
}

#[test]
fn registration_creates_new_without_overwriting_any_existing_status() {
    assert_eq!(register(None), Registration::CreateNew);
    for status in statuses() {
        assert_eq!(
            register(Some(status)),
            Registration::AlreadyRegistered(status)
        );
    }
}

#[test]
fn completion_only_promotes_new_to_known() {
    for status in statuses() {
        let expected = if status == VocabularyStatus::New {
            VocabularyStatus::Known
        } else {
            status
        };
        assert_eq!(status_after_completion(status), expected);
    }
}

fn statuses() -> [VocabularyStatus; 5] {
    use VocabularyStatus::*;
    [New, Learning, Familiar, Known, Ignored]
}
