use super::*;
use crate::tests::reader_work;
use yomoku_core::{document::VocabularyId, reader::ReaderVocabulary};

struct Fixture {
    database: TestDatabase,
    _index: TestDatabase,
    dictionary: Dictionary,
    storage: Storage,
    imported: AnalyzedPaste,
    key: EffectiveReaderTokenKey,
    id: VocabularyId,
}

impl Fixture {
    fn new() -> Self {
        let database = TestDatabase::new();
        let index = dictionary();
        let dictionary = Dictionary::open(index.path()).unwrap();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "試験語 試験語 別語");
        let key = key(&imported);
        let id = storage
            .vocabulary_entry(&Surface::new("試験語").unwrap())
            .unwrap()
            .unwrap()
            .id;
        Self {
            database,
            _index: index,
            dictionary,
            storage,
            imported,
            key,
            id,
        }
    }

    fn prime(&mut self) {
        self.storage
            .load_effective_reader_word(&self.dictionary, &self.key, 0)
            .unwrap()
            .unwrap();
        reader_work::take();
    }

    fn save(
        &mut self,
        status: VocabularyStatus,
    ) -> Result<Option<ReaderVocabulary>, ReaderWordError> {
        self.storage
            .set_effective_reader_status(&self.key, 0, self.id, status, 1234)
    }
}

#[test]
fn lookup_and_status_validation_prime_one_selection_and_hits_only_read_selected_surface() {
    let mut f = Fixture::new();
    reader_work::take();
    f.save(VocabularyStatus::Learning).unwrap().unwrap();
    let (documents, chapters, surfaces) = reader_work::take();
    assert_eq!((documents, chapters), (1, 1));
    assert!(surfaces.contains(&"別語".to_owned()));
    for status in [
        VocabularyStatus::Known,
        VocabularyStatus::Known,
        VocabularyStatus::New,
    ] {
        assert_eq!(f.save(status).unwrap().unwrap().status, status);
        assert_eq!(reader_work::take(), (0, 0, vec!["試験語".to_owned()]));
    }
    f.prime();
    assert_eq!(
        f.save(VocabularyStatus::Familiar).unwrap().unwrap().status,
        VocabularyStatus::Familiar
    );
    assert_eq!(reader_work::take(), (0, 0, vec!["試験語".to_owned()]));
    let snapshot = f
        .storage
        .load_effective_reader_tokens(f.key.document_id, f.key.page)
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot
            .tokens
            .iter()
            .filter(|t| t.surface == "試験語")
            .count(),
        2
    );
    assert!(snapshot
        .tokens
        .iter()
        .filter(|t| t.surface == "試験語")
        .all(|t| t.vocabulary.unwrap().status == VocabularyStatus::Familiar));
    let dates: Vec<i64> = f.storage.connection.prepare("SELECT occurred_at_unix_ms FROM vocabulary_status_events WHERE cause = 'manual' ORDER BY id").unwrap().query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(dates, [1234; 4]);
}

#[test]
fn different_segment_page_and_key_never_reuse_the_cached_selection() {
    let mut f = Fixture::new();
    f.prime();
    f.storage
        .set_effective_reader_status(&f.key, 1, f.id, VocabularyStatus::Known, 1234)
        .unwrap()
        .unwrap();
    assert_eq!(reader_work::take().0, 1);
    // The one-entry cache now belongs to segment 1.
    f.save(VocabularyStatus::Learning).unwrap().unwrap();
    assert_eq!(reader_work::take().0, 1);
    let mut other_page = f.key.clone();
    other_page.page.range.end =
        TextOffset::at(&f.imported.document.chapter.text, "試験語".len()).unwrap();
    f.storage
        .set_effective_reader_status(&other_page, 0, f.id, VocabularyStatus::Known, 1234)
        .unwrap()
        .unwrap();
    assert_eq!(reader_work::take().0, 1);
    let mut stale = other_page;
    stale
        .segmentation
        .profile
        .engine_revision
        .push_str("-stale");
    assert!(matches!(
        f.storage
            .set_effective_reader_status(&stale, 0, f.id, VocabularyStatus::New, 1234),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert_eq!(reader_work::take().0, 1);
    assert!(f.storage.validated_reader_selection.is_none());
    f.prime();
    assert!(matches!(
        f.storage
            .set_effective_reader_status(&f.key, u64::MAX, f.id, VocabularyStatus::New, 1234),
        Err(ReaderWordError::InvalidSelection)
    ));
}

#[test]
fn failed_lookup_and_invalid_status_timestamp_clear_prior_validation() {
    let mut f = Fixture::new();
    f.prime();
    assert!(matches!(
        f.storage
            .load_effective_reader_word(&f.dictionary, &f.key, u64::MAX),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert!(f.storage.validated_reader_selection.is_none());
    f.prime();
    assert!(matches!(
        f.storage
            .set_effective_reader_status(&f.key, 0, f.id, VocabularyStatus::Known, -1),
        Err(ReaderWordError::Storage(StorageError::InvalidVocabulary))
    ));
    assert!(f.storage.validated_reader_selection.is_none());
    f.save(VocabularyStatus::Known).unwrap().unwrap();
    assert_eq!(reader_work::take().0, 1);
}

#[test]
fn local_dml_ddl_and_rolled_back_writes_force_full_validation() {
    for sql in [
        "UPDATE vocabulary SET status = 'familiar' WHERE surface = '試験語'",
        "CREATE TABLE cache_test (value INTEGER)",
        "BEGIN; UPDATE vocabulary SET status = 'known' WHERE surface = '別語'; ROLLBACK;",
    ] {
        let mut f = Fixture::new();
        f.prime();
        f.storage.connection.execute_batch(sql).unwrap();
        f.save(VocabularyStatus::Learning).unwrap().unwrap();
        assert_eq!(reader_work::take().0, 1, "{sql}");
        f.save(VocabularyStatus::Known).unwrap().unwrap();
        assert_eq!(reader_work::take(), (0, 0, vec!["試験語".to_owned()]));
    }
}

#[test]
fn local_and_external_changes_reject_corrupt_changed_or_deleted_analysis_and_document() {
    for external in [false, true] {
        for (sql, invalid_selection) in [
            (
                "UPDATE analyzed_tokens SET surface = 'corrupt' WHERE token_index = 2",
                false,
            ),
            (
                "UPDATE analyzed_tokens SET reading = 'ベツ' WHERE token_index = 2",
                true,
            ),
            ("DELETE FROM chapter_analyses", true),
            ("DELETE FROM documents", true),
        ] {
            let mut f = Fixture::new();
            f.prime();
            let writer = Connection::open(f.database.path()).unwrap();
            writer.pragma_update(None, "foreign_keys", true).unwrap();
            let connection = if external {
                &writer
            } else {
                &f.storage.connection
            };
            connection.execute_batch(sql).unwrap();
            let result = f.save(VocabularyStatus::Known);
            if invalid_selection {
                assert!(
                    matches!(result, Err(ReaderWordError::InvalidSelection)),
                    "external={external}, {sql}: {result:?}"
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Err(ReaderWordError::Storage(StorageError::InvalidAnalysis))
                    ),
                    "{result:?}"
                );
            }
            assert_eq!(reader_work::take().0, 1);
            assert!(f.storage.validated_reader_selection.is_none());
            let events: i64 = f
                .storage
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(events, 0);
        }
    }
}

#[test]
fn external_commits_in_rollback_and_wal_modes_force_validation_and_live_history() {
    for journal in ["DELETE", "WAL"] {
        let mut f = Fixture::new();
        f.storage
            .connection
            .pragma_update(None, "journal_mode", journal)
            .unwrap();
        f.prime();
        let writer = Connection::open(f.database.path()).unwrap();
        writer
            .execute(
                "UPDATE vocabulary SET status = 'familiar' WHERE surface = '試験語'",
                [],
            )
            .unwrap();
        f.save(VocabularyStatus::Known).unwrap().unwrap();
        assert_eq!(reader_work::take().0, 1);
        let old: String = writer
            .query_row(
                "SELECT old_status FROM vocabulary_status_events WHERE cause = 'manual'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old, "familiar");
        f.save(VocabularyStatus::Learning).unwrap().unwrap();
        assert_eq!(reader_work::take(), (0, 0, vec!["試験語".to_owned()]));
    }
}

#[test]
fn missing_replaced_and_wrong_vocabulary_ids_never_register_or_retarget() {
    for external in [false, true] {
        for replace in [false, true] {
            let mut f = Fixture::new();
            f.prime();
            let writer = Connection::open(f.database.path()).unwrap();
            writer.pragma_update(None, "foreign_keys", true).unwrap();
            let connection = if external {
                &writer
            } else {
                &f.storage.connection
            };
            connection
                .execute("DELETE FROM vocabulary WHERE surface = '試験語'", [])
                .unwrap();
            if replace {
                connection.execute("INSERT INTO vocabulary (surface, status, registered_at_unix_ms) VALUES ('試験語', 'new', 1)", []).unwrap();
                assert!(matches!(
                    f.save(VocabularyStatus::Known),
                    Err(ReaderWordError::InvalidSelection)
                ));
            } else {
                assert!(f.save(VocabularyStatus::Known).unwrap().is_none());
                assert!(f
                    .storage
                    .vocabulary_entry(&Surface::new("試験語").unwrap())
                    .unwrap()
                    .is_none());
            }
            assert!(f.storage.validated_reader_selection.is_none());
        }
    }
    let mut f = Fixture::new();
    f.prime();
    let wrong = f
        .storage
        .vocabulary_entry(&Surface::new("別語").unwrap())
        .unwrap()
        .unwrap()
        .id;
    assert!(matches!(
        f.storage
            .set_effective_reader_status(&f.key, 0, wrong, VocabularyStatus::Known, 1234),
        Err(ReaderWordError::InvalidSelection)
    ));
}

#[test]
fn write_and_commit_failures_roll_back_and_retry_keeps_exact_timestamp_without_duplicate_events() {
    for sql in [
        "CREATE TRIGGER fail_status BEFORE UPDATE ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture failure'); END;",
        "CREATE TRIGGER fail_status BEFORE INSERT ON vocabulary_status_events WHEN NEW.cause = 'manual' BEGIN SELECT RAISE(ABORT, 'fixture failure'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_status AFTER INSERT ON vocabulary_status_events WHEN NEW.cause = 'manual' BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let mut f = Fixture::new();
        f.storage.connection.execute_batch(sql).unwrap();
        f.prime();
        assert!(matches!(
            f.save(VocabularyStatus::Known),
            Err(ReaderWordError::Storage(StorageError::Sqlite(_)))
        ));
        assert!(f.storage.validated_reader_selection.is_none());
        assert_eq!(
            f.storage
                .vocabulary_entry(&Surface::new("試験語").unwrap())
                .unwrap()
                .unwrap()
                .status,
            VocabularyStatus::New
        );
        f.storage
            .connection
            .execute_batch("DROP TRIGGER fail_status")
            .unwrap();
        reader_work::take();
        f.save(VocabularyStatus::Known).unwrap().unwrap();
        assert_eq!(reader_work::take().0, 1);
        f.save(VocabularyStatus::Known).unwrap().unwrap();
        assert_eq!(reader_work::take(), (0, 0, vec!["試験語".to_owned()]));
        let event: (String, String, i64, i64) = f.storage.connection.query_row("SELECT old_status, new_status, occurred_at_unix_ms, COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
        assert_eq!(event, ("new".into(), "known".into(), 1234, 1));
    }
}

#[test]
fn unexpected_trigger_writes_discard_validation_even_when_status_save_succeeds() {
    let mut f = Fixture::new();
    f.storage.connection.execute_batch("CREATE TRIGGER mutate_analysis AFTER UPDATE ON vocabulary BEGIN UPDATE analyzed_tokens SET reading = 'ベツ' WHERE token_index = 2; END;").unwrap();
    f.prime();
    f.save(VocabularyStatus::Known).unwrap().unwrap();
    assert_eq!(reader_work::take().0, 0);
    assert!(f.storage.validated_reader_selection.is_none());
    assert!(matches!(
        f.save(VocabularyStatus::Learning),
        Err(ReaderWordError::InvalidSelection)
    ));
    assert_eq!(reader_work::take().0, 1);
}
