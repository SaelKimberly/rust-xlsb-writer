use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{XlNullableWideString, XlStringExt};

/// `2.4.809`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtStyle {
    pub ixf: u32,
    pub grbitObj1: u16,
    pub iStyBuiltIn: u8,
    pub iLevel: u8,
    pub stName: XlNullableWideString,
}

impl Default for BrtStyle {
    fn default() -> Self {
        Self {
            ixf: 0,
            grbitObj1: 0x0001,
            iStyBuiltIn: 0,
            iLevel: 0,
            stName: XlNullableWideString::construct(Some("Normal")),
        }
    }
}

impl Unchecked for BrtStyle {}

assign_id!(BrtStyle);
