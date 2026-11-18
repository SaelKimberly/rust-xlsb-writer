use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{XlNullableWideString, XlWideString};

/// `2.4.316`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBundleSh {
    pub hsState: u32,
    pub iTabID: u32,
    pub strRelID: XlNullableWideString,
    pub strName: XlWideString,
}

impl Unchecked for BrtBundleSh {}

assign_id!(BrtBundleSh);
