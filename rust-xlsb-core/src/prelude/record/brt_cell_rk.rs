use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{Cell, RkNumber};

/// `2.4.350`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellRk {
    pub cell: Cell,
    pub value: RkNumber,
}

impl Checked for BrtCellRk {
    fn check(&self) -> Result<(), &'static str> {
        self.cell.check_range()
    }
}

assign_id!(BrtCellRk);
