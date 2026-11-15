use std::{convert::Infallible, fs};

use rusqlite::Connection;
use serde_json::json;
use yomoku_core::{
    completion::AnalyzedCompletionOutcome,
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::{
        EffectiveReaderChapter, EffectiveReaderTokenKey, EffectiveReaderWordDetails,
    },
    pagination::{LogicalPages, PageLength},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, positions::position, TestDatabase};
use crate::reader_words::*;
use crate::{dictionary::Dictionary, AnalyzedPaste, Storage, StorageError};

fn dictionary() -> TestDatabase {
    let file = TestDatabase::new();
    let connection = Connection::open(file.path()).unwrap();
    connection.execute_batch("PRAGMA application_id = 0x594f4449; PRAGMA user_version = 1;
        CREATE TABLE entries (source TEXT, entry_id INTEGER, payload TEXT, PRIMARY KEY(source, entry_id)) WITHOUT ROWID;
        CREATE TABLE forms (surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER, PRIMARY KEY(surface, source, entry_id)) WITHOUT ROWID;").unwrap();
    for (source, payload) in [
        (
            "JMdict",
            json!({
                "kanji": [{"text": "試験語"}, {"text": "別語"}],
                "readings": [{"text": "しけんご"}, {"text": "しけんことば", "re_restr": ["試験語"]}],
                "senses": [
                    {"pos": ["noun (common) (futsuumeishi)"], "glosses": [{"text": "test word", "lang": "eng"}]},
                    {"stagr": ["しけんことば"], "glosses": [{"text": "another sense", "lang": "eng"}]},
                    {"stagk": ["別語"], "glosses": [{"text": "restricted", "lang": "eng"}]}
                ]
            }),
        ),
        (
            "JMnedict",
            json!({
                "kanji": [{"text": "試験語"}], "readings": [{"text": "しけんご"}],
                "names": [{"name_type": ["place name"], "translations": [{"text": "Test Place", "lang": "eng"}]}]
            }),
        ),
    ] {
        connection
            .execute(
                "INSERT INTO entries VALUES (?1, 1, ?2)",
                (source, payload.to_string()),
            )
            .unwrap();
        connection
            .execute("INSERT INTO forms VALUES ('試験語', ?1, 1)", [source])
            .unwrap();
    }
    file
}

#[test]
fn epub_selected_word_keeps_chapter_analysis_dictionary_live_status_and_lifetime_count() {
    use super::epub_fixture;
    use crate::EpubImportOutcome;
    use yomoku_core::{
        completion::PageCompletionIntent, pagination::ReadingPosition, reader::ReaderPage,
    };

    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = match storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>試験語だった 試験語だった</p>"),
            "本.epub",
            1,
            |chapter| {
                let mut analysis = fixture(chapter)?;
                for token in &mut analysis.tokens {
                    if token.surface == "試験語だった" {
                        token.dictionary_form = Some(Surface::new("試験語").unwrap());
                    }
                }
                Ok::<_, Infallible>(analysis)
            },
        )
        .unwrap()
    {
        EpubImportOutcome::Created(result) => *result,
        _ => panic!("expected EPUB"),
    };
    let id = imported.document.document_id;
    let chapter = &imported.document.chapters[1].chapter;
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::START,
    };
    let document = storage
        .save_document_reading_position(id, position)
        .unwrap()
        .unwrap();
    let page = ReaderPage::open_document(&document, PageLength::default()).unwrap();
    let key = storage
        .load_effective_reader_tokens(id, page.anchor)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let details = storage
        .load_effective_reader_word(&dictionary, &key, 0)
        .unwrap()
        .unwrap();
    assert_eq!(details.encountered, 0);
    assert_eq!(details.candidates.words.len(), 1);
    assert_eq!(details.candidates.names.len(), 1);
    assert_eq!(
        details.token.vocabulary.unwrap().status,
        VocabularyStatus::New
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let command = storage
        .prepare_fixture_completion(PageCompletionIntent {
            document_id: id,
            page: page.anchor,
            page_length: PageLength::default(),
            departing_position: position,
            completed_at_unix_ms: 100,
            analysis_id: imported.analysis_ids[1],
        })
        .unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&command).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
    );
    let details = storage
        .load_effective_reader_word(&dictionary, &key, 0)
        .unwrap()
        .unwrap();
    assert_eq!(details.encountered, 2);
    assert_eq!(
        details.token.vocabulary.unwrap().status,
        VocabularyStatus::Known
    );
    storage
        .set_vocabulary_status(
            details.token.vocabulary.unwrap().id,
            VocabularyStatus::Familiar,
            101,
        )
        .unwrap();
    let updated = storage
        .load_effective_reader_word(&dictionary, &key, 1)
        .unwrap()
        .unwrap();
    assert_eq!(updated.encountered, 2);
    assert_eq!(
        updated.token.vocabulary.unwrap().status,
        VocabularyStatus::Familiar
    );
}

fn word_analysis(
    chapter: &yomoku_core::document::ChapterText,
) -> Result<yomoku_core::analysis::ChapterAnalysis, Infallible> {
    let mut analysis = fixture(chapter)?;
    for token in &mut analysis.tokens {
        if token.surface == "試験語だった" {
            token.reading = Some("シケンゴダッタ".into());
            token.dictionary_form = Some(Surface::new("試験語").unwrap());
        }
    }
    Ok::<_, Infallible>(analysis)
}

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .analyzed_fixture(&PasteText::new(text.into()).unwrap(), 100, |chapter| {
            word_analysis(chapter)
        })
        .unwrap()
}

fn key(imported: &AnalyzedPaste) -> EffectiveReaderTokenKey {
    let chapter = &imported.document.chapter;
    EffectiveReaderTokenKey {
        segmentation: EffectiveReaderChapter::resolve(
            chapter,
            imported.analysis_id,
            &word_analysis(chapter).unwrap(),
        )
        .unwrap()
        .into_segmentation(),
        document_id: imported.document.document_id,
        page: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: 0.try_into().unwrap(),
                end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
            },
        },
    }
}

fn complete(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
) -> yomoku_core::effective_reader::EffectivePageCompletion {
    let document = &imported.document;
    let command = storage
        .prepare_fixture_completion(yomoku_core::completion::PageCompletionIntent {
            analysis_id: imported.analysis_id,
            document_id: document.document_id,
            page: key(imported).page,
            page_length: storage.page_length().unwrap(),
            departing_position: position(document, 0),
            completed_at_unix_ms: 500,
        })
        .unwrap();
    assert!(matches!(
        storage.record_effective_page_completion(&command).unwrap(),
        AnalyzedCompletionOutcome::Recorded { .. }
    ));
    command
}

fn lookup(
    storage: &mut Storage,
    dictionary: &Dictionary,
    imported: &AnalyzedPaste,
    index: u64,
) -> EffectiveReaderWordDetails {
    storage
        .load_effective_reader_word(dictionary, &key(imported), index)
        .unwrap()
        .unwrap()
}

#[test]
fn read_only_details_keep_all_applicable_word_name_views_and_exact_inflected_identity() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語だった 試験語");
    storage
        .save_document_reading_position(
            imported.document.document_id,
            position(&imported.document, 1 + "試験語だった".len()),
        )
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let index_before = fs::read(index.path()).unwrap();
    let details = lookup(&mut storage, &dictionary, &imported, 0);
    assert_eq!(details.key, key(&imported));
    assert_eq!(details.token.segment_index, 0);
    assert_eq!(details.token.surface, "試験語だった");
    assert_eq!(
        details.token.metadata.reading.as_deref(),
        Some("シケンゴダッタ")
    );
    assert_eq!(details.encountered, 0);
    assert_eq!(
        details.token.vocabulary.unwrap().status,
        VocabularyStatus::New
    );
    assert_eq!(details.candidates.surface, "試験語だった");
    assert_eq!(
        details.candidates.dictionary_form.as_deref(),
        Some("試験語")
    );
    assert_eq!(details.candidates.words.len(), 1);
    assert_eq!(details.candidates.names.len(), 1);
    let word = &details.candidates.words[0];
    assert_eq!(word.entry_id, details.candidates.names[0].entry_id);
    assert_eq!(word.entry.senses.len(), 3); // Complete payload; applicable view excludes third.
    assert_eq!(word.forms[0].form, "試験語");
    assert_eq!(word.forms[0].matches.len(), 2); // Never filter to analyzer reading.
    assert_eq!(word.forms[0].matches[0].sense_indices, [0]);
    assert_eq!(word.forms[0].matches[1].sense_indices, [0, 1]);
    assert_eq!(
        details.candidates.names[0].entry.names[0].translations[0].text,
        "Test Place"
    );
    assert_eq!(lookup(&mut storage, &dictionary, &imported, 0), details);
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(fs::read(index.path()).unwrap(), index_before);
}

#[test]
fn counts_include_repetitions_rereads_and_other_documents_but_never_base_forms_or_incomplete_pages()
{
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "試験語だった 試験語だった 試験語");
    let second = import(&mut storage, "試験語だった 試験語");

    assert_eq!(lookup(&mut storage, &dictionary, &first, 0).encountered, 0);
    let command = complete(&mut storage, &first);
    assert_eq!(
        storage.record_effective_page_completion(&command).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(lookup(&mut storage, &dictionary, &first, 0).encountered, 2);
    assert_eq!(lookup(&mut storage, &dictionary, &first, 1).encountered, 2);
    assert_eq!(lookup(&mut storage, &dictionary, &second, 0).encountered, 2);
    complete(&mut storage, &second);
    storage
        .save_document_reading_position(first.document.document_id, position(&first.document, 0))
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap();
    complete(&mut storage, &first);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(lookup(&mut storage, &dictionary, &first, 0).encountered, 5);
    assert_eq!(lookup(&mut storage, &dictionary, &first, 2).encountered, 3);
    let id = lookup(&mut storage, &dictionary, &first, 0)
        .token
        .vocabulary
        .unwrap()
        .id;
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        storage.set_vocabulary_status(id, status, 600).unwrap();
        let details = lookup(&mut storage, &dictionary, &second, 0);
        assert_eq!(details.token.vocabulary.unwrap().status, status);
        assert_eq!(details.encountered, 5);
    }
}

#[test]
fn document_deletion_preserves_counts_vocabulary_deletion_erases_them_without_lookup_registration()
{
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "試験語 試験語");
    let second = import(&mut storage, "試験語");
    complete(&mut storage, &first);
    let old_id = lookup(&mut storage, &dictionary, &second, 0)
        .token
        .vocabulary
        .unwrap()
        .id;
    storage.delete_document(first.document.document_id).unwrap();
    assert!(storage
        .load_effective_reader_word(&dictionary, &key(&first), 0)
        .unwrap()
        .is_none());
    assert_eq!(lookup(&mut storage, &dictionary, &second, 0).encountered, 2);
    let reimported = import(&mut storage, "試験語 試験語");
    assert_eq!(
        lookup(&mut storage, &dictionary, &reimported, 0).encountered,
        2
    );
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(old_id)])
        .unwrap();
    let absent = lookup(&mut storage, &dictionary, &second, 0);
    assert!(absent.token.vocabulary.is_none());
    assert_eq!(absent.encountered, 0);
    assert!(!absent.candidates.words.is_empty());
    assert!(storage
        .vocabulary_entry(&Surface::new("試験語").unwrap())
        .unwrap()
        .is_none());
    complete(&mut storage, &second);
    let fresh = lookup(&mut storage, &dictionary, &second, 0);
    assert_ne!(fresh.token.vocabulary.unwrap().id, old_id);
    assert_eq!(fresh.encountered, 1);
}

#[test]
fn reflow_fragments_keep_full_unicode_surface_with_one_lifetime_count() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(
        &mut storage,
        &format!("{}。{}。", "𠮷".repeat(600), "猫".repeat(1000)),
    );
    let chapter = &imported.document.chapter;
    // Complete each actual logical page: a crossing token belongs only to its start page.
    let pages = LogicalPages::new(std::slice::from_ref(chapter), PageLength::default()).unwrap();
    for page in pages.pages() {
        let command = storage
            .prepare_fixture_completion(yomoku_core::completion::PageCompletionIntent {
                analysis_id: imported.analysis_id,
                document_id: imported.document.document_id,
                page: *page,
                page_length: PageLength::default(),
                departing_position: position(
                    &imported.document,
                    i64::from(page.range.start) as usize,
                ),
                completed_at_unix_ms: 500,
            })
            .unwrap();
        storage.record_effective_page_completion(&command).unwrap();
    }
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for preset in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(chapter), preset).unwrap();
        let mut fragments = String::new();
        for page in pages.pages() {
            let details = storage
                .load_effective_reader_word(
                    &dictionary,
                    &EffectiveReaderTokenKey {
                        page: *page,
                        ..key(&imported)
                    },
                    0,
                )
                .unwrap()
                .unwrap();
            assert_eq!(details.token.surface, chapter.text);
            assert_eq!(details.encountered, 1);
            assert!(details.candidates.words.is_empty());
            assert!(details.candidates.names.is_empty());
            fragments.push_str(
                details
                    .token
                    .display_range
                    .slice(page.slice(chapter).unwrap())
                    .unwrap(),
            );
        }
        assert_eq!(fragments, chapter.text);
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn exact_unicode_no_matches_keep_counts_separate_without_normalization() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "が が か\u{3099} 𠮷 試験語\0");
    complete(&mut storage, &imported);
    for (index, count) in [(0, 2), (1, 2), (2, 1), (3, 1), (4, 1)] {
        let details = lookup(&mut storage, &dictionary, &imported, index);
        assert_eq!(details.encountered, count);
        assert!(details.candidates.words.is_empty());
        assert!(details.candidates.names.is_empty());
        assert_eq!(details.candidates.surface, details.token.surface);
    }
}

#[test]
fn corrupt_dictionary_fails_whole_lookup_and_can_retry_without_user_state_changes() {
    let database = TestDatabase::new();
    let index = dictionary();
    let mut dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "試験語");
    let expected = lookup(&mut storage, &dictionary, &imported, 0);
    drop(dictionary);
    let writer = Connection::open(index.path()).unwrap();
    let payload: String = writer
        .query_row(
            "SELECT payload FROM entries WHERE source = 'JMnedict'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    writer
        .execute(
            "UPDATE entries SET payload = '{}' WHERE source = 'JMnedict'",
            [],
        )
        .unwrap();
    dictionary = Dictionary::open(index.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let corrupt_before = fs::read(index.path()).unwrap();
    assert!(matches!(
        storage.load_effective_reader_word(&dictionary, &key(&imported), 0),
        Err(ReaderWordError::Dictionary(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(fs::read(index.path()).unwrap(), corrupt_before);
    drop(dictionary);
    // Test-only replacement of the index, after dropping its reader.
    writer
        .execute(
            "UPDATE entries SET payload = ?1 WHERE source = 'JMnedict'",
            [payload],
        )
        .unwrap();
    dictionary = Dictionary::open(index.path()).unwrap();
    assert_eq!(lookup(&mut storage, &dictionary, &imported, 0), expected);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[path = "reader_words/effective.rs"]
mod effective;

#[path = "reader_words/status_cache.rs"]
mod status_cache;

mod epub_io;
