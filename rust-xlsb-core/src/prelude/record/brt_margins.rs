use deku::{DekuRead, DekuWrite};

use crate::prelude::Xnum;
use crate::{Unchecked, assign_id};

/// `2.4.709`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtMargins {
    pub xnumLeft: Xnum,
    pub xnumRight: Xnum,
    pub xnumTop: Xnum,
    pub xnumBottom: Xnum,
    pub xnumHeader: Xnum,
    pub xnumFooter: Xnum,
}

impl Unchecked for BrtMargins {}

assign_id!(BrtMargins);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn margins_are_six_xnums() {
        let margins = BrtMargins {
            xnumLeft: Xnum::from(0.7),
            xnumRight: Xnum::from(0.7),
            xnumTop: Xnum::from(0.75),
            xnumBottom: Xnum::from(0.75),
            xnumHeader: Xnum::from(0.3),
            xnumFooter: Xnum::from(0.3),
        };

        let bytes = margins.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[0..8], 0.7_f64.to_le_bytes());
        assert_eq!(&bytes[40..48], 0.3_f64.to_le_bytes());
    }
}
