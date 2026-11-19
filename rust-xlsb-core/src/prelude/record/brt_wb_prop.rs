use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::CodeName;

/// `2.4.868`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtWbProp {
    pub flags: u32,
    pub dwThemeVersion: u32,
    pub strName: CodeName,
}

impl Unchecked for BrtWbProp {}

assign_id!(BrtWbProp);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn default_props_have_no_theme_version_or_code_name() {
        let bytes = BrtWbProp::default().to_bytes().expect("serializes");

        assert_eq!(bytes, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
