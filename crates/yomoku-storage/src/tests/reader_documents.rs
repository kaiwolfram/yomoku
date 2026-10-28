use yomoku_core::{
    active_time::PendingReadingTime,
    completion::{AnalyzedCompletionOutcome, AnalyzedPageCompletion, PageCompletionIntent},
    document::{DocumentId, PasteText, ReaderDocument, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    reader::{DocumentReaderSnapshot, ReaderAnalysis, ReaderPage},
    reader_formatting::ReaderTextStyle,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn import(storage: &mut Storage, second: &str) -> AnalyzedEpub {
    match storage
        .import_analyzed_epub(&epub_fixture::book(true, second), "本.epub", 10, fixture)
        .unwrap()
    {
        EpubImportOutcome::Created(result) => *result,
        _ => panic!("expected new EPUB"),
    }
}

fn snapshot(storage: &mut Storage, id: DocumentId) -> DocumentReaderSnapshot {
    storage.load_document_reader_snapshot(id).unwrap().unwrap()
}

fn position(document: &ReaderDocument, index: usize, offset: usize) -> ReadingPosition {
    let chapter = &document.chapters[index];
    ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::at(&chapter.text, offset).unwrap(),
    }
}

fn prepare(storage: &mut Storage, id: DocumentId) -> AnalyzedPageCompletion {
    let snapshot = snapshot(storage, id);
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    let ReaderAnalysis::Available(analysis_id) = snapshot
        .analyses
        .iter()
        .find(|analysis| analysis.chapter_id == page.anchor.chapter_id)
        .unwrap()
        .analysis
    else {
        panic!("missing fixture analysis")
    };
    storage
        .prepare_page_completion(PageCompletionIntent {
            document_id: id,
            page: page.anchor,
            page_length: snapshot.page_length,
            departing_position: page.position,
            completed_at_unix_ms: 100,
            analysis_id,
        })
        .unwrap()
}

fn record(storage: &mut Storage, command: AnalyzedPageCompletion) -> AnalyzedCompletionOutcome {
    storage
        .record_analyzed_page_completion(command.completion, command.analysis_id)
        .unwrap()
}

fn checkpoint(
    storage: &mut Storage,
    document: &ReaderDocument,
    position: ReadingPosition,
    ms: i64,
) {
    storage
        .save_document_reading_position(document.document_id, position)
        .unwrap();
    let token = storage
        .load_pending_reading_time(document.document_id, position)
        .unwrap();
    let saved = storage
        .save_pending_reading_time(token, PendingReadingTime::from_millis(ms).unwrap())
        .unwrap();
    assert_eq!(
        storage
            .save_pending_reading_time(token, PendingReadingTime::from_millis(ms).unwrap())
            .unwrap(),
        saved
    );
}

fn counts(storage: &Storage) -> Vec<i64> {
    [
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "vocabulary_status_events",
        "completion_commands",
    ]
    .map(|table| {
        storage
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    })
    .into()
}

#[test]
fn all_formats_have_complete_read_only_snapshots_with_original_analysis_ids() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let paste = storage
        .create_analyzed_paste(&PasteText::new("猫 𠮷\0".into()).unwrap(), 1, fixture)
        .unwrap();
    let txt = match storage
        .import_analyzed_txt(
            "猫 犬".as_bytes(),
            "本.txt",
            Some(yomoku_import::txt::TextEncoding::Utf8),
            2,
            fixture,
        )
        .unwrap()
    {
        TxtImportOutcome::Created(result) => result,
        _ => panic!("expected TXT"),
    };
    let srt = match storage
        .import_analyzed_srt(
            b"1\n00:00:00,000 --> 00:00:01,000\nEnglish",
            "本.srt",
            None,
            3,
            fixture,
        )
        .unwrap()
    {
        SrtImportOutcome::Created(result) => result,
        _ => panic!("expected SRT"),
    };
    let epub = import(&mut storage, "<p>犬 犬 𠮷 か&#x3099;</p>");
    let missing = storage
        .create_paste(&PasteText::new("旧本".into()).unwrap())
        .unwrap();
    storage.save_page_length(PageLength::Characters750).unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for imported in [&paste, &txt, &srt] {
        let snapshot = snapshot(&mut storage, imported.document.document_id);
        assert_eq!(
            snapshot.document,
            ReaderDocument::from(imported.document.clone())
        );
        assert_eq!(snapshot.page_length, PageLength::Characters750);
        assert_eq!(snapshot.analyses.len(), 1);
        assert_eq!(
            snapshot.analyses[0].analysis,
            ReaderAnalysis::Available(imported.analysis_id)
        );
        assert_eq!(
            snapshot.analyses[0].chapter_id,
            imported.document.chapter.chapter_id
        );
    }
    let snapshot = snapshot(&mut storage, epub.document.document_id);
    snapshot.validate().unwrap();
    let first = &snapshot.document.chapters[0];
    let styles = &snapshot.document.formatting[0];
    assert_eq!(styles.chapter_id, first.chapter_id);
    assert_eq!(styles.spans.len(), 4); // Inert internal link has no reader annotation.
    for (style, expected) in [
        (ReaderTextStyle::Heading(1), "第一章 出会い"),
        (ReaderTextStyle::Emphasis, "出会い"),
        (ReaderTextStyle::Strong, "𠮷😀"),
        (
            ReaderTextStyle::ExternalLink("https://example.org/book?q=1&x=2".into()),
            "外部",
        ),
    ] {
        let span = styles
            .spans
            .iter()
            .find(|span| span.style == style)
            .unwrap();
        assert_eq!(span.range.slice(&first.text).unwrap(), expected);
    }
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    assert_eq!(page.formatting, styles.spans);
    let illustrations = &snapshot.document.images;
    assert_eq!(illustrations.len(), 1);
    assert_eq!(illustrations[0].chapter_id, first.chapter_id);
    assert_eq!(illustrations[0].images.len(), 1);
    let image = &illustrations[0].images[0];
    assert_eq!(image.range.slice(&first.text).unwrap(), "\u{fffc}");
    assert_eq!(image.asset.path, "OPS/images/挿絵.png");
    assert_eq!(image.asset.media_type, "image/png");
    assert_eq!(image.alt, "挿絵");
    assert_eq!(page.images, illustrations[0].images);
    assert!(snapshot.document.formatting[1].spans.is_empty());
    assert_eq!(
        snapshot.document.chapters,
        epub.document
            .chapters
            .iter()
            .map(|item| item.chapter.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        snapshot
            .analyses
            .iter()
            .map(|item| item.analysis)
            .collect::<Vec<_>>(),
        epub.analysis_ids
            .iter()
            .copied()
            .map(ReaderAnalysis::Available)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        storage
            .load_document_reader_snapshot(missing.document_id)
            .unwrap()
            .unwrap()
            .analyses[0]
            .analysis,
        ReaderAnalysis::Missing
    );
    assert!(storage
        .load_document_reader_snapshot(DocumentId::try_from(i64::MAX).unwrap())
        .unwrap()
        .is_none());
    assert_eq!(fs::read(database.path()).unwrap(), before);
    // The legacy wire shape continues to reject multi-chapter books explicitly.
    assert!(matches!(
        storage.load_reader_snapshot(epub.document.document_id),
        Err(StorageError::InvalidDocument)
    ));
}

#[test]
fn source_ruby_survives_txt_epub_reopen_reflow_and_position_saves_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let txt = match storage
        .import_analyzed_txt(
            "😀｜𠮷野《よしの》 か\u{3099} ｜猫《ソースねこ》\0終".as_bytes(),
            "ruby.txt",
            Some(yomoku_import::txt::TextEncoding::Utf8),
            1,
            fixture,
        )
        .unwrap()
    {
        TxtImportOutcome::Created(imported) => imported.document.document_id,
        _ => panic!("expected TXT"),
    };
    let epub = import(
        &mut storage,
        "<p>😀<ruby>𠮷野<rt>ソースよしの</rt></ruby> か&#x3099; <ruby>猫<rt>ねこ</rt></ruby></p>",
    )
    .document
    .document_id;
    for id in [txt, epub] {
        let loaded = snapshot(&mut storage, id);
        let document = &loaded.document;
        let last = document.chapters.len() - 1;
        let annotations = document
            .source_ruby
            .iter()
            .find(|r| r.chapter_id == document.chapters[last].chapter_id)
            .unwrap();
        assert_eq!(annotations.ruby.len(), 2);
        assert_eq!(
            annotations.ruby[0]
                .range
                .slice(&document.chapters[last].text)
                .unwrap(),
            "𠮷野"
        );
        assert_eq!(
            annotations.ruby[1]
                .range
                .slice(&document.chapters[last].text)
                .unwrap(),
            "猫"
        );
        assert_eq!(
            annotations.ruby[0].reading,
            if id == txt {
                "よしの"
            } else {
                "ソースよしの"
            }
        );
        let exact = ReadingPosition {
            offset: annotations.ruby[0].range.end,
            ..position(document, last, 0)
        };
        let saved = storage
            .save_document_reading_position(id, exact)
            .unwrap()
            .unwrap();
        assert_eq!(saved.source_ruby, document.source_ruby);
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        let before_counts = counts(&storage);
        for length in PageLength::ALL {
            storage.save_page_length(length).unwrap();
            let before = fs::read(database.path()).unwrap();
            storage
                .connection
                .pragma_update(None, "query_only", true)
                .unwrap();
            let current = snapshot(&mut storage, id);
            current.validate().unwrap();
            assert_eq!(current.document, saved);
            assert_eq!(current.analyses, loaded.analyses);
            let page = ReaderPage::open_document(&current.document, length).unwrap();
            assert_eq!(page.position, exact);
            assert_eq!(page.source_ruby, annotations.ruby);
            let tokens = storage
                .load_reader_tokens(id, page.anchor)
                .unwrap()
                .unwrap();
            assert_eq!(tokens.page, page.anchor);
            assert_eq!(counts(&storage), before_counts);
            assert_eq!(fs::read(database.path()).unwrap(), before);
            storage
                .connection
                .pragma_update(None, "query_only", false)
                .unwrap();
        }
        assert_eq!(
            storage
                .open_document_reader_snapshot(id)
                .unwrap()
                .unwrap()
                .document,
            saved
        );
    }
}

#[test]
fn source_ruby_is_independent_of_status_deleted_vocabulary_and_missing_analysis() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "<p><ruby>猫<rt>ソースねこ</rt></ruby></p>");
    let id = imported.document.document_id;
    let document = snapshot(&mut storage, id).document;
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    for (index, status) in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ]
    .into_iter()
    .enumerate()
    {
        storage
            .set_vocabulary_status(cat.id, status, 100 + index as i64)
            .unwrap();
        assert_eq!(
            snapshot(&mut storage, id).document.source_ruby,
            document.source_ruby
        );
    }
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE chapter_id = ?1",
            [i64::from(document.chapters[1].chapter_id)],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let current = snapshot(&mut storage, id);
    assert_eq!(current.document.source_ruby, document.source_ruby);
    assert_eq!(current.analyses[1].analysis, ReaderAnalysis::Missing);
    let page = ReaderPage::at_document(
        &current.document,
        position(&document, 1, 0),
        PageLength::default(),
    )
    .unwrap();
    assert_eq!(page.source_ruby[0].reading, "ソースねこ");
    assert!(storage
        .load_reader_tokens(id, page.anchor)
        .unwrap()
        .unwrap()
        .tokens
        .is_empty());
    assert!(storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .is_none());
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn corrupted_txt_source_ruby_fails_reader_load_and_position_save_without_repair() {
    for sql in [
        "UPDATE source_ruby SET start_byte = 1",
        "UPDATE source_ruby SET end_byte = 999",
        "UPDATE source_ruby SET ruby_index = 1",
        "UPDATE source_ruby SET reading = ''",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let id = match storage
            .import_analyzed_txt("｜猫《ねこ》".as_bytes(), "ruby.txt", None, 1, fixture)
            .unwrap()
        {
            TxtImportOutcome::Created(imported) => imported.document.document_id,
            _ => panic!("expected TXT"),
        };
        let document = snapshot(&mut storage, id).document;
        storage
            .connection
            .execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        storage.connection.execute_batch(sql).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.load_document_reader_snapshot(id),
            Err(StorageError::InvalidDocument)
        ));
        assert!(matches!(
            storage.save_document_reading_position(id, position(&document, 0, 0)),
            Err(StorageError::InvalidDocument)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn shared_image_references_survive_chapter_movement_reflow_and_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "<p>😀前<img src='../images/挿絵.png' alt='別の説明 𠮷'/>後</p><p><img src='../images/挿絵.png'/></p>");
    let id = imported.document.document_id;
    let document = snapshot(&mut storage, id).document;
    assert_eq!(document.images.len(), 2);
    let before_counts = counts(&storage);
    let second = &document.images[1];
    assert_eq!(second.chapter_id, document.chapters[1].chapter_id);
    assert_eq!(second.images.len(), 2);
    assert_eq!(second.images[0].asset, document.images[0].images[0].asset);
    assert_eq!(second.images[1].asset, second.images[0].asset);
    assert_eq!(second.images[0].alt, "別の説明 𠮷");
    assert_eq!(second.images[1].alt, "");
    let exact = ReadingPosition {
        offset: second.images[0].range.start,
        ..position(&document, 1, 0)
    };
    let saved = storage
        .save_document_reading_position(id, exact)
        .unwrap()
        .unwrap();
    assert_eq!(saved.images, document.images);
    assert_eq!(saved.formatting, document.formatting);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    for length in PageLength::ALL {
        storage.save_page_length(length).unwrap();
        let before = fs::read(database.path()).unwrap();
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        let loaded = snapshot(&mut storage, id);
        assert_eq!(loaded.document, saved);
        let page = ReaderPage::open_document(&loaded.document, loaded.page_length).unwrap();
        assert_eq!(page.position, exact);
        assert_eq!(page.images, second.images);
        for image in &page.images {
            assert_eq!(image.range.slice(&page.text).unwrap(), "\u{fffc}");
        }
        assert_eq!(fs::read(database.path()).unwrap(), before);
        storage
            .connection
            .pragma_update(None, "query_only", false)
            .unwrap();
    }
    let reopened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    assert_eq!(reopened.document, saved);
    assert_eq!(counts(&storage), before_counts);
}

#[test]
fn all_presets_complete_chapters_in_order_with_exact_time_tokens_progress_and_finish() {
    for length in PageLength::ALL {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(
            &mut storage,
            &format!("<p>{}</p><p>𠮷 か&#x3099; 犬</p>", "猫 犬 。 ".repeat(850)),
        );
        let id = imported.document.document_id;
        storage.save_page_length(length).unwrap();
        let document = snapshot(&mut storage, id).document;
        let dog = storage
            .vocabulary_entry(&Surface::new("犬").unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(dog.id, VocabularyStatus::Learning, 11)
            .unwrap();
        let pages = LogicalPages::new(&document.chapters, length)
            .unwrap()
            .pages()
            .to_vec();
        let mut occurrences = 0;
        let mut known = 0;
        let mut final_command = None;
        for (index, anchor) in pages.iter().enumerate() {
            let before = snapshot(&mut storage, id);
            let page = ReaderPage::open_document(&before.document, length).unwrap();
            assert_eq!(page.anchor, *anchor);
            assert_eq!(page.page_number, index as u64 + 1);
            checkpoint(&mut storage, &before.document, page.position, 123);
            let tokens = storage.load_reader_tokens(id, *anchor).unwrap().unwrap();
            let chapter_index = document
                .chapters
                .iter()
                .position(|chapter| chapter.chapter_id == anchor.chapter_id)
                .unwrap();
            assert_eq!(
                tokens.analysis,
                ReaderAnalysis::Available(imported.analysis_ids[chapter_index])
            );
            occurrences += tokens
                .tokens
                .iter()
                .filter(|token| token.anchor.range.start >= anchor.range.start)
                .count() as i64;
            let command = prepare(&mut storage, id);
            assert_eq!(command.completion.pending_time.milliseconds(), 123);
            let outcome = record(&mut storage, command);
            let AnalyzedCompletionOutcome::Recorded { newly_known } = outcome else {
                panic!("expected new completion")
            };
            known += newly_known;
            let after = snapshot(&mut storage, id);
            let last_page = index == pages.len() - 1;
            assert_eq!(after.document.finished_at_least_once, last_page);
            assert_eq!(
                storage.list_documents().unwrap()[0],
                after.document.library_summary().unwrap()
            );
            assert_eq!(
                storage
                    .pending_page_time(id, *anchor, length)
                    .unwrap()
                    .milliseconds(),
                0
            );
            assert_eq!(
                record(&mut storage, command),
                AnalyzedCompletionOutcome::AlreadyRecorded
            );
            assert_eq!(snapshot(&mut storage, id), after);
            final_command = Some(command);
        }
        assert!(known > 0);
        assert_eq!(
            storage
                .vocabulary_entry(&Surface::new("犬").unwrap())
                .unwrap()
                .unwrap()
                .status,
            VocabularyStatus::Learning
        );
        assert_eq!(counts(&storage)[2], occurrences);
        let total_time: i64 = storage
            .connection
            .query_row(
                "SELECT SUM(active_time_ms) FROM reading_completions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(total_time, pages.len() as i64 * 123);
        let completed = snapshot(&mut storage, id);
        assert_eq!(
            completed
                .document
                .library_summary()
                .unwrap()
                .progress_percent,
            100
        );
        let before = counts(&storage);
        storage
            .save_document_reading_position(id, position(&document, 0, 0))
            .unwrap();
        assert_eq!(counts(&storage), before);
        assert!(storage.list_documents().unwrap()[0].finished_at_least_once);
        assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
        let reread = prepare(&mut storage, id);
        assert_eq!(
            record(&mut storage, reread),
            AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
        );
        assert!(counts(&storage)[2] > occurrences);
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        let reopened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        assert_eq!(
            reopened.document.reading_position,
            Some(position(&document, 0, 0))
        );
        assert!(reopened.document.finished_at_least_once);
        assert_eq!(
            record(&mut storage, final_command.unwrap()),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        assert_eq!(snapshot(&mut storage, id), reopened);
        storage
            .connection
            .execute("DELETE FROM documents WHERE id = ?1", [i64::from(id)])
            .unwrap();
        assert_eq!(
            record(&mut storage, final_command.unwrap()),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        assert!(storage.load_document_reader_snapshot(id).unwrap().is_none());
    }
}

#[test]
fn chapter_local_time_survives_reflow_and_only_departing_chapter_is_consumed() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "<p>猫 犬 𠮷</p>");
    let id = imported.document.document_id;
    let document = snapshot(&mut storage, id).document;
    let first_eof = position(&document, 0, document.chapters[0].text.len());
    let second_start = position(&document, 1, 0);
    checkpoint(&mut storage, &document, first_eof, 111);
    checkpoint(&mut storage, &document, second_start, 222);
    for length in PageLength::ALL {
        storage.save_page_length(length).unwrap();
        let pages = LogicalPages::new(&document.chapters, length).unwrap();
        assert_eq!(
            storage
                .pending_page_time(id, pages.pages()[0], length)
                .unwrap()
                .milliseconds(),
            111
        );
        assert_eq!(
            storage
                .pending_page_time(id, pages.pages()[1], length)
                .unwrap()
                .milliseconds(),
            222
        );
        assert_eq!(
            snapshot(&mut storage, id).document.reading_position,
            Some(second_start)
        );
    }
    storage
        .save_document_reading_position(id, first_eof)
        .unwrap();
    let command = prepare(&mut storage, id);
    assert_eq!(command.completion.pending_time.milliseconds(), 111);
    record(&mut storage, command);
    assert_eq!(
        snapshot(&mut storage, id).document.reading_position,
        Some(second_start)
    );
    assert!(!snapshot(&mut storage, id).document.finished_at_least_once);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        prepare(&mut storage, id)
            .completion
            .pending_time
            .milliseconds(),
        222
    );
    let second = prepare(&mut storage, id);
    record(&mut storage, second);
    assert!(snapshot(&mut storage, id).document.finished_at_least_once);
}

#[test]
fn foreign_chapters_scalar_splits_wrong_analysis_and_missing_analysis_fail_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let a = import(&mut storage, "<p>猫 犬 𠮷</p>");
    let b = import(&mut storage, "<p>別の本</p>");
    let id = a.document.document_id;
    let document = snapshot(&mut storage, id).document;
    let other = snapshot(&mut storage, b.document.document_id).document;
    let command = prepare(&mut storage, id);
    let before = counts(&storage);
    for bad in [
        position(&other, 0, 0),
        ReadingPosition {
            offset: 1.try_into().unwrap(),
            ..position(&document, 0, 0)
        },
    ] {
        assert!(storage.save_document_reading_position(id, bad).is_err());
        assert!(storage.load_pending_reading_time(id, bad).is_err());
    }
    let foreign = ReaderPage::open_document(&other, PageLength::default()).unwrap();
    assert!(storage.load_reader_tokens(id, foreign.anchor).is_err());
    assert!(storage
        .pending_page_time(id, foreign.anchor, PageLength::default())
        .is_err());
    for wrong in [a.analysis_ids[1], b.analysis_ids[0]] {
        assert!(matches!(
            storage.record_analyzed_page_completion(command.completion, wrong),
            Err(StorageError::InvalidAnalysis)
        ));
    }
    assert_eq!(counts(&storage), before);
    assert_eq!(snapshot(&mut storage, id).document, document);
    // Missing is per chapter. Reads expose absence; completion cannot invent B.
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(a.analysis_ids[1])],
        )
        .unwrap();
    let missing = snapshot(&mut storage, id);
    assert_eq!(
        missing.analyses[0].analysis,
        ReaderAnalysis::Available(a.analysis_ids[0])
    );
    assert_eq!(missing.analyses[1].analysis, ReaderAnalysis::Missing);
    let second =
        ReaderPage::at_document(&document, position(&document, 1, 0), PageLength::default())
            .unwrap();
    let tokens = storage
        .load_reader_tokens(id, second.anchor)
        .unwrap()
        .unwrap();
    assert_eq!(tokens.analysis, ReaderAnalysis::Missing);
    assert!(tokens.tokens.is_empty());
    record(&mut storage, command);
    let second_command = yomoku_core::completion::PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        page: second.anchor,
        departing_position: second.position,
        ..command.completion
    };
    let before = snapshot(&mut storage, id);
    assert!(storage
        .record_analyzed_page_completion(second_command, a.analysis_ids[1])
        .is_err());
    assert_eq!(snapshot(&mut storage, id), before);
}

#[test]
fn completion_write_and_commit_failures_roll_back_time_history_status_position_and_finish() {
    for table in [
        "reading_completions",
        "completion_analyses",
        "vocabulary_status_events",
        "reading_occurrences",
        "pending_reading_time",
        "reading_positions",
        "documents",
        "deferred",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "<p>犬 犬</p>");
        let id = imported.document.document_id;
        let document = snapshot(&mut storage, id).document;
        checkpoint(&mut storage, &document, position(&document, 1, 0), 456);
        let command = prepare(&mut storage, id);
        let before = snapshot(&mut storage, id);
        let before_counts = counts(&storage);
        let before_vocab = storage
            .vocabulary_entry(&Surface::new("犬").unwrap())
            .unwrap();
        if table == "deferred" {
            storage.connection.execute_batch("CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_reader AFTER UPDATE ON documents BEGIN INSERT INTO deferred_failure VALUES (-1); END;").unwrap();
        } else {
            let operation = if matches!(table, "pending_reading_time" | "documents") {
                "UPDATE"
            } else {
                "INSERT"
            };
            storage.connection.execute_batch(&format!("CREATE TRIGGER fail_reader BEFORE {operation} ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        assert!(
            matches!(
                storage.record_analyzed_page_completion(command.completion, command.analysis_id),
                Err(StorageError::Sqlite(_))
            ),
            "{table}"
        );
        assert_eq!(snapshot(&mut storage, id), before, "{table}");
        assert_eq!(counts(&storage), before_counts, "{table}");
        assert_eq!(
            storage
                .vocabulary_entry(&Surface::new("犬").unwrap())
                .unwrap(),
            before_vocab
        );
        assert_eq!(
            storage
                .pending_page_time(id, command.completion.page, command.completion.page_length)
                .unwrap()
                .milliseconds(),
            456
        );
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_reader")
            .unwrap();
        assert_eq!(
            record(&mut storage, command),
            AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
        );
        assert!(snapshot(&mut storage, id).document.finished_at_least_once);
    }
}

#[test]
fn explicit_finished_open_rolls_back_on_reset_analysis_or_commit_failure_then_retries() {
    for failure in ["write", "commit", "analysis"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "<p>犬</p>");
        let id = imported.document.document_id;
        let document = snapshot(&mut storage, id).document;
        storage
            .save_document_reading_position(id, position(&document, 1, 0))
            .unwrap();
        let command = prepare(&mut storage, id);
        record(&mut storage, command);
        let before = snapshot(&mut storage, id);
        match failure {
            "write" => storage.connection.execute_batch("CREATE TRIGGER fail_open BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap(),
            "commit" => storage.connection.execute_batch("CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_open AFTER UPDATE ON reading_positions BEGIN INSERT INTO deferred_failure VALUES (-1); END;").unwrap(),
            _ => { storage.connection.execute("UPDATE chapter_analyses SET engine_revision = ' ' WHERE id = ?1", [i64::from(imported.analysis_ids[1])]).unwrap(); },
        }
        assert!(storage.open_document_reader_snapshot(id).is_err());
        if failure == "analysis" {
            storage.connection.execute("UPDATE chapter_analyses SET engine_revision = 'import-fixture@1' WHERE id = ?1", [i64::from(imported.analysis_ids[1])]).unwrap();
        } else {
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_open")
                .unwrap();
        }
        assert_eq!(snapshot(&mut storage, id), before);
        let opened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        assert_eq!(
            opened.document.reading_position,
            Some(position(&document, 0, 0))
        );
        assert!(opened.document.finished_at_least_once);
        storage.connection.execute_batch("CREATE TRIGGER fail_open BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        let repeated = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        assert_eq!(
            repeated.last_read_sequence,
            Some(opened.last_read_sequence.unwrap() + 1)
        );
        assert_eq!(
            repeated,
            DocumentReaderSnapshot {
                last_read_sequence: repeated.last_read_sequence,
                ..opened
            }
        );
    }
}

#[test]
fn invalid_saved_book_content_or_extra_single_text_chapters_never_truncate_or_repair() {
    for damage in [
        "chapter",
        "text",
        "position",
        "asset",
        "image_range",
        "image_asset",
        "extra",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "<p>犬</p>");
        let mut id = imported.document.document_id;
        let document = snapshot(&mut storage, id).document;
        match damage {
            "chapter" => {
                storage
                    .connection
                    .execute(
                        "DELETE FROM chapter_texts WHERE chapter_id = ?1",
                        [i64::from(document.chapters[1].chapter_id)],
                    )
                    .unwrap();
            }
            "text" => {
                storage
                    .connection
                    .execute(
                        "UPDATE chapter_texts SET text = '破損' WHERE chapter_id = ?1",
                        [i64::from(document.chapters[1].chapter_id)],
                    )
                    .unwrap();
            }
            "position" => {
                crate::documents::write_position_in(
                    &storage.connection,
                    id,
                    ReadingPosition {
                        offset: 1.try_into().unwrap(),
                        ..position(&document, 1, 0)
                    },
                )
                .unwrap();
            }
            "asset" => {
                storage
                    .connection
                    .execute(
                        "UPDATE epub_assets SET bytes = X'00' WHERE document_id = ?1",
                        [i64::from(id)],
                    )
                    .unwrap();
            }
            "image_range" => {
                storage.connection.execute(
                    "UPDATE epub_inline_images SET start_byte = start_byte + 1 WHERE document_id = ?1",
                    [i64::from(id)],
                ).unwrap();
            }
            "image_asset" => {
                // A valid FK may still reference a different asset's corrupt
                // metadata. The complete reader load must reject the book.
                storage.connection.execute(
                    "UPDATE epub_assets SET media_type = 'text/html' WHERE document_id = ?1 AND path = 'OPS/images/挿絵.png'",
                    [i64::from(id)],
                ).unwrap();
            }
            _ => {
                let paste = storage
                    .create_paste(&PasteText::new("猫".into()).unwrap())
                    .unwrap();
                id = paste.document_id;
                storage
                    .connection
                    .execute(
                        "INSERT INTO chapters (source_id, chapter_index) VALUES (?1, 1)",
                        [i64::from(paste.chapter.source_id)],
                    )
                    .unwrap();
            }
        }
        let before = fs::read(database.path()).unwrap();
        assert!(
            storage.load_document_reader_snapshot(id).is_err(),
            "{damage}"
        );
        assert!(
            storage.open_document_reader_snapshot(id).is_err(),
            "{damage}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before, "{damage}");
    }
}

#[test]
fn image_only_final_chapter_completes_with_zero_new_words() {
    use yomoku_core::analysis::ChapterAnalysis;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    // The synthetic word fixture must also classify image placeholders correctly.
    let imported = match storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p><img src='../images/挿絵.png'/></p>"),
            "images.epub",
            1,
            |chapter| {
                let mut analysis = fixture(chapter)?;
                for token in &mut analysis.tokens {
                    if token.surface == "\u{fffc}" {
                        token.is_japanese = false;
                    }
                }
                Ok::<ChapterAnalysis, std::convert::Infallible>(analysis)
            },
        )
        .unwrap()
    {
        EpubImportOutcome::Created(result) => *result,
        _ => panic!("expected EPUB"),
    };
    let id = imported.document.document_id;
    let document = snapshot(&mut storage, id).document;
    let first = prepare(&mut storage, id);
    record(&mut storage, first);
    assert!(!snapshot(&mut storage, id).document.finished_at_least_once);
    assert_eq!(
        snapshot(&mut storage, id).document.reading_position,
        Some(position(&document, 1, 0))
    );
    let page =
        ReaderPage::open_document(&snapshot(&mut storage, id).document, PageLength::default())
            .unwrap();
    assert!(storage
        .load_reader_tokens(id, page.anchor)
        .unwrap()
        .unwrap()
        .tokens
        .is_empty());
    let final_page = prepare(&mut storage, id);
    assert_eq!(
        record(&mut storage, final_page),
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
    );
    assert!(snapshot(&mut storage, id).document.finished_at_least_once);
}
