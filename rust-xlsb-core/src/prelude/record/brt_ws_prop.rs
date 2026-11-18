use deku::{DekuRead, DekuWrite};

use super::BrtColor;
use crate::Unchecked;
use crate::assign_id;
use crate::prelude::CodeName;

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Eq, Hash, Default)]
#[deku(bit_order = "lsb")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "bit layout mirrors the MS-XLSB record"
)]
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

    pub brtcolorTab: BrtColor,

    pub rwSync: u32, // TODO: RwNullable
    pub colSync: u32,

    #[deku(
        reader = "CodeName::from_reader_with_ctx(deku::reader, ())",
        writer = "CodeName::to_writer(&self.strName, deku::writer, ())"
    )]
    pub strName: CodeName,
}

impl Unchecked for BrtWsProp {}

assign_id!(BrtWsProp);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn fit_to_page_is_the_first_bit_of_the_second_byte() {
        let prop = BrtWsProp {
            fFitToPage: true,
            ..BrtWsProp::default()
        };

        let bytes = prop.to_bytes().expect("serializes");
        assert_eq!(&bytes[0..2], &[0x00, 0x01]);
    }
}
