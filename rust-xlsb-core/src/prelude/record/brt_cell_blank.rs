use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::Cell;

/// `2.4.328`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellBlank {
    pub cell: Cell,
}

impl Checked for BrtCellBlank {
    fn check(&self) -> Result<(), &'static str> {
        self.cell.check_range()
    }
}

assign_id!(BrtCellBlank);
