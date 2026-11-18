use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::Xnum;

/// `2.4.189`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[deku(bit_order = "lsb")]
pub struct BrtPane {
    pub xnumXSplit: Xnum,
    pub xnumYSplit: Xnum,
    pub rwTop: u32,
    pub colLeft: u32,
    pub pnnAct: u32,
    #[deku(bits = 1)]
    pub fFrozen: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    pub fFrozenNoSplit: bool,
}

impl Checked for BrtPane {
    fn check(&self) -> Result<(), &'static str> {
        if self.rwTop > 1_048_575 || self.colLeft > 16_383 {
            return Err("pane top-left cell out of range");
        }
        if self.pnnAct > 3 {
            return Err("pane number out of range");
        }
        if self.fFrozen && self.fFrozenNoSplit {
            return Err("fFrozen and fFrozenNoSplit are mutually exclusive");
        }
        Ok(())
    }
}

assign_id!(BrtPane);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn frozen_flag_is_the_lowest_bit_of_the_last_byte() {
        assert_eq!(
            BrtPane {
                fFrozen: true,
                ..BrtPane::default()
            }
            .to_bytes()
            .expect("serializes")[28],
            0x01
        );
        assert_eq!(
            BrtPane {
                fFrozenNoSplit: true,
                ..BrtPane::default()
            }
            .to_bytes()
            .expect("serializes")[28],
            0x02
        );
    }
}
