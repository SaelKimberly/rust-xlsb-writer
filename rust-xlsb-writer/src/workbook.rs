//! Workbook owner and save entry points.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use crate::worksheet::{Worksheet, validate_sheet_name};
use crate::{Error, Result};

/// Formula calculation mode for a workbook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculationMode {
    /// Recalculate on load and whenever a dependency changes.
    Automatic,
    /// Recalculate only when the user requests it.
    Manual,
    /// Recalculate automatically, except for Data Tables.
    Partial,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkbookSettings {
    pub calculation_mode: Option<CalculationMode>,
    pub read_only_recommended: bool,
    pub tab_ratio: Option<u8>,
}

/// A workbook that owns one or more worksheets.
pub struct Workbook {
    worksheets: Vec<Worksheet>,
    settings: WorkbookSettings,
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
            settings: WorkbookSettings {
                calculation_mode: None,
                read_only_recommended: false,
                tab_ratio: None,
            },
        }
    }

    /// Recommends opening the workbook in read-only mode.
    pub const fn read_only_recommended(&mut self) -> &mut Self {
        self.settings.read_only_recommended = true;
        self
    }

    /// Sets the formula calculation mode.
    pub const fn set_calculation_mode(&mut self, mode: CalculationMode) -> &mut Self {
        self.settings.calculation_mode = Some(mode);
        self
    }

    /// Sets the worksheet tab bar ratio as a percentage from 0 to 100.
    ///
    /// Values above 100 are ignored.
    pub fn set_tab_ratio(&mut self, ratio: u8) -> &mut Self {
        if ratio > 100 {
            eprintln!("Tab ratio {ratio} outside Excel range: 0 <= ratio <= 100.");
            return self;
        }
        self.settings.tab_ratio = Some(ratio);
        self
    }

    /// Adds a worksheet with a default `SheetN` name.
    pub fn add_worksheet(&mut self) -> &mut Worksheet {
        let name = format!("Sheet{}", self.worksheets.len() + 1);
        self.worksheets.push(Worksheet::new_named(name));
        self.worksheets
            .last_mut()
            .expect("worksheet was just pushed")
    }

    /// Adds a standalone worksheet to the workbook.
    ///
    /// Keeps the worksheet's name. An unnamed worksheet created with
    /// [`Worksheet::new`] receives the default `SheetN` name.
    pub fn push_worksheet(&mut self, mut worksheet: Worksheet) {
        if worksheet.name_ref().is_empty() {
            let name = format!("Sheet{}", self.worksheets.len() + 1);
            worksheet
                .set_name(name)
                .expect("default sheet name is valid");
        }
        self.worksheets.push(worksheet);
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

    /// Returns a worksheet by exact, case-sensitive name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownWorksheetNameOrIndex`] when no worksheet has the
    /// name.
    pub fn worksheet_from_name(&mut self, sheetname: &str) -> Result<&mut Worksheet> {
        let index = self
            .worksheets
            .iter()
            .position(|worksheet| worksheet.name_ref() == sheetname)
            .ok_or_else(|| Error::UnknownWorksheetNameOrIndex(sheetname.to_owned()))?;
        self.worksheet_from_index(index)
    }

    /// Returns the mutable worksheet vector.
    pub const fn worksheets_mut(&mut self) -> &mut Vec<Worksheet> {
        &mut self.worksheets
    }

    /// Returns the worksheet vector.
    pub const fn worksheets(&mut self) -> &Vec<Worksheet> {
        &self.worksheets
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
        crate::package::build_package(&self.worksheets, self.settings)
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
    fn behavior_settings_do_not_block_saving() {
        let mut workbook = Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "settings")
            .expect("writes");
        workbook
            .read_only_recommended()
            .set_calculation_mode(CalculationMode::Manual)
            .set_tab_ratio(40);

        let bytes = workbook.save_to_buffer().expect("saves");
        let mut opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(
            opened.worksheet_range("Sheet1").expect("reads").get((0, 0)),
            Some(&Data::String("settings".to_owned()))
        );
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

    #[test]
    fn finds_a_worksheet_by_exact_name() {
        let mut workbook = Workbook::new();
        workbook.add_worksheet().set_name("Data").expect("name");
        workbook.add_worksheet().set_name("Other").expect("name");

        workbook
            .worksheet_from_name("Other")
            .expect("finds Other")
            .write(0, 0, "found")
            .expect("writes");

        assert!(matches!(
            workbook.worksheet_from_name("other"),
            Err(Error::UnknownWorksheetNameOrIndex(name)) if name == "other"
        ));

        let bytes = workbook.save_to_buffer().expect("saves");
        let mut opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(
            opened.worksheet_range("Other").expect("reads").get((0, 0)),
            Some(&Data::String("found".to_owned()))
        );
    }

    #[test]
    fn pushed_worksheets_keep_names_or_get_a_default() {
        let mut standalone = Worksheet::new();
        standalone.set_name("Data").expect("name");
        standalone.write(0, 0, "standalone").expect("writes");

        let mut workbook = Workbook::new();
        workbook.push_worksheet(Worksheet::new());
        assert_eq!(workbook.worksheets()[0].name(), "Sheet1");

        workbook.push_worksheet(standalone);

        assert_eq!(
            workbook
                .worksheets()
                .iter()
                .map(Worksheet::name)
                .collect::<Vec<_>>(),
            ["Sheet1".to_owned(), "Data".to_owned()]
        );

        let bytes = workbook.save_to_buffer().expect("saves");
        let mut opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(
            opened.sheet_names(),
            ["Sheet1".to_owned(), "Data".to_owned()]
        );
        assert_eq!(
            opened.worksheet_range("Data").expect("reads").get((0, 0)),
            Some(&Data::String("standalone".to_owned()))
        );
    }

    #[test]
    fn worksheet_collection_accessors_share_state() {
        let mut workbook = Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();

        workbook.worksheets_mut().swap(0, 1);
        workbook.worksheets_mut()[0]
            .write(0, 0, "swapped")
            .expect("writes");

        assert_eq!(workbook.worksheets()[0].name(), "Sheet2");
        assert_eq!(workbook.worksheets()[1].name(), "Sheet1");

        let bytes = workbook.save_to_buffer().expect("saves");
        let mut opened = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("opens");
        assert_eq!(
            opened.worksheet_range("Sheet2").expect("reads").get((0, 0)),
            Some(&Data::String("swapped".to_owned()))
        );
    }
}
