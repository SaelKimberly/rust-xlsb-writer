use deku::{DekuRead, DekuWrite};

use crate::prelude::UncheckedRfX;
use crate::{Checked, Unchecked, assign_id};

/// `AutoFilter` range header; `2.4.8`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginAFilter {
    pub rfx: UncheckedRfX,
}

impl Checked for BrtBeginAFilter {
    fn check(&self) -> Result<(), &'static str> {
        self.rfx.check()
    }
}

assign_id!(BrtBeginAFilter);

/// End of an `AutoFilter` collection; `2.4.365`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndAFilter {}

impl Unchecked for BrtEndAFilter {}

assign_id!(BrtEndAFilter);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn encodes_the_autofilter_range_and_empty_end_marker() {
        let record = BrtBeginAFilter {
            rfx: UncheckedRfX {
                rwFirst: 1,
                rwLast: 3,
                colFirst: 2,
                colLast: 4,
            },
        };
        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 16);
        assert_eq!(&bytes[..4], &1_u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &3_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &2_u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &4_u32.to_le_bytes());

        let (rest, decoded) = BrtBeginAFilter::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
        assert_eq!(
            BrtEndAFilter::default().to_bytes().expect("bytes"),
            Vec::<u8>::new()
        );
    }
}
