use deku::{DekuRead, DekuWrite};

/// `2.5.10`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone, Copy)]
#[deku(
    bit_order = "lsb",
    ctx = "order: deku::ctx::Order",
    ctx_default = "deku::ctx::Order::Lsb0"
)]
pub struct Cell {
    pub column: u32,
    #[deku(bits = 24)]
    pub iStyleRef: u32,
    #[deku(bits = 1, pad_bits_after = "7")]
    pub fPhShow: bool,
}

impl Cell {
    pub(crate) const fn check_range(&self) -> Result<(), &'static str> {
        if self.column > 16_383 {
            Err("column index out of range")
        } else {
            Ok(())
        }
    }
}

/// `2.5.9`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone, Copy)]
#[deku(ctx = "_: deku::ctx::Order", ctx_default = "deku::ctx::Order::Lsb0")]
pub struct BrtColSpan {
    pub colMic: u32,
    pub colLast: u32,
}
