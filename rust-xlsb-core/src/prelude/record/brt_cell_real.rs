use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{Cell, Xnum};

/// `2.4.347`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellReal {
    pub cell: Cell,
    pub xnum: Xnum,
}

impl Checked for BrtCellReal {
    fn check(&self) -> Result<(), &'static str> {
        self.cell.check_range()
    }
}

assign_id!(BrtCellReal);
