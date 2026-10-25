//! Platform-independent domain rules for Yomoku.
//!
//! Persistence, tokenization, and platform effects belong outside this crate.
pub mod active_time;
pub mod analysis;
pub mod app;
pub mod completion;
pub mod dictionary;
pub mod document;
pub mod pagination;
pub mod reader;
pub mod storage;
pub mod vocabulary;
