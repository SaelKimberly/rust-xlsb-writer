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
pub struct BrtBookView {
    pub xWn: i32,
    pub yWn: i32,
    pub dxWn: u32,
    pub dyWn: u32,
    pub iTabRatio: u32,
    pub itabFirst: u32,
    pub itabCur: u32,

    #[deku(bits = 1)]
    pub fHidden: bool,
    #[deku(bits = 1)]
    pub fVeryHidden: bool,
    #[deku(bits = 1)]
    pub fIconic: bool,
    #[deku(bits = 1)]
    pub fDspHScroll: bool,
    #[deku(bits = 1)]
    pub fDspVScroll: bool,
    #[deku(bits = 1)]
    pub fBotAdornment: bool,
    #[deku(bits = 1)]
    pub fAFDateGroup: bool,
    #[deku(bits = 1)]
    pub unused: bool,
}

impl Unchecked for BrtBookView {}

assign_id!(BrtBookView);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn scrollbar_flags_start_at_the_first_bit() {
        let view = BrtBookView {
            fDspHScroll: true,
            ..BrtBookView::default()
        };

        let bytes = view.to_bytes().expect("serializes");
        assert_eq!(bytes[28], 0x08);
    }
}
