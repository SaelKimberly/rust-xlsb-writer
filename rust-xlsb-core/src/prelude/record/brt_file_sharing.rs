use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::XlNullableWideString;

/// `2.4.678`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFileSharing {
    pub fReadOnlyRec: u16,
    pub wResPass: u16,
    pub stUserName: XlNullableWideString,
}

impl Unchecked for BrtFileSharing {}

assign_id!(BrtFileSharing);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::XlStringExt;

    #[test]
    fn read_only_flag_and_null_user_are_serialized() {
        let record = BrtFileSharing {
            fReadOnlyRec: 1,
            stUserName: XlNullableWideString::construct(None::<&str>),
            ..BrtFileSharing::default()
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes, [1, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
