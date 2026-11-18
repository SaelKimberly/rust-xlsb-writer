use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{Bold, BrtColor, FontFlags, XlStringExt, XlWideString};

/// `2.4.690`
#[derive(DekuRead, DekuWrite, Clone, Debug, Hash, PartialEq, Eq)]
pub struct BrtFont {
    pub dyHeight: u16,
    pub grbit: FontFlags,
    pub bls: Bold,
    pub sss: u16,
    pub uls: u8,
    pub bFamily: u8,
    pub bCharSet: u8,
    pub unused: u8,
    pub brtColor: BrtColor,
    pub bFontScheme: u8,
    pub name: XlWideString,
}

impl Default for BrtFont {
    fn default() -> Self {
        Self {
            dyHeight: 220,
            grbit: FontFlags::default(),
            bls: Bold::default(),
            sss: 0,
            uls: 0,
            bFamily: 2,
            bCharSet: 0,
            unused: 0,
            brtColor: BrtColor::default(),
            bFontScheme: 2,
            name: XlWideString::construct(Some("Calibri")),
        }
    }
}

impl Unchecked for BrtFont {}

assign_id!(BrtFont);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_matches_excel() {
        let font = BrtFont::default();

        assert_eq!(font.dyHeight, 220);
        assert_eq!(font.name.to_optional_string().as_deref(), Some("Calibri"));
    }
}
