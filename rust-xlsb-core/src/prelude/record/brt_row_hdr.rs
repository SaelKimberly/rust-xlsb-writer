#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec count field is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::BrtColSpan;

/// `2.4.770`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "bit layout mirrors the MS-XLSB record"
)]
pub struct BrtRowHdr {
    pub rw: u32,
    pub ixfe: u32,
    pub miyRw: u16,

    #[deku(bits = 1)]
    pub fExtraAsc: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    pub fExtraDsc: bool,

    #[deku(bits = 3)]
    pub iOutLevel: u8,
    #[deku(bits = 1)]
    pub fCollapsed: bool,
    #[deku(bits = 1)]
    pub fDyZero: bool,
    #[deku(bits = 1)]
    pub fUnsynced: bool,
    #[deku(bits = 1)]
    pub fGhostDirty: bool,
    #[deku(bits = 1)]
    pub fReserved: bool,

    #[deku(bits = 1, pad_bits_after = "7")]
    pub fPhShow: bool,

    #[deku(update = "self.rgBrtColspan.len() as u32")]
    pub ccolspan: u32,
    #[deku(count = "ccolspan")]
    pub rgBrtColspan: Vec<BrtColSpan>,
}

impl Checked for BrtRowHdr {
    fn check(&self) -> Result<(), &'static str> {
        if self.rw > 1_048_575 {
            return Err("row index out of range");
        }
        if self.ccolspan > 16 {
            return Err("ccolspan must not exceed 16");
        }
        Ok(())
    }
}

assign_id!(BrtRowHdr);
