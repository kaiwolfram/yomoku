use yomoku_core::{
    document::VocabularyId,
    library::WordStatusCounts,
    statistics::{StatusTimelineBuilder, StatusTimelinePoint, VocabularyStatusChange},
    vocabulary::VocabularyStatus::{self, *},
};

fn event(
    event_id: u64,
    vocabulary_id: i64,
    timestamp: i64,
    old_status: Option<VocabularyStatus>,
    new_status: VocabularyStatus,
) -> VocabularyStatusChange {
    VocabularyStatusChange {
        event_id,
        vocabulary_id: VocabularyId::try_from(vocabulary_id).unwrap(),
        occurred_at_unix_ms: timestamp,
        old_status,
        new_status,
    }
}

fn point(
    timestamp: i64,
    [new, learning, familiar, known, ignored]: [u64; 5],
) -> StatusTimelinePoint {
    StatusTimelinePoint {
        occurred_at_unix_ms: timestamp,
        known_dictionary_forms: 0,
        counts: WordStatusCounts {
            new,
            learning,
            familiar,
            known,
            ignored,
        },
    }
}

#[test]
fn empty_history_has_no_synthetic_origin_or_points() {
    assert!(StatusTimelineBuilder::default()
        .finish()
        .unwrap()
        .is_empty());
}

#[test]
fn registration_jumps_and_all_five_statuses_are_absolute_counts() {
    let mut timeline = StatusTimelineBuilder::default();
    for change in [
        event(1, 1, 100, None, New),
        event(2, 2, 100, None, New),
        event(3, 3, 100, None, New),
        event(4, 4, 100, None, New),
        event(5, 5, 100, None, New),
        event(6, 2, 200, Some(New), Learning),
        event(7, 3, 200, Some(New), Familiar),
        event(8, 4, 200, Some(New), Known),
        event(9, 5, 200, Some(New), Ignored),
        event(10, 4, 300, Some(Known), New),
        event(11, 2, 400, Some(Learning), Known),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            point(100, [5, 0, 0, 0, 0]),
            point(200, [1, 1, 1, 1, 1]),
            point(300, [2, 1, 1, 0, 1]),
            point(400, [2, 0, 1, 1, 1]),
        ]
    );
}

#[test]
fn equal_timestamps_coalesce_registration_and_multiple_changes_without_transient_counts() {
    let mut timeline = StatusTimelineBuilder::default();
    for change in [
        event(1, 1, 0, None, New),
        event(2, 1, 0, Some(New), Known),
        event(3, 2, 0, None, New),
        event(4, 1, 0, Some(Known), Ignored),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(timeline.finish().unwrap(), [point(0, [1, 0, 0, 0, 1])]);
}

#[test]
fn clock_regression_preserves_each_entries_causal_order_without_shifting_other_entries() {
    let mut timeline = StatusTimelineBuilder::default();
    for change in [
        event(1, 1, 200, None, New),
        event(2, 1, 100, Some(New), Known),
        event(3, 2, 50, None, New),
        event(4, 1, 300, Some(Known), Learning),
        event(5, 1, 250, Some(Learning), Familiar),
        event(6, 2, 250, Some(New), Ignored),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            point(50, [1, 0, 0, 0, 0]),
            point(200, [1, 0, 0, 1, 0]),
            point(250, [0, 0, 0, 1, 1]),
            point(300, [0, 0, 1, 0, 1]),
        ]
    );
}

#[test]
fn invalid_ids_dates_registrations_noops_and_broken_chains_are_rejected_before_mutation() {
    for invalid in [
        event(0, 2, 200, None, New),
        event(1, 2, 200, None, New),
        event(2, 2, -1, None, New),
        event(2, 2, 200, None, Known),
        event(2, 2, 200, Some(New), Known),
        event(2, 1, 200, None, New),
        event(2, 1, 200, Some(New), New),
        event(2, 1, 200, Some(Known), Learning),
    ] {
        let mut timeline = StatusTimelineBuilder::default();
        timeline.push(event(1, 1, 100, None, New)).unwrap();
        assert!(timeline.push(invalid).is_err(), "{invalid:?}");
        timeline.push(event(2, 1, 200, Some(New), Known)).unwrap();
        assert_eq!(
            timeline.finish().unwrap(),
            [point(100, [1, 0, 0, 0, 0]), point(200, [0, 0, 0, 1, 0])]
        );
    }
}

#[test]
fn sparse_ids_and_maximum_timestamps_remain_exact() {
    let mut timeline = StatusTimelineBuilder::default();
    timeline
        .push(event(50, i64::MAX, i64::MAX, None, New))
        .unwrap();
    timeline
        .push(event(u64::MAX, i64::MAX, i64::MAX, Some(New), Known))
        .unwrap();
    assert_eq!(
        timeline.finish().unwrap(),
        [point(i64::MAX, [0, 0, 0, 1, 0])]
    );
}

fn linked_timeline(links: &[(i64, &str)]) -> StatusTimelineBuilder {
    let mut timeline = StatusTimelineBuilder::default();
    for &(id, form) in links {
        timeline
            .link_dictionary_form(VocabularyId::try_from(id).unwrap(), form.into())
            .unwrap();
    }
    timeline
}

fn known_point(timestamp: i64, counts: [u64; 5], forms: u64) -> StatusTimelinePoint {
    StatusTimelinePoint {
        known_dictionary_forms: forms,
        ..point(timestamp, counts)
    }
}

#[test]
fn shared_forms_count_once_until_the_last_known_surface_is_demoted() {
    let mut timeline = linked_timeline(&[(1, "食べる"), (1, "食べる"), (2, "食べる")]);
    for change in [
        event(1, 1, 100, None, New),
        event(2, 2, 100, None, New),
        event(3, 3, 100, None, New), // Unresolved: no invented surface link.
        event(4, 1, 200, Some(New), Known),
        event(5, 2, 300, Some(New), Known),
        event(6, 3, 300, Some(New), Known),
        event(7, 1, 400, Some(Known), Learning),
        event(8, 2, 500, Some(Known), Familiar),
        event(9, 1, 600, Some(Learning), Known),
        event(10, 1, 700, Some(Known), Ignored),
        event(11, 2, 800, Some(Familiar), Known),
        event(12, 2, 900, Some(Known), New),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            known_point(100, [3, 0, 0, 0, 0], 0),
            known_point(200, [2, 0, 0, 1, 0], 1),
            known_point(300, [0, 0, 0, 3, 0], 1),
            known_point(400, [0, 1, 0, 2, 0], 1),
            known_point(500, [0, 1, 1, 1, 0], 0),
            known_point(600, [0, 0, 1, 2, 0], 1),
            known_point(700, [0, 0, 1, 1, 1], 0),
            known_point(800, [0, 0, 0, 2, 1], 1),
            known_point(900, [1, 0, 0, 1, 1], 0),
        ]
    );
}

#[test]
fn exact_spellings_deduplicate_without_unicode_normalization_or_context_selection() {
    // Multiple resolutions for one vocabulary generation all retain evidence.
    let mut timeline = linked_timeline(&[
        (1, "が"),
        (1, "か\u{3099}"),
        (1, "𠮷"),
        (2, "が"),
        (3, "食べる"),
    ]);
    for change in [
        event(1, 1, 0, None, New),
        event(2, 2, 0, None, New),
        event(3, 3, 0, None, New),
        event(4, 1, 10, Some(New), Known),
        event(5, 2, 10, Some(New), Known),
        event(6, 1, 20, Some(Known), New),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            known_point(0, [3, 0, 0, 0, 0], 0),
            known_point(10, [1, 0, 0, 2, 0], 3),
            known_point(20, [2, 0, 0, 1, 0], 1),
        ]
    );
}

#[test]
fn shared_form_union_uses_dates_instead_of_global_event_order() {
    let mut timeline = linked_timeline(&[(1, "食べる"), (2, "食べる")]);
    for change in [
        event(1, 1, 100, None, New),
        event(2, 2, 100, None, New),
        event(3, 1, 300, Some(New), Known),
        event(4, 2, 200, Some(New), Known),
        event(5, 1, 400, Some(Known), New),
        event(6, 2, 500, Some(Known), New),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            known_point(100, [2, 0, 0, 0, 0], 0),
            known_point(200, [1, 0, 0, 1, 0], 1),
            known_point(300, [0, 0, 0, 2, 0], 1),
            known_point(400, [1, 0, 0, 1, 0], 1),
            known_point(500, [2, 0, 0, 0, 0], 0),
        ]
    );
}

#[test]
fn simultaneous_handoffs_and_regressing_dates_have_no_transient_form_counts() {
    let mut timeline = linked_timeline(&[(1, "食べる"), (2, "食べる")]);
    for change in [
        event(1, 1, 100, None, New),
        event(2, 1, 50, Some(New), Known), // Clamped to registration.
        event(3, 2, 100, None, New),
        event(4, 1, 200, Some(Known), Ignored),
        event(5, 2, 200, Some(New), Known), // Same-time handoff.
        event(6, 2, 150, Some(Known), Learning), // Clamped to 200.
        event(7, 2, 200, Some(Learning), Known),
    ] {
        timeline.push(change).unwrap();
    }
    assert_eq!(
        timeline.finish().unwrap(),
        [
            known_point(100, [1, 0, 0, 1, 0], 1),
            known_point(200, [0, 0, 0, 1, 1], 1),
        ]
    );
}

#[test]
fn links_must_be_nonempty_and_supplied_before_events() {
    let id = VocabularyId::try_from(1).unwrap();
    let mut timeline = StatusTimelineBuilder::default();
    assert!(timeline.link_dictionary_form(id, String::new()).is_err());
    timeline.link_dictionary_form(id, "猫".into()).unwrap();
    timeline.push(event(1, 1, 100, None, New)).unwrap();
    assert!(timeline.link_dictionary_form(id, "犬".into()).is_err());
    timeline.push(event(2, 1, 200, Some(New), Known)).unwrap();
    assert_eq!(
        timeline.finish().unwrap(),
        [
            known_point(100, [1, 0, 0, 0, 0], 0),
            known_point(200, [0, 0, 0, 1, 0], 1)
        ]
    );
}
