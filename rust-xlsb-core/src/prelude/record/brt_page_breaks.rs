use deku::{DekuRead, DekuWrite};

use crate::{Unchecked, assign_id};

/// Horizontal page-break collection header; `2.4.197`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginRwBrk {
    pub ibrkMac: u32,
    pub ibrkManMac: u32,
}

impl Unchecked for BrtBeginRwBrk {}

assign_id!(BrtBeginRwBrk);

/// Vertical page-break collection header; `2.4.25`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginColBrk {
    pub ibrkMac: u32,
    pub ibrkManMac: u32,
}

impl Unchecked for BrtBeginColBrk {}

assign_id!(BrtBeginColBrk);

/// Single manual page break; `2.4.315`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBrk {
    pub unRwCol: u32,
    pub unColRwStrt: u32,
    pub unColRwEnd: u32,
    pub fMan: u32,
    pub fPivot: u32,
}

impl Unchecked for BrtBrk {}

assign_id!(BrtBrk);

/// End of a horizontal page-break collection.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndRwBrk {}

impl Unchecked for BrtEndRwBrk {}

assign_id!(BrtEndRwBrk);

/// End of a vertical page-break collection.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndColBrk {}

impl Unchecked for BrtEndColBrk {}

assign_id!(BrtEndColBrk);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn break_records_have_fixed_widths() {
        assert_eq!(BrtBeginRwBrk::default().to_bytes().expect("bytes").len(), 8);
        assert_eq!(
            BrtBeginColBrk::default().to_bytes().expect("bytes").len(),
            8
        );
        assert_eq!(BrtBrk::default().to_bytes().expect("bytes").len(), 20);
        assert_eq!(
            BrtEndRwBrk::default().to_bytes().expect("bytes"),
            Vec::<u8>::new()
        );
        assert_eq!(
            BrtEndColBrk::default().to_bytes().expect("bytes"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn manual_break_fields_are_five_u32s() {
        let page_break = BrtBrk {
            unRwCol: 20,
            unColRwStrt: 0,
            unColRwEnd: 16_383,
            fMan: 1,
            fPivot: 0,
        };
        let bytes = page_break.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 20);
        assert_eq!(&bytes[0..4], &20_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &16_383_u32.to_le_bytes());
    }
}
