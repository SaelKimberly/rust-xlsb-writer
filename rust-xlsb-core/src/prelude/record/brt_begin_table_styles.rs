use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::XlNullableWideString;

/// `2.4.284`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginTableStyles {
    pub cts: u32,
    pub strDefList: XlNullableWideString,
    pub strDefPivot: XlNullableWideString,
}

impl Unchecked for BrtBeginTableStyles {}

assign_id!(BrtBeginTableStyles);

/// `2.4.746`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndTableStyles {}

impl Unchecked for BrtEndTableStyles {}

assign_id!(BrtEndTableStyles);
