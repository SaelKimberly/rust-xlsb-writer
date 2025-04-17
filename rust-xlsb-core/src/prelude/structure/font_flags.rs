use deku::{DekuRead, DekuWrite};

use crate::prelude::Unchecked;

#[derive(DekuRead, DekuWrite, Default, PartialEq, Debug, Hash)]
pub struct FontFlags {
    #[deku(bits = 1, pad_bits_before = "1")]
    pub fItalic: bool,
    #[deku(bits = 1, pad_bits_before = "1")]
    pub fStrikeOut: bool,
    #[deku(bits = 1)]
    pub fOutline: bool,
    #[deku(bits = 1)]
    pub fShadow: bool,
    #[deku(bits = 1)]
    pub fCondense: bool,
    #[deku(bits = 1, pad_bytes_after = "1")]
    pub fExtend: bool,
}

impl Unchecked for FontFlags {}
