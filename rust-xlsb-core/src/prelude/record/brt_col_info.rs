use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;

/// `2.4.336`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[deku(
    bit_order = "lsb",
    ctx = "order: deku::ctx::Order",
    ctx_default = "deku::ctx::Order::Lsb0"
)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "bit layout mirrors the MS-XLSB record"
)]
pub struct BrtColInfo {
    pub colFirst: u32,
    pub colLast: u32,
    pub coldx: u32,
    pub ixfe: u32,

    #[deku(bits = 1)]
    pub fHidden: bool,
    #[deku(bits = 1)]
    pub fUserSet: bool,
    #[deku(bits = 1)]
    pub fBestFit: bool,
    #[deku(bits = 1, pad_bits_after = "4")]
    pub fPhonetic: bool,

    #[deku(bits = 3)]
    pub iOutLevel: u8,
    #[deku(bits = 1)]
    pub unused: bool,
    #[deku(bits = 1, pad_bits_after = "3")]
    pub fCollapsed: bool,
}

impl Checked for BrtColInfo {
    fn check(&self) -> Result<(), &'static str> {
        if self.colFirst > 16_383 || self.colLast > 16_383 {
            return Err("column index out of range");
        }
        if self.colFirst > self.colLast {
            return Err("column range is reversed");
        }
        if self.coldx > 65_535 {
            return Err("column width out of range");
        }
        if self.iOutLevel > 7 {
            return Err("outline level must not exceed 7");
        }
        Ok(())
    }
}

assign_id!(BrtColInfo);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn outline_bits_land_in_the_expected_byte() {
        let column = BrtColInfo {
            fHidden: true,
            iOutLevel: 2,
            fCollapsed: true,
            ..BrtColInfo::default()
        };

        let bytes = column.to_bytes().expect("serializes");
        assert_eq!(&bytes[16..18], &[0x01, 0x12]);
    }
}
