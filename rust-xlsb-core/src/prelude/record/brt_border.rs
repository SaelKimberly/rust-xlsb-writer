use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::Blxf;

/// `2.4.314`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBorder {
    #[deku(bits = 1)]
    pub fBdrDiagDown: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    pub fBdrDiagUp: bool,

    pub blxfTop: Blxf,
    pub blxfBottom: Blxf,
    pub blxfLeft: Blxf,
    pub blxfRight: Blxf,
    pub blxfDiag: Blxf,
}

impl Unchecked for BrtBorder {}

assign_id!(BrtBorder);
