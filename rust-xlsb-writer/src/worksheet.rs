//! Worksheet data model and public write/layout API.

use std::collections::{BTreeMap, HashMap};

use crate::data::{CellData, IntoExcelData, IntoExcelDateTime};
use crate::{Error, Format, Result};

pub const MAX_ROW: u32 = 1_048_575;
pub const MAX_COL: u16 = 16_383;
const MAX_STRING_UTF16: usize = 32_767;

/// Zero-based row number.
pub type RowNum = u32;

/// Zero-based column number.
pub type ColNum = u16;

#[derive(Debug, Clone)]
pub struct Cell {
    pub(crate) data: CellData,
    pub(crate) format: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct RowOptions {
    pub(crate) height: Option<f64>,
    pub(crate) format: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColumnOptions {
    pub(crate) width: Option<f64>,
    pub(crate) format: Option<u32>,
}

/// One worksheet with sparse cells, row options, and column options.
pub struct Worksheet {
    name: String,
    cells: BTreeMap<u32, BTreeMap<u16, Cell>>,
    rows: BTreeMap<u32, RowOptions>,
    columns: BTreeMap<u16, ColumnOptions>,
    formats: Vec<Format>,
    format_ids: HashMap<Format, u32>,
}

impl Worksheet {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            cells: BTreeMap::new(),
            rows: BTreeMap::new(),
            columns: BTreeMap::new(),
            formats: Vec::new(),
            format_ids: HashMap::new(),
        }
    }

    /// Sets the worksheet name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidSheetName`] when the name is empty, longer than
    /// 31 characters, or contains `[ ] : * ? / \`.
    pub fn set_name(&mut self, name: impl Into<String>) -> Result<&mut Self> {
        let name = name.into();
        validate_sheet_name(&name)?;
        self.name = name;
        Ok(self)
    }

    /// Returns the worksheet name.
    #[must_use]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Writes a value without formatting.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the string is too
    /// long, or the value cannot be represented.
    pub fn write(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: impl IntoExcelData,
    ) -> Result<&mut Self> {
        data.write(self, row, col)
    }

    /// Writes a value with formatting.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the string is too
    /// long, or the value cannot be represented.
    pub fn write_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: impl IntoExcelData,
        format: &Format,
    ) -> Result<&mut Self> {
        data.write_with_format(self, row, col, format)
    }

    /// Writes an iterator as a row.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_row<I>(&mut self, row: RowNum, col: ColNum, data: I) -> Result<&mut Self>
    where
        I: IntoIterator,
        I::Item: IntoExcelData,
    {
        for (offset, value) in data.into_iter().enumerate() {
            let offset = u16::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = col.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            value.write(self, row, target)?;
        }
        Ok(self)
    }

    /// Writes an iterator as a column.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_column<I>(&mut self, row: RowNum, col: ColNum, data: I) -> Result<&mut Self>
    where
        I: IntoIterator,
        I::Item: IntoExcelData,
    {
        for (offset, value) in data.into_iter().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = row.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            value.write(self, target, col)?;
        }
        Ok(self)
    }

    /// Writes a string.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the string is
    /// longer than 32,767 UTF-16 code units.
    pub fn write_string(
        &mut self,
        row: RowNum,
        col: ColNum,
        value: impl Into<String>,
    ) -> Result<&mut Self> {
        self.write(row, col, value.into())
    }

    /// Writes a formatted string.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the string is
    /// longer than 32,767 UTF-16 code units.
    pub fn write_string_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        value: impl Into<String>,
        format: &Format,
    ) -> Result<&mut Self> {
        self.write_with_format(row, col, value.into(), format)
    }

    /// Writes a number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range.
    pub fn write_number(
        &mut self,
        row: RowNum,
        col: ColNum,
        value: impl Into<f64>,
    ) -> Result<&mut Self> {
        self.write(row, col, value.into())
    }

    /// Writes a formatted number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range.
    pub fn write_number_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        value: impl Into<f64>,
        format: &Format,
    ) -> Result<&mut Self> {
        self.write_with_format(row, col, value.into(), format)
    }

    /// Writes a boolean.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range.
    pub fn write_boolean(&mut self, row: RowNum, col: ColNum, value: bool) -> Result<&mut Self> {
        self.write(row, col, value)
    }

    /// Writes a formatted boolean.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range.
    pub fn write_boolean_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        value: bool,
        format: &Format,
    ) -> Result<&mut Self> {
        self.write_with_format(row, col, value, format)
    }

    /// Writes a formatted blank cell.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range.
    pub fn write_blank(&mut self, row: RowNum, col: ColNum, format: &Format) -> Result<&mut Self> {
        self.write_with_format(row, col, (), format)
    }

    /// Writes a date.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the date cannot
    /// be represented.
    pub fn write_date(
        &mut self,
        row: RowNum,
        col: ColNum,
        date: impl IntoExcelDateTime,
    ) -> Result<&mut Self> {
        let cell_data = date.into_cell_data()?;
        self.store(row, col, cell_data, None)
    }

    /// Writes a formatted date.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the date cannot
    /// be represented.
    pub fn write_date_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        date: impl IntoExcelDateTime,
        format: &Format,
    ) -> Result<&mut Self> {
        let cell_data = date.into_cell_data()?;
        self.store(row, col, cell_data, Some(format))
    }

    /// Writes a time.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the time cannot
    /// be represented.
    pub fn write_time(
        &mut self,
        row: RowNum,
        col: ColNum,
        time: impl IntoExcelDateTime,
    ) -> Result<&mut Self> {
        let cell_data = time.into_cell_data()?;
        self.store(row, col, cell_data, None)
    }

    /// Writes a formatted time.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the time cannot
    /// be represented.
    pub fn write_time_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        time: impl IntoExcelDateTime,
        format: &Format,
    ) -> Result<&mut Self> {
        let cell_data = time.into_cell_data()?;
        self.store(row, col, cell_data, Some(format))
    }

    /// Writes a datetime.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the datetime
    /// cannot be represented.
    pub fn write_datetime(
        &mut self,
        row: RowNum,
        col: ColNum,
        datetime: impl IntoExcelDateTime,
    ) -> Result<&mut Self> {
        let cell_data = datetime.into_cell_data()?;
        self.store(row, col, cell_data, None)
    }

    /// Writes a formatted datetime.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range or the datetime
    /// cannot be represented.
    pub fn write_datetime_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        datetime: impl IntoExcelDateTime,
        format: &Format,
    ) -> Result<&mut Self> {
        let cell_data = datetime.into_cell_data()?;
        self.store(row, col, cell_data, Some(format))
    }

    /// Sets one column width in character units.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the column is out of range, or
    /// [`Error::ValueNotRepresentable`] when the width is outside `0..=255`.
    pub fn set_column_width(&mut self, col: ColNum, width: impl Into<f64>) -> Result<&mut Self> {
        let width = width.into();
        Self::check_col(col)?;
        validate_column_width(width)?;
        self.columns.entry(col).or_default().width = Some(width);
        Ok(self)
    }

    /// Sets a column range width in character units.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::ValueNotRepresentable`] when the width is outside `0..=255`.
    pub fn set_column_range_width(
        &mut self,
        first: ColNum,
        last: ColNum,
        width: impl Into<f64>,
    ) -> Result<&mut Self> {
        if first > last {
            return Err(Error::RowColumnOrder);
        }
        Self::check_col(first)?;
        Self::check_col(last)?;
        let width = width.into();
        validate_column_width(width)?;
        for col in first..=last {
            self.columns.entry(col).or_default().width = Some(width);
        }
        Ok(self)
    }

    /// Sets one column format.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the column is out of range.
    pub fn set_column_format(&mut self, col: ColNum, format: &Format) -> Result<&mut Self> {
        Self::check_col(col)?;
        let format = self.intern_format(format);
        self.columns.entry(col).or_default().format = Some(format);
        Ok(self)
    }

    /// Sets a column range format.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`, or
    /// [`Error::RowColumnLimit`] when the range is out of bounds.
    pub fn set_column_range_format(
        &mut self,
        first: ColNum,
        last: ColNum,
        format: &Format,
    ) -> Result<&mut Self> {
        if first > last {
            return Err(Error::RowColumnOrder);
        }
        Self::check_col(first)?;
        Self::check_col(last)?;
        let format = self.intern_format(format);
        for col in first..=last {
            self.columns.entry(col).or_default().format = Some(format);
        }
        Ok(self)
    }

    /// Sets one row height in points.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row is out of range, or
    /// [`Error::ValueNotRepresentable`] when the height is outside `0..=409.5`.
    pub fn set_row_height(&mut self, row: RowNum, height: impl Into<f64>) -> Result<&mut Self> {
        let height = height.into();
        Self::check_row(row)?;
        validate_row_height(height)?;
        self.rows.entry(row).or_default().height = Some(height);
        Ok(self)
    }

    /// Sets one row format.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row is out of range.
    pub fn set_row_format(&mut self, row: RowNum, format: &Format) -> Result<&mut Self> {
        Self::check_row(row)?;
        let format = self.intern_format(format);
        self.rows.entry(row).or_default().format = Some(format);
        Ok(self)
    }

    pub(crate) fn store(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: CellData,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        if let CellData::String(text) = &data
            && text.encode_utf16().count() > MAX_STRING_UTF16
        {
            return Err(Error::StringTooLong);
        }
        let format = format.map(|format| self.intern_format(format));
        self.cells
            .entry(row)
            .or_default()
            .insert(col, Cell { data, format });
        Ok(self)
    }

    fn intern_format(&mut self, format: &Format) -> u32 {
        if let Some(&index) = self.format_ids.get(format) {
            return index;
        }
        let index = u32::try_from(self.formats.len()).expect("format count fits u32");
        self.formats.push(format.clone());
        self.format_ids.insert(format.clone(), index);
        index
    }

    const fn check_row(row: RowNum) -> Result<()> {
        if row > MAX_ROW {
            Err(Error::RowColumnLimit)
        } else {
            Ok(())
        }
    }

    const fn check_col(col: ColNum) -> Result<()> {
        if col > MAX_COL {
            Err(Error::RowColumnLimit)
        } else {
            Ok(())
        }
    }

    #[must_use]
    pub const fn cells(&self) -> &BTreeMap<u32, BTreeMap<u16, Cell>> {
        &self.cells
    }

    #[must_use]
    pub const fn row_options(&self) -> &BTreeMap<u32, RowOptions> {
        &self.rows
    }

    #[must_use]
    pub const fn column_options(&self) -> &BTreeMap<u16, ColumnOptions> {
        &self.columns
    }

    pub(crate) fn formats(&self) -> &[Format] {
        &self.formats
    }

    pub(crate) fn name_ref(&self) -> &str {
        &self.name
    }
}

pub fn validate_sheet_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.chars().count() > 31
        || name
            .chars()
            .any(|ch| matches!(ch, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
    {
        Err(Error::InvalidSheetName(name.to_owned()))
    } else {
        Ok(())
    }
}

fn validate_column_width(width: f64) -> Result<()> {
    if width.is_finite() && (0.0..=255.0).contains(&width) {
        Ok(())
    } else {
        Err(Error::ValueNotRepresentable(
            "column width is outside 0..=255",
        ))
    }
}

fn validate_row_height(height: f64) -> Result<()> {
    if height.is_finite() && (0.0..=409.5).contains(&height) {
        Ok(())
    } else {
        Err(Error::ValueNotRepresentable(
            "row height is outside 0..=409.5",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CellData;

    #[test]
    fn writes_cells_and_styles() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet
            .write_with_format(0, 0, "hello", &Format::new().set_bold())
            .expect("writes");

        assert_eq!(worksheet.formats().len(), 1);
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| (&cell.data, cell.format)),
            Some((&CellData::String("hello".to_owned()), Some(0)))
        );
    }

    #[test]
    fn rejects_bounds_and_long_strings() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.write(MAX_ROW + 1, 0, 1_i32),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.write(0, MAX_COL + 1, 1_i32),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.write(0, 0, "x".repeat(32_768)),
            Err(Error::StringTooLong)
        ));
    }

    #[test]
    fn later_column_width_overrides_earlier_width() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_column_width(1, 10.0).expect("width");
        worksheet.set_column_width(1, 20.0).expect("width");

        assert_eq!(
            worksheet
                .column_options()
                .get(&1)
                .and_then(|options| options.width),
            Some(20.0)
        );
    }

    #[test]
    fn sets_row_and_column_formats() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let format = Format::new().set_bold();
        worksheet.set_row_format(0, &format).expect("row");
        worksheet.set_column_format(0, &format).expect("column");

        assert_eq!(worksheet.formats().len(), 1);
        assert_eq!(
            worksheet
                .row_options()
                .get(&0)
                .and_then(|options| options.format),
            Some(0)
        );
        assert_eq!(
            worksheet
                .column_options()
                .get(&0)
                .and_then(|options| options.format),
            Some(0)
        );
    }

    #[test]
    fn validates_sheet_names() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(worksheet.set_name("Data").is_ok());
        assert!(matches!(
            worksheet.set_name(""),
            Err(Error::InvalidSheetName(_))
        ));
        assert!(matches!(
            worksheet.set_name("x".repeat(32)),
            Err(Error::InvalidSheetName(_))
        ));
        assert!(matches!(
            worksheet.set_name("bad/name"),
            Err(Error::InvalidSheetName(_))
        ));
    }

    #[test]
    fn write_row_keeps_the_cells_before_the_failure() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.write_row(0, MAX_COL, [1_i32, 2_i32]),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&MAX_COL))
                .map(|cell| &cell.data),
            Some(&CellData::Integer(1))
        );
    }

    #[test]
    fn rejects_reversed_column_range() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_column_range_width(2, 1, 10.0),
            Err(Error::RowColumnOrder)
        ));
    }
}
