//! Offline Sudachi adapter. No SQLite, UI, registration or reading side effects.

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
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

pub const ENGINE_REVISION: &str =
    "sudachi.rs@5de0410b6edfd428fc510cc9763440772e9619d5;yomoku-adapter-v1;embedded-defaults";
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
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "cannot read Sudachi Full dictionary: {error}"),
            Self::DictionaryMismatch => f.write_str("Sudachi Full dictionary checksum mismatch"),
            Self::Engine(error) => write!(f, "Sudachi analysis failed: {error}"),
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

/// Load once on an I/O worker and reuse. The verified owned bytes stay immutable
/// for the dictionary's lifetime; no unsafe mapping or subsequent file read is used.
/// Device memory/startup measurements are required before Android integration.
pub struct Analyzer {
    dictionary: JapaneseDictionary,
}

impl Analyzer {
    pub fn open_full(path: impl AsRef<Path>) -> Result<Self, AnalysisError> {
        let bytes = fs::read(path).map_err(AnalysisError::Io)?;
        let checksum: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if checksum != DICTIONARY_SHA256 {
            return Err(AnalysisError::DictionaryMismatch);
        }
        let config = Config::new_embedded().map_err(SudachiError::from)?;
        let dictionary = JapaneseDictionary::from_cfg_storage(
            &config,
            SudachiDicData::new(Storage::Owned(bytes)),
        )?;
        Ok(Self { dictionary })
    }

    /// Default Mode B, independently of any future UI preference.
    pub fn analyze(&self, chapter: &ChapterText) -> Result<ChapterAnalysis, AnalysisError> {
        self.analyze_mode(chapter, SplitMode::default())
    }

    pub fn analyze_mode(
        &self,
        chapter: &ChapterText,
        mode: SplitMode,
    ) -> Result<ChapterAnalysis, AnalysisError> {
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
                tokens.push(AnalyzedToken {
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
                    reading: resolved
                        .then(|| morpheme.reading_form())
                        .filter(|reading| !reading.trim().is_empty())
                        .map(str::to_owned),
                    dictionary_form: resolved
                        .then(|| Surface::new(morpheme.dictionary_form()).ok())
                        .flatten(),
                });
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
        Ok(result)
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
        fs::remove_file(path).unwrap();
    }
}
