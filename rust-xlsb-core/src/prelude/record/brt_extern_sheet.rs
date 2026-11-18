use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;

/// `2.5.173`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct Xti {
    pub externalLink: u32,
    pub firstSheet: i32,
    pub lastSheet: i32,
}

impl Unchecked for Xti {}

/// `2.4.672`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtExternSheet {
    pub cXti: u32,
    #[deku(count = "*cXti as usize")]
    pub rgXti: Vec<Xti>,
}

impl Unchecked for BrtExternSheet {}

assign_id!(BrtExternSheet);

/// `2.4.824`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtSupSelf {}

impl Unchecked for BrtSupSelf {}

assign_id!(BrtSupSelf);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn xti_is_three_little_endian_words() {
        let bytes = Xti {
            externalLink: 0,
            firstSheet: 2,
            lastSheet: 2,
        }
        .to_bytes()
        .expect("serializes");

        assert_eq!(bytes, [0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0]);
    }
}
