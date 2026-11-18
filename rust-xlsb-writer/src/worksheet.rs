//! Worksheet data model and public write/layout API.

use std::collections::{BTreeMap, HashMap};

use rust_xlsb_core::prelude::UncheckedRfX;

use crate::data::{CellData, IntoExcelData, IntoExcelDateTime};
use crate::{Color, Error, Format, Result};

pub const MAX_ROW: u32 = 1_048_575;
pub const MAX_COL: u16 = 16_383;
const MAX_STRING_UTF16: usize = 32_767;
const DEFAULT_ZOOM: u16 = 100;
const MIN_ZOOM: u16 = 10;
const MAX_ZOOM: u16 = 400;
const DEFAULT_PRINT_SCALE: u16 = 100;
const MAX_PAGE_DIMENSION: u16 = 32_767;
const MAX_MARGIN: f64 = 49.0;
const DEFAULT_MARGIN_LEFT: f64 = 0.7;
const DEFAULT_MARGIN_RIGHT: f64 = 0.7;
const DEFAULT_MARGIN_TOP: f64 = 0.75;
const DEFAULT_MARGIN_BOTTOM: f64 = 0.75;
const DEFAULT_MARGIN_HEADER: f64 = 0.3;
const DEFAULT_MARGIN_FOOTER: f64 = 0.3;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "worksheet view state maps directly to independent BIFF flags"
)]
pub struct SheetView {
    pub(crate) freeze: Option<(RowNum, ColNum)>,
    pub(crate) selection: Option<UncheckedRfX>,
    pub(crate) top_left: Option<(RowNum, ColNum)>,
    pub(crate) freeze_top_cell: Option<(RowNum, ColNum)>,
    pub(crate) zoom: u16,
    pub(crate) show_gridlines: bool,
    pub(crate) show_zeros: bool,
    pub(crate) right_to_left: bool,
    pub(crate) selected: bool,
}

impl Default for SheetView {
    fn default() -> Self {
        Self {
            freeze: None,
            selection: None,
            top_left: None,
            freeze_top_cell: None,
            zoom: DEFAULT_ZOOM,
            show_gridlines: true,
            show_zeros: true,
            right_to_left: false,
            selected: false,
        }
    }
}

impl SheetView {
    // freeze_top_cell alone is not a view change; set_freeze_panes gates it.
    pub(crate) const fn is_default(&self) -> bool {
        self.freeze.is_none()
            && self.selection.is_none()
            && self.top_left.is_none()
            && self.zoom == DEFAULT_ZOOM
            && self.show_gridlines
            && self.show_zeros
            && !self.right_to_left
            && !self.selected
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "print state maps directly to independent BIFF flags"
)]
pub struct PageSetup {
    pub(crate) landscape: bool,
    pub(crate) paper_size: u8,
    pub(crate) left_to_right: bool,
    pub(crate) margin_left: f64,
    pub(crate) margin_right: f64,
    pub(crate) margin_top: f64,
    pub(crate) margin_bottom: f64,
    pub(crate) margin_header: f64,
    pub(crate) margin_footer: f64,
    pub(crate) print_gridlines: bool,
    pub(crate) print_headings: bool,
    pub(crate) center_horizontally: bool,
    pub(crate) center_vertically: bool,
    pub(crate) black_and_white: bool,
    pub(crate) draft: bool,
    pub(crate) print_scale: u16,
    pub(crate) fit_to_pages: Option<(u16, u16)>,
    pub(crate) first_page: Option<u16>,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            landscape: false,
            paper_size: 0,
            left_to_right: false,
            margin_left: DEFAULT_MARGIN_LEFT,
            margin_right: DEFAULT_MARGIN_RIGHT,
            margin_top: DEFAULT_MARGIN_TOP,
            margin_bottom: DEFAULT_MARGIN_BOTTOM,
            margin_header: DEFAULT_MARGIN_HEADER,
            margin_footer: DEFAULT_MARGIN_FOOTER,
            print_gridlines: false,
            print_headings: false,
            center_horizontally: false,
            center_vertically: false,
            black_and_white: false,
            draft: false,
            print_scale: DEFAULT_PRINT_SCALE,
            fit_to_pages: None,
            first_page: None,
        }
    }
}

impl PageSetup {
    pub(crate) const fn has_print_options(&self) -> bool {
        self.print_gridlines
            || self.print_headings
            || self.center_horizontally
            || self.center_vertically
    }

    pub(crate) fn has_margins(&self) -> bool {
        self.margin_left != DEFAULT_MARGIN_LEFT
            || self.margin_right != DEFAULT_MARGIN_RIGHT
            || self.margin_top != DEFAULT_MARGIN_TOP
            || self.margin_bottom != DEFAULT_MARGIN_BOTTOM
            || self.margin_header != DEFAULT_MARGIN_HEADER
            || self.margin_footer != DEFAULT_MARGIN_FOOTER
    }

    pub(crate) const fn has_page_setup(&self) -> bool {
        self.landscape
            || self.paper_size != 0
            || self.left_to_right
            || self.black_and_white
            || self.draft
            || self.print_scale != DEFAULT_PRINT_SCALE
            || self.fit_to_pages.is_some()
            || self.first_page.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFooter {
    pub(crate) header: Option<String>,
    pub(crate) footer: Option<String>,
    pub(crate) scale_with_doc: bool,
    pub(crate) align_with_page: bool,
}

impl Default for HeaderFooter {
    fn default() -> Self {
        Self {
            header: None,
            footer: None,
            scale_with_doc: true,
            align_with_page: true,
        }
    }
}

impl HeaderFooter {
    pub(crate) const fn is_default(&self) -> bool {
        self.header.is_none()
            && self.footer.is_none()
            && self.scale_with_doc
            && self.align_with_page
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    VeryHidden,
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
    view: SheetView,
    visibility: Visibility,
    active: bool,
    first_tab: bool,
    tab_color: Option<Color>,
    default_row_height: Option<f64>,
    page_setup: PageSetup,
    header_footer: HeaderFooter,
    page_breaks: Vec<u32>,
    vertical_page_breaks: Vec<u32>,
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
            view: SheetView::default(),
            visibility: Visibility::Visible,
            active: false,
            first_tab: false,
            tab_color: None,
            default_row_height: None,
            page_setup: PageSetup::default(),
            header_footer: HeaderFooter::default(),
            page_breaks: Vec::new(),
            vertical_page_breaks: Vec::new(),
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

    /// Sets one column width in pixels.
    ///
    /// A width of `0` hides the column.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the column is out of range, or
    /// [`Error::ValueNotRepresentable`] when the width exceeds the Excel limit.
    pub fn set_column_width_pixels(&mut self, col: ColNum, width: u32) -> Result<&mut Self> {
        if width == 0 {
            return self.set_column_hidden(col);
        }
        self.set_column_width(col, pixels_to_width(width))
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

    /// Sets a column range width in pixels.
    ///
    /// A width of `0` hides the columns.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::ValueNotRepresentable`] when the width exceeds the Excel limit.
    pub fn set_column_range_width_pixels(
        &mut self,
        first: ColNum,
        last: ColNum,
        width: u32,
    ) -> Result<&mut Self> {
        if width == 0 {
            return self.set_column_range_hidden(first, last);
        }
        self.set_column_range_width(first, last, pixels_to_width(width))
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

    /// Sets one row height in pixels.
    ///
    /// A height of `0` hides the row.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row is out of range, or
    /// [`Error::ValueNotRepresentable`] when the height exceeds the Excel limit.
    pub fn set_row_height_pixels(&mut self, row: RowNum, height: u32) -> Result<&mut Self> {
        if height == 0 {
            return self.set_row_hidden(row);
        }
        self.set_row_height(row, f64::from(height) * 0.75)
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

    /// Freezes rows above and columns left of a cell.
    ///
    /// A zero row or column freezes only the other axis. Calling again
    /// replaces the previous freeze.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row or column is out of range.
    pub fn set_freeze_panes(&mut self, row: RowNum, col: ColNum) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        self.view.freeze = Some((row, col));
        Ok(self)
    }

    /// Sets the selected cell range.
    ///
    /// Calling again replaces the previous selection.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed, or
    /// [`Error::RowColumnLimit`] when it is out of bounds.
    pub fn set_selection(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<&mut Self> {
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;
        self.view.selection = Some(UncheckedRfX {
            rwFirst: first_row,
            rwLast: last_row,
            colFirst: u32::from(first_col),
            colLast: u32::from(last_col),
        });
        Ok(self)
    }

    /// Sets the first cell displayed in the top-left corner of the sheet view.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row or column is out of range.
    pub fn set_top_left_cell(&mut self, row: RowNum, col: ColNum) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        self.view.top_left = Some((row, col));
        Ok(self)
    }

    /// Sets the first visible cell of the frozen pane.
    ///
    /// Has no effect until [`Self::set_freeze_panes`] is called.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row or column is out of range.
    pub fn set_freeze_panes_top_cell(&mut self, row: RowNum, col: ColNum) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        self.view.freeze_top_cell = Some((row, col));
        Ok(self)
    }

    /// Sets the worksheet zoom percentage.
    ///
    /// Values outside `10..=400` are ignored.
    pub fn set_zoom(&mut self, zoom: u16) -> &mut Self {
        if (MIN_ZOOM..=MAX_ZOOM).contains(&zoom) {
            self.view.zoom = zoom;
        }
        self
    }

    /// Turns screen gridlines on or off.
    pub const fn set_screen_gridlines(&mut self, enable: bool) -> &mut Self {
        self.view.show_gridlines = enable;
        self
    }

    /// Hides zero values when `enable` is true.
    pub const fn set_hidden_zeroes(&mut self, enable: bool) -> &mut Self {
        self.view.show_zeros = !enable;
        self
    }

    /// Displays the worksheet right-to-left when `enable` is true.
    pub const fn set_right_to_left(&mut self, enable: bool) -> &mut Self {
        self.view.right_to_left = enable;
        self
    }

    /// Hides the worksheet when `enable` is true.
    ///
    /// Hiding clears the active and selected state.
    pub const fn set_hidden(&mut self, enable: bool) -> &mut Self {
        self.visibility = if enable {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        if enable {
            self.active = false;
            self.view.selected = false;
        }
        self
    }

    /// Hides the worksheet without a user-interface unhide option.
    ///
    /// Hiding clears the active and selected state.
    pub const fn set_very_hidden(&mut self, enable: bool) -> &mut Self {
        self.visibility = if enable {
            Visibility::VeryHidden
        } else {
            Visibility::Visible
        };
        if enable {
            self.active = false;
            self.view.selected = false;
        }
        self
    }

    /// Marks the worksheet as active when `enable` is true.
    ///
    /// An active worksheet is also selected and visible.
    pub const fn set_active(&mut self, enable: bool) -> &mut Self {
        self.active = enable;
        if enable {
            self.view.selected = true;
            self.visibility = Visibility::Visible;
        }
        self
    }

    /// Marks the worksheet tab as selected when `enable` is true.
    pub const fn set_selected(&mut self, enable: bool) -> &mut Self {
        self.view.selected = enable;
        if enable {
            self.visibility = Visibility::Visible;
        }
        self
    }

    /// Makes this worksheet the first visible tab when `enable` is true.
    pub const fn set_first_tab(&mut self, enable: bool) -> &mut Self {
        self.first_tab = enable;
        if enable {
            self.visibility = Visibility::Visible;
        }
        self
    }

    /// Sets the worksheet tab color.
    ///
    /// Default and automatic colors clear the tab color. Invalid RGB or theme
    /// values are ignored.
    pub fn set_tab_color(&mut self, color: impl Into<Color>) -> &mut Self {
        let color = color.into();
        if matches!(color, Color::Default | Color::Automatic) {
            self.tab_color = None;
        } else if color.is_valid() {
            self.tab_color = Some(color);
        }
        self
    }

    /// Sets the default row height in points.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValueNotRepresentable`] when the height is outside
    /// `0..=409.5`.
    pub fn set_default_row_height(&mut self, height: impl Into<f64>) -> Result<&mut Self> {
        let height = height.into();
        validate_row_height(height)?;
        self.default_row_height = Some(height);
        Ok(self)
    }

    /// Sets the default row height in pixels.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValueNotRepresentable`] when the height exceeds the
    /// Excel limit.
    pub fn set_default_row_height_pixels(&mut self, height: u32) -> Result<&mut Self> {
        self.set_default_row_height(f64::from(height) * 0.75)
    }

    /// Sets the printed page orientation to landscape.
    pub const fn set_landscape(&mut self) -> &mut Self {
        self.page_setup.landscape = true;
        self
    }

    /// Sets the printed page orientation to portrait.
    pub const fn set_portrait(&mut self) -> &mut Self {
        self.page_setup.landscape = false;
        self
    }

    /// Sets the printer paper size.
    ///
    /// `0` uses the printer default. See the MS-XLSB paper-size table for the
    /// standard values.
    pub const fn set_paper_size(&mut self, paper_size: u8) -> &mut Self {
        self.page_setup.paper_size = paper_size;
        self
    }

    /// Sets the page print order.
    ///
    /// `true` prints down then over, the default; `false` prints over then
    /// down.
    pub const fn set_page_order(&mut self, enable: bool) -> &mut Self {
        self.page_setup.left_to_right = !enable;
        self
    }

    /// Sets the printed page margins in inches.
    ///
    /// Values outside `0..49` are ignored individually; negative values keep
    /// the current margin.
    pub fn set_margins(
        &mut self,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        header: f64,
        footer: f64,
    ) -> &mut Self {
        let setup = &mut self.page_setup;
        if valid_margin(left) {
            setup.margin_left = left;
        }
        if valid_margin(right) {
            setup.margin_right = right;
        }
        if valid_margin(top) {
            setup.margin_top = top;
        }
        if valid_margin(bottom) {
            setup.margin_bottom = bottom;
        }
        if valid_margin(header) {
            setup.margin_header = header;
        }
        if valid_margin(footer) {
            setup.margin_footer = footer;
        }
        self
    }

    /// Prints gridlines when `enable` is true.
    pub const fn set_print_gridlines(&mut self, enable: bool) -> &mut Self {
        self.page_setup.print_gridlines = enable;
        self
    }

    /// Prints row and column headings when `enable` is true.
    pub const fn set_print_headings(&mut self, enable: bool) -> &mut Self {
        self.page_setup.print_headings = enable;
        self
    }

    /// Centers the printed output horizontally when `enable` is true.
    pub const fn set_print_center_horizontally(&mut self, enable: bool) -> &mut Self {
        self.page_setup.center_horizontally = enable;
        self
    }

    /// Centers the printed output vertically when `enable` is true.
    pub const fn set_print_center_vertically(&mut self, enable: bool) -> &mut Self {
        self.page_setup.center_vertically = enable;
        self
    }

    /// Sets the print scale percentage.
    ///
    /// Values outside `10..=400` are ignored. Setting a scale disables
    /// fit-to-pages.
    pub fn set_print_scale(&mut self, scale: u16) -> &mut Self {
        if (MIN_ZOOM..=MAX_ZOOM).contains(&scale) {
            self.page_setup.print_scale = scale;
            self.page_setup.fit_to_pages = None;
        }
        self
    }

    /// Fits the printed output to a page width and height.
    ///
    /// Values above `32767` are ignored. Setting fit-to-pages disables an
    /// explicit print scale.
    pub const fn set_print_fit_to_pages(&mut self, width: u16, height: u16) -> &mut Self {
        if width <= MAX_PAGE_DIMENSION && height <= MAX_PAGE_DIMENSION {
            self.page_setup.fit_to_pages = Some((width, height));
            self.page_setup.print_scale = DEFAULT_PRINT_SCALE;
        }
        self
    }

    /// Prints in black and white when `enable` is true.
    pub const fn set_print_black_and_white(&mut self, enable: bool) -> &mut Self {
        self.page_setup.black_and_white = enable;
        self
    }

    /// Omits graphics when printing when `enable` is true.
    pub const fn set_print_draft(&mut self, enable: bool) -> &mut Self {
        self.page_setup.draft = enable;
        self
    }

    /// Sets the first printed page number.
    ///
    /// Values above `32767` are ignored.
    pub const fn set_print_first_page_number(&mut self, page_number: u16) -> &mut Self {
        if page_number <= MAX_PAGE_DIMENSION {
            self.page_setup.first_page = Some(page_number);
        }
        self
    }

    /// Sets the printed page header.
    ///
    /// Accepts the `&P`, `&N`, `&D`, `&T`, `&F`, `&A`, `&Z`, `&L`, `&C`,
    /// `&R`, `&B`, `&I`, `&U`, and `&&` control codes, plus the reference
    /// aliases such as `&[Page]` and `&[Tab]`. Strings longer than 255 UTF-16
    /// code units are ignored.
    pub fn set_header(&mut self, header: impl Into<String>) -> &mut Self {
        if let Some(header) = normalize_header_footer(header.into()) {
            self.header_footer.header = Some(header);
        }
        self
    }

    /// Sets the printed page footer.
    ///
    /// Uses the same control codes and 255-code-unit limit as
    /// [`Self::set_header`].
    pub fn set_footer(&mut self, footer: impl Into<String>) -> &mut Self {
        if let Some(footer) = normalize_header_footer(footer.into()) {
            self.header_footer.footer = Some(footer);
        }
        self
    }

    /// Scales headers and footers with the document when `enable` is true.
    ///
    /// Enabled by default.
    pub const fn set_header_footer_scale_with_doc(&mut self, enable: bool) -> &mut Self {
        self.header_footer.scale_with_doc = enable;
        self
    }

    /// Aligns headers and footers with the page margins when `enable` is true.
    ///
    /// Enabled by default.
    pub const fn set_header_footer_align_with_page(&mut self, enable: bool) -> &mut Self {
        self.header_footer.align_with_page = enable;
        self
    }

    /// Sets horizontal page breaks between rows.
    ///
    /// Zero entries are ignored. The remaining entries are sorted and
    /// deduplicated. An empty slice leaves the current breaks unchanged; a
    /// non-empty slice with only zeros clears them.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a row is above
    /// [`MAX_ROW`], or [`Error::TooManyPageBreaks`] when more than 1023 breaks
    /// remain.
    pub fn set_page_breaks(&mut self, breaks: &[RowNum]) -> Result<&mut Self> {
        if let Some(breaks) = normalize_page_breaks(breaks, MAX_ROW)? {
            self.page_breaks = breaks;
        }
        Ok(self)
    }

    /// Sets vertical page breaks between columns.
    ///
    /// Zero entries are ignored. The remaining entries are sorted and
    /// deduplicated. An empty slice leaves the current breaks unchanged; a
    /// non-empty slice with only zeros clears them.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a column is above
    /// [`MAX_COL`], or [`Error::TooManyPageBreaks`] when more than 1023 breaks
    /// remain.
    pub fn set_vertical_page_breaks(&mut self, breaks: &[u32]) -> Result<&mut Self> {
        if let Some(breaks) = normalize_page_breaks(breaks, u32::from(MAX_COL))? {
            self.vertical_page_breaks = breaks;
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

    pub(crate) const fn view(&self) -> &SheetView {
        &self.view
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
    pub(crate) const fn visibility(&self) -> Visibility {
        self.visibility
    }

    pub(crate) const fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) const fn is_first_tab(&self) -> bool {
        self.first_tab
    }

    pub(crate) const fn tab_color(&self) -> Option<Color> {
        self.tab_color
    }

    pub(crate) const fn default_row_height(&self) -> Option<f64> {
        self.default_row_height
    }

    pub(crate) const fn page_setup(&self) -> &PageSetup {
        &self.page_setup
    }

    pub(crate) const fn header_footer(&self) -> &HeaderFooter {
        &self.header_footer
    }

    pub(crate) fn page_breaks(&self) -> &[u32] {
        &self.page_breaks
    }

    pub(crate) fn vertical_page_breaks(&self) -> &[u32] {
        &self.vertical_page_breaks
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

fn normalize_header_footer(text: String) -> Option<String> {
    const MAX_CODE_UNITS: usize = 255;

    let text = if text.contains("&[") {
        expand_header_footer(&text)
    } else {
        text
    };
    (text.encode_utf16().count() <= MAX_CODE_UNITS).then_some(text)
}

fn expand_header_footer(text: &str) -> String {
    let mut expanded = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("&[") {
        expanded.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find(']') else {
            expanded.push_str(&rest[start..]);
            return expanded;
        };
        let replacement = match &after[..end] {
            "Tab" => Some("&A"),
            "Date" => Some("&D"),
            "File" => Some("&F"),
            "Page" => Some("&P"),
            "Path" => Some("&Z"),
            "Time" => Some("&T"),
            "Pages" => Some("&N"),
            "Picture" => Some("&G"),
            _ => None,
        };
        if let Some(replacement) = replacement {
            expanded.push_str(replacement);
            rest = &after[end + 1..];
        } else {
            expanded.push('&');
            rest = &rest[start + 1..];
        }
    }

    expanded.push_str(rest);
    expanded
}

fn normalize_page_breaks(breaks: &[u32], maximum: u32) -> Result<Option<Vec<u32>>> {
    const MAX_PAGE_BREAKS: usize = 1023;

    if breaks.is_empty() {
        return Ok(None);
    }

    let mut breaks: Vec<u32> = breaks.iter().copied().filter(|value| *value != 0).collect();
    if breaks.iter().any(|value| *value > maximum) {
        return Err(Error::RowColumnLimit);
    }
    breaks.sort_unstable();
    breaks.dedup();
    if breaks.len() > MAX_PAGE_BREAKS {
        return Err(Error::TooManyPageBreaks);
    }
    Ok(Some(breaks))
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

fn valid_margin(margin: f64) -> bool {
    (0.0..MAX_MARGIN).contains(&margin)
}

fn pixels_to_width(pixels: u32) -> f64 {
    let pixels = f64::from(pixels);
    if pixels <= 12.0 {
        pixels / 12.0
    } else {
        (pixels - 5.0) / 7.0
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
    fn stores_and_replaces_sheet_view_state() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_freeze_panes(2, 3).expect("freeze");
        worksheet.set_selection(1, 1, 2, 4).expect("selection");
        worksheet.set_top_left_cell(4, 5).expect("top left");

        assert_eq!(worksheet.view().freeze, Some((2, 3)));
        assert_eq!(worksheet.view().top_left, Some((4, 5)));
        assert_eq!(
            worksheet.view().selection,
            Some(UncheckedRfX {
                rwFirst: 1,
                rwLast: 2,
                colFirst: 1,
                colLast: 4,
            })
        );

        worksheet.set_freeze_panes(1, 0).expect("replace");
        worksheet.set_selection(0, 0, 0, 1).expect("replace");
        worksheet.set_top_left_cell(0, 1).expect("replace");
        assert_eq!(worksheet.view().freeze, Some((1, 0)));
        assert_eq!(worksheet.view().top_left, Some((0, 1)));
    }

    #[test]
    fn stores_view_display_options_and_freeze_top_cell() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet
            .set_zoom(150)
            .set_screen_gridlines(false)
            .set_hidden_zeroes(true)
            .set_right_to_left(true);
        worksheet.set_freeze_panes(1, 2).expect("freeze");
        worksheet.set_freeze_panes_top_cell(4, 3).expect("top cell");

        assert_eq!(worksheet.view().zoom, 150);
        assert!(!worksheet.view().show_gridlines);
        assert!(!worksheet.view().show_zeros);
        assert!(worksheet.view().right_to_left);
        assert_eq!(worksheet.view().freeze_top_cell, Some((4, 3)));

        worksheet.set_zoom(9);
        assert_eq!(worksheet.view().zoom, 150);
    }

    #[test]
    fn pixel_dimensions_convert_and_zero_hides() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_row_height_pixels(3, 40).expect("row pixels");
        worksheet
            .set_column_width_pixels(4, 64)
            .expect("column pixels");
        worksheet
            .set_column_range_width_pixels(5, 6, 64)
            .expect("range pixels");
        worksheet
            .set_column_width_pixels(10, 12)
            .expect("12 pixels");
        worksheet
            .set_column_width_pixels(11, 13)
            .expect("13 pixels");
        worksheet
            .set_column_width_pixels(12, 0)
            .expect("hidden column");
        worksheet.set_row_height_pixels(7, 0).expect("hidden row");
        worksheet
            .set_column_range_width_pixels(8, 9, 0)
            .expect("hidden columns");

        assert_eq!(
            worksheet
                .row_options()
                .get(&3)
                .and_then(|options| options.height),
            Some(30.0)
        );
        let expected = (64.0 - 5.0) / 7.0;
        for col in 4..=6 {
            assert_eq!(
                worksheet
                    .column_options()
                    .get(&col)
                    .and_then(|options| options.width),
                Some(expected)
            );
        }
        assert_eq!(
            worksheet
                .column_options()
                .get(&10)
                .and_then(|options| options.width),
            Some(1.0)
        );
        assert_eq!(
            worksheet
                .column_options()
                .get(&11)
                .and_then(|options| options.width),
            Some(8.0 / 7.0)
        );
        assert!(
            worksheet
                .column_options()
                .get(&12)
                .is_some_and(|column| column.hidden)
        );
        assert!(
            worksheet
                .row_options()
                .get(&7)
                .is_some_and(|row| row.hidden)
        );
        for col in 8..=9 {
            assert!(
                worksheet
                    .column_options()
                    .get(&col)
                    .is_some_and(|column| column.hidden)
            );
        }
    }

    #[test]
    fn rejects_invalid_freeze_top_cell_without_mutation() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_freeze_panes_top_cell(MAX_ROW + 1, 0),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.set_freeze_panes_top_cell(0, MAX_COL + 1),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(worksheet.view().freeze_top_cell, None);
    }

    #[test]
    fn rejects_invalid_view_state_without_mutation() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_freeze_panes(MAX_ROW + 1, 0),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.set_top_left_cell(0, MAX_COL + 1),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.set_selection(2, 0, 1, 0),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.set_selection(0, 0, MAX_ROW + 1, 0),
            Err(Error::RowColumnLimit)
        ));
        assert!(worksheet.view().is_default());
    }

    #[test]
    fn header_footer_aliases_and_flags_are_stored() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet
            .set_header("&L&[Tab]&C&[Page] of &[Pages]")
            .set_footer("&R&[File] &[Date] &[Time] &[Path] &[Picture]")
            .set_header_footer_scale_with_doc(false)
            .set_header_footer_align_with_page(false);

        assert_eq!(
            worksheet.header_footer().header.as_deref(),
            Some("&L&A&C&P of &N")
        );
        assert_eq!(
            worksheet.header_footer().footer.as_deref(),
            Some("&R&F &D &T &Z &G")
        );
        assert!(!worksheet.header_footer().scale_with_doc);
        assert!(!worksheet.header_footer().align_with_page);
        assert!(!worksheet.header_footer().is_default());
    }

    #[test]
    fn header_footer_length_is_checked_after_expansion() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let maximum = "x".repeat(255);
        worksheet.set_header(maximum.clone());
        assert_eq!(
            worksheet.header_footer().header.as_deref(),
            Some(maximum.as_str())
        );

        worksheet.set_header("y".repeat(256));
        assert_eq!(
            worksheet.header_footer().header.as_deref(),
            Some(maximum.as_str())
        );

        worksheet.set_footer("&[Page]".repeat(128));
        assert!(worksheet.header_footer().footer.is_none());
    }

    #[test]
    fn page_breaks_sort_deduplicate_and_clear() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet
            .set_page_breaks(&[40, 20, 20, 0, 60])
            .expect("horizontal");
        worksheet
            .set_vertical_page_breaks(&[5, 3, 5])
            .expect("vertical");

        assert_eq!(worksheet.page_breaks(), [20, 40, 60]);
        assert_eq!(worksheet.vertical_page_breaks(), [3, 5]);

        worksheet.set_page_breaks(&[]).expect("empty no-op");
        assert_eq!(worksheet.page_breaks(), [20, 40, 60]);

        worksheet.set_page_breaks(&[0]).expect("clear");
        assert_eq!(worksheet.page_breaks(), []);
    }

    #[test]
    fn page_break_limits_do_not_mutate_state() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_page_breaks(&[20]).expect("horizontal");

        assert!(matches!(
            worksheet.set_page_breaks(&[MAX_ROW + 1]),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.set_vertical_page_breaks(&[u32::from(MAX_COL) + 1]),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(worksheet.page_breaks(), [20]);

        let too_many: Vec<u32> = (1..=1024).collect();
        assert!(matches!(
            worksheet.set_page_breaks(&too_many),
            Err(Error::TooManyPageBreaks)
        ));
        assert_eq!(worksheet.page_breaks(), [20]);

        worksheet.set_page_breaks(&too_many[..1023]).expect("limit");
        assert_eq!(worksheet.page_breaks().len(), 1023);
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

    #[test]
    fn tab_state_follows_visibility_rules() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_active(true);
        assert!(worksheet.is_active());
        assert!(worksheet.view().selected);
        assert_eq!(worksheet.visibility(), Visibility::Visible);

        worksheet.set_hidden(true);
        assert_eq!(worksheet.visibility(), Visibility::Hidden);
        assert!(!worksheet.is_active());
        assert!(!worksheet.view().selected);

        worksheet.set_active(true);
        assert_eq!(worksheet.visibility(), Visibility::Visible);

        worksheet.set_very_hidden(true);
        assert_eq!(worksheet.visibility(), Visibility::VeryHidden);
        assert!(!worksheet.is_active());
        assert!(!worksheet.view().selected);

        worksheet.set_selected(true);
        assert_eq!(worksheet.visibility(), Visibility::Visible);
        worksheet.set_first_tab(true);
        assert!(worksheet.is_first_tab());
    }

    #[test]
    fn tab_color_ignores_invalid_values_and_clears_defaults() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_tab_color(Color::Red);
        assert_eq!(worksheet.tab_color(), Some(Color::Red));

        worksheet.set_tab_color(Color::RGB(0x0100_0000));
        assert_eq!(worksheet.tab_color(), Some(Color::Red));

        worksheet.set_tab_color(Color::Theme(10, 0));
        assert_eq!(worksheet.tab_color(), Some(Color::Red));

        worksheet.set_tab_color(Color::Default);
        assert_eq!(worksheet.tab_color(), None);
    }

    #[test]
    fn default_row_height_validates_and_converts_pixels() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        worksheet.set_default_row_height(20.0).expect("height");
        assert_eq!(worksheet.default_row_height(), Some(20.0));

        worksheet.set_default_row_height_pixels(32).expect("pixels");
        assert_eq!(worksheet.default_row_height(), Some(24.0));

        assert!(matches!(
            worksheet.set_default_row_height(409.6),
            Err(Error::ValueNotRepresentable(_))
        ));
    }
}
