use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::app::Event;

#[test]
fn anki_export_preparation_and_clear_have_no_shell_codec() {
    use yomoku_core::{
        anki::AnkiSelection,
        document::{TextAnchor, TextOffset, TextRange},
        effective_session::{EffectivePageIdentity, EffectiveReaderSelection},
        reader::{ReaderAnalysis, ReaderTokenKey},
    };
    let selection = AnkiSelection::Effective {
        selection: EffectiveReaderSelection {
            page: EffectivePageIdentity {
                revision: 1,
                base: ReaderTokenKey {
                    document_id: 1.try_into().unwrap(),
                    page: TextAnchor {
                        source_id: 2.try_into().unwrap(),
                        chapter_id: 3.try_into().unwrap(),
                        range: TextRange {
                            start: TextOffset::START,
                            end: 3.try_into().unwrap(),
                        },
                    },
                    analysis: ReaderAnalysis::Available(4.try_into().unwrap()),
                },
            },
            segment_index: 0,
        },
        lookup_revision: 1,
    };
    for event in [
        Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        },
        Event::ClearPreparedAnkiExport,
        Event::BeginAnkiLive,
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}
