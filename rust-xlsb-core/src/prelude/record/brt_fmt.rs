use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::XlWideString;

/// `2.4.688`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFmt {
    pub ifmt: u16,
    pub stFmtCode: XlWideString,
}

impl Unchecked for BrtFmt {}

assign_id!(BrtFmt);
