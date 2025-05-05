use deku::{DekuRead, DekuWrite};

use super::BrtColor;
use crate::Unchecked;
use crate::assign_id;
use crate::prelude::CodeName;

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash)]
pub struct BrtWsProp {
    #[deku(bits = 1, pad_bits_after = "2")]
    pub fShowAutoBreaks: bool,

    #[deku(bits = 1)]
    pub fPublish: bool,
    #[deku(bits = 1)]
    pub fDialog: bool,
    #[deku(bits = 1)]
    pub fApplyStyles: bool,
    #[deku(bits = 1)]
    pub fRowSumsBelow: bool,
    #[deku(bits = 1)]
    pub fColSumsRight: bool,
    #[deku(bits = 1, pad_bits_after = "1")]
    pub fFitToPage: bool,

    #[deku(bits = 1, pad_bits_after = "1")]
    pub fShowOutlineSymbols: bool,

    #[deku(bits = 1)]
    pub fSyncHoriz: bool,
    #[deku(bits = 1)]
    pub fSyncVert: bool,
    #[deku(bits = 1)]
    pub fAltExprEval: bool,
    #[deku(bits = 1)]
    pub fAltFormulaEntry: bool,
    #[deku(bits = 1)]
    pub fFilterMode: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    pub fCondFmtCalc: bool,

    brtcolorTab: BrtColor,

    rwSync: u32, // TODO: RwNullable
    colSync: u32,

    strName: CodeName,
}

impl Unchecked for BrtWsProp {}

assign_id!(BrtWsProp);
