use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{Cell, SimpleRichStr, XlStringExt};

/// `2.4.807`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtSSTItem {
    pub richStr: SimpleRichStr,
}

impl Default for BrtSSTItem {
    fn default() -> Self {
        Self {
            richStr: SimpleRichStr::construct(Some("")),
        }
    }
}

impl Checked for BrtSSTItem {
    fn check(&self) -> Result<(), &'static str> {
        if self.richStr.is_null() {
            return Err("SST item string must not be null");
        }
        Ok(())
    }
}

assign_id!(BrtSSTItem);

/// `2.4.337`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellIsst {
    pub cell: Cell,
    pub isst: u32,
}

impl Checked for BrtCellIsst {
    fn check(&self) -> Result<(), &'static str> {
        self.cell.check_range()
    }
}

assign_id!(BrtCellIsst);
