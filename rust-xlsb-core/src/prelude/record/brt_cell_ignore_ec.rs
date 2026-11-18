use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::UncheckedSqRfX;

/// `2.4.322`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellIgnoreEC {
    /// LSB-first ignored-error flags; bits 9..31 are reserved zero.
    pub grbit: u32,
    pub sqrfxu: UncheckedSqRfX,
}

impl BrtCellIgnoreEC {
    pub const F_CALC_ERROR: u32 = 1 << 0;
    pub const F_EMPTY_CELL_REF: u32 = 1 << 1;
    pub const F_NUM_STORED_AS_TEXT: u32 = 1 << 2;
    pub const F_INCONSIST_RANGE: u32 = 1 << 3;
    pub const F_INCONSIST_FMLA: u32 = 1 << 4;
    pub const F_TEXT_DATE_INSUFF: u32 = 1 << 5;
    pub const F_UNPROT_FMLA: u32 = 1 << 6;
    pub const F_DATA_VALIDATION: u32 = 1 << 7;
    pub const F_CALC_COL: u32 = 1 << 8;
}

impl Checked for BrtCellIgnoreEC {
    fn check(&self) -> Result<(), &'static str> {
        if self.grbit & !0x01FF != 0 {
            return Err("ignored-error flags contain reserved bits");
        }
        if self.sqrfxu.crfx >= 8_192 {
            return Err("ignored-error range count must be less than 8192");
        }
        self.sqrfxu.check()
    }
}

assign_id!(BrtCellIgnoreEC);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    use crate::prelude::UncheckedRfX;

    #[test]
    fn flags_pack_into_the_first_two_bytes() {
        let record = BrtCellIgnoreEC {
            grbit: BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT | BrtCellIgnoreEC::F_CALC_COL,
            ..BrtCellIgnoreEC::default()
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(&bytes[..4], [0x04, 0x01, 0, 0]);
        assert_eq!(bytes.len(), 8);
    }

    #[test]
    fn roundtrips_flags_and_ranges() {
        let record = BrtCellIgnoreEC {
            grbit: BrtCellIgnoreEC::F_CALC_ERROR | BrtCellIgnoreEC::F_DATA_VALIDATION,
            sqrfxu: UncheckedSqRfX {
                crfx: 2,
                rgRefx: vec![
                    UncheckedRfX {
                        rwFirst: 1,
                        rwLast: 2,
                        colFirst: 3,
                        colLast: 4,
                    },
                    UncheckedRfX {
                        rwFirst: 5,
                        rwLast: 5,
                        colFirst: 6,
                        colLast: 7,
                    },
                ],
            },
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 4 + 4 + 32);
        let (rest, decoded) = BrtCellIgnoreEC::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0, []);
        assert_eq!(decoded, record);
    }
}
