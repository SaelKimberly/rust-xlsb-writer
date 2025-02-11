use deku::{DekuRead, DekuWrite};

use super::BrtColor;

#[derive(DekuRead, DekuWrite, Debug, Hash, Default)]
#[deku(id_type = "u8")]
#[repr(u8)]
pub(crate) enum BorderType {
    #[deku(id = 0x00, default)]
    #[default]
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

#[derive(DekuRead, DekuWrite, Hash, Debug, Default)]
pub(crate) struct Blxf {
    dg: BorderType,
    brtColor: BrtColor,
}

#[derive(DekuRead, DekuWrite, Hash, Debug, Default)]
/// 2.4.311
pub(crate) struct BrtBorder {
    #[deku(bits = 1)]
    fBdrDiagDown: bool,
    #[deku(bits = 1)]
    fBdrDiagUp: bool,
    #[deku(pad_bits_before = "6")]
    blxfTop: Blxf,
    blxfBottom: Blxf,
    blxfLeft: Blxf,
    blxfRight: Blxf,
    blxfDiag: Blxf,
}
