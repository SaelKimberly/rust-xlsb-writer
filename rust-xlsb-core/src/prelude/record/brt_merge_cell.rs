use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::UncheckedRfX;

/// `2.4.713`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtMergeCell {
    pub rfx: UncheckedRfX,
}

impl Checked for BrtMergeCell {
    fn check(&self) -> Result<(), &'static str> {
        self.rfx.check()
    }
}

assign_id!(BrtMergeCell);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn roundtrips_merge_range() {
        let cell = BrtMergeCell {
            rfx: UncheckedRfX {
                rwFirst: 1,
                rwLast: 3,
                colFirst: 2,
                colLast: 4,
            },
        };

        let bytes = cell.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 16);
        let (_, decoded) = BrtMergeCell::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(decoded, cell);
    }
}
