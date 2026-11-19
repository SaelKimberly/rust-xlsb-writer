use deku::{DekuRead, DekuWrite};

use crate::prelude::XlNullableWideString;
use crate::{CheckedBiff, KnownID, prelude::Unchecked};

/// Links a worksheet to its VML note drawing; MS-XLSB `2.4.703`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtLegacyDrawing {
    pub stRelId: XlNullableWideString,
}

impl Unchecked for BrtLegacyDrawing {}

impl CheckedBiff for BrtLegacyDrawing {
    fn id(&self) -> KnownID {
        KnownID::BrtLegacyDrawing
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use crate::prelude::{BrtLegacyDrawing, XlNullableWideString, XlStringExt};

    #[test]
    fn legacy_drawing_roundtrips_relationship_id() {
        let record = BrtLegacyDrawing {
            stRelId: XlNullableWideString::construct(Some("rId2")),
        };
        let bytes = record.to_bytes().expect("serialize legacy drawing");
        assert_eq!(bytes, [4, 0, 0, 0, b'r', 0, b'I', 0, b'd', 0, b'2', 0]);
        let (rest, decoded) =
            BrtLegacyDrawing::from_bytes((&bytes, 0)).expect("deserialize legacy drawing");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
    }
}
