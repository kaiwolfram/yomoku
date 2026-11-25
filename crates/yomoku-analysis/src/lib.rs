//! Offline Sudachi adapter. No SQLite, UI, registration or reading side effects.

mod conjugations;

use std::{fs, path::Path, sync::LazyLock};

use regex::Regex;
use sha2::{Digest, Sha256};
use sudachi::{
    analysis::stateful_tokenizer::StatefulTokenizer,
    config::Config,
    dic::{dictionary::JapaneseDictionary, storage::Storage, storage::SudachiDicData},
    prelude::{Mode, MorphemeList, SudachiError},
    sentence_detector::{NonBreakChecker, SentenceDetector},
};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, ImportAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

pub const LEGACY_ENGINE_REVISION: &str =
    "sudachi.rs@5de0410b6edfd428fc510cc9763440772e9619d5+android-loader-v1;yomoku-adapter-v1;embedded-defaults";
/// Version 2 joins conjugations in production C while preserving raw A/B.
pub const CONJUGATION_ENGINE_REVISION: &str =
    "sudachi.rs@5de0410b6edfd428fc510cc9763440772e9619d5+android-loader-v1;yomoku-adapter-v2-conjugations;embedded-defaults";
/// Version 3 also joins copula stems with grammatical endings.
pub const COPULA_ENGINE_REVISION: &str =
    "sudachi.rs@5de0410b6edfd428fc510cc9763440772e9619d5+android-loader-v1;yomoku-adapter-v3-copulas;embedded-defaults";
/// Version 4 also joins numerals with grammatically tagged counter suffixes.
pub const ENGINE_REVISION: &str =
    "sudachi.rs@5de0410b6edfd428fc510cc9763440772e9619d5+android-loader-v1;yomoku-adapter-v4-counters;embedded-defaults";
pub const DICTIONARY_SHA256: &str =
    "7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1";
pub const DICTIONARY_REVISION: &str =
    "sudachidict-full-20261015;sha256:7c797531dea76f06b629fad39cb3a5bdb3a9e54c67ef3d9a869e61418426a7d1";

#[derive(Debug)]
pub enum AnalysisError {
    Io(std::io::Error),
    DictionaryMismatch,
    Engine(SudachiError),
    InvalidOutput,
    UnsupportedProfile,
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "cannot read Sudachi Full dictionary: {error}"),
            Self::DictionaryMismatch => f.write_str("Sudachi Full dictionary checksum mismatch"),
            Self::Engine(error) => write!(f, "Sudachi analysis failed: {error}"),
            Self::UnsupportedProfile => {
                f.write_str("unsupported dictionary component analysis profile")
            }
            Self::InvalidOutput => {
                f.write_str("Sudachi output cannot preserve exact source anchors")
            }
        }
    }
}

impl std::error::Error for AnalysisError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Engine(error) => Some(error),
            _ => None,
        }
    }
}

impl From<SudachiError> for AnalysisError {
    fn from(error: SudachiError) -> Self {
        Self::Engine(error)
    }
}

/// Load once on an I/O worker and reuse. Both loaders verify the exact storage
/// passed to Sudachi. Production imports use the verified private publisher.
pub struct Analyzer {
    dictionary: JapaneseDictionary,
}

impl Analyzer {
    pub fn open_full(path: impl AsRef<Path>) -> Result<Self, AnalysisError> {
        let bytes = fs::read(path).map_err(AnalysisError::Io)?;
        Self::from_storage(Storage::Owned(bytes))
    }

    /// Map a pinned Full file without retaining an owned copy of its bytes.
    /// The mapping is held by Sudachi until this analyzer is dropped. The file
    /// handle may be closed after this call; verification reads the mapping itself.
    ///
    /// # Safety
    /// The caller must ensure the underlying file is never modified or truncated,
    /// in this or any other process, from this call until the analyzer is dropped.
    /// A read-only handle or successful checksum alone does not ensure this.
    /// Use only with controlled, immutable files; keep the safe owned loader for
    /// arbitrary paths. Android's private publisher preserves this lifetime rule.
    pub unsafe fn open_full_mapped(file: &fs::File) -> Result<Self, AnalysisError> {
        // SAFETY: the caller guarantees immutable backing bytes for the entire
        // mapping lifetime, including checksum verification and dictionary parsing.
        let mapping = unsafe { memmap2::Mmap::map(file) }.map_err(AnalysisError::Io)?;
        Self::from_storage(Storage::File(mapping))
    }

    fn from_storage(storage: Storage) -> Result<Self, AnalysisError> {
        let checksum: String = Sha256::digest(storage.as_ref())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if checksum != DICTIONARY_SHA256 {
            return Err(AnalysisError::DictionaryMismatch);
        }
        let config = Config::new_embedded().map_err(SudachiError::from)?;
        let dictionary =
            JapaneseDictionary::from_cfg_storage(&config, SudachiDicData::new(storage))?;
        Ok(Self { dictionary })
    }

    /// Mode C with the base conjugation and counter rules.
    pub fn analyze(&self, chapter: &ChapterText) -> Result<ChapterAnalysis, AnalysisError> {
        self.analyze_mode(chapter, SplitMode::default())
    }

    /// Import-only grammar repair, with context for dictionary matching on the I/O worker.
    pub fn analyze_for_import(
        &self,
        chapter: &ChapterText,
    ) -> Result<ImportAnalysis, AnalysisError> {
        self.analyze_chapter(chapter, SplitMode::C, true)
    }

    /// A/B semantics and dictionary bytes are unchanged from version 1. Honor
    /// its profile for lazy components of existing books; never regenerate old C.
    pub fn analyze_components(
        &self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, AnalysisError> {
        use yomoku_core::analysis::{
            COMPOUND_IMPORT_POLICY_SUFFIX, IMPORT_POLICY_SUFFIX, IMPORT_POLICY_V3_SUFFIX,
            IMPORT_POLICY_V4_SUFFIX, IMPORT_POLICY_V5_SUFFIX, LEGACY_IMPORT_POLICY_SUFFIX,
            WORD_DICTIONARY_PREFIX,
        };
        let engine = [
            IMPORT_POLICY_SUFFIX,
            IMPORT_POLICY_V5_SUFFIX,
            IMPORT_POLICY_V4_SUFFIX,
            IMPORT_POLICY_V3_SUFFIX,
            COMPOUND_IMPORT_POLICY_SUFFIX,
            LEGACY_IMPORT_POLICY_SUFFIX,
        ]
        .into_iter()
        .find_map(|suffix| profile.engine_revision.strip_suffix(suffix));
        let dictionary = profile
            .dictionary_revision
            .split_once(WORD_DICTIONARY_PREFIX);
        let (engine, dictionary) = match (engine, dictionary) {
            (Some(engine), Some((dictionary, hash)))
                if hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()) =>
            {
                (engine, dictionary)
            }
            (None, None) => (
                profile.engine_revision.as_str(),
                profile.dictionary_revision.as_str(),
            ),
            _ => return Err(AnalysisError::UnsupportedProfile),
        };
        if mode == SplitMode::C
            || !matches!(
                engine,
                ENGINE_REVISION
                    | COPULA_ENGINE_REVISION
                    | CONJUGATION_ENGINE_REVISION
                    | LEGACY_ENGINE_REVISION
            )
            || dictionary != DICTIONARY_REVISION
        {
            return Err(AnalysisError::UnsupportedProfile);
        }
        let mut analysis = self.analyze_mode(chapter, mode)?;
        analysis.profile = profile.clone();
        Ok(analysis)
    }

    pub fn analyze_mode(
        &self,
        chapter: &ChapterText,
        mode: SplitMode,
    ) -> Result<ChapterAnalysis, AnalysisError> {
        Ok(self.analyze_chapter(chapter, mode, false)?.analysis)
    }

    fn analyze_chapter(
        &self,
        chapter: &ChapterText,
        mode: SplitMode,
        for_import: bool,
    ) -> Result<ImportAnalysis, AnalysisError> {
        let mut tokenizer = StatefulTokenizer::new(
            &self.dictionary,
            match mode {
                SplitMode::A => Mode::A,
                SplitMode::B => Mode::B,
                SplitMode::C => Mode::C,
            },
        );
        let mut morphemes = MorphemeList::empty(&self.dictionary);
        let detector = SentenceDetector::with_limit(4096);
        let checker = NonBreakChecker::new(self.dictionary.lexicon());
        let mut tokens = Vec::new();
        let mut context = Vec::new();
        let mut base = 0;
        while base < chapter.text.len() {
            let remaining = &chapter.text[base..];
            // Negative means a provisional boundary, including long sentences
            // without punctuation. Respect it to stay below Sudachi's u16 limit.
            let length = detector.get_eos(remaining, Some(&checker))?.unsigned_abs();
            if length == 0 || !remaining.is_char_boundary(length) {
                return Err(AnalysisError::InvalidOutput);
            }
            tokenizer.reset().push_str(&remaining[..length]);
            tokenizer.do_tokenize()?;
            morphemes.collect_results(&mut tokenizer)?;
            let mut sentence = Vec::new();
            for morpheme in morphemes.iter() {
                let start = base + morpheme.begin();
                let end = base + morpheme.end();
                let surface = chapter
                    .text
                    .get(start..end)
                    .ok_or(AnalysisError::InvalidOutput)?;
                // Normalization expansions may emit empty original spans.
                // Whitespace remains in canonical text; core permits these gaps.
                if surface.is_empty() || surface.chars().all(char::is_whitespace) {
                    continue;
                }
                if surface != &*morpheme.surface() {
                    return Err(AnalysisError::InvalidOutput);
                }
                let is_japanese =
                    is_japanese_word(surface, morpheme.part_of_speech(), morpheme.is_oov());
                let resolved = is_japanese && !morpheme.is_oov();
                let token = AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: TextRange {
                            start: TextOffset::at(&chapter.text, start)
                                .map_err(|_| AnalysisError::InvalidOutput)?,
                            end: TextOffset::at(&chapter.text, end)
                                .map_err(|_| AnalysisError::InvalidOutput)?,
                        },
                    },
                    surface: surface.to_owned(),
                    is_japanese,
                    reading: (resolved
                        || (for_import
                            && conjugations::standalone_small_tsu(surface)
                            && !morpheme.is_oov()))
                    .then(|| morpheme.reading_form())
                    .filter(|reading| !reading.trim().is_empty())
                    .map(str::to_owned),
                    dictionary_form: resolved
                        .then(|| Surface::new(morpheme.dictionary_form()).ok())
                        .flatten(),
                };
                if mode == SplitMode::C {
                    sentence.push(conjugations::Morpheme {
                        token,
                        pos: morpheme.part_of_speech().to_vec(),
                        normalized_form: morpheme.normalized_form().to_owned(),
                    });
                } else {
                    tokens.push(token);
                }
            }
            if mode == SplitMode::C {
                for (token, grammar) in conjugations::join(&sentence, for_import) {
                    tokens.push(token);
                    if for_import {
                        context.push(grammar);
                    }
                }
            }
            base += length;
        }
        let result = ChapterAnalysis {
            profile: AnalysisProfile {
                engine_revision: ENGINE_REVISION.into(),
                dictionary_revision: DICTIONARY_REVISION.into(),
            },
            mode,
            tokens,
        };
        result
            .validate(chapter)
            .map_err(|_| AnalysisError::InvalidOutput)?;
        Ok(ImportAnalysis {
            analysis: result,
            context,
        })
    }
}

fn is_japanese_word(surface: &str, pos: &[String], is_oov: bool) -> bool {
    static SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"[\p{Hiragana}\p{Katakana}\p{Han}]")
            .expect("constant Unicode script expression")
    });
    // Sudachi's fallback OOV provider labels unsupported supplementary kanji as
    // symbols. Their original Japanese script still makes them vocabulary, with
    // no fabricated dictionary form or reading.
    (is_oov
        || !matches!(
            pos.first().map(String::as_str),
            Some("補助記号" | "記号" | "空白")
        ))
        && SCRIPT.is_match(surface)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_script_requires_a_word_pos() {
        let noun = vec!["名詞".into()];
        for word in ["𠮷", "ｶﾀｶﾅ", "漢字", "あ", "々", "漢A"] {
            assert!(is_japanese_word(word, &noun, false), "{word}");
        }
        for other in ["English", "123", "📖", "。", "\0", "e\u{301}"] {
            assert!(!is_japanese_word(other, &noun, false), "{other}");
        }
        assert!(!is_japanese_word("漢", &["補助記号".into()], false));
        assert!(is_japanese_word("𰻞", &["補助記号".into()], true));
    }

    #[test]
    fn rejects_unpinned_bytes_before_dictionary_parsing() {
        let path =
            std::env::temp_dir().join(format!("yomoku-bad-dictionary-{}", std::process::id()));
        fs::write(&path, b"wrong dictionary").unwrap();
        assert!(matches!(
            Analyzer::open_full(&path),
            Err(AnalysisError::DictionaryMismatch)
        ));
        let file = fs::File::open(&path).unwrap();
        // SAFETY: this test exclusively owns the file and removes it only after
        // the rejected mapping is dropped; no writers exist during the call.
        assert!(matches!(
            unsafe { Analyzer::open_full_mapped(&file) },
            Err(AnalysisError::DictionaryMismatch)
        ));
        drop(file);
        fs::remove_file(path).unwrap();
    }
}
