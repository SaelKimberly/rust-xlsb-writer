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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowOptions {
    pub(crate) height: Option<f64>,
    pub(crate) format: Option<u32>,
    pub(crate) hidden: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColumnOptions {
    pub(crate) width: Option<f64>,
    pub(crate) format: Option<u32>,
    pub(crate) hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeRange {
    pub(crate) first_row: u32,
    pub(crate) first_col: u16,
    pub(crate) last_row: u32,
    pub(crate) last_col: u16,
}

/// One worksheet with sparse cells, row options, and column options.
pub struct Worksheet {
    name: String,
    cells: BTreeMap<u32, BTreeMap<u16, Cell>>,
    rows: BTreeMap<u32, RowOptions>,
    columns: BTreeMap<u16, ColumnOptions>,
    formats: Vec<Format>,
    format_ids: HashMap<Format, u32>,
    merged_ranges: Vec<MergeRange>,
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
            merged_ranges: Vec::new(),
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

    /// Writes an iterator as a formatted row.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_row_with_format<I>(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: I,
        format: &Format,
    ) -> Result<&mut Self>
    where
        I: IntoIterator,
        I::Item: IntoExcelData,
    {
        for (offset, value) in data.into_iter().enumerate() {
            let offset = u16::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = col.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            value.write_with_format(self, row, target, format)?;
        }
        Ok(self)
    }

    /// Writes an iterator as a formatted column.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_column_with_format<I>(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: I,
        format: &Format,
    ) -> Result<&mut Self>
    where
        I: IntoIterator,
        I::Item: IntoExcelData,
    {
        for (offset, value) in data.into_iter().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = row.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            value.write_with_format(self, target, col, format)?;
        }
        Ok(self)
    }

    /// Writes an iterator of rows as a matrix.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_row_matrix<I, II>(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: I,
    ) -> Result<&mut Self>
    where
        I: IntoIterator<Item = II>,
        II: IntoIterator,
        II::Item: IntoExcelData,
    {
        for (offset, values) in data.into_iter().enumerate() {
            let offset = u32::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = row.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            self.write_row(target, col, values)?;
        }
        Ok(self)
    }

    /// Writes an iterator of columns as a matrix.
    ///
    /// # Errors
    ///
    /// Returns the first write error; cells written before it remain.
    pub fn write_column_matrix<I, II>(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: I,
    ) -> Result<&mut Self>
    where
        I: IntoIterator<Item = II>,
        II: IntoIterator,
        II::Item: IntoExcelData,
    {
        for (offset, values) in data.into_iter().enumerate() {
            let offset = u16::try_from(offset).map_err(|_| Error::RowColumnLimit)?;
            let target = col.checked_add(offset).ok_or(Error::RowColumnLimit)?;
            self.write_column(row, target, values)?;
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

    /// Hides one row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row is out of range.
    pub fn set_row_hidden(&mut self, row: RowNum) -> Result<&mut Self> {
        Self::check_row(row)?;
        self.rows.entry(row).or_default().hidden = true;
        Ok(self)
    }

    /// Unhides one row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row is out of range.
    pub fn set_row_unhidden(&mut self, row: RowNum) -> Result<&mut Self> {
        Self::check_row(row)?;
        if let Some(options) = self.rows.get_mut(&row) {
            options.hidden = false;
        }
        Ok(self)
    }

    /// Hides one column.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the column is out of range.
    pub fn set_column_hidden(&mut self, col: ColNum) -> Result<&mut Self> {
        Self::check_col(col)?;
        self.columns.entry(col).or_default().hidden = true;
        Ok(self)
    }

    /// Hides an inclusive column range.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`, or
    /// [`Error::RowColumnLimit`] when the range is out of bounds.
    pub fn set_column_range_hidden(&mut self, first: ColNum, last: ColNum) -> Result<&mut Self> {
        if first > last {
            return Err(Error::RowColumnOrder);
        }
        Self::check_col(first)?;
        Self::check_col(last)?;
        for col in first..=last {
            self.columns.entry(col).or_default().hidden = true;
        }
        Ok(self)
    }

    /// Applies a format to one cell without changing its value.
    ///
    /// A missing cell becomes a formatted blank cell.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row or column is out of range.
    pub fn set_cell_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        format: &Format,
    ) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        let format = self.intern_format(format);
        self.cells
            .entry(row)
            .or_default()
            .entry(col)
            .and_modify(|cell| cell.format = Some(format))
            .or_insert(Cell {
                data: CellData::Blank,
                format: Some(format),
            });
        Ok(self)
    }

    /// Applies a format to an inclusive rectangular range without changing
    /// values.
    ///
    /// Missing cells become formatted blank cells.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed, or
    /// [`Error::RowColumnLimit`] when it is out of bounds.
    pub fn set_range_format(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        format: &Format,
    ) -> Result<&mut Self> {
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;
        let format = self.intern_format(format);
        for row in first_row..=last_row {
            let cells = self.cells.entry(row).or_default();
            for col in first_col..=last_col {
                cells
                    .entry(col)
                    .and_modify(|cell| cell.format = Some(format))
                    .or_insert(Cell {
                        data: CellData::Blank,
                        format: Some(format),
                    });
            }
        }
        Ok(self)
    }

    /// Removes a cell value and format.
    ///
    /// Out-of-range coordinates are ignored.
    pub fn clear_cell(&mut self, row: RowNum, col: ColNum) -> &mut Self {
        if row > MAX_ROW || col > MAX_COL {
            return self;
        }
        let remove_row = self.cells.get_mut(&row).is_some_and(|cells| {
            cells.remove(&col);
            cells.is_empty()
        });
        if remove_row {
            self.cells.remove(&row);
        }
        self
    }

    /// Removes a cell format while preserving its value.
    ///
    /// Out-of-range coordinates are ignored.
    pub fn clear_cell_format(&mut self, row: RowNum, col: ColNum) -> &mut Self {
        if row > MAX_ROW || col > MAX_COL {
            return self;
        }
        let remove_row = self.cells.get_mut(&row).is_some_and(|cells| {
            if let Some(cell) = cells.get_mut(&col) {
                cell.format = None;
                if cell.data == CellData::Blank {
                    cells.remove(&col);
                }
                cells.is_empty()
            } else {
                false
            }
        });
        if remove_row {
            self.cells.remove(&row);
        }
        self
    }

    /// Merges a rectangular range and writes a string into its top-left cell.
    ///
    /// The rest of the range becomes formatted blanks. Overlapping ranges and
    /// single-cell ranges are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] for a reversed range,
    /// [`Error::RowColumnLimit`] for out-of-bounds coordinates,
    /// [`Error::MergeRangeSingleCell`] for a one-cell range, or
    /// [`Error::MergeRangeOverlaps`] when the range intersects an existing
    /// merge.
    pub fn merge_range(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        string: &str,
        format: &Format,
    ) -> Result<&mut Self> {
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;
        if first_row == last_row && first_col == last_col {
            return Err(Error::MergeRangeSingleCell);
        }

        let range = MergeRange {
            first_row,
            first_col,
            last_row,
            last_col,
        };
        if self
            .merged_ranges
            .iter()
            .any(|existing| ranges_overlap(*existing, range))
        {
            return Err(Error::MergeRangeOverlaps);
        }

        let format = self.intern_format(format);
        self.store_with_index(
            first_row,
            first_col,
            CellData::String(string.to_owned()),
            Some(format),
        )?;
        for col in first_col + 1..=last_col {
            self.store_with_index(first_row, col, CellData::Blank, Some(format))?;
        }
        for row in first_row + 1..=last_row {
            for col in first_col..=last_col {
                self.store_with_index(row, col, CellData::Blank, Some(format))?;
            }
        }
        self.merged_ranges.push(range);
        Ok(self)
    }

    pub(crate) fn store(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: CellData,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        let format = format.map(|format| self.intern_format(format));
        self.store_with_index(row, col, data, format)?;
        Ok(self)
    }

    fn store_with_index(
        &mut self,
        row: RowNum,
        col: ColNum,
        data: CellData,
        format: Option<u32>,
    ) -> Result<()> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        if let CellData::String(text) = &data
            && text.encode_utf16().count() > MAX_STRING_UTF16
        {
            return Err(Error::StringTooLong);
        }
        self.cells
            .entry(row)
            .or_default()
            .insert(col, Cell { data, format });
        Ok(())
    }

    pub(crate) fn merged_ranges(&self) -> &[MergeRange] {
        &self.merged_ranges
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

const fn ranges_overlap(left: MergeRange, right: MergeRange) -> bool {
    left.first_row <= right.last_row
        && right.first_row <= left.last_row
        && left.first_col <= right.last_col
        && right.first_col <= left.last_col
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
    fn formatted_row_and_column_writes_apply_one_format() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let format = Format::new().set_bold();
        worksheet
            .write_row_with_format(0, 0, [1_i32, 2_i32], &format)
            .expect("row");
        worksheet
            .write_column_with_format(1, 2, [3_i32, 4_i32], &format)
            .expect("column");

        assert_eq!(worksheet.formats().len(), 1);
        for (row, col) in [(0, 0), (0, 1), (1, 2), (2, 2)] {
            let cell = worksheet
                .cells()
                .get(&row)
                .and_then(|row| row.get(&col))
                .expect("cell");
            assert_eq!(cell.format, Some(0));
        }
    }

    #[test]
    fn writes_row_and_column_matrices() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet
            .write_row_matrix(0, 0, [[1_i32, 2_i32], [3_i32, 4_i32]])
            .expect("row matrix");
        worksheet
            .write_column_matrix(3, 0, [[5_i32, 6_i32], [7_i32, 8_i32]])
            .expect("column matrix");

        for (row, col, expected) in [
            (0, 0, 1),
            (0, 1, 2),
            (1, 0, 3),
            (1, 1, 4),
            (3, 0, 5),
            (4, 0, 6),
            (3, 1, 7),
            (4, 1, 8),
        ] {
            let cell = worksheet
                .cells()
                .get(&row)
                .and_then(|row| row.get(&col))
                .expect("matrix cell");
            assert_eq!(cell.data, CellData::Integer(expected));
        }
    }

    #[test]
    fn cell_and_range_format_edits_preserve_or_clear_data() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let bold = Format::new().set_bold();
        let italic = Format::new().set_italic();
        worksheet.write(0, 0, 42_i32).expect("writes");

        worksheet.set_cell_format(0, 0, &bold).expect("format");
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| (&cell.data, cell.format)),
            Some((&CellData::Integer(42), Some(0)))
        );

        worksheet
            .set_range_format(1, 1, 1, 2, &italic)
            .expect("range format");
        assert_eq!(
            worksheet
                .cells()
                .get(&1)
                .and_then(|row| row.get(&2))
                .map(|cell| (&cell.data, cell.format)),
            Some((&CellData::Blank, Some(1)))
        );

        worksheet.clear_cell_format(0, 0);
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| (&cell.data, cell.format)),
            Some((&CellData::Integer(42), None))
        );
        worksheet.clear_cell(0, 0);
        assert!(worksheet.cells().get(&0).is_none());
        worksheet.clear_cell_format(1, 1);
        assert!(
            worksheet
                .cells()
                .get(&1)
                .and_then(|row| row.get(&1))
                .is_none()
        );
    }

    #[test]
    fn hidden_row_and_column_options_are_reversible() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_row_hidden(0).expect("hide row");
        worksheet.set_column_hidden(1).expect("hide col");
        worksheet.set_column_range_hidden(2, 3).expect("hide cols");

        assert!(worksheet.row_options().get(&0).is_some_and(|o| o.hidden));
        assert!(worksheet.column_options().get(&1).is_some_and(|o| o.hidden));
        assert!(worksheet.column_options().get(&3).is_some_and(|o| o.hidden));

        worksheet.set_row_unhidden(0).expect("unhide row");
        assert!(!worksheet.row_options().get(&0).is_some_and(|o| o.hidden));

        worksheet.set_row_unhidden(5).expect("unhide absent row");
        assert!(worksheet.row_options().get(&5).is_none());
    }

    #[test]
    fn merge_range_writes_string_and_formatted_blanks() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let format = Format::new().set_bold();
        worksheet
            .merge_range(0, 0, 1, 2, "Merged", &format)
            .expect("merge");

        assert_eq!(worksheet.formats().len(), 1);
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| (&cell.data, cell.format)),
            Some((&CellData::String("Merged".to_owned()), Some(0)))
        );
        for (row, col) in [(0, 1), (0, 2), (1, 0), (1, 1), (1, 2)] {
            assert_eq!(
                worksheet
                    .cells()
                    .get(&row)
                    .and_then(|row| row.get(&col))
                    .map(|cell| (&cell.data, cell.format)),
                Some((&CellData::Blank, Some(0)))
            );
        }
        assert_eq!(worksheet.merged_ranges().len(), 1);
    }

    #[test]
    fn merge_range_rejects_single_cell_and_overlap() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.merge_range(0, 0, 0, 0, "one", &Format::new()),
            Err(Error::MergeRangeSingleCell)
        ));
        worksheet
            .merge_range(0, 0, 1, 1, "first", &Format::new())
            .expect("first merge");
        assert!(matches!(
            worksheet.merge_range(1, 1, 2, 2, "overlap", &Format::new()),
            Err(Error::MergeRangeOverlaps)
        ));
    }

    #[test]
    fn rejects_reversed_range_format() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_range_format(2, 0, 1, 0, &Format::new()),
            Err(Error::RowColumnOrder)
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
