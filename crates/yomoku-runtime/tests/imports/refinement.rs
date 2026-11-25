use super::*;
use yomoku_core::{
    analysis::{ImportAnalysis, IMPORT_POLICY_SUFFIX, WORD_DICTIONARY_PREFIX},
    effective_session::EffectiveReaderSelection,
};
use yomoku_storage::dictionary::IMPORT_DICTIONARY_SHA256;

struct RealAnalyzer(yomoku_analysis::Analyzer);
impl PasteAnalyzer for RealAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        self.0
            .analyze(chapter)
            .map_err(|_| DocumentError::Unavailable)
    }
    fn analyze_for_import(
        &mut self,
        chapter: &ChapterText,
    ) -> Result<ImportAnalysis, DocumentError> {
        self.0
            .analyze_for_import(chapter)
            .map_err(|_| DocumentError::Unavailable)
    }
    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        self.0
            .analyze_components(chapter, mode, profile)
            .map_err(|_| DocumentError::Unavailable)
    }
}

fn real_loader() -> AnalyzerLoader {
    Box::new(|| {
        let path = std::env::var_os("YOMOKU_SUDACHI_DICTIONARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/sudachi/full-20261015/system_full.dic")
            });
        let analyzer = if std::env::var("YOMOKU_SUDACHI_STORAGE").as_deref() == Ok("mapped") {
            let file = std::fs::File::open(path).unwrap();
            // SAFETY: the real-data suite keeps the pinned asset immutable.
            unsafe { yomoku_analysis::Analyzer::open_full_mapped(&file) }
        } else {
            yomoku_analysis::Analyzer::open_full(path)
        }
        .unwrap();
        Ok(Box::new(RealAnalyzer(analyzer)))
    })
}

fn real_router(path: PathBuf) -> (EffectRouter, AnalyzerLifecycle) {
    let index = std::env::var_os("YOMOKU_WORD_DICTIONARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.local/dictionaries/index-v1-20261025/dictionary.sqlite3")
        });
    yomoku_runtime::effect_router(path, real_loader(), Box::new(move || Ok(index.clone()))).unwrap()
}

#[test]
#[ignore = "requires pinned Sudachi Full and word dictionaries; run tools/test-sudachi.sh"]
fn all_import_formats_refine_dictionary_missing_words_and_preserve_unicode_history_and_duplicates()
{
    let database = Database::new();
    let (router, lifecycle) = real_router(database.path());
    let text = concat!(
        "学生だった。制服姿。食べました。国家公務員。新幹線通学。𠮷か\u{3099}😀。制服姿。ひとつ。ふたつ。みっつ。一つ。二個。",
        "すごっ。端っこ。猫っぽかった。走って帰る。その日。動き出した。それって何だっけ。",
        "その 日。その、日。その。日。その日暮らし。日本語学校教育。その日。"
    );
    let events = [
        Event::CreatePaste(text.into()),
        Event::ImportTxt(TxtImport {
            original_bytes: text.as_bytes().to_vec(),
            filename: "refined.txt".into(),
            encoding: None,
        }),
        Event::ImportSrt(SrtImport {
            original_bytes: format!("1\n00:00:00,000 --> 00:00:01,000\n{text}\n").into_bytes(),
            filename: "refined.srt".into(),
            encoding: None,
        }),
        Event::ImportEpub(yomoku_core::imports::EpubImport {
            original_bytes: epub_fixture::book(false, &format!("<p>{text}</p>")),
            filename: "refined.epub".into(),
        }),
    ];
    let mut storage = Storage::open(database.path()).unwrap();
    for event in events {
        let view = dispatch_with_timeout(&router, event.clone(), Duration::from_secs(90));
        assert_eq!(view.document_error, None);
        let id = view.import_summary.unwrap().document_id;
        let document = storage.load_document_reader_snapshot(id).unwrap().unwrap();
        let mut occurrences = Vec::new();
        let mut checked_chapter = false;
        for (chapter, metadata) in document.document.chapters.iter().zip(&document.analyses) {
            let ReaderAnalysis::Available(analysis_id) = metadata.analysis else {
                panic!("analysis required");
            };
            let analysis = storage.chapter_analysis(analysis_id).unwrap().unwrap();
            analysis.validate(chapter).unwrap();
            assert!(analysis
                .profile
                .engine_revision
                .ends_with(IMPORT_POLICY_SUFFIX));
            assert!(analysis.profile.dictionary_revision.ends_with(&format!(
                "{WORD_DICTIONARY_PREFIX}{IMPORT_DICTIONARY_SHA256}"
            )));
            if chapter.text.contains(text) {
                checked_chapter = true;
                let japanese: Vec<_> = analysis
                    .tokens
                    .iter()
                    .filter(|t| t.is_japanese)
                    .map(|t| t.surface.as_str())
                    .collect();
                assert_eq!(
                    japanese,
                    [
                        "学生",
                        "だった",
                        "制服",
                        "姿",
                        "食べました",
                        "国家公務員",
                        "新幹線",
                        "通学",
                        "𠮷",
                        "か\u{3099}",
                        "制服",
                        "姿",
                        "ひとつ",
                        "ふたつ",
                        "みっつ",
                        "一つ",
                        // Dictionary-missing counted forms still undergo refinement.
                        "二",
                        "個",
                        "すごっ",
                        "端っこ",
                        "猫っぽかった",
                        "走って",
                        "帰る",
                        "その日",
                        "動き出した",
                        "それ",
                        "って",
                        "何",
                        "だ",
                        "っけ",
                        "その",
                        "日",
                        "その",
                        "日",
                        "その",
                        "日",
                        "その日暮らし",
                        "日本語",
                        "学校教育",
                        "その日",
                    ]
                );
                for token in &analysis.tokens {
                    assert_eq!(token.anchor.slice(chapter).unwrap(), token.surface);
                }
                let ate = analysis
                    .tokens
                    .iter()
                    .find(|t| t.surface == "食べました")
                    .unwrap();
                assert_eq!(ate.dictionary_form.as_ref().unwrap().as_str(), "食べる");
                let copula = analysis
                    .tokens
                    .iter()
                    .find(|t| t.surface == "だった")
                    .unwrap();
                assert_eq!(copula.dictionary_form.as_ref().unwrap().as_str(), "だ");
                assert_eq!(copula.reading.as_deref(), Some("ダッタ"));
                for surface in ["ひとつ", "ふたつ", "みっつ", "一つ"] {
                    let counted = analysis
                        .tokens
                        .iter()
                        .find(|t| t.surface == surface)
                        .unwrap();
                    assert_eq!(counted.dictionary_form.as_ref().unwrap().as_str(), surface);
                }
                for (surface, base, reading) in [
                    ("その日", "その日", "そのひ"),
                    ("動き出した", "動き出す", "うごきだす"),
                ] {
                    let token = analysis
                        .tokens
                        .iter()
                        .find(|t| t.surface == surface)
                        .unwrap();
                    assert_eq!(token.dictionary_form.as_ref().unwrap().as_str(), base);
                    assert_eq!(token.reading.as_deref(), Some(reading));
                }
            }
            occurrences.extend(
                analysis
                    .tokens
                    .iter()
                    .filter(|t| t.is_japanese)
                    .map(|t| (t.anchor, t.surface.clone())),
            );
        }
        assert!(checked_chapter);
        assert!(storage
            .vocabulary_entry(&Surface::new("制服姿").unwrap())
            .unwrap()
            .is_none());
        for (_, surface) in &occurrences {
            assert!(storage
                .vocabulary_entry(&Surface::new(surface).unwrap())
                .unwrap()
                .is_some());
        }
        let before = totals(&database.path());
        let duplicate = dispatch(&router, event);
        assert_eq!(
            duplicate.import_summary.unwrap().outcome,
            yomoku_core::app::ImportOutcome::Existing
        );
        assert_eq!(totals(&database.path()), before);
        let mut current = dispatch(&router, Event::OpenReader(id));
        let mut looked_up = [false; 4];
        loop {
            let page = current.reader.clone().unwrap();
            current = dispatch(
                &router,
                Event::LoadReaderTokens(yomoku_core::reader::ReaderTokenKey {
                    document_id: id,
                    page: page.anchor,
                    analysis: current.reader_analysis.unwrap(),
                }),
            );
            let loaded = current.effective_reader_tokens.unwrap();
            for ((surface, base, dictionary_entry), checked) in [
                ("端っこ", "端っこ", 2181850),
                ("走って", "走る", 1402540),
                ("その日", "その日", 2002320),
                ("動き出した", "動き出す", 2012540),
            ]
            .into_iter()
            .zip(&mut looked_up)
            {
                if let Some(token) = loaded.tokens.iter().find(|t| t.surface == surface) {
                    let details = settled(
                        &router,
                        Event::LoadEffectiveReaderWord(EffectiveReaderSelection {
                            page: loaded.identity,
                            segment_index: token.segment_index,
                        }),
                    )
                    .effective_reader_word
                    .unwrap()
                    .details
                    .unwrap();
                    assert_eq!(details.token.anchor, token.anchor);
                    assert_eq!(details.candidates.surface, surface);
                    assert_eq!(details.candidates.dictionary_form.as_deref(), Some(base));
                    assert!(
                        details
                            .candidates
                            .words
                            .iter()
                            .any(|entry| entry.entry_id == dictionary_entry
                                && entry.forms.iter().any(|form| form.form == base)),
                        "{surface}: {details:?}"
                    );
                    *checked = true;
                }
            }
            current = settled(
                &router,
                Event::CompleteEffectiveReaderPage {
                    identity: loaded.identity,
                    position: page.position,
                    completed_at_unix_ms: 1000,
                },
            );
            if current.reader.is_none() {
                break;
            }
        }
        assert!(looked_up.into_iter().all(|checked| checked));
        let sql = Connection::open(database.path()).unwrap();
        let mut statement = sql
            .prepare(
                "SELECT c.chapter_id, o.start_byte, o.end_byte, o.surface
            FROM reading_occurrences o JOIN reading_completions c USING(command_id)
            WHERE c.document_id=?1 ORDER BY c.command_id, o.token_index",
            )
            .unwrap();
        let saved: Vec<(i64, i64, i64, String)> = statement
            .query_map([i64::from(id)], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            saved,
            occurrences
                .into_iter()
                .map(|(anchor, surface)| {
                    (
                        i64::from(anchor.chapter_id),
                        i64::from(anchor.range.start),
                        i64::from(anchor.range.end),
                        surface,
                    )
                })
                .collect::<Vec<_>>()
        );
    }
    drop(router);
    drop(lifecycle);
}

fn settled(router: &EffectRouter, event: Event) -> ViewModel {
    let (send, receive) = mpsc::channel();
    router.update(event, move |_| {
        let _ = send.send(());
    });
    loop {
        receive.recv_timeout(TIMEOUT).unwrap();
        let view = router.view();
        if !view.storage_busy {
            assert_eq!(view.document_error, None);
            return view;
        }
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full and word dictionaries; run tools/test-sudachi.sh"]
fn japanese_grammar_boundaries_keep_written_spans_and_return_whole_word_definitions() {
    let database = Database::new();
    let (router, _lifecycle) = real_router(database.path());
    let text = concat!(
        "二枚しかないです。無いです。高くないです。食べないです。いいです。危ないです。なかったです。",
        "奇妙がられる。持って行った。持って行ってしまった。済ませて戻る。思いだせない。",
        "店員として働く。どこかで待つ。しなければならない。",
        "いいですか。どこか？どこ か。店員とし、て。店員としている。",
        "食べなければ帰る。食べている。"
    );
    let imported = dispatch_with_timeout(
        &router,
        Event::CreatePaste(text.into()),
        Duration::from_secs(90),
    );
    assert_eq!(imported.document_error, None);
    let id = imported.import_summary.unwrap().document_id;
    let opened = dispatch(&router, Event::OpenReader(id));
    let page = opened.reader.unwrap();
    let loaded = dispatch(
        &router,
        Event::LoadReaderTokens(yomoku_core::reader::ReaderTokenKey {
            document_id: id,
            page: page.anchor,
            analysis: opened.reader_analysis.unwrap(),
        }),
    )
    .effective_reader_tokens
    .unwrap();
    assert_eq!(
        loaded
            .tokens
            .iter()
            .map(|token| token.surface.as_str())
            .collect::<Vec<_>>(),
        [
            "二枚",
            "しか",
            "ない",
            "です",
            "無い",
            "です",
            "高くない",
            "です",
            "食べない",
            "です",
            "いいです",
            "危ないです",
            "なかったです",
            "奇妙",
            "がられる",
            "持って行った",
            "持って行ってしまった",
            "済ませ",
            "て",
            "戻る",
            "思いだせない",
            "店員",
            "として",
            "働く",
            "どこか",
            "で",
            "待つ",
            "し",
            "なければならない",
            "いいです",
            "か",
            "どこ",
            "か",
            "どこ",
            "か",
            "店員",
            "と",
            "し",
            "て",
            "店員",
            "と",
            "している",
            "食べなけれ",
            "ば",
            "帰る",
            "食べている",
        ]
    );
    for (surface, base, reading, entry_id, gloss) in [
        ("ない", "ない", "ナイ", 1529520, "nonexistent"),
        ("です", "です", "デス", 1628500, "be"),
        ("無い", "無い", "ナイ", 1529520, "nonexistent"),
        ("奇妙", "奇妙", "キミョウ", 1219490, "odd"),
        ("がられる", "がる", "ガラレル", 1631750, "to think"),
        (
            "持って行った",
            "持って行く",
            "モッテイッタ",
            1315700,
            "to take (something) along",
        ),
        (
            "持って行ってしまった",
            "持って行く",
            "モッテイッテシマッタ",
            1315700,
            "to take (something) along",
        ),
        ("済ませ", "済ませる", "スマセ", 1295040, "to finish"),
        (
            "思いだせない",
            "思い出す",
            "オモイダセナイ",
            1309260,
            "to remember",
        ),
        (
            "として",
            "として",
            "トシテ",
            1008590,
            "as (i.e. in the role of)",
        ),
        ("どこか", "どこか", "ドコカ", 1189000, "somewhere"),
        ("し", "する", "シ", 1157170, "to do"),
        (
            "なければならない",
            "なければならない",
            "ナケレバナラナイ",
            1632350,
            "have to do",
        ),
        ("食べなけれ", "食べる", "タベナケレ", 1358280, "to eat"),
        ("食べている", "食べる", "タベテイル", 1358280, "to eat"),
    ] {
        let token = loaded.tokens.iter().find(|t| t.surface == surface).unwrap();
        assert_eq!(token.display_range.slice(&page.text).unwrap(), surface);
        assert_eq!(token.reading.as_deref(), Some(reading), "{surface}");
        assert!(token.vocabulary.is_some(), "{surface}");
        let details = settled(
            &router,
            Event::LoadEffectiveReaderWord(EffectiveReaderSelection {
                page: loaded.identity,
                segment_index: token.segment_index,
            }),
        )
        .effective_reader_word
        .unwrap()
        .details
        .unwrap();
        assert_eq!(details.token.anchor, token.anchor);
        assert_eq!(details.candidates.surface, surface);
        assert_eq!(details.candidates.dictionary_form.as_deref(), Some(base));
        assert!(
            details
                .candidates
                .words
                .iter()
                .any(|word| word.entry_id == entry_id
                    && word
                        .forms
                        .iter()
                        .filter(|form| form.form == base)
                        .flat_map(|form| &form.matches)
                        .flat_map(|matched| &matched.sense_indices)
                        .any(|&index| word.entry.senses[index]
                            .glosses
                            .iter()
                            .any(|g| g.text == gloss))),
            "{surface} should return {gloss}: {:?}",
            details.candidates
        );
        router.update(Event::DismissReaderWord, |_| {});
    }
}

#[test]
fn dictionary_load_and_later_membership_failures_roll_back_each_format_and_duplicates_skip_assets()
{
    use std::sync::atomic::AtomicBool;
    fn schema(path: &PathBuf) {
        Connection::open(path).unwrap().execute_batch("PRAGMA application_id=0x594f4449; PRAGMA user_version=1;
            CREATE TABLE IF NOT EXISTS entries(source TEXT, entry_id INTEGER, payload TEXT);
            CREATE TABLE IF NOT EXISTS forms(surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER);").unwrap();
    }
    let events = [
        Event::CreatePaste("猫".into()),
        Event::ImportTxt(TxtImport {
            original_bytes: "猫".as_bytes().to_vec(),
            filename: "retry.txt".into(),
            encoding: Some(TextEncoding::Utf8),
        }),
        Event::ImportSrt(SrtImport {
            original_bytes: "1\n00:00:00,000 --> 00:00:01,000\n猫\n".as_bytes().to_vec(),
            filename: "retry.srt".into(),
            encoding: Some(TextEncoding::Utf8),
        }),
        Event::ImportEpub(yomoku_core::imports::EpubImport {
            original_bytes: epub_fixture::book(false, "<p>猫</p>"),
            filename: "retry.epub".into(),
        }),
    ];
    for event in events {
        let database = Database::new();
        let index = database.0.join("words.sqlite3");
        schema(&index);
        let unavailable = Arc::new(AtomicBool::new(true));
        let corrupt = Arc::new(AtomicBool::new(true));
        let loads = Arc::new(AtomicUsize::new(0));
        let word_path = index.clone();
        let fail = unavailable.clone();
        let calls = loads.clone();
        let analyzer_path = index.clone();
        let damage = corrupt.clone();
        let loader: AnalyzerLoader = Box::new(move || {
            let path = analyzer_path.clone();
            let damage = damage.clone();
            Ok(Box::new(move |chapter: &ChapterText| {
                let analysis = fixture(chapter);
                // Fault injection mutates only this synthetic index. Production
                // publisher bytes remain immutable. EPUB fails in its later chapter.
                let sql = Connection::open(&path).unwrap();
                if chapter.text.trim() == "猫" && damage.load(Ordering::Relaxed) {
                    sql.execute_batch("DROP TABLE forms").unwrap();
                } else {
                    for token in &analysis.tokens {
                        sql.execute("INSERT INTO forms VALUES (?1,'JMdict',1)", [&token.surface])
                            .unwrap();
                    }
                }
                Ok(analysis)
            }))
        });
        let (router, lifecycle) = yomoku_runtime::effect_router(
            database.path(),
            loader,
            Box::new(move || {
                calls.fetch_add(1, Ordering::Relaxed);
                if fail.load(Ordering::Relaxed) {
                    return Err(DocumentError::Unavailable);
                }
                // Acquiring a second writer here proves asset loading precedes the
                // user-database write transaction.
                let sql = Connection::open(word_path.with_file_name("import.sqlite3")).unwrap();
                sql.execute_batch("BEGIN IMMEDIATE; ROLLBACK").unwrap();
                Ok(word_path.clone())
            }),
        )
        .unwrap();
        let baseline = {
            Storage::open(database.path()).unwrap();
            totals(&database.path())
        };
        assert_eq!(
            dispatch(&router, event.clone()).document_error,
            Some(DocumentError::Unavailable)
        );
        assert_eq!(totals(&database.path()), baseline);
        unavailable.store(false, Ordering::Relaxed);
        assert_eq!(
            dispatch(&router, event.clone()).document_error,
            Some(DocumentError::Unavailable)
        );
        assert_eq!(totals(&database.path()), baseline);
        corrupt.store(false, Ordering::Relaxed);
        schema(&index);
        let created = dispatch(&router, event.clone());
        assert_eq!(created.document_error, None);
        assert_eq!(created.documents.len(), 1);
        unavailable.store(true, Ordering::Relaxed);
        let before = loads.load(Ordering::Relaxed);
        let duplicate = dispatch(&router, event);
        assert_eq!(duplicate.document_error, None);
        assert_eq!(
            duplicate.import_summary.unwrap().outcome,
            yomoku_core::app::ImportOutcome::Existing
        );
        assert_eq!(loads.load(Ordering::Relaxed), before);
        drop(router);
        drop(lifecycle);
    }
}
