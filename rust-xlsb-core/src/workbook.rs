//! The first real workbook model: an owner of the shared-string table and the
//! style registry.
//!
//! Only the parts needed by value conversion and worksheet styling exist so
//! far; sheet data and package assembly arrive with the worksheet writer.

use std::collections::BTreeMap;

use crate::context::WbContext;
use crate::styles::{CellStyle, StyleRegistry};
use crate::{Error, Result, SSTHolder};

const MAX_ROW: u32 = 1_048_575;
const MAX_COL: u32 = 16_383;

/// Owns the workbook shared-string table and style registry for one write.
pub struct Workbook {
    sst: SSTHolder,
    styles: StyleRegistry,
}

impl Workbook {
    /// Creates a workbook backed by an in-memory shared-string table and the
    /// default style.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Sst`] if the in-memory SQLite table cannot be
    /// initialized.
    pub fn new() -> Result<Self> {
        Ok(Self {
            sst: SSTHolder::create_in_memory()?,
            styles: StyleRegistry::new(),
        })
    }

    /// Wraps a caller-owned write-session shared-string table.
    ///
    /// The holder must not be a table reopened from disk; see the design spec.
    #[must_use]
    pub fn with_sst(sst: SSTHolder) -> Self {
        Self {
            sst,
            styles: StyleRegistry::new(),
        }
    }

    /// Returns a worksheet bound to this workbook's shared strings and styles.
    #[must_use]
    pub const fn worksheet(&self) -> Worksheet<'_> {
        Worksheet {
            parent: self,
            column_styles: BTreeMap::new(),
            row_styles: BTreeMap::new(),
        }
    }

    /// Returns the workbook style registry.
    #[must_use]
    pub const fn styles(&self) -> &StyleRegistry {
        &self.styles
    }

    /// Registers a custom number format and returns its `iFmt` id.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if the id space is exhausted.
    pub fn add_number_format(&mut self, code: &str) -> Result<u16> {
        self.styles.add_number_format(code)
    }

    /// Registers a cell style and returns its zero-based `BrtXF` index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if a style table would exceed the
    /// MS-XLSB record limit.
    pub fn add_style(&mut self, style: CellStyle) -> Result<u32> {
        self.styles.add(style)
    }
}

impl WbContext for Workbook {
    fn shared_strings(&self) -> &SSTHolder {
        &self.sst
    }
}

/// A worksheet context. Values converted through it intern into the parent
/// workbook's shared-string table, and style assignments reference the parent
/// workbook's style registry.
pub struct Worksheet<'wb> {
    parent: &'wb Workbook,
    column_styles: BTreeMap<u32, (u32, u32)>,
    row_styles: BTreeMap<u32, u32>,
}

impl Worksheet<'_> {
    /// Assigns a default style to columns `first..=last`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::StyleOutOfRange`] if `style` is not registered in the
    /// parent workbook, [`Error::ValidationError`] if the column range is
    /// invalid or overlaps an existing range.
    pub fn set_column_style(&mut self, first: u32, last: u32, style: u32) -> Result<()> {
        self.check_style(style)?;
        if first > last || last > MAX_COL {
            return Err(Error::ValidationError(
                "column range is out of bounds".to_owned(),
            ));
        }
        if self
            .column_styles
            .iter()
            .any(|(&existing_first, &(existing_last, _))| {
                first <= existing_last && existing_first <= last
            })
        {
            return Err(Error::ValidationError(
                "column style ranges overlap".to_owned(),
            ));
        }
        self.column_styles.insert(first, (last, style));
        Ok(())
    }

    /// Assigns a default style to one row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::StyleOutOfRange`] if `style` is not registered in the
    /// parent workbook, or [`Error::ValidationError`] if `row` is out of range.
    pub fn set_row_style(&mut self, row: u32, style: u32) -> Result<()> {
        self.check_style(style)?;
        if row > MAX_ROW {
            return Err(Error::ValidationError(
                "row index is out of bounds".to_owned(),
            ));
        }
        self.row_styles.insert(row, style);
        Ok(())
    }

    /// Column style ranges as `(first, last, style)` in column order.
    pub fn column_styles(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.column_styles
            .iter()
            .map(|(&first, &(last, style))| (first, last, style))
    }

    /// Row styles as `(row, style)` in row order.
    pub fn row_styles(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.row_styles.iter().map(|(&row, &style)| (row, style))
    }

    fn check_style(&self, style: u32) -> Result<()> {
        let len = self.parent.styles.len();
        if style < len {
            Ok(())
        } else {
            Err(Error::StyleOutOfRange { index: style, len })
        }
    }
}

impl WbContext for Worksheet<'_> {
    fn shared_strings(&self) -> &SSTHolder {
        self.parent.shared_strings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::WbContext;

    #[test]
    fn worksheet_shares_the_workbook_sst() {
        let workbook = Workbook::new().expect("in-memory workbook");
        let worksheet = workbook.worksheet();

        let hello = worksheet.shared_string("hello").expect("interns");
        assert_eq!(hello.id(), 0);
        assert_eq!(workbook.shared_strings().len(), 1);

        let again = workbook.shared_string("hello").expect("interns");
        assert_eq!(again, hello);
        assert_eq!(worksheet.shared_strings().len(), 1);
    }

    #[test]
    fn ids_follow_pushes() {
        let workbook = Workbook::new().expect("in-memory workbook");
        let first = workbook.shared_string("a").expect("interns");
        let second = workbook.shared_string("b").expect("interns");
        assert_eq!((first.id(), second.id()), (0, 1));
        assert_eq!(workbook.shared_strings().len(), 2);
    }

    #[test]
    fn empty_sst_is_reported() {
        let workbook = Workbook::new().expect("in-memory workbook");
        assert!(workbook.shared_strings().is_empty());
    }

    #[test]
    fn worksheet_rejects_unknown_styles() {
        let mut workbook = Workbook::new().expect("in-memory workbook");
        let style = workbook.add_style(CellStyle::default()).expect("adds");
        assert_eq!(style, 0);
        let mut worksheet = workbook.worksheet();

        assert!(matches!(
            worksheet.set_row_style(0, 1),
            Err(Error::StyleOutOfRange { index: 1, len: 1 })
        ));

        worksheet.set_row_style(0, style).expect("valid style");
    }

    #[test]
    fn worksheet_rejects_overlapping_columns() {
        let workbook = Workbook::new().expect("in-memory workbook");
        let mut worksheet = workbook.worksheet();

        worksheet.set_column_style(1, 3, 0).expect("first range");
        assert!(matches!(
            worksheet.set_column_style(3, 5, 0),
            Err(Error::ValidationError(_))
        ));
        worksheet.set_column_style(4, 5, 0).expect("adjacent range");
    }
}
