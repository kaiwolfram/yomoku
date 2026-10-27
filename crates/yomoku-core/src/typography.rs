//! Global visual preferences; never participate in logical pagination or anchors.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderFontSize {
    Sp14,
    #[default]
    Sp16,
    Sp18,
    Sp20,
    Sp24,
}

impl ReaderFontSize {
    pub const ALL: [Self; 5] = [Self::Sp14, Self::Sp16, Self::Sp18, Self::Sp20, Self::Sp24];

    pub const fn sp(self) -> i64 {
        match self {
            Self::Sp14 => 14,
            Self::Sp16 => 16,
            Self::Sp18 => 18,
            Self::Sp20 => 20,
            Self::Sp24 => 24,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum ReaderLineSpacing {
    Percent125,
    #[default]
    Percent150,
    Percent175,
    Percent200,
}

impl ReaderLineSpacing {
    pub const ALL: [Self; 4] = [
        Self::Percent125,
        Self::Percent150,
        Self::Percent175,
        Self::Percent200,
    ];

    pub const fn percent(self) -> i64 {
        match self {
            Self::Percent125 => 125,
            Self::Percent150 => 150,
            Self::Percent175 => 175,
            Self::Percent200 => 200,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReaderTypography {
    pub font_size: ReaderFontSize,
    pub line_spacing: ReaderLineSpacing,
}
