//! Workbook theme parts.

const THEME_XML_2023: &str = include_str!("theme_2023.xml");

/// Workbook theme selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Excel2023,
}

impl Theme {
    pub(crate) const fn xml(self) -> &'static str {
        match self {
            Self::Excel2023 => THEME_XML_2023,
        }
    }

    pub(crate) const fn version(self) -> u32 {
        match self {
            Self::Excel2023 => 202_300,
        }
    }
}
