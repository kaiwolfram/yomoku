use yomoku_core::{analysis::SplitMode, document::PasteText, vocabulary::Surface};

use super::*;

fn setup(version: usize) -> (TestDatabase, Storage) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
    let mut storage = Storage { connection };
    let document = storage
        .create_paste(&PasteText::new(" \n猫 𠮷 か\u{3099} English 。 📖 \0\r\n　".into()).unwrap())
        .unwrap();
    let analysis = super::super::imports::fixture(&document.chapter).unwrap();
    storage
        .store_chapter_analysis(document.chapter.chapter_id, &analysis)
        .unwrap();
    (database, storage)
}

#[test]
fn backup_candidate_analyses_reject_gaps_overlaps_missing_tokens_and_wrong_exact_surfaces() {
    for version in [7, 11, 18, 20, 23, 24] {
        for sql in [
            "DELETE FROM analyzed_tokens",
            "DELETE FROM analyzed_tokens WHERE token_index = 1",
            "DELETE FROM analyzed_tokens WHERE token_index = 6", // Nonwhitespace NUL tail.
            "UPDATE analyzed_tokens SET token_index = token_index + 100",
            "UPDATE analyzed_tokens SET surface = '犬' WHERE token_index = 0",
            "UPDATE analyzed_tokens SET start_byte = 1 WHERE token_index = 1",
            "UPDATE analyzed_tokens SET start_byte = start_byte + 1 WHERE token_index = 1",
            "UPDATE analyzed_tokens SET end_byte = end_byte - 1 WHERE token_index = 1",
            "UPDATE analyzed_tokens SET end_byte = 99999 WHERE token_index = 0",
            "UPDATE analyzed_tokens SET end_byte = 9223372036854775807 WHERE token_index = 0",
        ] {
            let (database, storage) = setup(version);
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_analyses_reject_blank_and_invalid_utf8_profiles_and_metadata() {
    for version in [7, 23, 24] {
        for sql in [
            "UPDATE chapter_analyses SET engine_revision = '　\n'",
            "UPDATE chapter_analyses SET dictionary_revision = '　\n'",
            "UPDATE chapter_analyses SET engine_revision = CAST(X'ff' AS TEXT)",
            "UPDATE chapter_analyses SET dictionary_revision = CAST(X'ff' AS TEXT)",
            "UPDATE analyzed_tokens SET surface = CAST(X'ff' AS TEXT) WHERE token_index = 0",
            "UPDATE analyzed_tokens SET reading = '　\n' WHERE token_index = 0",
            "UPDATE analyzed_tokens SET dictionary_form = '　\n' WHERE token_index = 0",
            "UPDATE analyzed_tokens SET reading = CAST(X'ff' AS TEXT) WHERE token_index = 0",
            "UPDATE analyzed_tokens SET dictionary_form = CAST(X'ff' AS TEXT) WHERE token_index = 0",
        ] {
            let (database, storage) = setup(version);
            storage.connection.execute_batch(sql).unwrap();
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}

#[test]
fn backup_candidate_analyses_apply_existing_japanese_surface_domain_without_classification_guessing(
) {
    for japanese in [false, true] {
        let (database, mut storage) = setup(24);
        let mut analysis = storage
            .chapter_analysis(1.try_into().unwrap())
            .unwrap()
            .unwrap();
        analysis.profile.engine_revision = "whitespace-token".into();
        let mut whitespace = analysis.tokens[0].clone();
        whitespace.anchor.range.start = 0.try_into().unwrap();
        whitespace.anchor.range.end = 2.try_into().unwrap();
        whitespace.surface = " \n".into();
        whitespace.is_japanese = false;
        analysis.tokens.insert(0, whitespace);
        let id = storage
            .store_chapter_analysis(1.try_into().unwrap(), &analysis)
            .unwrap();
        storage.connection.execute(
            "UPDATE analyzed_tokens SET is_japanese = ?1 WHERE analysis_id = ?2 AND token_index = 0",
            (japanese, i64::from(id)),
        ).unwrap();
        if japanese {
            check_rejected(&database, 24, CandidateError::InvalidData);
        } else {
            accept_unchanged(&database, 24);
        }
    }
}

#[test]
fn backup_candidate_analyses_preserve_every_populated_prefix_all_modes_and_literal_metadata() {
    for version in 7..=24 {
        let (database, mut storage) = setup(version);
        let mut analysis = storage
            .chapter_analysis(1.try_into().unwrap())
            .unwrap()
            .unwrap();
        analysis.profile.engine_revision = " historical\0𠮷 engine ".into();
        analysis.profile.dictionary_revision = " historical\0 dictionary ".into();
        analysis.tokens[0].reading = Some(" ネコ\0𠮷 ".into());
        analysis.tokens[0].dictionary_form = Some(Surface::new(" 猫\0𠮷 ").unwrap());
        // Preserve supplied classification, even for Latin text; do not rerun heuristics.
        analysis.tokens[3].is_japanese = true;
        for mode in [SplitMode::A, SplitMode::C, SplitMode::B] {
            analysis.mode = mode;
            storage
                .store_chapter_analysis(1.try_into().unwrap(), &analysis)
                .unwrap();
        }
        storage
            .create_paste(&PasteText::new("legacy without analysis".into()).unwrap())
            .unwrap();
        let before = fs::read(database.path()).unwrap();
        let expected = super::super::backup_snapshot::state(&storage.connection);
        let directory = database.0.join("migration");
        let checked = candidate(&directory, &before)
            .check_schema(version as u32, before.len() as u64, || false)
            .unwrap();
        let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
        let state = super::super::backup_snapshot::state(&copy);
        for table in ["chapter_analyses", "analyzed_tokens", "chapter_texts"] {
            assert_eq!(
                state.iter().find(|(name, _)| name == table),
                expected.iter().find(|(name, _)| name == table)
            );
        }
        drop(copy);
        checked.discard().unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn backup_candidate_analyses_preserve_full_i64_ids_and_legacy_absence() {
    for id in [9_007_199_254_740_993_i64, i64::MAX] {
        let (database, mut storage) = setup(24);
        let mut analysis = storage
            .chapter_analysis(1.try_into().unwrap())
            .unwrap()
            .unwrap();
        analysis.mode = SplitMode::B;
        storage
            .connection
            .execute(
                "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'chapter_analyses'",
                [id - 1],
            )
            .unwrap();
        assert_eq!(
            i64::from(
                storage
                    .store_chapter_analysis(1.try_into().unwrap(), &analysis)
                    .unwrap()
            ),
            id
        );
        // Explicit legacy absence remains valid without synthesizing a replacement.
        storage.connection.execute_batch("DELETE FROM analyzed_tokens WHERE analysis_id = 1; DELETE FROM chapter_analyses WHERE id = 1").unwrap();
        accept_unchanged(&database, 24);
    }
}

#[test]
fn backup_candidate_analyses_cancel_every_pre_and_post_migration_poll() {
    for version in [7, 24] {
        let (database, _storage) = setup(version);
        let bytes = fs::read(database.path()).unwrap();
        let mut polls = 0;
        candidate(&database.0.join("count"), &bytes)
            .check_schema(version as u32, bytes.len() as u64, || {
                polls += 1;
                false
            })
            .unwrap()
            .discard()
            .unwrap();
        for stop in 1..=polls {
            let directory = database.0.join("cancel");
            let mut calls = 0;
            assert!(matches!(
                candidate(&directory, &bytes).check_schema(
                    version as u32,
                    bytes.len() as u64,
                    || {
                        calls += 1;
                        calls == stop
                    },
                ),
                Err(CandidateError::Cancelled)
            ));
            assert!(!directory.exists());
            assert_eq!(fs::read(database.path()).unwrap(), bytes);
        }
    }
}
