use deku::{DekuRead, DekuWrite};

use crate::prelude::{XlNullableWideString, XlStringExt};
use crate::{Unchecked, assign_id};

/// `2.4.722`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtPageSetup {
    pub iPaperSize: u32,
    pub iScale: u32,
    pub iRes: u32,
    pub iVRes: u32,
    pub iCopies: u32,
    pub iPageStart: i32,
    pub iFitWidth: u32,
    pub iFitHeight: u32,
    /// LSB-first flag word from `fLeftToRight` through `iErrors`.
    pub grbit: u16,
    pub szRelID: XlNullableWideString,
}

impl BrtPageSetup {
    pub const F_LEFT_TO_RIGHT: u16 = 1 << 0;
    pub const F_LANDSCAPE: u16 = 1 << 1;
    pub const F_NO_COLOR: u16 = 1 << 3;
    pub const F_DRAFT: u16 = 1 << 4;
    pub const F_NOTES: u16 = 1 << 5;
    pub const F_NO_ORIENT: u16 = 1 << 6;
    pub const F_USE_PAGE: u16 = 1 << 7;
    pub const F_END_NOTES: u16 = 1 << 8;

    /// Creates a page setup with no printer-settings relationship.
    #[must_use]
    pub fn without_printer_settings() -> Self {
        Self {
            szRelID: XlNullableWideString::construct(None::<&str>),
            ..Self::default()
        }
    }
}

impl Unchecked for BrtPageSetup {}

assign_id!(BrtPageSetup);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn example_flags_and_null_relationship() {
        let setup = BrtPageSetup {
            grbit: BrtPageSetup::F_LANDSCAPE | BrtPageSetup::F_DRAFT | (2 << 9),
            ..BrtPageSetup::without_printer_settings()
        };

        let bytes = setup.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 38);
        assert_eq!(&bytes[32..34], &[0x12, 0x04]);
        assert_eq!(&bytes[34..38], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
