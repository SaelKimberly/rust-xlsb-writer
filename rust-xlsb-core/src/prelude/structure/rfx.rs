use deku::{DekuRead, DekuWrite};

use crate::Checked;

/// `2.5.154`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone, Copy)]
pub struct UncheckedRfX {
    pub rwFirst: u32,
    pub rwLast: u32,
    pub colFirst: u32,
    pub colLast: u32,
}

impl Checked for UncheckedRfX {
    fn check(&self) -> Result<(), &'static str> {
        if self.rwFirst > self.rwLast {
            return Err("rwFirst must not be greater than rwLast");
        }
        if self.colFirst > self.colLast {
            return Err("colFirst must not be greater than colLast");
        }
        if self.rwFirst > 1_048_575 || self.rwLast > 1_048_575 {
            return Err("row index out of range");
        }
        if self.colFirst > 16_383 || self.colLast > 16_383 {
            return Err("column index out of range");
        }
        Ok(())
    }
}
