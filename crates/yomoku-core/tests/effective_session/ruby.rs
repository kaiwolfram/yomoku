use super::*;
use yomoku_core::{pagination::ReadingPosition, reader::ReaderPage, reader_ruby::ReaderRuby};

fn page(snapshot: &EffectiveReaderTokenSnapshot) -> ReaderPage {
    ReaderPage {
        document_id: snapshot.document_id,
        title: "本".into(),
        finished_at_least_once: false,
        text: "か\u{3099}\0。猫".into(),
        anchor: snapshot.page,
        position: ReadingPosition {
            source_id: snapshot.page.source_id,
            chapter_id: snapshot.page.chapter_id,
            offset: snapshot.page.range.start,
        },
        page_number: 1,
        total_pages: 2,
        formatting: vec![],
        images: vec![],
        source_ruby: vec![ReaderRuby {
            range: range(0, 3),
            reading: "source𠮷か\u{3099}\0".into(),
        }],
    }
}

#[test]
fn retained_ruby_uses_exact_metadata_and_source_without_status_or_manual_fallbacks() {
    for manual in [false, true] {
        for status in [
            None,
            Some(VocabularyStatus::New),
            Some(VocabularyStatus::Known),
        ] {
            let mut snapshot = snapshot_with_manual_split(manual);
            let mut page = page(&snapshot);
            let analysis = base(&snapshot).analysis;
            for token in &mut snapshot.tokens {
                token.vocabulary = status.map(|status| ReaderVocabulary {
                    id: 55.try_into().unwrap(),
                    status,
                });
            }
            let mut session = EffectiveReaderSession::default();
            let request = session.begin_load(base(&snapshot)).unwrap();
            assert!(session.accept(request, snapshot));
            let retained = session.page().unwrap().clone();
            let mut expected = page.source_ruby.clone();
            if !manual {
                expected.push(ReaderRuby {
                    range: range(10, 13),
                    reading: "読み猫犬".into(),
                });
            }
            assert_eq!(session.furigana(&page, analysis).unwrap(), expected);
            // Without source, the clipped crossing token keeps its whole reading.
            page.source_ruby.clear();
            expected.insert(
                0,
                ReaderRuby {
                    range: range(0, 7),
                    reading: "読み𠮷か\u{3099}\0".into(),
                },
            );
            expected.remove(1); // source base no longer present
            assert_eq!(session.furigana(&page, analysis).unwrap(), expected);
            assert_eq!(session.page(), Some(&retained));
        }
    }
}

#[test]
fn pending_failed_foreign_missing_and_invalidated_pages_leave_source_ruby_usable() {
    let snapshot = snapshot();
    let page = page(&snapshot);
    let key = base(&snapshot);
    let mut session = EffectiveReaderSession::default();
    let source_only = |session: &EffectiveReaderSession| {
        assert_eq!(
            session.furigana(&page, key.analysis).unwrap(),
            page.source_ruby
        );
    };
    source_only(&session);
    let request = session.begin_load(key).unwrap();
    source_only(&session);
    assert!(session.fail(request));
    source_only(&session);
    let request = session.begin_load(key).unwrap();
    assert!(session.accept(request, snapshot.clone()));
    assert!(session.furigana(&page, key.analysis).unwrap().len() > 1);
    for field in 0..5 {
        let mut foreign = page.clone();
        let mut analysis = key.analysis;
        match field {
            0 => foreign.document_id = 99.try_into().unwrap(),
            1 => foreign.anchor.source_id = 99.try_into().unwrap(),
            2 => foreign.anchor.chapter_id = 99.try_into().unwrap(),
            3 => analysis = ReaderAnalysis::Available(99.try_into().unwrap()),
            _ => analysis = ReaderAnalysis::Missing,
        }
        assert_eq!(
            session.furigana(&foreign, analysis).unwrap(),
            page.source_ruby
        );
    }
    let refresh = session.begin_load(key).unwrap();
    source_only(&session);
    assert!(!session.accept(request, snapshot));
    source_only(&session);
    assert!(!session.accept(
        refresh,
        EffectiveReaderTokenSnapshot {
            original_boundaries: vec![],
            document_id: key.document_id,
            page: key.page,
            segmentation: None,
            tokens: vec![],
        }
    )); // available B request cannot accept Missing
    assert!(session.fail(refresh));
    let missing = ReaderTokenKey {
        analysis: ReaderAnalysis::Missing,
        ..key
    };
    let request = session.begin_load(missing).unwrap();
    assert!(session.accept(
        request,
        EffectiveReaderTokenSnapshot {
            original_boundaries: vec![],
            document_id: key.document_id,
            page: key.page,
            segmentation: None,
            tokens: vec![],
        }
    ));
    assert_eq!(
        session.furigana(&page, ReaderAnalysis::Missing).unwrap(),
        page.source_ruby
    );
    session.invalidate();
    source_only(&session);
}

#[test]
fn malformed_effective_text_or_ranges_discard_all_fallbacks_and_invalid_source_errors() {
    let snapshot = snapshot();
    let page = page(&snapshot);
    let key = base(&snapshot);
    for field in 0..12 {
        let mut invalid = snapshot.clone();
        let first_index = invalid.tokens[0].segment_index;
        let token = &mut invalid.tokens[1];
        match field {
            0 => token.surface.push('別'),
            1 => token.anchor.source_id = 99.try_into().unwrap(),
            2 => token.anchor.chapter_id = 99.try_into().unwrap(),
            3 => token.display_range = range(11, 13), // inside scalar
            4 => token.display_range = range(10, 14),
            5 => token.display_range = range(10, 10),
            6 => token.segment_index = first_index,
            7 => token.metadata.as_mut().unwrap().reading = Some("\t　".into()),
            8 => token.anchor.range = range(14, 13),
            9 => token.anchor.range.end = i64::MAX.try_into().unwrap(),
            10 => invalid.tokens.reverse(),
            _ => invalid.tokens[0].display_range.end = 8.try_into().unwrap(),
        }
        let mut session = EffectiveReaderSession::default();
        let request = session.begin_load(key).unwrap();
        assert!(session.accept(request, invalid));
        assert_eq!(
            session.furigana(&page, key.analysis).unwrap(),
            page.source_ruby,
            "case {field}"
        );
    }
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(key).unwrap();
    assert!(session.accept(request, snapshot));
    for field in 0..4 {
        let mut invalid = page.clone();
        match field {
            0 => invalid.text = invalid.text.replace('猫', "犬"),
            1 => invalid.anchor.range.end = i64::MAX.try_into().unwrap(),
            2 => invalid.text.push('別'),
            _ => invalid.text = invalid.text.replace('か', "き"),
        }
        assert_eq!(
            session.furigana(&invalid, key.analysis).unwrap(),
            page.source_ruby
        );
    }
    for field in 0..3 {
        let mut invalid = page.clone();
        match field {
            0 => invalid.source_ruby[0].range = range(1, 3),
            1 => invalid.source_ruby[0].reading.clear(),
            _ => invalid.source_ruby.push(invalid.source_ruby[0].clone()),
        }
        assert!(session.furigana(&invalid, key.analysis).is_err());
        session.invalidate();
        assert!(session.furigana(&invalid, key.analysis).is_err());
    }
}
