//! Layered diagnostics: SQL stepping without column decoding, pure replay over
//! predecoded fixture input, then the complete production repository projection.
//! This module is only compiled into the opt-in example, never the library.

use super::*;
use rusqlite::{Connection, StatementStatus};
use yomoku_core::{
    document::VocabularyId,
    statistics::{StatusTimelineBuilder, StatusTimelinePoint, VocabularyStatusChange},
};

// Share the production SQL rather than maintaining a profiling copy.
const EVENTS: &str = include_str!("../../../yomoku-storage/src/statistics/status_events.sql");
const LINKS: &str = include_str!("../../../yomoku-storage/src/statistics/status_links.sql");
const LEGACY: &str = include_str!("legacy_status.sql");

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

fn status(value: &str) -> VocabularyStatus {
    match value {
        "new" => VocabularyStatus::New,
        "learning" => VocabularyStatus::Learning,
        "familiar" => VocabularyStatus::Familiar,
        "known" => VocabularyStatus::Known,
        "ignored" => VocabularyStatus::Ignored,
        _ => panic!("invalid fixture status"),
    }
}

pub(super) fn measure(storage: &Storage, path: &Path, iterations: usize) {
    let connection =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    // Fix a snapshot for both split statements. File guards in the caller also
    // cover diagnostics. Setup/extraction/validation stay outside timed samples.
    let transaction = connection.unchecked_transaction().unwrap();
    let mut statement = transaction.prepare(LINKS).unwrap();
    let links = statement
        .query_map([], |row| {
            Ok((
                VocabularyId::try_from(row.get::<_, i64>(0)?).unwrap(),
                row.get::<_, String>(1)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut statement = transaction.prepare(EVENTS).unwrap();
    let events = statement
        .query_map([], |row| {
            Ok(VocabularyStatusChange {
                event_id: u64::try_from(row.get::<_, i64>(0)?).unwrap(),
                vocabulary_id: VocabularyId::try_from(row.get::<_, i64>(1)?).unwrap(),
                occurred_at_unix_ms: row.get(6)?,
                old_status: row.get::<_, Option<String>>(3)?.as_deref().map(status),
                new_status: status(&row.get::<_, String>(4)?),
            })
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let replay = || -> Vec<StatusTimelinePoint> {
        let mut builder = StatusTimelineBuilder::default();
        for (id, form) in &links {
            builder.link_dictionary_form(*id, form.clone()).unwrap();
        }
        for event in &events {
            builder.push(*event).unwrap();
        }
        builder.finish().unwrap()
    };
    // The isolated replay intentionally excludes storage decoding/provenance
    // validation; require exact output equality with the validated public API.
    assert_eq!(replay(), storage.vocabulary_status_timeline().unwrap());
    let legacy = scan(&transaction, LEGACY);
    let event_scan = scan(&transaction, EVENTS);
    let link_scan = scan(&transaction, LINKS);
    assert_eq!(legacy.0, events.len() + links.len());
    assert_eq!(event_scan.0, events.len());
    assert_eq!(link_scan.0, links.len());
    eprintln!("status_sql_legacy_rows={} vm_steps={} sorts={} split_rows={} vm_steps={} sorts={} replay_input_events={} links={}",
        legacy.0, legacy.1, legacy.2, event_scan.0 + link_scan.0,
        event_scan.1 + link_scan.1, event_scan.2 + link_scan.2, events.len(), links.len());
    super::measure("status_sql_legacy_step_only", iterations, || {
        scan(&transaction, LEGACY)
    });
    super::measure("status_sql_split_step_only", iterations, || {
        (scan(&transaction, LINKS), scan(&transaction, EVENTS))
    });
    super::measure("status_rust_replay_predecoded", iterations, replay);
    super::measure("repository_status_timeline", iterations, || {
        storage.vocabulary_status_timeline().unwrap()
    });
}
