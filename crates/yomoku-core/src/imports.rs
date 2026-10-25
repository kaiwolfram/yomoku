//! Typed file input. Decoding, analysis and persistence belong to the worker.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum TextEncoding {
    Utf8,
    ShiftJis,
}

/// Original file bytes and the literal picker display name, never a path/URI.
/// Retry uncertain decoding with the same bytes and an explicit encoding.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct TxtImport {
    pub original_bytes: Vec<u8>,
    pub filename: String,
    pub encoding: Option<TextEncoding>,
}
