//! Step the exact production metric-sort queries without decoding their columns.
//! Only the opt-in harness compiles this module; all reads share one snapshot.

use super::*;
use rusqlite::{params, Connection, StatementStatus};

fn scan(connection: &Connection, sql: &str, offset: u64) -> (usize, i32, i32) {
    let mut statement = connection.prepare(sql).unwrap();
    let mut rows = statement
        .query(params!["", i64::try_from(offset).unwrap()])
        .unwrap();
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

pub(super) fn measure(storage: &Storage, path: &Path, iterations: usize) {
    let connection =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let transaction = connection.unchecked_transaction().unwrap();
    for sort in [VocabularySort::Frequency, VocabularySort::LastSeen] {
        let query = VocabularyQuery {
            sort,
            ..Default::default()
        };
        let count = storage.vocabulary_page(&query, 0).unwrap().matching_count;
        let mut offsets = vec![0, count.saturating_sub(100)];
        offsets.dedup();
        for offset in offsets {
            let sql = super::browser_sql::page_sql(sort, "v.surface >= ?");
            let expected = storage.vocabulary_page(&query, offset).unwrap();
            // Verify every raw SQL field against the fully validated repository
            // output before measuring column-free stepping.
            let mut statement = transaction.prepare(&sql).unwrap();
            let mut rows = statement
                .query(params!["", i64::try_from(offset).unwrap()])
                .unwrap();
            for expected in &expected.rows {
                let row = rows.next().unwrap().unwrap();
                assert_eq!(row.get::<_, i64>(0).unwrap(), i64::from(expected.id));
                assert_eq!(row.get::<_, String>(1).unwrap(), expected.surface);
                assert_eq!(
                    row.get::<_, String>(2).unwrap(),
                    format!("{:?}", expected.status).to_lowercase()
                );
                assert_eq!(
                    row.get::<_, i64>(3).unwrap(),
                    expected.registered_at_unix_ms
                );
                assert_eq!(
                    row.get::<_, i64>(4).unwrap(),
                    i64::try_from(expected.encountered_count).unwrap()
                );
                assert_eq!(
                    row.get::<_, Option<i64>>(5).unwrap(),
                    expected.last_seen_unix_ms
                );
                assert!(row.get::<_, bool>(6).unwrap());
            }
            assert!(rows.next().unwrap().is_none());
            let work = scan(&transaction, &sql, offset);
            assert_eq!(work.0, expected.rows.len());
            eprintln!(
                "browser_sql sort={sort:?} offset={offset} rows={} vm_steps={} sorts={}",
                work.0, work.1, work.2
            );
            super::measure(
                &format!("browser_sql_{sort:?}_offset_{offset}"),
                iterations,
                || scan(&transaction, &sql, offset),
            );
        }
    }
}
