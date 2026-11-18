//! Workbook owner and save entry points.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use crate::worksheet::{Worksheet, validate_sheet_name};
use crate::{Error, Result};

/// A workbook that owns one or more worksheets.
pub struct Workbook {
    worksheets: Vec<Worksheet>,
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

impl Workbook {
    /// Creates an empty workbook.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            worksheets: Vec::new(),
        }
    }

    /// Adds a worksheet with a default `SheetN` name.
    pub fn add_worksheet(&mut self) -> &mut Worksheet {
        let name = format!("Sheet{}", self.worksheets.len() + 1);
        self.worksheets.push(Worksheet::new(name));
        self.worksheets
            .last_mut()
            .expect("worksheet was just pushed")
    }

    /// Returns a worksheet by zero-based index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::WorksheetIndex`] when the index is out of range.
    pub fn worksheet_from_index(&mut self, index: usize) -> Result<&mut Worksheet> {
        self.worksheets
            .get_mut(index)
            .ok_or(Error::WorksheetIndex(index))
    }

    /// Saves the workbook to a path.
    ///
    /// # Errors
    ///
    /// Returns a validation error before touching the target, or an I/O error
    /// while creating or writing the file.
    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let bytes = self.save_to_buffer()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Saves the workbook to a byte buffer.
    ///
    /// # Errors
    ///
    /// Returns a validation, style, shared-string, or record error.
    pub fn save_to_buffer(&mut self) -> Result<Vec<u8>> {
        self.prepare()?;
        crate::package::build_package(&self.worksheets)
    }

    /// Saves the workbook to a writer.
    ///
    /// # Errors
    ///
    /// Returns a validation, package, or I/O error.
    pub fn save_to_writer<W: Write + Send>(&mut self, mut writer: W) -> Result<()> {
        let bytes = self.save_to_buffer()?;
        writer.write_all(&bytes)?;
        Ok(())
    }

    fn prepare(&mut self) -> Result<()> {
        if self.worksheets.is_empty() {
            self.add_worksheet();
        }
        let mut names = HashSet::new();
        for sheet in &self.worksheets {
            let name = sheet.name_ref();
            validate_sheet_name(name)?;
            if !names.insert(name.to_lowercase()) {
                return Err(Error::SheetNameReused(name.to_owned()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Data, Reader, open_workbook_auto_from_rs};
    use std::io::Cursor;

    #[test]
    fn saves_values_to_a_buffer() {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.write(0, 0, "hello").expect("writes");
        sheet.write(1, 0, 42_i32).expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let mut opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        let range = opened.worksheet_range("Sheet1").expect("reads");

        assert_eq!(range.get((0, 0)), Some(&Data::String("hello".to_owned())));
        assert_eq!(range.get((1, 0)), Some(&Data::Int(42)));
    }

    #[test]
    fn saves_multiple_sheets_and_defaults_to_one() {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().write(0, 0, "one").expect("writes");
        workbook.add_worksheet().write(0, 0, "two").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(
            opened.sheet_names(),
            ["Sheet1".to_owned(), "Sheet2".to_owned()]
        );

        let mut empty = Workbook::new();
        let bytes = empty.save_to_buffer().expect("saves");
        let opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(opened.sheet_names(), ["Sheet1".to_owned()]);
    }

    #[test]
    fn rejects_duplicate_sheet_names() {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().set_name("Data").expect("name");
        workbook.add_worksheet().set_name("data").expect("name");

        assert!(matches!(
            workbook.save_to_buffer(),
            Err(Error::SheetNameReused(_))
        ));
    }

    #[test]
    fn save_to_writer_matches_buffer() {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let buffer = workbook.save_to_buffer().expect("buffer");
        let mut written = Vec::new();
        workbook.save_to_writer(&mut written).expect("writer");

        assert_eq!(buffer, written);
    }

    #[test]
    fn worksheet_index_is_checked() {
        let mut workbook = Workbook::new();
        assert!(matches!(
            workbook.worksheet_from_index(0),
            Err(Error::WorksheetIndex(0))
        ));
    }
}
