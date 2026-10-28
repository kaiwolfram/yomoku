//! Platform-independent domain rules for Yomoku.
//!
//! Persistence, tokenization, and platform effects belong outside this crate.
pub mod active_time;
pub mod analysis;
pub mod app;
pub mod completion;
pub mod dictionary;
pub mod document;
pub mod effective_overrides;
pub mod effective_reader;
pub mod effective_session;
pub mod effective_storage;
pub mod imports;
pub mod library;
pub mod pagination;
pub mod reader;
pub mod reader_formatting;
pub mod reader_images;
pub mod reader_ruby;
pub mod segmentation;
pub mod storage;
pub mod typography;
pub mod vocabulary;
