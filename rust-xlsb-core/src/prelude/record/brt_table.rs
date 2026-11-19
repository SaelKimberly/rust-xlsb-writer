use deku::{DekuRead, DekuWrite};

use crate::prelude::{CellParsedFormula, UncheckedRfX, XlNullableWideString, XlStringExt};
use crate::{Checked, Unchecked, assign_id};

const NO_DXF: u32 = 0xffff_ffff;

/// Beginning of a table definition; MS-XLSB `2.4.100`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtBeginList {
    pub rfxList: UncheckedRfX,
    pub lt: u32,
    pub idList: u32,
    pub crwHeader: u32,
    pub crwTotals: u32,
    pub flags: u32,
    pub nDxfHeader: u32,
    pub nDxfData: u32,
    pub nDxfAgg: u32,
    pub nDxfBorder: u32,
    pub nDxfHeaderBorder: u32,
    pub nDxfAggBorder: u32,
    pub dwConnID: u32,
    pub stName: XlNullableWideString,
    pub stDisplayName: XlNullableWideString,
    pub stComment: XlNullableWideString,
    pub stStyleHeader: XlNullableWideString,
    pub stStyleData: XlNullableWideString,
    pub stStyleAgg: XlNullableWideString,
}

impl BrtBeginList {
    /// Total row has ever been displayed.
    pub const F_SHOWN_TOTAL_ROW: u32 = 0x01;
    /// Single-cell table.
    pub const F_SINGLE_CELL: u32 = 0x02;
    /// Insert row is visible.
    pub const F_FORCE_INSERT_TO_BE_VISIBLE: u32 = 0x04;
    /// Inserting the row inserted cells.
    pub const F_INSERT_ROW_INS_CELLS: u32 = 0x08;
    /// Published to a server.
    pub const F_PUBLISHED: u32 = 0x10;

    fn check_string(value: &XlNullableWideString, label: &'static str) -> Result<(), &'static str> {
        if !value.is_null() && value.len() > 255 {
            return Err(label);
        }
        Ok(())
    }
}

impl Default for BrtBeginList {
    fn default() -> Self {
        Self {
            rfxList: UncheckedRfX {
                rwFirst: 0,
                rwLast: 1,
                colFirst: 0,
                colLast: 0,
            },
            lt: 0,
            idList: 1,
            crwHeader: 1,
            crwTotals: 0,
            flags: 0,
            nDxfHeader: NO_DXF,
            nDxfData: NO_DXF,
            nDxfAgg: NO_DXF,
            nDxfBorder: NO_DXF,
            nDxfHeaderBorder: NO_DXF,
            nDxfAggBorder: NO_DXF,
            dwConnID: 0,
            stName: XlNullableWideString::construct(Some("Table1")),
            stDisplayName: XlNullableWideString::construct(Some("Table1")),
            stComment: XlNullableWideString::default(),
            stStyleHeader: XlNullableWideString::default(),
            stStyleData: XlNullableWideString::default(),
            stStyleAgg: XlNullableWideString::default(),
        }
    }
}

impl Checked for BrtBeginList {
    fn check(&self) -> Result<(), &'static str> {
        self.rfxList.check()?;
        if !matches!(self.lt, 0 | 2 | 3) {
            return Err("invalid table type");
        }
        if self.idList == 0 || self.idList > 0xff_ff_ff_fe {
            return Err("table id out of range");
        }
        if self.crwHeader > 1 || self.crwTotals > 1 {
            return Err("header/total row flags must be boolean");
        }
        if self.flags & !0x1f != 0 {
            return Err("reserved table flags must be zero");
        }

        let rows = self
            .rfxList
            .rwLast
            .saturating_sub(self.rfxList.rwFirst)
            .saturating_add(1);
        if rows <= self.crwHeader + self.crwTotals {
            return Err("table range must exceed its header and total rows");
        }
        if self.stDisplayName.is_null() {
            return Err("table display name must not be null");
        }
        Self::check_string(&self.stName, "table name exceeds 255 characters")?;
        Self::check_string(
            &self.stDisplayName,
            "table display name exceeds 255 characters",
        )?;
        Self::check_string(&self.stComment, "table comment exceeds 255 characters")?;
        Self::check_string(&self.stStyleHeader, "header style exceeds 255 characters")?;
        Self::check_string(&self.stStyleData, "data style exceeds 255 characters")?;
        Self::check_string(&self.stStyleAgg, "total style exceeds 255 characters")?;
        Ok(())
    }
}

assign_id!(BrtBeginList);

/// End of a table definition; MS-XLSB `2.4.454`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndList {}

impl Unchecked for BrtEndList {}

assign_id!(BrtEndList);

/// Beginning of a table-column definition; MS-XLSB `2.4.101`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtBeginListCol {
    pub idField: u32,
    pub ilta: u32,
    pub nDxfHdr: u32,
    pub nDxfInsertRow: u32,
    pub nDxfAgg: u32,
    pub idqsif: u32,
    pub stName: XlNullableWideString,
    pub stCaption: XlNullableWideString,
    pub stTotal: XlNullableWideString,
    pub stStyleHeader: XlNullableWideString,
    pub stStyleInsertRow: XlNullableWideString,
    pub stStyleAgg: XlNullableWideString,
}

impl Default for BrtBeginListCol {
    fn default() -> Self {
        Self {
            idField: 1,
            ilta: 0,
            nDxfHdr: NO_DXF,
            nDxfInsertRow: NO_DXF,
            nDxfAgg: NO_DXF,
            idqsif: 0,
            stName: XlNullableWideString::default(),
            stCaption: XlNullableWideString::construct(Some("Column1")),
            stTotal: XlNullableWideString::default(),
            stStyleHeader: XlNullableWideString::default(),
            stStyleInsertRow: XlNullableWideString::default(),
            stStyleAgg: XlNullableWideString::default(),
        }
    }
}

impl Checked for BrtBeginListCol {
    fn check(&self) -> Result<(), &'static str> {
        if self.idField == 0 {
            return Err("table column id must be at least 1");
        }
        if self.ilta > 9 {
            return Err("invalid table total function");
        }
        if self.stCaption.is_null() {
            return Err("table column caption must not be null");
        }
        for value in [
            &self.stName,
            &self.stCaption,
            &self.stStyleHeader,
            &self.stStyleInsertRow,
            &self.stStyleAgg,
        ] {
            if !value.is_null() && value.len() > 255 {
                return Err("table column string exceeds 255 characters");
            }
        }
        if !self.stTotal.is_null() && self.stTotal.len() > 8189 {
            return Err("table total string exceeds 8189 characters");
        }
        Ok(())
    }
}

assign_id!(BrtBeginListCol);

/// Total row formula for one table column; MS-XLSB `2.4.708`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtListTrFmla {
    pub flags: u8,
    pub formula: CellParsedFormula,
}

impl BrtListTrFmla {
    /// The formula is an array formula.
    pub const F_ARRAY: u8 = 0x02;
}

impl Checked for BrtListTrFmla {
    fn check(&self) -> Result<(), &'static str> {
        if self.flags & !Self::F_ARRAY != 0 {
            return Err("reserved table total formula flags must be zero");
        }
        self.formula.check()
    }
}

assign_id!(BrtListTrFmla);

/// Beginning of the table-column collection; MS-XLSB `2.4.102`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginListCols {
    pub nCols: u32,
}

impl Unchecked for BrtBeginListCols {}

assign_id!(BrtBeginListCols);

/// End of a table-column definition; MS-XLSB `2.4.455`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndListCol {}

impl Unchecked for BrtEndListCol {}

assign_id!(BrtEndListCol);

/// End of the table-column collection; MS-XLSB `2.4.456`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndListCols {}

impl Unchecked for BrtEndListCols {}

assign_id!(BrtEndListCols);

/// Beginning of a worksheet table-part reference collection; MS-XLSB `2.4.103`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginListParts {
    pub cParts: u32,
}

impl Unchecked for BrtBeginListParts {}

assign_id!(BrtBeginListParts);

/// Reference to one Table part; MS-XLSB `2.4.707`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtListPart {
    pub stRelID: XlNullableWideString,
}

impl Default for BrtListPart {
    fn default() -> Self {
        Self {
            stRelID: XlNullableWideString::construct(Some("rId1")),
        }
    }
}

impl Checked for BrtListPart {
    fn check(&self) -> Result<(), &'static str> {
        if !self.stRelID.is_null() && self.stRelID.len() > 255 {
            return Err("table relationship id exceeds 255 characters");
        }
        Ok(())
    }
}

assign_id!(BrtListPart);

/// End of a worksheet table-part reference collection; MS-XLSB `2.4.457`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndListParts {}

impl Unchecked for BrtEndListParts {}

assign_id!(BrtEndListParts);

/// Table style flags and name; MS-XLSB `2.4.849`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtTableStyleClient {
    pub flags: u16,
    pub stStyleName: XlNullableWideString,
}

impl BrtTableStyleClient {
    /// Apply the first-column style elements.
    pub const F_FIRST_COLUMN: u16 = 0x01;
    /// Apply the last-column style elements.
    pub const F_LAST_COLUMN: u16 = 0x02;
    /// Apply row-stripe style elements.
    pub const F_ROW_STRIPES: u16 = 0x04;
    /// Apply column-stripe style elements.
    pub const F_COLUMN_STRIPES: u16 = 0x08;
    /// Apply row-header style elements.
    pub const F_ROW_HEADERS: u16 = 0x10;
    /// Apply column-header style elements.
    pub const F_COLUMN_HEADERS: u16 = 0x20;
}

impl Default for BrtTableStyleClient {
    fn default() -> Self {
        Self {
            flags: Self::F_ROW_STRIPES,
            stStyleName: XlNullableWideString::construct(Some("TableStyleMedium9")),
        }
    }
}

impl Checked for BrtTableStyleClient {
    fn check(&self) -> Result<(), &'static str> {
        if self.flags & !0x3f != 0 {
            return Err("reserved table style flags must be zero");
        }
        if !self.stStyleName.is_null() && self.stStyleName.len() > 255 {
            return Err("table style name exceeds 255 characters");
        }
        Ok(())
    }
}

assign_id!(BrtTableStyleClient);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn begin_list_checks_range_id_flags_and_strings() {
        let mut record = BrtBeginList::default();
        record.check().expect("valid");
        assert_eq!(record.to_bytes().expect("bytes").len(), 112);

        record.rfxList.rwLast = 0;
        assert!(record.check().is_err(), "header row must fit");
        record = BrtBeginList::default();
        record.idList = 0;
        assert!(record.check().is_err(), "id must be non-zero");
        record = BrtBeginList::default();
        record.flags = 0x20;
        assert!(record.check().is_err(), "reserved flag");
        record = BrtBeginList::default();
        record.stName = XlNullableWideString::construct(Some("x".repeat(256)));
        assert!(record.check().is_err(), "name limit");
        record = BrtBeginList::default();
        record.stDisplayName = XlNullableWideString::default();
        assert!(record.check().is_err(), "display name required");
    }

    #[test]
    fn begin_list_round_trips_fixed_fields_and_strings() {
        let record = BrtBeginList {
            rfxList: UncheckedRfX {
                rwFirst: 2,
                rwLast: 9,
                colFirst: 1,
                colLast: 4,
            },
            idList: 7,
            ..BrtBeginList::default()
        };
        let bytes = record.to_bytes().expect("bytes");
        let (_, parsed) = BrtBeginList::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(parsed, record);
        assert_eq!(&bytes[..4], &2_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &1_u32.to_le_bytes());
    }

    #[test]
    fn begin_list_col_checks_id_caption_and_limits() {
        let mut record = BrtBeginListCol::default();
        record.check().expect("valid");
        assert_eq!(record.to_bytes().expect("bytes").len(), 62);

        record.idField = 0;
        assert!(record.check().is_err(), "id required");
        record = BrtBeginListCol::default();
        record.stCaption = XlNullableWideString::default();
        assert!(record.check().is_err(), "caption required");
        record = BrtBeginListCol::default();
        record.stTotal = XlNullableWideString::construct(Some("x".repeat(8_190)));
        assert!(record.check().is_err(), "total limit");
        record = BrtBeginListCol::default();
        record.ilta = 10;
        assert!(record.check().is_err(), "total function limit");
    }

    #[test]
    fn list_total_formula_rejects_reserved_flags() {
        let mut record = BrtListTrFmla::default();
        record.check().expect("valid");
        assert_eq!(record.to_bytes().expect("bytes").len(), 12);

        record.flags = 0x04;
        assert!(record.check().is_err(), "reserved flags");
    }

    #[test]
    fn style_client_packs_flags_before_name() {
        let record = BrtTableStyleClient::default();
        record.check().expect("valid");
        let bytes = record.to_bytes().expect("bytes");
        assert_eq!(bytes.len(), 40);
        assert_eq!(
            &bytes[..2],
            &BrtTableStyleClient::F_ROW_STRIPES.to_le_bytes()
        );
        assert_eq!(&bytes[2..6], &17_u32.to_le_bytes());
        assert!(
            BrtTableStyleClient {
                flags: 0x40,
                ..BrtTableStyleClient::default()
            }
            .check()
            .is_err()
        );
    }
}
