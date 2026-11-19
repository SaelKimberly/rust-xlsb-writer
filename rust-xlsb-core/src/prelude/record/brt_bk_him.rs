use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::XlNullableWideString;

#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtBkHim {
    pub rgb: XlNullableWideString,
}

impl Unchecked for BrtBkHim {}

assign_id!(BrtBkHim);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;
    use rust_xlsb_core::prelude::{BrtBkHim, XlNullableWideString, XlStringExt};

    #[test]
    fn rel_id_uses_a_utf16_length_prefix() {
        let record = BrtBkHim {
            rgb: XlNullableWideString::construct(Some("rId1")),
        };
        assert_eq!(
            record.to_bytes().expect("record encodes"),
            [4, 0, 0, 0, b'r', 0, b'I', 0, b'd', 0, b'1', 0]
        );
    }
}
