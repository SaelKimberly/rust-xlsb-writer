use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::UncheckedRfX;

/// `2.4.872`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtWsDim {
    pub rfx: UncheckedRfX,
}

impl Checked for BrtWsDim {
    fn check(&self) -> Result<(), &'static str> {
        self.rfx.check()
    }
}

assign_id!(BrtWsDim);
