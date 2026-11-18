use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{BErr, Cell};

/// `2.4.336`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellError {
    pub cell: Cell,
    pub bError: BErr,
}

impl Checked for BrtCellError {
    fn check(&self) -> Result<(), &'static str> {
        self.cell.check_range()
    }
}

assign_id!(BrtCellError);
