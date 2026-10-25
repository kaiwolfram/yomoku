//! Pure file parsers. Callers own source bytes, filenames, I/O and transactions.
//!
//! Canonical text is produced before analysis/anchor allocation. Never hash this
//! text for file deduplication: identity belongs to the untouched source bytes.

pub mod epub;
pub mod srt;
pub mod txt;
