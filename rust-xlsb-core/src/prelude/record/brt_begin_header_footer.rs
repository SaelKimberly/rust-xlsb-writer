use deku::{DekuRead, DekuWrite};

use crate::prelude::HeaderFooterString;
use crate::{Unchecked, assign_id};

/// `2.4.313`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtBeginHeaderFooter {
    /// LSB-first flag word; bits 4..15 are reserved zero.
    pub grbit: u16,
    pub stHeader: HeaderFooterString,
    pub stFooter: HeaderFooterString,
    pub stHeaderEven: HeaderFooterString,
    pub stFooterEven: HeaderFooterString,
    pub stHeaderFirst: HeaderFooterString,
    pub stFooterFirst: HeaderFooterString,
}

impl BrtBeginHeaderFooter {
    pub const F_DIFF_ODD_EVEN: u16 = 1 << 0;
    pub const F_DIFF_FIRST: u16 = 1 << 1;
    pub const F_SCALE_WITH_DOC: u16 = 1 << 2;
    pub const F_ALIGN_MARGINS: u16 = 1 << 3;
}

impl Default for BrtBeginHeaderFooter {
    fn default() -> Self {
        Self {
            grbit: Self::F_SCALE_WITH_DOC | Self::F_ALIGN_MARGINS,
            stHeader: HeaderFooterString::default(),
            stFooter: HeaderFooterString::default(),
            stHeaderEven: HeaderFooterString::default(),
            stFooterEven: HeaderFooterString::default(),
            stHeaderFirst: HeaderFooterString::default(),
            stFooterFirst: HeaderFooterString::default(),
        }
    }
}

impl Unchecked for BrtBeginHeaderFooter {}

assign_id!(BrtBeginHeaderFooter);

/// `2.4.314`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndHeaderFooter {}

impl Unchecked for BrtEndHeaderFooter {}

assign_id!(BrtEndHeaderFooter);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::XlStringExt;

    #[test]
    fn default_flags_and_six_null_strings() {
        let record = BrtBeginHeaderFooter::default();
        let bytes = record.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 26);
        assert_eq!(&bytes[0..2], &[0x0C, 0x00]);
    }

    #[test]
    fn all_header_footer_flags_fit_the_first_bit_byte() {
        let record = BrtBeginHeaderFooter {
            grbit: BrtBeginHeaderFooter::F_DIFF_ODD_EVEN
                | BrtBeginHeaderFooter::F_DIFF_FIRST
                | BrtBeginHeaderFooter::F_SCALE_WITH_DOC
                | BrtBeginHeaderFooter::F_ALIGN_MARGINS,
            ..BrtBeginHeaderFooter::default()
        };
        let bytes = record.to_bytes().expect("serializes");

        assert_eq!(&bytes[0..2], &[0x0F, 0x00]);
    }

    #[test]
    fn header_text_follows_the_flag_word() {
        let record = BrtBeginHeaderFooter {
            stHeader: HeaderFooterString::construct(Some("&CPage &P")),
            ..BrtBeginHeaderFooter::default()
        };
        let bytes = record.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 44);
        assert_eq!(&bytes[2..6], &[9, 0, 0, 0]);
        assert_eq!(record.stHeader.len(), 9);
    }
}
