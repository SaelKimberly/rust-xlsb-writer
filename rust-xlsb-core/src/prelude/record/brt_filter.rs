use deku::{DekuRead, DekuWrite};

use crate::prelude::{XlStringExt, XlWideString};
use crate::{Checked, Unchecked, assign_id};

/// `AutoFilter` column header; MS-XLSB `2.4.84`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[deku(bit_order = "lsb")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "bit layout mirrors the MS-XLSB record"
)]
pub struct BrtBeginFilterColumn {
    /// Zero-based column index relative to the `AutoFilter` range.
    pub dwCol: u32,
    #[deku(bits = 1)]
    pub fHideArrow: bool,
    #[deku(bits = 1, pad_bits_after = "14")]
    pub fNoBtn: bool,
}

impl Checked for BrtBeginFilterColumn {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

assign_id!(BrtBeginFilterColumn);

/// End of an `AutoFilter` column collection; MS-XLSB `2.4.438`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndFilterColumn {}

impl Unchecked for BrtEndFilterColumn {}

assign_id!(BrtEndFilterColumn);

/// Beginning of a list-filter collection; MS-XLSB `2.4.85`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginFilters {
    pub fBlank: u32,
    pub unused: u32,
}

impl Unchecked for BrtBeginFilters {}

assign_id!(BrtBeginFilters);

/// End of a list-filter collection; MS-XLSB `2.4.439`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndFilters {}

impl Unchecked for BrtEndFilters {}

assign_id!(BrtEndFilters);

/// One list-filter value; MS-XLSB `2.4.682`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtFilter {
    pub rgch: XlWideString,
}

impl Default for BrtFilter {
    fn default() -> Self {
        Self {
            rgch: XlWideString::construct(Some("Filter")),
        }
    }
}

impl Checked for BrtFilter {
    fn check(&self) -> Result<(), &'static str> {
        if !(1..=255).contains(&self.rgch.len()) {
            return Err("filter string must contain 1-255 UTF-16 code units");
        }
        Ok(())
    }
}

assign_id!(BrtFilter);

/// Beginning of a custom-filter collection; MS-XLSB `2.4.40`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginCustomFilters {
    pub fAnd: u32,
}

impl Unchecked for BrtBeginCustomFilters {}

assign_id!(BrtBeginCustomFilters);

/// End of a custom-filter collection; MS-XLSB `2.4.397`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndCustomFilters {}

impl Unchecked for BrtEndCustomFilters {}

assign_id!(BrtEndCustomFilters);

/// One custom-filter comparison; MS-XLSB `2.4.348`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtCustomFilter {
    /// `0x04` number, `0x06` string, `0x0C` blanks, `0x0E` non-blanks.
    pub vts: u8,
    /// `0x01` lt, `0x02` eq, `0x03` le, `0x04` gt, `0x05` ne, `0x06` ge.
    pub grbitSgn: u8,
    pub union: [u8; 8],
    #[deku(cond = "*vts == 0x06", default = "None")]
    pub vtsStringXls: Option<XlWideString>,
}

impl Default for BrtCustomFilter {
    fn default() -> Self {
        Self {
            vts: 0x04,
            grbitSgn: 0x02,
            union: [0; 8],
            vtsStringXls: None,
        }
    }
}

impl Checked for BrtCustomFilter {
    fn check(&self) -> Result<(), &'static str> {
        if !matches!(self.vts, 0x04 | 0x06 | 0x0C | 0x0E) {
            return Err("custom filter value type is not supported");
        }
        if !(0x01..=0x06).contains(&self.grbitSgn) {
            return Err("custom filter comparison is not supported");
        }
        match (self.vts, self.vtsStringXls.is_some()) {
            (0x06, false) => Err("string custom filter requires a string value"),
            (0x06, true) | (_, false) => Ok(()),
            (_, true) => Err("only a string custom filter carries a string value"),
        }
    }
}

assign_id!(BrtCustomFilter);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn filter_column_bits_follow_the_spec() {
        let record = BrtBeginFilterColumn {
            dwCol: 1,
            fHideArrow: true,
            fNoBtn: true,
        };
        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(bytes, [1, 0, 0, 0, 0x03, 0x00]);

        let (rest, decoded) = BrtBeginFilterColumn::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
    }

    #[test]
    fn custom_filter_carries_a_number_or_a_string() {
        let number = BrtCustomFilter {
            vts: 0x04,
            grbitSgn: 0x04,
            union: 70.0_f64.to_le_bytes(),
            vtsStringXls: None,
        };
        let bytes = number.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 10);
        assert_eq!(&bytes[..2], [0x04, 0x04]);
        assert_eq!(&bytes[2..], &70.0_f64.to_le_bytes());

        let string = BrtCustomFilter {
            vts: 0x06,
            grbitSgn: 0x02,
            union: [0; 8],
            vtsStringXls: Some(XlWideString::construct(Some("East"))),
        };
        let bytes = string.to_bytes().expect("serializes");
        assert_eq!(&bytes[..10], [0x06, 0x02, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bytes[10..14], &4_u32.to_le_bytes());

        let (_, decoded) = BrtCustomFilter::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(decoded, string);
    }

    #[test]
    fn filter_rejects_empty_and_oversized_strings() {
        let record = BrtFilter {
            rgch: XlWideString::construct(Some("")),
        };
        assert!(record.check().is_err());

        let record = BrtFilter {
            rgch: XlWideString::construct(Some("a".repeat(256))),
        };
        assert!(record.check().is_err());
    }
}
