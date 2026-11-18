use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::{UncheckedRfX, XlNullableWideString, XlWideString};

/// `2.4.693`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtHLink {
    pub rfx: UncheckedRfX,
    pub relId: XlNullableWideString,
    pub location: XlWideString,
    pub tooltip: XlWideString,
    pub display: XlWideString,
}

impl Unchecked for BrtHLink {}

assign_id!(BrtHLink);

#[cfg(test)]
mod tests {
    use deku::DekuContainerRead;

    use super::*;
    use crate::prelude::{UncheckedRfX, XlNullableWideString, XlStringExt, XlWideString};

    #[test]
    fn non_empty_hlink_roundtrips() {
        let link = BrtHLink {
            rfx: UncheckedRfX {
                rwFirst: 1,
                rwLast: 1,
                colFirst: 2,
                colLast: 2,
            },
            relId: XlNullableWideString::construct(Some("")),
            location: XlWideString::construct(Some("A1")),
            tooltip: XlWideString::construct(Some("tip")),
            display: XlWideString::construct(Some("x")),
        };

        let bytes = deku::DekuContainerWrite::to_bytes(&link).expect("serializes");
        assert_eq!(bytes.len(), 44);
        let (_, parsed) = BrtHLink::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(parsed, link);
    }
}
