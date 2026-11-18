use deku::{DekuRead, DekuWrite};

use crate::{Unchecked, assign_id};

/// `2.4.875`
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
pub struct BrtWsFmtInfo {
    pub dxGCol: u32,
    pub cchDefColWidth: u16,
    pub miyDefRwHeight: u16,

    #[deku(bits = 1)]
    pub fUnsynced: bool,
    #[deku(bits = 1)]
    pub fDyZero: bool,
    #[deku(bits = 1)]
    pub fExAsc: bool,
    #[deku(bits = 1, pad_bits_after = "12")]
    pub fExDesc: bool,

    pub iOutLevelRw: u8,
    pub iOutLevelCol: u8,
}

impl Unchecked for BrtWsFmtInfo {}

assign_id!(BrtWsFmtInfo);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn unsynced_flag_is_the_first_bit_byte() {
        let info = BrtWsFmtInfo {
            fUnsynced: true,
            ..BrtWsFmtInfo::default()
        };

        let bytes = info.to_bytes().expect("serializes");
        assert_eq!(bytes[8], 0x01);
    }
}
