use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{RangeProtectionTitleSDRel, UncheckedRfX, UncheckedSqRfX};

/// `2.4.765`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtRangeProtection {
    pub protPwd: u16,
    pub sqRfX: UncheckedSqRfX,
    pub RangeProtectionTitleSDRel: RangeProtectionTitleSDRel,
}

impl Default for BrtRangeProtection {
    fn default() -> Self {
        Self {
            protPwd: 0,
            sqRfX: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![UncheckedRfX::default()],
            },
            RangeProtectionTitleSDRel: RangeProtectionTitleSDRel::default(),
        }
    }
}

impl Checked for BrtRangeProtection {
    fn check(&self) -> Result<(), &'static str> {
        if self.sqRfX.crfx == 0 || self.sqRfX.crfx >= 8_192 {
            return Err("protected range count must be between 1 and 8191");
        }
        self.sqRfX.check()?;
        self.RangeProtectionTitleSDRel.check()
    }
}

assign_id!(BrtRangeProtection);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    use crate::prelude::{XlStringExt, XlWideString};

    #[test]
    fn encodes_password_range_title_and_zero_descriptor() {
        let record = BrtRangeProtection {
            protPwd: 0x83AF,
            sqRfX: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![UncheckedRfX {
                    rwFirst: 1,
                    rwLast: 3,
                    colFirst: 2,
                    colLast: 4,
                }],
            },
            RangeProtectionTitleSDRel: RangeProtectionTitleSDRel {
                rgchTitle: XlWideString::construct(Some("A")),
                dwsdRel: 0,
                pSDRel: Vec::new(),
            },
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[..2], [0xAF, 0x83]);
        assert_eq!(&bytes[2..6], [1, 0, 0, 0]);
        assert_eq!(&bytes[6..10], [1, 0, 0, 0]);
        assert_eq!(&bytes[10..14], [3, 0, 0, 0]);
        assert_eq!(&bytes[14..18], [2, 0, 0, 0]);
        assert_eq!(&bytes[18..22], [4, 0, 0, 0]);
        assert_eq!(&bytes[22..26], [1, 0, 0, 0]);
        assert_eq!(&bytes[26..28], [b'A', 0]);
        assert_eq!(&bytes[28..], [0, 0, 0, 0]);

        let (rest, decoded) = BrtRangeProtection::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0, []);
        assert_eq!(decoded, record);
    }
}
