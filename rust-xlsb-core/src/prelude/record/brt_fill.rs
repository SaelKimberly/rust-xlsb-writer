use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{BrtColor, Xnum};

/// `2.4.681`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFill {
    pub fls: u32,
    pub brtColorFore: BrtColor,
    pub brtColorBack: BrtColor,
    pub iGradientType: u32,
    pub xnumDegree: Xnum,
    pub xnumFillToLeft: Xnum,
    pub xnumFillToRight: Xnum,
    pub xnumFillToTop: Xnum,
    pub xnumFillToBottom: Xnum,
    pub cNumStop: u32,
}

impl Unchecked for BrtFill {}

assign_id!(BrtFill);
