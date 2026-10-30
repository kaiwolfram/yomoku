//! Opt-in host measurements over a disposable, production-seeded database.
//! No supplied database, dictionary, Android runtime or user files are opened.

use std::{
    convert::Infallible,
    hint::black_box,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::{Destination, Event, ViewModel},
    completion::PageCompletionIntent,
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    reader::ReaderPage,
    statistics::StatisticsTimeframe,
    vocabulary::{
        Surface, VocabularyBulkAction, VocabularyQuery, VocabularySort, VocabularyStatus,
    },
};
use yomoku_ffi::YomokuBridge;
use yomoku_runtime::ShellEffectFfi;
use yomoku_storage::Storage;

const DATE: i64 = 1_700_000_000_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yomoku-vocabulary-measure-{}-{nonce}",
            std::process::id()
        ));
        // Never reuse or remove a pre-existing directory.
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn database(&self) -> PathBuf {
        self.0.join("scratch.sqlite")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove only owned scratch directory");
    }
}

fn assert_same_bytes(left: &Path, right: &Path) {
    let mut left = std::fs::File::open(left).unwrap();
    let mut right = std::fs::File::open(right).unwrap();
    let mut remaining = left.metadata().unwrap().len();
    assert_eq!(remaining, right.metadata().unwrap().len());
    let mut buffers = [[0; 64 * 1024]; 2];
    while remaining > 0 {
        let length = remaining.min(buffers[0].len() as u64) as usize;
        left.read_exact(&mut buffers[0][..length]).unwrap();
        right.read_exact(&mut buffers[1][..length]).unwrap();
        assert_eq!(
            buffers[0][..length],
            buffers[1][..length],
            "read-only database bytes"
        );
        remaining -= length as u64;
    }
}

#[derive(Debug, PartialEq)]
struct Config {
    entries: usize,
    rounds: usize,
    rereads: usize,
    iterations: usize,
}

impl Config {
    fn parse(arguments: &[String]) -> Result<Self, &'static str> {
        if arguments.len() != 4 {
            return Err("usage: measure_vocabulary ENTRIES STATUS_ROUNDS REREADS ITERATIONS");
        }
        let values: Vec<usize> = arguments
            .iter()
            .map(|value| value.parse().map_err(|_| "arguments must be integers"))
            .collect::<Result<_, _>>()?;
        if !(100..=100_000).contains(&values[0])
            || values[1] > 100
            || values[2] > 10_000
            || !(1..=100).contains(&values[3])
        {
            return Err(
                "limits: entries 100..100000, rounds 0..100, rereads 0..10000, iterations 1..100",
            );
        }
        Ok(Self {
            entries: values[0],
            rounds: values[1],
            rereads: values[2],
            iterations: values[3],
        })
    }
}

fn analysis(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut cursor = 0;
    let tokens = chapter
        .text
        .split_whitespace()
        .map(|surface| {
            let start = cursor + chapter.text[cursor..].find(surface).unwrap();
            cursor = start + surface.len();
            AnalyzedToken {
                anchor: TextAnchor {
                    source_id: chapter.source_id,
                    chapter_id: chapter.chapter_id,
                    range: TextRange {
                        start: TextOffset::at(&chapter.text, start).unwrap(),
                        end: TextOffset::at(&chapter.text, cursor).unwrap(),
                    },
                },
                surface: surface.into(),
                is_japanese: true,
                reading: None,
                dictionary_form: Some(Surface::new(surface).unwrap()),
            }
        })
        .collect();
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "synthetic-vocabulary-measure@1".into(),
            dictionary_revision: "synthetic-vocabulary-measure@1".into(),
        },
        mode: SplitMode::B,
        tokens,
    })
}

fn seed(storage: &mut Storage, config: &Config) {
    let started = Instant::now();
    let surfaces: Vec<_> = (0..config.entries)
        .map(|index| Surface::new(format!("語{index:06}")).unwrap())
        .collect();
    assert_eq!(
        storage.register_vocabulary(&surfaces, DATE).unwrap(),
        config.entries
    );
    // A small repeated page gives a concentrated, committed encounter history.
    // Classification is synthetic; this does not measure Sudachi or tokenization.
    let text = surfaces[..25]
        .iter()
        .cycle()
        .take(50)
        .map(Surface::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let imported = storage
        .create_analyzed_paste(&PasteText::new(text).unwrap(), DATE, analysis)
        .unwrap();
    let snapshot = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    assert_eq!(page.text, imported.document.chapter.text);
    for index in 0..config.rereads {
        let frozen = storage
            .prepare_page_completion(PageCompletionIntent {
                document_id: imported.document.document_id,
                page: page.anchor,
                page_length: snapshot.page_length,
                departing_position: page.position,
                completed_at_unix_ms: DATE + index as i64 + 1,
                analysis_id: imported.analysis_id,
            })
            .unwrap();
        storage
            .record_analyzed_page_completion(frozen.completion, frozen.analysis_id)
            .unwrap();
        storage
            .save_document_reading_position(imported.document.document_id, page.position)
            .unwrap();
    }
    let ids = storage
        .vocabulary_matching_ids(&VocabularyQuery::default())
        .unwrap();
    let statuses = VocabularyQuery::default().statuses;
    // Rotate all five groups so final statuses are mixed and every later round
    // creates one real manual transition per surface through production APIs.
    for round in 0..config.rounds {
        for group in 0..5 {
            let group_ids: Vec<_> = ids.iter().skip(group).step_by(5).copied().collect();
            storage
                .set_vocabulary_statuses(
                    &group_ids,
                    statuses[(group + round + 1) % 5],
                    DATE + config.rereads as i64 + round as i64 + 1,
                )
                .unwrap();
        }
    }
    let totals = storage.lifetime_reading_totals().unwrap();
    assert_eq!(totals.japanese_occurrences, config.rereads as u64 * 50);
    assert_eq!(totals.active_time_ms, 0);
    let buckets = storage
        .statistics_buckets(StatisticsTimeframe::LastYear, DATE + 100_000)
        .unwrap();
    let values = buckets.buckets.last().unwrap().values;
    assert_eq!(values.reading, totals);
    let counts = values.statuses;
    assert_eq!(
        counts.new + counts.learning + counts.familiar + counts.known + counts.ignored,
        config.entries as u64
    );
    eprintln!("setup_ms={:.3} entries={} status_rounds={} completions={} occurrences={} schema={} sqlite={}",
        started.elapsed().as_secs_f64() * 1000., config.entries, config.rounds,
        config.rereads, totals.japanese_occurrences, storage.schema_version().unwrap(),
        rusqlite::version());
}

fn measure<T>(name: &str, iterations: usize, mut operation: impl FnMut() -> T) {
    drop(black_box(operation()));
    let samples = (0..iterations)
        .map(|_| {
            let start = Instant::now();
            drop(black_box(operation()));
            start.elapsed()
        })
        .collect();
    report(name, samples);
}

fn report(name: &str, samples: Vec<Duration>) {
    let mut sorted = samples.clone();
    sorted.sort_unstable();
    let count = sorted.len();
    let median = (sorted[(count - 1) / 2].as_secs_f64() + sorted[count / 2].as_secs_f64()) / 2.;
    let raw = samples
        .iter()
        .map(|sample| format!("{:.6}", sample.as_secs_f64() * 1000.))
        .collect::<Vec<_>>()
        .join(";");
    println!(
        "{name},{:.6},{:.6},{:.6},{raw}",
        sorted[0].as_secs_f64() * 1000.,
        median * 1000.,
        sorted[count - 1].as_secs_f64() * 1000.
    );
}

fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, value).unwrap();
    bytes
}

struct Session {
    bridge: YomokuBridge,
    callbacks: Receiver<(Instant, Vec<u8>)>,
}

impl Session {
    fn new(path: &Path) -> Self {
        let (sender, callbacks) = mpsc::channel();
        let (bridge, _) = YomokuBridge::with_effective_reader(
            path.into(),
            Box::new(|| panic!("Vocabulary must not load an analyzer")),
            Box::new(|| panic!("Vocabulary must not load a dictionary")),
            move |result| {
                sender.send((Instant::now(), result.unwrap())).unwrap();
            },
        )
        .unwrap();
        let session = Self { bridge, callbacks };
        session.dispatch(Event::SelectDestination(Destination::Vocabulary));
        session
    }

    fn dispatch(&self, event: Event) {
        let effects = self.bridge.update(&encode(&event)).unwrap();
        let _: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&effects).unwrap();
    }

    fn view(&self) -> ViewModel {
        BincodeFfiFormat::deserialize(&self.bridge.view().unwrap()).unwrap()
    }

    fn request(&self, event: Event) -> Duration {
        assert!(
            self.callbacks.try_recv().is_err(),
            "unexpected queued callback"
        );
        let event = encode(&event);
        let started = Instant::now();
        let effects = self.bridge.update(&event).unwrap();
        let _: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&effects).unwrap();
        let (finished, effects) = self
            .callbacks
            .recv_timeout(Duration::from_secs(30))
            .unwrap();
        let _: Requests<ShellEffectFfi> = BincodeFfiFormat::deserialize(&effects).unwrap();
        assert!(!self.view().storage_busy);
        finished.duration_since(started)
    }
}

fn bridge_measurements(path: &Path, config: &Config) {
    let session = Session::new(path);
    let mut first = Vec::new();
    let mut select_all = Vec::new();
    for index in 0..=config.iterations {
        // Navigation invalidates the loaded page and clears session selection.
        session.dispatch(Event::SelectDestination(Destination::Library));
        session.dispatch(Event::SelectDestination(Destination::Vocabulary));
        let load = session.request(Event::LoadVocabulary);
        assert_eq!(session.view().vocabulary.rows.len(), 100);
        let capture = session.request(Event::SelectAllVocabulary);
        let state = session.view().vocabulary;
        assert!(state.error.is_none() && state.bulk_error.is_none());
        assert_eq!(state.selection.as_ref().unwrap().len(), config.entries);
        if index > 0 {
            first.push(load);
            select_all.push(capture);
        }
    }
    report("fifo_first_page_callback", first);
    report("fifo_select_all_callback", select_all);

    // Measure the complete production ViewModel projection/encoding, including
    // retained rows and global selection, independently of repository/FIFO work.
    let mut targets = vec![100, config.entries.min(1_000)];
    targets.dedup();
    for target in targets {
        while session.view().vocabulary.rows.len() < target {
            session.request(Event::LoadMoreVocabulary);
        }
        session.dispatch(Event::RequestVocabularyBulkAction(
            VocabularyBulkAction::SetStatus {
                status: VocabularyStatus::Known,
                occurred_at_unix_ms: DATE + 100_000,
            },
        ));
        let state = session.view();
        let command = state.vocabulary.confirmation.as_ref().unwrap();
        assert_eq!(command.ids, *state.vocabulary.selection.as_ref().unwrap());
        let event = Event::ConfirmVocabularyBulkAction(command.clone());
        // Encode/decode confirmation without executing a mutation.
        let bytes = encode(&event);
        let decoded: Event = BincodeFfiFormat::deserialize(&bytes).unwrap();
        assert_eq!(decoded, event);
        eprintln!(
            "rows={target} selected={} view_bytes={} confirmation_event_bytes={}",
            config.entries,
            session.bridge.view().unwrap().len(),
            bytes.len()
        );
        measure(
            &format!("rows_{target}_view_project_encode"),
            config.iterations,
            || session.bridge.view().unwrap(),
        );
        let view_bytes = session.bridge.view().unwrap();
        measure(
            &format!("rows_{target}_view_decode_rust"),
            config.iterations,
            || BincodeFfiFormat::deserialize::<ViewModel>(&view_bytes).unwrap(),
        );
        measure(
            &format!("rows_{target}_confirm_encode"),
            config.iterations,
            || encode(&event),
        );
        session.dispatch(Event::CancelVocabularyBulkAction);
    }
    session.dispatch(Event::CancelVocabularySelection);
    assert!(session.view().vocabulary.selection.is_none());
}

fn main() {
    let config =
        Config::parse(&std::env::args().skip(1).collect::<Vec<_>>()).unwrap_or_else(|message| {
            eprintln!("{message}");
            std::process::exit(2);
        });
    let scratch = Scratch::new();
    let path = scratch.database();
    let mut storage = Storage::open(&path).unwrap();
    seed(&mut storage, &config);
    // Exact streaming comparison stays outside timings and uses bounded memory.
    let baseline = scratch.0.join("read-only-baseline.sqlite");
    let database_bytes = std::fs::copy(&path, &baseline).unwrap();
    let audit =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let integrity: String = audit
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert!(audit
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
    for table in [
        "vocabulary",
        "vocabulary_status_events",
        "reading_completions",
        "reading_occurrences",
        "vocabulary_dictionary_forms",
    ] {
        let count: i64 = audit
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        eprintln!("{table}_rows={count}");
    }
    drop(audit);
    eprintln!(
        "database_bytes={} profile={} arch={} os={}",
        database_bytes,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!("metric,min_ms,median_ms,max_ms,samples_ms");
    for sort in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
    ] {
        for (label, prefix) in [("all", ""), ("prefix", "語0000")] {
            let query = VocabularyQuery {
                sort,
                prefix: prefix.into(),
                ..Default::default()
            };
            let count = storage.vocabulary_page(&query, 0).unwrap().matching_count;
            let mut offsets = vec![0, count.saturating_sub(100)];
            offsets.dedup();
            for offset in offsets {
                measure(
                    &format!("repository_{sort:?}_{label}_offset_{offset}"),
                    config.iterations,
                    || storage.vocabulary_page(&query, offset).unwrap(),
                );
            }
        }
    }
    measure("repository_select_all", config.iterations, || {
        storage
            .vocabulary_matching_ids(&VocabularyQuery::default())
            .unwrap()
    });
    measure("repository_statistics_year", config.iterations, || {
        storage
            .statistics_buckets(StatisticsTimeframe::LastYear, DATE + 100_000)
            .unwrap()
    });
    bridge_measurements(&path, &config);
    assert_same_bytes(&path, &baseline);
    eprintln!("read_only=true scratch_cleanup=on_exit");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn invalid_arguments_fail_before_creating_a_fixture() {
        for values in [
            vec![],
            vec!["99", "1", "1", "1"],
            vec!["100", "101", "1", "1"],
            vec!["100", "1", "10001", "1"],
            vec!["100", "1", "1", "0"],
            vec!["100001", "1", "1", "1"],
            vec!["-1", "1", "1", "1"],
        ] {
            assert!(Config::parse(&arguments(&values)).is_err());
        }
        assert_eq!(
            Config::parse(&arguments(&["100", "0", "0", "2"])).unwrap(),
            Config {
                entries: 100,
                rounds: 0,
                rereads: 0,
                iterations: 2
            }
        );
    }

    #[test]
    fn production_history_fifo_and_frozen_prompts_preserve_bytes_and_cleanup() {
        let scratch = Scratch::new();
        let directory = scratch.0.clone();
        let path = scratch.database();
        let config = Config {
            entries: 105,
            rounds: 2,
            rereads: 2,
            iterations: 2,
        };
        let mut storage = Storage::open(&path).unwrap();
        seed(&mut storage, &config);
        let baseline = scratch.0.join("baseline.sqlite");
        std::fs::copy(&path, &baseline).unwrap();
        bridge_measurements(&path, &config);
        assert_same_bytes(&path, &baseline);
        drop(storage);
        drop(scratch);
        assert!(!directory.exists());
    }
}
