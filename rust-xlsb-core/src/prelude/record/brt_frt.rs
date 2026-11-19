//! Future-record collection wrappers.

use deku::{DekuRead, DekuWrite};

use crate::assign_id;
use crate::prelude::{Checked, FrtProductVersion};

/// `2.4.691`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFRTBegin {
    pub productVersion: FrtProductVersion,
}

impl Checked for BrtFRTBegin {
    fn check(&self) -> Result<(), &'static str> {
        self.productVersion.check()
    }
}

assign_id!(BrtFRTBegin);

/// `2.4.692`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFRTEnd {}

impl Checked for BrtFRTEnd {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtFRTEnd);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn frt_begin_uses_the_oracle_product_version() {
        let bytes = BrtFRTBegin::default().to_bytes().expect("serializes");
        assert_eq!(bytes, [0x02, 0x0E, 0x00, 0x00]);
        assert_eq!(
            BrtFRTEnd::default().to_bytes().expect("serializes").len(),
            0
        );
    }
}
