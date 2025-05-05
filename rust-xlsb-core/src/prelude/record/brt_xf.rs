use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash, Default)]
#[deku(id_type = "u8", bits = 3)]
pub enum HAlign {
    #[default]
    #[deku(id = 0)]
    General,
    #[deku(id = 1)]
    Left,
    #[deku(id = 2)]
    Center,
    #[deku(id = 3)]
    Right,
    #[deku(id = 4)]
    Fill,
    #[deku(id = 5)]
    Justify,
    #[deku(id = 6)]
    CenterAcrossSelection,
    #[deku(id = 7)]
    Distributed,
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash, Default)]
#[deku(id_type = "u8", bits = 3)]
pub enum VAlign {
    #[default]
    #[deku(id = 0)]
    Top,
    #[deku(id = 1)]
    Center,
    #[deku(id = 2)]
    Bottom,
    #[deku(id = 3)]
    Justify,
    #[deku(id = 4)]
    Distributed,
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash, Default)]
#[deku(id_type = "u8", bits = 2)]
pub enum ReadingOrder {
    #[default]
    #[deku(id = 0)]
    ContextDependent,
    #[deku(id = 1)]
    LeftToRight,
    #[deku(id = 2)]
    RightToLeft,
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash, Default)]
pub struct GrbitAtr {
    #[deku(bits = 1)]
    pub bit_1: bool,
    #[deku(bits = 1)]
    pub bit_2: bool,
    #[deku(bits = 1)]
    pub bit_3: bool,
    #[deku(bits = 1)]
    pub bit_4: bool,
    #[deku(bits = 1)]
    pub bit_5: bool,
    #[deku(bits = 1)]
    pub bit_6: bool,
}

#[derive(DekuRead, DekuWrite, Debug, PartialEq, Hash, Default)]
pub struct BrtXF {
    pub ixfeParent: u16,
    pub iFmt: u16,
    pub iFont: u16,
    pub iFill: u16,
    pub ixBorder: u16,
    pub trot: u8,
    pub indent: u8,

    pub alc: HAlign,
    pub alcv: VAlign,

    #[deku(bits = 1)]
    pub fWrap: bool,
    #[deku(bits = 1)]
    pub fJustLast: bool,
    #[deku(bits = 1)]
    pub fShrinkToFit: bool,
    #[deku(bits = 1)]
    pub fMergeCell: bool,

    pub iReadingOrder: ReadingOrder,

    #[deku(bits = 1)]
    pub fLocked: bool,
    #[deku(bits = 1)]
    pub fHidden: bool,
    #[deku(bits = 1)]
    pub fSxButton: bool,
    #[deku(bits = 1)]
    pub f123Prefix: bool,

    #[deku(pad_bits_after = "10")]
    pub xfGrbitAtr: GrbitAtr,
}

impl Unchecked for BrtXF {}

assign_id!(BrtXF);
