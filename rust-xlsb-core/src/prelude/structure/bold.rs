use deku::{DekuRead, DekuWrite};

use crate::prelude::Unchecked;

#[derive(DekuRead, DekuWrite, Default, Hash, Debug, PartialEq)]
#[repr(u16)]
#[deku(id_type = "u16")]
pub enum Bold {
    #[default]
    BLSNORMAL = 0x0190,
    BLSBOLD = 0x02BC,
}

impl Unchecked for Bold {}
