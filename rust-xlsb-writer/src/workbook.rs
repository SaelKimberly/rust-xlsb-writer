//! Workbook owner and save entry points.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use crate::defined_name::{DefinedName, sheet_index};
use crate::worksheet::{Worksheet, default_column_metrics, validate_sheet_name};
use crate::{DocProperties, Error, Format, Result};

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
    pub zip_compression_level: Option<u8>,
    pub zip_large_file: bool,
}

/// A workbook that owns one or more worksheets.
pub struct Workbook {
    worksheets: Vec<Worksheet>,
    settings: WorkbookSettings,
    doc_properties: Option<DocProperties>,
    default_format: Option<Format>,
    default_row_height: Option<u32>,
    default_col_width: Option<f64>,
    defined_names: Vec<DefinedName>,
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
                zip_compression_level: None,
                zip_large_file: false,
            },
            doc_properties: None,
            default_format: None,
            default_row_height: None,
            default_col_width: None,
            defined_names: Vec::new(),
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

    /// Sets the ZIP compression level for the XLSB package.
    ///
    /// Valid levels are 1 through 9, where 1 is fastest and 9 is smallest.
    /// Values outside that range are ignored. The default is uncompressed
    /// storage.
    pub fn set_zip_compression_level(&mut self, level: u8) -> &mut Self {
        if !(1..=9).contains(&level) {
            eprintln!("Zip compression level {level} outside range: 1 <= level <= 9.");
            return self;
        }
        self.settings.zip_compression_level = Some(level);
        self
    }

    /// Enables ZIP64 extensions for the XLSB package.
    ///
    /// ZIP64 is required when the archive or one of its parts can exceed
    /// 4 GiB. It is off by default.
    pub const fn use_zip_large_file(&mut self, enable: bool) -> &mut Self {
        self.settings.zip_large_file = enable;
        self
    }

    /// Defines a workbook or worksheet-scoped name.
    ///
    /// Use `Sheet1!Name` or `'Sheet 1'!Name` for a worksheet-scoped name. The
    /// formula may be a number, string, boolean, error, or an absolute A1 cell
    /// or range reference. Global references must include a sheet name; local
    /// references resolve against the name's worksheet.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the name violates Excel's naming rules.
    /// Formula shapes and sheet scopes are validated when the workbook is saved.
    pub fn define_name(&mut self, name: impl Into<String>, formula: &str) -> Result<&mut Self> {
        self.defined_names.push(DefinedName::new(name, formula)?);
        Ok(self)
    }

    /// Sets the workbook document properties.
    pub fn set_properties(&mut self, properties: &DocProperties) -> &mut Self {
        self.doc_properties = Some(properties.clone());
        self
    }

    /// Sets the default cell format and worksheet dimensions.
    ///
    /// Must be called before adding worksheets. `row_height` and `col_width`
    /// are in pixels; supported column widths are 56, 64, 72, 80, 96, 104, and
    /// 120.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] if a worksheet already exists or the
    /// column width is unsupported. Returns [`Error::ValueNotRepresentable`]
    /// if the row height exceeds Excel's limit.
    pub fn set_default_format(
        &mut self,
        format: &Format,
        row_height: u32,
        col_width: u32,
    ) -> Result<&mut Self> {
        if !self.worksheets.is_empty() {
            return Err(Error::Parameter(
                "default format must be set before adding worksheets".to_owned(),
            ));
        }
        let Some((digit_width, padding)) = default_column_metrics(col_width) else {
            return Err(Error::Parameter(format!(
                "unsupported default column width: {col_width}"
            )));
        };
        if f64::from(row_height) * 0.75 > 409.5 {
            return Err(Error::ValueNotRepresentable(
                "default row height exceeds 409.5 points",
            ));
        }

        self.default_format = Some(format.clone());
        self.default_row_height = Some(row_height);
        self.default_col_width =
            Some((f64::from(col_width) - f64::from(padding)) / f64::from(digit_width));
        Ok(self)
    }

    /// Adds a worksheet with a default `SheetN` name.
    pub fn add_worksheet(&mut self) -> &mut Worksheet {
        let name = format!("Sheet{}", self.worksheets.len() + 1);
        let mut worksheet = Worksheet::new_named(name);
        self.apply_default_dimensions(&mut worksheet);
        self.worksheets.push(worksheet);
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
        self.apply_default_dimensions(&mut worksheet);
        self.worksheets.push(worksheet);
    }

    fn apply_default_dimensions(&self, worksheet: &mut Worksheet) {
        if let (Some(format), Some(height), Some(width)) = (
            self.default_format.as_ref(),
            self.default_row_height,
            self.default_col_width,
        ) {
            worksheet.initialize_default_format(format, height, width);
        }
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
        crate::package::build_package(
            &self.worksheets,
            self.settings,
            self.doc_properties.as_ref(),
            self.default_format.as_ref(),
            &self.defined_names,
        )
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

        for sheet in &mut self.worksheets {
            sheet.apply_filters();
        }

        let mut defined_names = HashSet::new();
        for name in &self.defined_names {
            if let Some(scope) = name.scope.as_deref() {
                sheet_index(&self.worksheets, scope)?;
            }
            let key = format!(
                "{}!{}",
                name.scope.as_deref().unwrap_or("").to_lowercase(),
                name.name.to_lowercase()
            );
            if !defined_names.insert(key) {
                return Err(Error::Parameter(format!(
                    "defined name already used: {}",
                    name.name
                )));
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
    fn default_format_must_precede_worksheets() {
        let mut workbook = Workbook::new();
        let format = crate::Format::new().set_bold();

        workbook
            .set_default_format(&format, 24, 80)
            .expect("sets before worksheets");
        workbook.add_worksheet();

        assert!(matches!(
            workbook.set_default_format(&format, 24, 80),
            Err(Error::Parameter(_))
        ));
    }

    #[test]
    fn rejects_unsupported_default_column_width() {
        let mut workbook = Workbook::new();
        let format = crate::Format::new();

        assert!(matches!(
            workbook.set_default_format(&format, 24, 65),
            Err(Error::Parameter(_))
        ));
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
