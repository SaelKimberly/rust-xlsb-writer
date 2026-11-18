use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::Xnum;

/// `2.4.318`
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
pub struct BrtCalcProp {
    pub recalcID: u32,
    pub fAutoRecalc: u32,
    pub cCalcCount: u32,
    pub xnumDelta: Xnum,
    pub cUserThreadCount: i32,

    #[deku(bits = 1)]
    pub fFullCalcOnLoad: bool,
    #[deku(bits = 1)]
    pub fRefA1: bool,
    #[deku(bits = 1)]
    pub fIter: bool,
    #[deku(bits = 1)]
    pub fFullPrec: bool,
    #[deku(bits = 1)]
    pub fSomeUncalced: bool,
    #[deku(bits = 1)]
    pub fSaveRecalc: bool,
    #[deku(bits = 1)]
    pub fMTREnabled: bool,
    #[deku(bits = 1)]
    pub fUserSetThreadCount: bool,
    #[deku(bits = 1, pad_bits_after = "7")]
    pub fNoDeps: bool,
}

impl Unchecked for BrtCalcProp {}

assign_id!(BrtCalcProp);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn flags_land_in_lsb_order_after_the_fixed_fields() {
        let record = BrtCalcProp {
            fFullCalcOnLoad: true,
            fRefA1: true,
            fFullPrec: true,
            fSaveRecalc: true,
            ..BrtCalcProp::default()
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 26);
        assert_eq!(&bytes[24..26], [0b0010_1011, 0]);
    }
}
