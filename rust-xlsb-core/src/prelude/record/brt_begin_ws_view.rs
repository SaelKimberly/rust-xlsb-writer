use deku::{DekuRead, DekuWrite};

use crate::{Unchecked, assign_id};

/// `2.4.313`
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
pub struct BrtBeginWsView {
    #[deku(bits = 1)]
    pub fWnProt: bool,
    #[deku(bits = 1)]
    pub fDspFmla: bool,
    #[deku(bits = 1)]
    pub fDspGrid: bool,
    #[deku(bits = 1)]
    pub fDspRwCol: bool,
    #[deku(bits = 1)]
    pub fDspZeros: bool,
    #[deku(bits = 1)]
    pub fRightToLeft: bool,
    #[deku(bits = 1)]
    pub fSelected: bool,
    #[deku(bits = 1)]
    pub fDspRuler: bool,
    #[deku(bits = 1)]
    pub fDspGuts: bool,
    #[deku(bits = 1)]
    pub fDefaultHdr: bool,
    #[deku(bits = 1)]
    pub fWhitespaceHidden: bool,
    #[deku(bits = 5)]
    pub reserved1: u8,

    pub xlView: u32,
    pub rwTop: u32,
    pub colLeft: u32,
    pub icvHdr: u8,
    pub reserved2: u8,
    pub reserved3: u16,
    pub wScale: u16,
    pub wScaleNormal: u16,
    pub wScaleSLV: u16,
    pub wScalePLV: u16,
    pub iWbkView: u32,
}

impl Unchecked for BrtBeginWsView {}

assign_id!(BrtBeginWsView);

/// `2.4.314`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndWsView {}

impl Unchecked for BrtEndWsView {}

assign_id!(BrtEndWsView);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn gridlines_land_in_the_lsb_first_bit_byte() {
        let view = BrtBeginWsView {
            fDspGrid: true,
            ..BrtBeginWsView::default()
        };

        let bytes = view.to_bytes().expect("serializes");
        assert_eq!(bytes[0], 0x04);
        assert_eq!(bytes.len(), 30);
    }
}
