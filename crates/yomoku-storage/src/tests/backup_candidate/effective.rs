use serde_json::{json, Value};
use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride},
    completion::PageCompletionIntent,
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectivePageCompletion,
    reader::ReaderPage,
    vocabulary::Surface,
};

use super::*;

fn setup(version: usize, completed: bool) -> (TestDatabase, Storage, EffectivePageCompletion) {
    setup_text(
        version,
        completed,
        "食べた 食べた 猫 English 𠮷 か\u{3099}",
        0,
    )
}

fn setup_text(
    version: usize,
    completed: bool,
    text: &str,
    high_water: i64,
) -> (TestDatabase, Storage, EffectivePageCompletion) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
    if high_water > 0 {
        for table in [
            "sources",
            "chapters",
            "documents",
            "chapter_analyses",
            "segmentation_overrides",
            "completion_commands",
        ] {
            connection
                .execute(
                    "INSERT INTO sqlite_sequence(name, seq) VALUES (?1, ?2)",
                    (table, high_water),
                )
                .unwrap();
        }
    }
    let mut storage = Storage { connection };
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new(text.into()).unwrap(),
            100,
            super::super::imports::fixture,
        )
        .unwrap();
    let chapter = &imported.document.chapter;
    storage
        .save_segmentation_override(&SegmentationOverride {
            scope: OverrideScope::AllOccurrences,
            anchor: TextAnchor {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                range: TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(&chapter.text, "食べた".len()).unwrap(),
                },
            },
            surface: Surface::new("食べた").unwrap(),
            parts: ["食べ", "た"].map(|s| Surface::new(s).unwrap()).to_vec(),
        })
        .unwrap();
    let page_length = storage.page_length().unwrap();
    let page = ReaderPage::open(&imported.document, page_length).unwrap();
    let key = storage
        .load_effective_reader_tokens(imported.document.document_id, page.anchor)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    let frozen = storage
        .prepare_effective_page_completion(
            PageCompletionIntent {
                document_id: imported.document.document_id,
                page: page.anchor,
                page_length,
                departing_position: page.position,
                completed_at_unix_ms: 0,
                analysis_id: imported.analysis_id,
            },
            &key,
        )
        .unwrap();
    if completed {
        storage.record_effective_page_completion(&frozen).unwrap();
    }
    (database, storage, frozen)
}

fn replace(storage: &Storage, payload: &str) {
    storage
        .connection
        .execute(
            "UPDATE effective_completion_commands SET payload = ?1",
            [payload],
        )
        .unwrap();
}

#[test]
fn backup_candidate_effective_rejects_wire_shapes_unknown_fields_and_duplicate_keys() {
    let (_, _, frozen) = setup(24, false);
    let valid = serde_json::to_value(&frozen).unwrap();
    let mut invalid = vec![json!({}), json!([]), json!(null)];
    for path in [
        "",
        "/completion",
        "/completion/page",
        "/completion/page/range",
        "/completion/departing_position",
        "/key",
        "/key/page",
        "/key/segmentation",
        "/key/segmentation/profile",
        "/key/segmentation/segments/0",
        "/key/segmentation/segments/0/boundary",
        "/key/segmentation/segments/4/metadata",
        "/occurrences/0",
        "/occurrences/0/anchor",
        "/occurrences/0/display_range",
    ] {
        let mut value = valid.clone();
        value
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("untrusted-secret".into(), json!(true));
        invalid.push(value);
    }
    for path in [
        "/completion",
        "/key/segmentation/profile",
        "/key/segmentation/segments",
        "/occurrences",
    ] {
        let mut value = valid.clone();
        *value.pointer_mut(path).unwrap() = json!(null);
        invalid.push(value);
    }
    for (path, field) in [
        ("/key/segmentation/segments/0", "metadata"),
        ("/key/segmentation/segments/0/boundary", "override_id"),
        (
            "/key/segmentation/segments/0/boundary",
            "original_token_index",
        ),
        ("/key/segmentation/segments/4/metadata", "reading"),
        ("/key/segmentation/segments/4/metadata", "dictionary_form"),
        ("/occurrences/0", "metadata"),
        ("/occurrences/0", "vocabulary"),
    ] {
        let mut value = valid.clone();
        value
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        invalid.push(value);
    }
    let serialized = valid.to_string();
    for payload in invalid.iter().map(Value::to_string).chain([
        serialized.replacen("\"command_id\":1", "\"command_id\":1,\"command_id\":1", 1),
        serialized.replacen(
            "\"surface\":\"食べ\"",
            "\"surface\":\"食べ\",\"surface\":\"食べ\"",
            1,
        ),
    ]) {
        let (database, storage, _) = setup(24, false);
        replace(&storage, &payload);
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_effective_rejects_invalid_domains_and_inconsistent_frozen_projection() {
    for completed in [false, true] {
        for (path, bad) in [
            ("/completion/command_id", json!(2)),
            ("/completion/document_id", json!(0)),
            ("/completion/completed_at_unix_ms", json!(-1)),
            ("/completion/pending_time", json!(-1)),
            ("/completion/page_length", json!("Characters1")),
            ("/completion/page/range/start", json!(1)), // Inside a UTF-8 scalar.
            ("/completion/departing_position/offset", json!(1)),
            ("/completion/departing_position/source_id", json!(2)),
            ("/key/document_id", json!(2)),
            ("/key/page/range/end", json!(1)),
            (
                "/key/segmentation/profile/engine_revision",
                json!("\u{3000}"),
            ),
            ("/key/segmentation/profile/dictionary_revision", json!(" ")),
            ("/key/segmentation/original_analysis_id", json!(0)),
            ("/key/segmentation/segments/0/boundary/surface", json!("違")),
            (
                "/key/segmentation/segments/0/boundary/override_id",
                json!(null),
            ),
            (
                "/key/segmentation/segments/0/boundary/anchor/chapter_id",
                json!(2),
            ),
            (
                "/key/segmentation/segments/1/boundary/anchor/range/start",
                json!(0),
            ),
            ("/key/segmentation/segments/4/metadata/mode", json!("A")),
            (
                "/key/segmentation/segments/4/metadata/token_index",
                json!(-1),
            ),
            (
                "/key/segmentation/segments/4/metadata/reading",
                json!("\u{a0}"),
            ),
            (
                "/key/segmentation/segments/4/metadata/dictionary_form",
                json!(" "),
            ),
            (
                "/key/segmentation/segments/4/metadata/analysis_id",
                json!(2),
            ),
            ("/occurrences/0/segment_index", json!(1)),
            ("/occurrences/0/surface", json!("違う")),
            ("/occurrences/0/eligibility", json!("AnalyzerJapanese")),
            ("/occurrences/0/display_range/end", json!(3)),
            ("/occurrences/0/anchor/range/end", json!(3)),
            (
                "/occurrences/0/vocabulary",
                json!({"id": 1, "status": "New"}),
            ),
            ("/occurrences/4/metadata/reading", json!("ネコ")),
            ("/occurrences", json!([])),
        ] {
            let (database, storage, frozen) = setup(24, completed);
            let mut value = serde_json::to_value(&frozen).unwrap();
            *value.pointer_mut(path).unwrap() = bad;
            replace(&storage, &value.to_string());
            check_rejected(&database, 24, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_effective_checks_receipts_headers_and_every_surviving_occurrence() {
    for sql in [
        "UPDATE reading_completions SET active_time_ms = 1",
        "UPDATE reading_completions SET completed_at_unix_ms = 1",
        "UPDATE reading_completions SET page_length = 250",
        "UPDATE reading_completions SET departing_byte_offset = 3",
        "UPDATE reading_completions SET end_byte_offset = end_byte_offset + 1",
        "UPDATE reading_completions SET document_id = document_id + 1",
        "DELETE FROM vocabulary_status_events; DELETE FROM reading_occurrences; DELETE FROM completion_analyses",
        "UPDATE completion_analyses SET analysis_id = analysis_id + 1",
        "UPDATE completion_analyses SET engine_revision = 'other'",
        "UPDATE completion_analyses SET dictionary_revision = 'other'",
        "UPDATE completion_analyses SET split_mode = 'a'",
        "UPDATE reading_occurrences SET token_index = token_index + 100",
        "UPDATE reading_occurrences SET start_byte = start_byte + 1 WHERE token_index = 0",
        "UPDATE reading_occurrences SET end_byte = end_byte + 1 WHERE token_index = 0",
        "UPDATE reading_occurrences SET surface = '他' WHERE token_index = 0",
        "UPDATE reading_occurrences SET reading = '他' WHERE token_index = 0",
        "UPDATE reading_occurrences SET dictionary_form = '他' WHERE token_index = 0",
        "UPDATE reading_occurrences SET vocabulary_id = (SELECT id FROM vocabulary WHERE surface = '猫') WHERE token_index = 0",
    ] {
        let (database, storage, _) = setup(24, true);
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_effective_checks_historical_issued_ids_even_after_deletion() {
    for table in ["documents", "chapter_analyses", "segmentation_overrides"] {
        let (database, mut storage, frozen) = setup(24, false);
        storage
            .delete_document(frozen.completion().document_id)
            .unwrap();
        storage
            .connection
            .execute(
                "UPDATE sqlite_sequence SET seq = 0 WHERE name = ?1",
                [table],
            )
            .unwrap();
        // Rules are retained after document deletion, so controls may reject
        // their surviving row first. The detached document/analysis IDs are
        // checked by the frozen payload, including an unfinished reservation.
        check_rejected(&database, 24, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_effective_preserves_reservations_deleted_history_and_old_prefixes() {
    for version in [18, 19, 20, 21, 22, 23, 24] {
        for completed in [false, true] {
            let (database, mut storage, frozen) = setup(version, completed);
            // Reversal cannot reinterpret the frozen rule or occurrences.
            storage
                .reverse_segmentation_override(1.try_into().unwrap())
                .unwrap();
            storage
                .delete_document(frozen.completion().document_id)
                .unwrap();
            storage
                .connection
                .execute("DELETE FROM vocabulary WHERE surface IN ('猫', '食べ')", [])
                .unwrap();
            // Re-registering erased spelling does not recreate its old encounters.
            storage
                .register_vocabulary(&[Surface::new("食べ").unwrap()], i64::MAX)
                .unwrap();
            let before = fs::read(database.path()).unwrap();
            let directory = database.0.join("migrated-effective");
            let checked = candidate(&directory, &before)
                .check_schema(version as u32, before.len() as u64, || false)
                .unwrap();
            let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
            let payload: String = copy
                .query_row(
                    "SELECT payload FROM effective_completion_commands",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(payload, serde_json::to_string(&frozen).unwrap());
            assert_eq!(
                copy.query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                i64::from(completed)
            );
            drop(copy);
            checked.discard().unwrap();
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
    }
}

#[test]
fn backup_candidate_effective_rejects_corrupt_payload_before_migration() {
    for version in [18, 20, 23, 24] {
        let (database, storage, _) = setup(version, false);
        replace(&storage, "{}");
        check_rejected(&database, version as u32, CandidateError::InvalidData);
    }
}

#[test]
fn backup_candidate_effective_preserves_alternate_metadata_and_non_japanese_exclusion() {
    use yomoku_core::analysis::SplitMode;
    for mode in [SplitMode::A, SplitMode::C] {
        let (database, mut storage, first) = setup(24, false);
        let c = first.completion();
        let mut alternate = storage
            .chapter_analysis(first.key().segmentation.original_analysis_id)
            .unwrap()
            .unwrap();
        alternate.mode = mode;
        alternate.tokens = alternate
            .tokens
            .iter()
            .flat_map(|token| {
                if token.surface != "食べた" {
                    return vec![token.clone()];
                }
                let mut start = token.clone();
                start.surface = "食べ".into();
                start.anchor.range.end = (i64::from(start.anchor.range.start) + 6)
                    .try_into()
                    .unwrap();
                start.reading = Some(" タベ\0𠮷 ".into());
                start.dictionary_form = Some(Surface::new(" 食べる\0 ").unwrap());
                let mut end = token.clone();
                end.surface = "た".into();
                end.anchor.range.start = start.anchor.range.end;
                end.is_japanese = false;
                vec![start, end]
            })
            .collect();
        let alternate_id = storage
            .store_chapter_analysis(c.page.chapter_id, &alternate)
            .unwrap();
        let key = storage
            .load_effective_reader_tokens(c.document_id, c.page)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(
                PageCompletionIntent {
                    document_id: c.document_id,
                    page: c.page,
                    page_length: c.page_length,
                    departing_position: c.departing_position,
                    completed_at_unix_ms: i64::MAX,
                    analysis_id: first.key().segmentation.original_analysis_id,
                },
                &key,
            )
            .unwrap();
        assert_eq!(
            frozen.occurrences()[0]
                .metadata
                .as_ref()
                .unwrap()
                .analysis_id,
            alternate_id
        );
        assert!(!frozen
            .occurrences()
            .iter()
            .any(|token| token.surface == "た"));
        storage.record_effective_page_completion(&frozen).unwrap();
        storage.delete_document(c.document_id).unwrap();
        accept_unchanged(&database, 24);
    }
}

#[test]
fn backup_candidate_effective_preserves_full_i64_identities_and_exhausted_counters() {
    for high_water in [9_007_199_254_740_992, i64::MAX - 1] {
        let (database, storage, frozen) = setup_text(24, true, "食べた 猫 𠮷", high_water);
        assert_eq!(i64::from(frozen.completion().command_id), high_water + 1);
        assert_eq!(
            i64::from(frozen.key().segmentation.original_analysis_id),
            high_water + 1
        );
        accept_unchanged(&database, 24);
        drop(storage);
    }
}

#[test]
fn backup_candidate_effective_preserves_crossing_tokens_empty_occurrences_and_eof_departure() {
    // A start-owned occurrence may extend far beyond the departing page. All
    // later pages have no start-owned occurrences, though the word is selectable.
    let text = format!("食べた {}", "猫。".repeat(1_100));
    let (database, mut storage, first) = setup_text(24, true, &text, 0);
    let crossing = first.occurrences().last().unwrap();
    assert!(crossing.anchor.range.end > first.completion().page.range.end);
    let c = first.completion();
    loop {
        let document = storage
            .load_document_reader_snapshot(c.document_id)
            .unwrap()
            .unwrap();
        let page = ReaderPage::open_document(&document.document, document.page_length).unwrap();
        let key = storage
            .load_effective_reader_tokens(c.document_id, page.anchor)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(
                PageCompletionIntent {
                    document_id: c.document_id,
                    page: page.anchor,
                    page_length: document.page_length,
                    departing_position: page.position,
                    completed_at_unix_ms: 0,
                    analysis_id: first.key().segmentation.original_analysis_id,
                },
                &key,
            )
            .unwrap();
        assert!(frozen.occurrences().is_empty());
        storage.record_effective_page_completion(&frozen).unwrap();
        if page.position.offset == page.anchor.range.end {
            break;
        }
    }
    accept_unchanged(&database, 24);
}
