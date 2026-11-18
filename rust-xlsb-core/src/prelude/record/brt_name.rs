use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{XlNullableWideString, XlWideString};

/// `2.5.98.12`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct NameParsedFormula {
    pub cce: u32,
    #[deku(count = "*cce as usize")]
    pub rgce: Vec<u8>,
    pub cb: u32,
    #[deku(count = "*cb as usize")]
    pub rgcb: Vec<u8>,
}

/// `2.4.718`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtName {
    /// LSB-first flags: fHidden through fFutureFunction plus 14 reserved bits.
    pub flags: u32,
    pub chKey: u8,
    pub itab: u32,
    pub name: XlWideString,
    pub formula: NameParsedFormula,
    pub comment: XlNullableWideString,
}

impl BrtName {
    pub const F_HIDDEN: u32 = 1 << 0;
    pub const F_BUILTIN: u32 = 1 << 5;
}

impl Unchecked for BrtName {}

assign_id!(BrtName);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::XlStringExt;

    #[test]
    fn builtin_and_hidden_flags_are_the_first_bits() {
        let name = BrtName {
            flags: BrtName::F_BUILTIN | BrtName::F_HIDDEN,
            name: XlWideString::construct(Some("_xlnm.Print_Area")),
            formula: NameParsedFormula {
                cce: 1,
                rgce: vec![0],
                ..NameParsedFormula::default()
            },
            ..BrtName::default()
        };

        let bytes = name.to_bytes().expect("serializes");
        assert_eq!(bytes[0] & 0b10_0001, 0b10_0001);
    }
}
