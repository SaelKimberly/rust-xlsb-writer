use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;

/// `2.4.231`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginSst {
    pub cstTotal: u32,
    pub cstUnique: u32,
}

impl Unchecked for BrtBeginSst {}

assign_id!(BrtBeginSst);

/// `2.4.584`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndSst {}

impl Unchecked for BrtEndSst {}

assign_id!(BrtEndSst);
