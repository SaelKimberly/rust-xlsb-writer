use deku::{DekuRead, DekuWrite};

use crate::prelude::{BrtColor, Unchecked};

#[derive(DekuRead, DekuWrite, Default, Hash, Debug, PartialEq)]
#[deku(id_type = "u8")]
pub enum BorderType {
    #[default]
    #[deku(id = 0x00)]
    None = 0x00,
    #[deku(id = 0x01)]
    Thin = 0x01,
    #[deku(id = 0x02)]
    Medium = 0x02,
    #[deku(id = 0x03)]
    Dashed = 0x03,
    #[deku(id = 0x04)]
    Dotted = 0x04,
    #[deku(id = 0x05)]
    Thick = 0x05,
    #[deku(id = 0x06)]
    Double = 0x06,
    #[deku(id = 0x07)]
    Hairline = 0x07,
    #[deku(id = 0x08)]
    MediumDashed = 0x08,
    #[deku(id = 0x09)]
    DashDot = 0x09,
    #[deku(id = 0x0A)]
    MediumDashDot = 0x0A,
    #[deku(id = 0x0B)]
    DashDotDot = 0x0B,
    #[deku(id = 0x0C)]
    MediumDashDotDot = 0x0C,
    #[deku(id = 0x0D)]
    SlantDashDot = 0x0D,
}

impl Unchecked for BorderType {}

#[derive(DekuRead, DekuWrite, Default, Hash, Debug, PartialEq)]
pub struct Blxf {
    #[deku(pad_bytes_after = "1")]
    pub border_type: BorderType,
    pub color: BrtColor,
}

impl Unchecked for Blxf {}
