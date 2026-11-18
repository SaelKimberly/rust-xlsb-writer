use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;

/// `2.4.794`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each field is an independent MS-XLSB permission flag"
)]
pub struct BrtSheetProtection {
    pub protpwd: u16,
    pub fLocked: u32,
    pub fObjects: u32,
    pub fScenarios: u32,
    pub fFormatCells: u32,
    pub fFormatColumns: u32,
    pub fFormatRows: u32,
    pub fInsertColumns: u32,
    pub fInsertRows: u32,
    pub fInsertHyperlinks: u32,
    pub fDeleteColumns: u32,
    pub fDeleteRows: u32,
    pub fSelLockedCells: u32,
    pub fSort: u32,
    pub fAutoFilter: u32,
    pub fPivotTables: u32,
    pub fSelUnlockedCells: u32,
}

impl Unchecked for BrtSheetProtection {}

assign_id!(BrtSheetProtection);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn password_and_permission_flags_are_serialized() {
        let record = BrtSheetProtection {
            protpwd: 0x83AF,
            fLocked: 1,
            fObjects: 1,
            fScenarios: 1,
            fFormatCells: 1,
            fFormatColumns: 1,
            fFormatRows: 1,
            fInsertColumns: 1,
            fInsertRows: 1,
            fInsertHyperlinks: 1,
            fDeleteColumns: 1,
            fDeleteRows: 1,
            fSelLockedCells: 1,
            fSort: 1,
            fAutoFilter: 1,
            fPivotTables: 1,
            fSelUnlockedCells: 1,
        };

        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 66);
        assert_eq!(&bytes[..2], [0xAF, 0x83]);
        assert!(
            bytes[2..]
                .as_chunks::<4>()
                .0
                .iter()
                .all(|field| *field == [1, 0, 0, 0])
        );
    }
}
