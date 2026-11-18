use deku::{DekuRead, DekuWrite};

use crate::Unchecked;

/// `2.5.98.2`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone, Copy)]
#[deku(id_type = "u8")]
pub enum BErr {
    #[default]
    #[deku(id = "0x00")]
    Null,
    #[deku(id = "0x07")]
    Div0,
    #[deku(id = "0x0F")]
    Value,
    #[deku(id = "0x17")]
    Ref,
    #[deku(id = "0x1D")]
    Name,
    #[deku(id = "0x24")]
    Num,
    #[deku(id = "0x2A")]
    NA,
    #[deku(id = "0x2B")]
    GettingData,
}

impl Unchecked for BErr {}
