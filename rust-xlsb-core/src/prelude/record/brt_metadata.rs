//! Metadata part records used by dynamic array formulas.

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{XlStringExt, XlWideString};

/// `2.4.112`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtBeginMetadata {}

impl Checked for BrtBeginMetadata {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtBeginMetadata);

/// `2.4.466`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndMetadata {}

impl Checked for BrtEndMetadata {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtEndMetadata);

/// `2.4.75`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginEsmdtinfo {
    pub cMdtinfo: u32,
}

impl Default for BrtBeginEsmdtinfo {
    fn default() -> Self {
        Self { cMdtinfo: 1 }
    }
}

impl Checked for BrtBeginEsmdtinfo {
    fn check(&self) -> Result<(), &'static str> {
        if self.cMdtinfo == 0 {
            return Err("metadata type collection cannot be empty");
        }
        Ok(())
    }
}

assign_id!(BrtBeginEsmdtinfo);

/// `2.4.429`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndEsmdtinfo {}

impl Checked for BrtEndEsmdtinfo {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtEndEsmdtinfo);

/// `2.4.711`
///
/// `grbit` carries the independent `MdtFlags` bits as one wire `DWORD`.
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtMdtinfo {
    pub grbit: u32,
    pub metadataID: u32,
    pub stName: XlWideString,
}

impl Default for BrtMdtinfo {
    fn default() -> Self {
        Self {
            grbit: 0x506a_c0b0,
            metadataID: 0x0001_d4c0,
            stName: XlWideString::construct(Some("XLDAPR")),
        }
    }
}

impl Checked for BrtMdtinfo {
    fn check(&self) -> Result<(), &'static str> {
        if self.stName.is_null() {
            return Err("metadata type name cannot be null");
        }
        Ok(())
    }
}

assign_id!(BrtMdtinfo);

/// `2.4.73`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginEsfmd {
    pub cFmd: u32,
    pub stName: XlWideString,
}

impl Default for BrtBeginEsfmd {
    fn default() -> Self {
        Self {
            cFmd: 1,
            stName: XlWideString::construct(Some("XLDAPR")),
        }
    }
}

impl Checked for BrtBeginEsfmd {
    fn check(&self) -> Result<(), &'static str> {
        if self.cFmd == 0 {
            return Err("future metadata collection cannot be empty");
        }
        if self.stName.is_null() {
            return Err("future metadata type name cannot be null");
        }
        Ok(())
    }
}

assign_id!(BrtBeginEsfmd);

/// `2.4.427`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndEsfmd {}

impl Checked for BrtEndEsfmd {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtEndEsfmd);

/// `2.4.86`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtBeginFmd {}

impl Checked for BrtBeginFmd {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtBeginFmd);

/// `2.4.440`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndFmd {}

impl Checked for BrtEndFmd {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtEndFmd);

/// `2.4.74`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginEsmdb {
    pub cMdb: u32,
    pub fCellMeta: u32,
}

impl Default for BrtBeginEsmdb {
    fn default() -> Self {
        Self {
            cMdb: 1,
            fCellMeta: 1,
        }
    }
}

impl Checked for BrtBeginEsmdb {
    fn check(&self) -> Result<(), &'static str> {
        if self.cMdb == 0 {
            return Err("metadata block collection cannot be empty");
        }
        if self.fCellMeta > 1 {
            return Err("metadata block kind must be cell or value metadata");
        }
        Ok(())
    }
}

assign_id!(BrtBeginEsmdb);

/// `2.4.428`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndEsmdb {}

impl Checked for BrtEndEsmdb {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtEndEsmdb);

/// `2.5.94`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Mdir {
    pub iMdt: u32,
    pub mdd: u32,
}

impl Default for Mdir {
    fn default() -> Self {
        Self { iMdt: 1, mdd: 0 }
    }
}

/// `2.4.710`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtMdb {
    pub cMdir: u32,
    #[deku(count = "cMdir")]
    pub rgMdir: Vec<Mdir>,
}

impl Default for BrtMdb {
    fn default() -> Self {
        Self {
            cMdir: 1,
            rgMdir: vec![Mdir::default()],
        }
    }
}

impl Checked for BrtMdb {
    fn check(&self) -> Result<(), &'static str> {
        if self.cMdir == 0 {
            return Err("metadata block cannot be empty");
        }
        if usize::try_from(self.cMdir).ok() != Some(self.rgMdir.len()) {
            return Err("metadata block count does not match its entries");
        }
        Ok(())
    }
}

assign_id!(BrtMdb);

/// `2.4.325`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtCellMeta {
    pub icmb: i32,
}

impl Default for BrtCellMeta {
    fn default() -> Self {
        Self { icmb: 1 }
    }
}

impl Checked for BrtCellMeta {
    fn check(&self) -> Result<(), &'static str> {
        if self.icmb <= 0 {
            return Err("cell metadata index must be one-based");
        }
        Ok(())
    }
}

assign_id!(BrtCellMeta);

/// `2.4.60`
///
/// The oracle emits an empty begin record inside the future-metadata block.
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtBeginDynamicArrayPr {}

impl Checked for BrtBeginDynamicArrayPr {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtBeginDynamicArrayPr);

/// `2.4.416`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtEndDynamicArrayPr {
    pub fDynamic: bool,
    pub fCollapsed: bool,
}

impl Default for BrtEndDynamicArrayPr {
    fn default() -> Self {
        Self {
            fDynamic: true,
            fCollapsed: false,
        }
    }
}

impl Checked for BrtEndDynamicArrayPr {
    fn check(&self) -> Result<(), &'static str> {
        if self.fCollapsed {
            return Err("dynamic array metadata collapsed flag must be false");
        }
        Ok(())
    }
}

assign_id!(BrtEndDynamicArrayPr);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn dynamic_metadata_header_matches_the_oracle() {
        assert_eq!(
            BrtMdtinfo::default().to_bytes().expect("encodes"),
            [
                0xb0, 0xc0, 0x6a, 0x50, 0xc0, 0xd4, 0x01, 0x00, 0x06, 0x00, 0x00, 0x00, b'X', 0,
                b'L', 0, b'D', 0, b'A', 0, b'P', 0, b'R', 0
            ]
        );
    }

    #[test]
    fn metadata_block_is_one_cell_metadata_entry() {
        assert_eq!(
            BrtBeginEsmdb::default().to_bytes().expect("encodes"),
            [1, 0, 0, 0, 1, 0, 0, 0]
        );
        assert_eq!(
            BrtMdb::default().to_bytes().expect("encodes"),
            [1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn dynamic_end_flags_are_dynamic_and_not_collapsed() {
        assert_eq!(
            BrtEndDynamicArrayPr::default().to_bytes().expect("encodes"),
            [1, 0]
        );
    }
}
