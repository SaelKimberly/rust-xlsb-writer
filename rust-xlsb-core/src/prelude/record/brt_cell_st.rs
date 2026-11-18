use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{Cell, XlWideString};

/// `2.4.335`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellSt {
    pub cell: Cell,
    pub value: XlWideString,
}

impl Checked for BrtCellSt {
    fn check(&self) -> Result<(), &'static str> {
        if self.value.len() > 32_767 {
            return Err("cell string must not exceed 32767 characters");
        }
        self.cell.check_range()
    }
}

assign_id!(BrtCellSt);
