//! Layered diagnostics: SQL stepping, production row/provenance decoding, pure
//! replay over predecoded input, then the complete repository projection.
//! This module is only compiled into the opt-in example, never the library.

use super::*;
use rusqlite::{Connection, StatementStatus};
use yomoku_core::statistics::{StatusTimelineBuilder, StatusTimelinePoint};
use yomoku_storage::StorageError;

// Share the production SQL rather than maintaining a profiling copy.
const EVENTS: &str = include_str!("../../../yomoku-storage/src/statistics/status_events.sql");

fn scan(connection: &Connection, sql: &str) -> (usize, i32, i32) {
    let mut statement = connection.prepare(sql).unwrap();
    let mut rows = statement.query([]).unwrap();
    let mut count = 0;
    while rows.next().unwrap().is_some() {
        count += 1;
    }
    drop(rows);
    (
        count,
        statement.get_status(StatementStatus::VmStep),
        statement.get_status(StatementStatus::Sort),
    )
}

// Compile the production decoder and enum parser, including provenance checks.
#[path = "../../../yomoku-storage/src/vocabulary/status.rs"]
mod status;
#[path = "../../../yomoku-storage/src/statistics/status_row.rs"]
mod status_row;
use status::decode_status;

fn decode_events(connection: &Connection) -> usize {
    let mut statement = connection.prepare(EVENTS).unwrap();
    let mut rows = statement.query([]).unwrap();
    let mut count = 0;
    while let Some(row) = rows.next().unwrap() {
        black_box(status_row::decode(row).unwrap());
        count += 1;
    }
    count
}

pub(super) fn measure(storage: &Storage, path: &Path, iterations: usize) {
    let connection =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Fix a snapshot for diagnostic reads. File guards in the caller also
    // cover diagnostics. Setup/extraction/validation stay outside timed samples.
    let transaction = connection.unchecked_transaction().unwrap();
    let mut statement = transaction.prepare(EVENTS).unwrap();
    let mut rows = statement.query([]).unwrap();
    let mut events = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        events.push(status_row::decode(row).unwrap());
    }
    let replay = || -> Vec<StatusTimelinePoint> {
        let mut builder = StatusTimelineBuilder::default();
        for event in &events {
            builder.push(*event).unwrap();
        }
        builder.finish().unwrap()
    };
    // The isolated replay intentionally excludes storage decoding/provenance
    // validation; require exact output equality with the validated public API.
    assert_eq!(replay(), storage.vocabulary_status_timeline().unwrap());
    let event_scan = scan(&transaction, EVENTS);
    assert_eq!(event_scan.0, events.len());
    assert_eq!(decode_events(&transaction), events.len());
    eprintln!(
        "status_sql_rows={} vm_steps={} sorts={} replay_input_events={}",
        event_scan.0,
        event_scan.1,
        event_scan.2,
        events.len()
    );
    super::measure("status_sql_step_only", iterations, || {
        scan(&transaction, EVENTS)
    });
    super::measure("status_sql_decode_validate", iterations, || {
        decode_events(&transaction)
    });
    super::measure("status_rust_replay_predecoded", iterations, replay);
    super::measure("repository_status_timeline", iterations, || {
        storage.vocabulary_status_timeline().unwrap()
    });
}
