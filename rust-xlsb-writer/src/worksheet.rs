//! Worksheet data model and public write/layout API.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rust_xlsb_core::prelude::UncheckedRfX;

use crate::conditional_format::{ConditionalFormat, ConditionalFormatRange};
use crate::data::{
    ArrayFormulaCell, CellData, FormulaCell, FormulaResult, IntoExcelData, IntoExcelDateTime,
    RichRun, RichString,
};
use crate::data_validation::{DataValidation, DataValidationRange};
use crate::filter::FilterCondition;
use crate::format::BorderSide;
use crate::formula::{Formula, cached_result, compile, result_is_too_long, subtotal_formula};
use crate::protection::{ProtectionOptions, SheetProtection, UnprotectedRange, hash_password};
use crate::range_list::RangeList;
use crate::sparkline::{Sparkline, SparklineCell, SparklineGroup, SparklineRange};
use crate::table::{Table, TableFunction, TableRange};
use crate::{Color, Error, Format, Image, Result, Url};

pub const MAX_ROW: u32 = 1_048_575;
pub const MAX_COL: u16 = 16_383;
const MAX_STRING_UTF16: usize = 32_767;
const DEFAULT_ZOOM: u16 = 100;
const MIN_ZOOM: u16 = 10;
const MAX_ZOOM: u16 = 400;
const DEFAULT_PRINT_SCALE: u16 = 100;
const MAX_PAGE_DIMENSION: u16 = 32_767;
const MAX_MARGIN: f64 = 49.0;
const MAX_AUTOFIT_WIDTH_PIXELS: u32 = 1790;
const MAX_OUTLINE_LEVEL: u8 = 7;
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
    pub(crate) level: u8,
    pub(crate) collapsed: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColumnOptions {
    pub(crate) width: Option<f64>,
    pub(crate) format: Option<u32>,
    pub(crate) hidden: bool,
    pub(crate) level: u8,
    pub(crate) collapsed: bool,
    pub(crate) best_fit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IgnoreError {
    /// Numbers stored as text.
    NumberStoredAsText,
    /// Formula evaluation errors such as divide by zero.
    FormulaError,
    /// Formulas that differ from surrounding formulas.
    FormulaDiffers,
    /// Formulas that refer to empty cells.
    FormulaRefersToEmptyCells,
    /// Formulas that omit cells in a range.
    FormulaOmitsCells,
    /// Cells in a table that violate data validation.
    DataValidationError,
    /// Two-digit text representations of a year.
    TwoDigitTextYear,
    /// Unlocked cells containing formulas.
    UnlockedCellsWithFormula,
    /// Formulas that differ from the calculated-column formula.
    InconsistentColumnFormula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeRange {
    pub(crate) first_row: u32,
    pub(crate) first_col: u16,
    pub(crate) last_row: u32,
    pub(crate) last_col: u16,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the sheet serializer"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ViewMode {
    #[default]
    Normal,
    PageLayout,
    PageBreakPreview,
}

impl ViewMode {
    pub(crate) const fn xl_view(self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::PageLayout => 2,
            Self::PageBreakPreview => 1,
        }
    }

    pub(crate) const fn shows_ruler(self) -> bool {
        matches!(self, Self::PageLayout)
    }
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
    pub(crate) view_mode: ViewMode,
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
            view_mode: ViewMode::Normal,
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
            && matches!(self.view_mode, ViewMode::Normal)
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
#[allow(
    clippy::struct_excessive_bools,
    reason = "worksheet state mirrors independent BIFF flags"
)]
pub struct Worksheet {
    name: String,
    cells: BTreeMap<u32, BTreeMap<u16, Cell>>,
    rows: BTreeMap<u32, RowOptions>,
    columns: BTreeMap<u16, ColumnOptions>,
    formats: Vec<Format>,
    format_ids: HashMap<Format, u32>,
    merged_ranges: Vec<MergeRange>,
    autofilter: Option<UncheckedRfX>,
    filter_conditions: BTreeMap<ColNum, FilterCondition>,
    filter_automatic_off: bool,
    filter_hidden_rows: BTreeSet<RowNum>,
    has_rich_strings: bool,
    default_formula_result: Box<str>,
    conditional_formats: Vec<ConditionalFormatRange>,
    view: SheetView,
    visibility: Visibility,
    active: bool,
    first_tab: bool,
    tab_color: Option<Color>,
    default_format: Option<Format>,
    default_row_height: Option<f64>,
    default_col_width: Option<f64>,
    page_setup: PageSetup,
    header_footer: HeaderFooter,
    page_breaks: Vec<u32>,
    vertical_page_breaks: Vec<u32>,
    print_area: Option<(RowNum, ColNum, RowNum, ColNum)>,
    repeat_rows: Option<(RowNum, RowNum)>,
    repeat_columns: Option<(ColNum, ColNum)>,
    outline_row_level: u8,
    outline_col_level: u8,
    outline_symbols_above: bool,
    outline_symbols_to_left: bool,
    hide_unused_rows: bool,
    nan_value: Option<String>,
    infinity_value: Option<String>,
    neg_infinity_value: Option<String>,
    hyperlinks: BTreeMap<(RowNum, ColNum), Url>,
    protection: Option<SheetProtection>,
    unprotected_ranges: Vec<UnprotectedRange>,
    ignored_errors: BTreeMap<IgnoreError, Vec<UncheckedRfX>>,
    ignored_error_ranges: BTreeSet<(RowNum, ColNum, RowNum, ColNum)>,
    data_validations: Vec<DataValidationRange>,
    tables: Vec<TableRange>,
    sparkline_groups: Vec<SparklineGroup>,
    background_image: Option<Image>,
    max_autofit_row: RowNum,
    max_autofit_width: u32,
}

const fn rfx_overlaps(first: UncheckedRfX, last: UncheckedRfX) -> bool {
    first.rwFirst <= last.rwLast
        && last.rwFirst <= first.rwLast
        && first.colFirst <= last.colLast
        && last.colFirst <= first.colLast
}

impl Default for Worksheet {
    fn default() -> Self {
        Self::new()
    }
}

impl Worksheet {
    /// Creates an empty worksheet with no name.
    ///
    /// The worksheet is not attached to a workbook. Write to it freely, then
    /// add it with [`Workbook::push_worksheet`](crate::Workbook::push_worksheet),
    /// which assigns a default `SheetN` name if none was set.
    #[must_use]
    pub fn new() -> Self {
        Self::new_named(String::new())
    }

    pub(crate) fn new_named(name: String) -> Self {
        Self {
            name,
            cells: BTreeMap::new(),
            rows: BTreeMap::new(),
            columns: BTreeMap::new(),
            formats: Vec::new(),
            format_ids: HashMap::new(),
            merged_ranges: Vec::new(),
            autofilter: None,
            filter_conditions: BTreeMap::new(),
            filter_automatic_off: false,
            filter_hidden_rows: BTreeSet::new(),
            has_rich_strings: false,
            default_formula_result: "0".into(),
            conditional_formats: Vec::new(),
            view: SheetView::default(),
            visibility: Visibility::Visible,
            active: false,
            first_tab: false,
            tab_color: None,
            default_format: None,
            default_row_height: None,
            default_col_width: None,
            page_setup: PageSetup::default(),
            header_footer: HeaderFooter::default(),
            page_breaks: Vec::new(),
            vertical_page_breaks: Vec::new(),
            print_area: None,
            repeat_rows: None,
            repeat_columns: None,
            outline_row_level: 0,
            outline_col_level: 0,
            outline_symbols_above: false,
            outline_symbols_to_left: false,
            hide_unused_rows: false,
            nan_value: None,
            infinity_value: None,
            neg_infinity_value: None,
            hyperlinks: BTreeMap::new(),
            protection: None,
            unprotected_ranges: Vec::new(),
            ignored_errors: BTreeMap::new(),
            ignored_error_ranges: BTreeSet::new(),
            data_validations: Vec::new(),
            tables: Vec::new(),
            sparkline_groups: Vec::new(),
            background_image: None,
            max_autofit_row: MAX_ROW,
            max_autofit_width: MAX_AUTOFIT_WIDTH_PIXELS,
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

    /// Writes a string with per-run font formatting.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the segment list is empty or a segment
    /// is empty, [`Error::RowColumnLimit`] when the cell is out of range,
    /// [`Error::StringTooLong`] when the flattened text exceeds 32,767 UTF-16
    /// code units, or [`Error::ValueNotRepresentable`] for an invalid run font.
    pub fn write_rich_string(
        &mut self,
        row: RowNum,
        col: ColNum,
        rich_string: &[(&Format, &str)],
    ) -> Result<&mut Self> {
        self.write_rich_string_inner(row, col, rich_string, None)
    }

    /// Writes a string with per-run and cell-wide formatting.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::write_rich_string`], plus errors from
    /// the cell-wide format.
    pub fn write_rich_string_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        rich_string: &[(&Format, &str)],
        format: &Format,
    ) -> Result<&mut Self> {
        self.write_rich_string_inner(row, col, rich_string, Some(format))
    }

    fn write_rich_string_inner(
        &mut self,
        row: RowNum,
        col: ColNum,
        rich_string: &[(&Format, &str)],
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        if rich_string.is_empty() {
            return Err(Error::Parameter(
                "rich string must contain at least one segment".to_owned(),
            ));
        }

        let mut text = String::new();
        let mut runs = Vec::with_capacity(rich_string.len());
        let mut total = 0_usize;
        for (run_format, segment) in rich_string {
            if segment.is_empty() {
                return Err(Error::Parameter(
                    "rich string segments must not be empty".to_owned(),
                ));
            }
            let segment_len = segment.encode_utf16().count();
            let offset = total;
            total = total.checked_add(segment_len).ok_or(Error::StringTooLong)?;
            if total > MAX_STRING_UTF16 {
                return Err(Error::StringTooLong);
            }
            runs.push(RichRun {
                ich: u16::try_from(offset)
                    .expect("rich string length is bounded before conversion"),
                font: run_format.to_rich_font()?,
            });
            text.push_str(segment);
        }

        let format = format.map(|format| self.intern_format(format));
        self.store_with_index(
            row,
            col,
            CellData::RichString(RichString { text, runs }),
            format,
        )?;
        self.has_rich_strings = true;
        Ok(self)
    }

    /// Writes a hyperlink.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the URL type is
    /// unknown, a URL or tooltip exceeds Excel's length limit, or the text is
    /// too long.
    pub fn write_url(
        &mut self,
        row: RowNum,
        col: ColNum,
        link: impl Into<Url>,
    ) -> Result<&mut Self> {
        self.store_url(row, col, link.into(), None)
    }

    /// Writes a hyperlink with alternative visible text.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the URL type is
    /// unknown, a URL or tooltip exceeds Excel's length limit, or the text is
    /// too long.
    pub fn write_url_with_text(
        &mut self,
        row: RowNum,
        col: ColNum,
        link: impl Into<Url>,
        text: impl Into<String>,
    ) -> Result<&mut Self> {
        self.store_url(row, col, link.into().set_text(text), None)
    }

    /// Writes a hyperlink with a caller-supplied cell format.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the URL type is
    /// unknown, a URL or tooltip exceeds Excel's length limit, or the text is
    /// too long.
    pub fn write_url_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        link: impl Into<Url>,
        format: &Format,
    ) -> Result<&mut Self> {
        self.store_url(row, col, link.into(), Some(format))
    }

    /// Writes a hyperlink with text, tooltip, and an optional format.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, the URL type is
    /// unknown, a URL or tooltip exceeds Excel's length limit, or the text is
    /// too long.
    pub fn write_url_with_options(
        &mut self,
        row: RowNum,
        col: ColNum,
        link: impl Into<Url>,
        text: impl Into<String>,
        tip: impl Into<String>,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        let link = link.into().set_text(text).set_tip(tip);
        self.store_url(row, col, link, format)
    }

    fn store_url(
        &mut self,
        row: RowNum,
        col: ColNum,
        mut link: Url,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        link.initialize()?;
        let format = format.map_or_else(
            || {
                self.default_format
                    .clone()
                    .unwrap_or_default()
                    .set_hyperlink()
            },
            Format::clone,
        );
        self.write_string_with_format(row, col, link.text.clone(), &format)?;
        self.hyperlinks.insert((row, col), link);
        Ok(self)
    }

    /// Writes a number.
    ///
    /// NaN and infinity become display strings because XLSB has no non-finite
    /// number cell. See [`Self::set_nan_value`] and
    /// [`Self::set_infinity_value`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range,
    /// or [`Error::StringTooLong`] when a configured non-finite display string
    /// exceeds 32,767 UTF-16 code units.
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
    /// NaN and infinity become display strings because XLSB has no non-finite
    /// number cell. See [`Self::set_nan_value`] and
    /// [`Self::set_infinity_value`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the row/column is out of range,
    /// or [`Error::StringTooLong`] when a configured non-finite display string
    /// exceeds 32,767 UTF-16 code units.
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

    /// Writes a formula.
    ///
    /// The formula uses US-English Excel syntax. A leading `=` is optional.
    /// The writer stores a cached result of `0` unless
    /// [`Formula::set_result`](crate::Formula::set_result) or
    /// [`Self::set_formula_result`] supplies one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the cell is out of range or
    /// [`Error::FormulaError`] when the formula is malformed or uses syntax
    /// this writer does not encode.
    pub fn write_formula(
        &mut self,
        row: RowNum,
        col: ColNum,
        formula: impl Into<Formula>,
    ) -> Result<&mut Self> {
        self.store_formula(row, col, formula.into(), None)
    }

    /// Writes a formula with formatting.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::write_formula`].
    pub fn write_formula_with_format(
        &mut self,
        row: RowNum,
        col: ColNum,
        formula: impl Into<Formula>,
        format: &Format,
    ) -> Result<&mut Self> {
        self.store_formula(row, col, formula.into(), Some(format))
    }

    /// Writes a static array formula to a range.
    ///
    /// The range must contain the cells that receive the array result. The
    /// top-left cell stores the cached result; the remaining cells are filled
    /// with numeric zero. A leading `=` and surrounding `{}` are optional.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a range endpoint is out of
    /// range, [`Error::RowColumnOrder`] when the first endpoint follows the
    /// last, or [`Error::FormulaError`] when the formula is malformed.
    pub fn write_array_formula(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        formula: impl Into<Formula>,
    ) -> Result<&mut Self> {
        self.store_array_formula(
            first_row,
            first_col,
            last_row,
            last_col,
            formula.into(),
            None,
        )
    }

    /// Writes a formatted static array formula to a range.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::write_array_formula`].
    pub fn write_array_formula_with_format(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        formula: impl Into<Formula>,
        format: &Format,
    ) -> Result<&mut Self> {
        self.store_array_formula(
            first_row,
            first_col,
            last_row,
            last_col,
            formula.into(),
            Some(format),
        )
    }

    /// Replaces the cached result of an existing formula cell.
    ///
    /// Scalar and array formulas are updated; non-formula cells and empty
    /// cells are ignored.
    pub fn set_formula_result(
        &mut self,
        row: RowNum,
        col: ColNum,
        result: impl Into<String>,
    ) -> &mut Self {
        if let Some(cell) = self
            .cells
            .get_mut(&row)
            .and_then(|cells| cells.get_mut(&col))
            && let CellData::Formula(FormulaCell { result: cached, .. })
            | CellData::ArrayFormula(ArrayFormulaCell { result: cached, .. }) = &mut cell.data
        {
            *cached = cached_result(&result.into());
        }
        self
    }

    /// Sets the cached result used by formulas written afterward.
    ///
    /// Defaults to `0`.
    pub fn set_formula_result_default(&mut self, result: impl Into<String>) -> &mut Self {
        self.default_formula_result = result.into().into_boxed_str();
        self
    }

    pub(crate) fn store_formula(
        &mut self,
        row: RowNum,
        col: ColNum,
        formula: Formula,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        Self::check_row(row)?;
        Self::check_col(col)?;
        let (text, result) = formula.into_parts();
        let result = result.as_deref().map_or_else(
            || cached_result(&self.default_formula_result),
            cached_result,
        );
        if result_is_too_long(&result) {
            return Err(Error::StringTooLong);
        }
        let formula = compile(&text)?;
        self.store(
            row,
            col,
            CellData::Formula(FormulaCell { formula, result }),
            format,
        )?;
        Ok(self)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the public range API mirrors the reference signature"
    )]
    pub(crate) fn store_array_formula(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        formula: Formula,
        format: Option<&Format>,
    ) -> Result<&mut Self> {
        Self::check_row(first_row)?;
        Self::check_col(first_col)?;
        Self::check_row(last_row)?;
        Self::check_col(last_col)?;
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }

        let (text, result) = formula.into_parts();
        let result = result.as_deref().map_or_else(
            || cached_result(&self.default_formula_result),
            cached_result,
        );
        if result_is_too_long(&result) {
            return Err(Error::StringTooLong);
        }

        let formula = compile(&text)?;
        let format = format.map(|format| self.intern_format(format));
        self.store_with_index(
            first_row,
            first_col,
            CellData::ArrayFormula(ArrayFormulaCell {
                formula,
                range: UncheckedRfX {
                    rwFirst: first_row,
                    rwLast: last_row,
                    colFirst: u32::from(first_col),
                    colLast: u32::from(last_col),
                },
                result,
            }),
            format,
        )?;

        for row in first_row..=last_row {
            for col in first_col..=last_col {
                if row == first_row && col == first_col {
                    continue;
                }
                self.store_with_index(row, col, CellData::Number(0.0), format)?;
            }
        }
        Ok(self)
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
        self.set_column_width_internal(col, width.into(), false)
    }

    /// Sets one column width in pixels for an auto-fitted column.
    ///
    /// This marks the column as best-fit, so Excel can widen it when a numeric
    /// value gains digits. A width of `0` hides the column.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the column is out of range, or
    /// [`Error::ValueNotRepresentable`] when the width exceeds the Excel limit.
    pub fn set_column_autofit_width(&mut self, col: ColNum, width: u32) -> Result<&mut Self> {
        if width == 0 {
            return self.set_column_hidden(col);
        }
        self.set_column_width_internal(col, pixels_to_width(width), true)
    }

    fn set_column_width_internal(
        &mut self,
        col: ColNum,
        width: f64,
        best_fit: bool,
    ) -> Result<&mut Self> {
        Self::check_col(col)?;
        validate_column_width(width)?;
        let options = self.columns.entry(col).or_default();
        options.width = Some(width);
        options.best_fit = best_fit;
        Ok(self)
    }

    fn store_autofit_width(&mut self, col: ColNum, pixels: u32) {
        let width = pixels_to_width(pixels);
        let options = self.columns.entry(col).or_default();
        if options
            .width
            .is_none_or(|existing| options.best_fit || width > existing)
        {
            options.width = Some(width);
            options.best_fit = true;
        }
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
            let options = self.columns.entry(col).or_default();
            options.width = Some(width);
            options.best_fit = false;
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

    /// Autofits column widths to the widest cell in each column.
    ///
    /// Widths use approximate Calibri 11 character metrics, matching the
    /// reference writer. Multi-line strings use their widest line. A manual
    /// column width is preserved unless the calculated width is wider. Columns
    /// containing only blank or error cells are left unchanged.
    ///
    /// Use [`set_autofit_max_row`](Self::set_autofit_max_row) and
    /// [`set_autofit_max_width`](Self::set_autofit_max_width) to bound the
    /// calculation.
    pub fn autofit(&mut self) -> &mut Self {
        let mut max_widths: BTreeMap<ColNum, u32> = BTreeMap::new();

        for (&row, cells) in &self.cells {
            if row >= self.max_autofit_row {
                break;
            }
            for (&col, cell) in cells {
                let mut width = autofit_cell_width(&cell.data);
                if width == 0 {
                    continue;
                }
                if self.autofilter.is_some_and(|range| {
                    row == range.rwFirst
                        && u32::from(col) >= range.colFirst
                        && u32::from(col) <= range.colLast
                }) {
                    width += 16;
                }
                width = (width + 7).min(self.max_autofit_width);
                if width == 0 {
                    continue;
                }
                max_widths
                    .entry(col)
                    .and_modify(|max| *max = (*max).max(width))
                    .or_insert(width);
            }
        }

        for (col, pixels) in max_widths {
            self.store_autofit_width(col, pixels);
        }
        self
    }

    /// Sets the row limit for [`autofit`](Self::autofit).
    ///
    /// Rows at or above `max_row` are ignored. The default limit is
    /// [`MAX_ROW`], so the last worksheet row is not measured.
    pub fn set_autofit_max_row(&mut self, max_row: RowNum) -> &mut Self {
        self.max_autofit_row = max_row.min(MAX_ROW);
        self
    }

    /// Sets the maximum column width in pixels used by [`autofit`](Self::autofit).
    ///
    /// The value is capped at Excel's 1790-pixel autofit limit.
    pub fn set_autofit_max_width(&mut self, max_width: u32) -> &mut Self {
        self.max_autofit_width = max_width.min(MAX_AUTOFIT_WIDTH_PIXELS);
        self
    }

    /// Autofits columns up to a maximum width.
    #[deprecated(
        since = "0.1.0",
        note = "use `set_autofit_max_width()` and `autofit()` instead"
    )]
    pub fn autofit_to_max_width(&mut self, max_width: u32) -> &mut Self {
        self.set_autofit_max_width(max_width);
        self.autofit()
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

    /// Groups an inclusive row range into an outline.
    ///
    /// Repeated calls nest groups. The maximum outline level is seven.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::MaxGroupLevelExceeded`] when the group would exceed level 7.
    pub fn group_rows(&mut self, first: RowNum, last: RowNum) -> Result<&mut Self> {
        self.set_grouped_rows(first, last, false)
    }

    /// Groups and collapses an inclusive row range.
    ///
    /// Collapsed rows are hidden, and the row after the range receives the
    /// outline summary marker.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::MaxGroupLevelExceeded`] when the group would exceed level 7.
    pub fn group_rows_collapsed(&mut self, first: RowNum, last: RowNum) -> Result<&mut Self> {
        self.set_grouped_rows(first, last, true)
    }

    /// Groups an inclusive column range into an outline.
    ///
    /// Repeated calls nest groups. The maximum outline level is seven.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::MaxGroupLevelExceeded`] when the group would exceed level 7.
    pub fn group_columns(&mut self, first: ColNum, last: ColNum) -> Result<&mut Self> {
        self.set_grouped_columns(first, last, false)
    }

    /// Groups and collapses an inclusive column range.
    ///
    /// Collapsed columns are hidden, and the column after the range receives
    /// the outline summary marker.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when `first > last`,
    /// [`Error::RowColumnLimit`] when the range is out of bounds, or
    /// [`Error::MaxGroupLevelExceeded`] when the group would exceed level 7.
    pub fn group_columns_collapsed(&mut self, first: ColNum, last: ColNum) -> Result<&mut Self> {
        self.set_grouped_columns(first, last, true)
    }

    /// Places row outline summary symbols above their groups.
    ///
    /// The default is below the group.
    pub const fn group_symbols_above(&mut self, enable: bool) -> &mut Self {
        self.outline_symbols_above = enable;
        self
    }

    /// Places column outline summary symbols to the left of their groups.
    ///
    /// The default is to the right of the group.
    pub const fn group_symbols_to_left(&mut self, enable: bool) -> &mut Self {
        self.outline_symbols_to_left = enable;
        self
    }

    fn set_grouped_rows(
        &mut self,
        first: RowNum,
        last: RowNum,
        collapsed: bool,
    ) -> Result<&mut Self> {
        if first > last {
            return Err(Error::RowColumnOrder);
        }
        Self::check_row(first)?;
        Self::check_row(last)?;

        let mut max_level = 0;
        for row in first..=last {
            let next = self.rows.get(&row).map_or(1, |options| options.level + 1);
            if next > MAX_OUTLINE_LEVEL {
                return Err(Error::MaxGroupLevelExceeded);
            }
            max_level = max_level.max(next);
        }

        for row in first..=last {
            let options = self.rows.entry(row).or_default();
            options.level += 1;
            if collapsed {
                options.hidden = true;
            }
        }
        self.outline_row_level = self.outline_row_level.max(max_level);

        if collapsed
            && let Some(marker) = last.checked_add(1)
            && marker <= MAX_ROW
        {
            self.rows.entry(marker).or_default().collapsed = true;
        }
        Ok(self)
    }

    fn set_grouped_columns(
        &mut self,
        first: ColNum,
        last: ColNum,
        collapsed: bool,
    ) -> Result<&mut Self> {
        if first > last {
            return Err(Error::RowColumnOrder);
        }
        Self::check_col(first)?;
        Self::check_col(last)?;

        let mut max_level = 0;
        for col in first..=last {
            let next = self
                .columns
                .get(&col)
                .map_or(1, |options| options.level + 1);
            if next > MAX_OUTLINE_LEVEL {
                return Err(Error::MaxGroupLevelExceeded);
            }
            max_level = max_level.max(next);
        }

        for col in first..=last {
            let options = self.columns.entry(col).or_default();
            options.level += 1;
            if collapsed {
                options.hidden = true;
            }
        }
        self.outline_col_level = self.outline_col_level.max(max_level);

        if collapsed
            && let Some(marker) = last.checked_add(1)
            && marker <= MAX_COL
        {
            self.columns.entry(marker).or_default().collapsed = true;
        }
        Ok(self)
    }

    pub(crate) const fn outline_levels(&self) -> (u8, u8) {
        (self.outline_row_level, self.outline_col_level)
    }

    pub(crate) const fn outline_symbols(&self) -> (bool, bool) {
        (self.outline_symbols_above, self.outline_symbols_to_left)
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

    /// Displays the worksheet in normal view.
    pub const fn set_view_normal(&mut self) -> &mut Self {
        self.view.view_mode = ViewMode::Normal;
        self
    }

    /// Displays the worksheet in page layout view.
    pub const fn set_view_page_layout(&mut self) -> &mut Self {
        self.view.view_mode = ViewMode::PageLayout;
        self
    }

    /// Displays the worksheet in page break preview.
    pub const fn set_view_page_break_preview(&mut self) -> &mut Self {
        self.view.view_mode = ViewMode::PageBreakPreview;
        self
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

    /// Sets the string written for `f64::NAN`.
    ///
    /// Defaults to `NAN`; XLSB has no NaN number value.
    pub fn set_nan_value(&mut self, value: impl Into<String>) -> &mut Self {
        self.nan_value = Some(value.into());
        self
    }

    /// Sets the string written for `f64::INFINITY`.
    ///
    /// Defaults to `INF`; XLSB has no infinity number value.
    pub fn set_infinity_value(&mut self, value: impl Into<String>) -> &mut Self {
        self.infinity_value = Some(value.into());
        self
    }

    /// Sets the string written for `f64::NEG_INFINITY`.
    ///
    /// Defaults to `-INF`.
    pub fn set_neg_infinity_value(&mut self, value: impl Into<String>) -> &mut Self {
        self.neg_infinity_value = Some(value.into());
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

    /// Protects the worksheet without a password.
    ///
    /// Existing protection options and password are preserved.
    pub fn protect(&mut self) -> &mut Self {
        self.protection
            .get_or_insert_with(|| SheetProtection::new(ProtectionOptions::new()));
        self
    }

    /// Protects the worksheet with a weak legacy password verifier.
    ///
    /// Excel sheet passwords are not encryption. Only ASCII passwords are
    /// supported.
    pub fn protect_with_password(&mut self, password: &str) -> &mut Self {
        let protection = self
            .protection
            .get_or_insert_with(|| SheetProtection::new(ProtectionOptions::new()));
        protection.password_hash = hash_password(password);
        self
    }

    /// Protects the worksheet with explicit options.
    ///
    /// An existing password is preserved.
    pub fn protect_with_options(&mut self, options: &ProtectionOptions) -> &mut Self {
        let protection = self
            .protection
            .get_or_insert_with(|| SheetProtection::new(ProtectionOptions::new()));
        protection.options = *options;
        self
    }

    pub(crate) const fn protection(&self) -> Option<SheetProtection> {
        self.protection
    }

    /// Unprotects a range in a protected worksheet.
    ///
    /// The range gets the default name `RangeN`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed, or
    /// [`Error::RowColumnLimit`] when it is out of bounds.
    pub fn unprotect_range(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<&mut Self> {
        self.unprotect_range_with_options(first_row, first_col, last_row, last_col, "", "")
    }

    /// Unprotects a range with an explicit name and optional password.
    ///
    /// An empty name becomes `RangeN`. An empty password means no password.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed,
    /// [`Error::RowColumnLimit`] when it is out of bounds, or
    /// [`Error::Parameter`] when the name exceeds 255 UTF-16 code units.
    pub fn unprotect_range_with_options(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        name: &str,
        password: &str,
    ) -> Result<&mut Self> {
        Self::check_range(first_row, first_col, last_row, last_col)?;
        let name = if name.is_empty() {
            let mut index = self.unprotected_ranges.len() + 1;
            loop {
                let candidate = format!("Range{index}");
                if !self
                    .unprotected_ranges
                    .iter()
                    .any(|range| range.name == candidate)
                {
                    break candidate;
                }
                index += 1;
            }
        } else {
            if self
                .unprotected_ranges
                .iter()
                .any(|range| range.name == name)
            {
                return Err(Error::Parameter(format!(
                    "protected range name '{name}' is already used"
                )));
            }
            name.to_owned()
        };
        if name.encode_utf16().count() > 255 {
            return Err(Error::Parameter(
                "protected range name exceeds 255 UTF-16 code units".to_owned(),
            ));
        }
        self.unprotected_ranges.push(UnprotectedRange {
            range: UncheckedRfX {
                rwFirst: first_row,
                rwLast: last_row,
                colFirst: u32::from(first_col),
                colLast: u32::from(last_col),
            },
            name,
            password_hash: hash_password(password),
        });
        Ok(self)
    }

    /// Ignores an Excel error in one cell.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the cell is out of bounds or
    /// [`Error::Parameter`] when the cell already has an ignored-error rule.
    pub fn ignore_error(
        &mut self,
        row: RowNum,
        col: ColNum,
        error_type: IgnoreError,
    ) -> Result<&mut Self> {
        self.ignore_error_range(row, col, row, col, error_type)
    }

    /// Ignores an Excel error in a range of cells.
    ///
    /// Ranges with the same error type share one record. Overlapping but
    /// non-identical ranges are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed,
    /// [`Error::RowColumnLimit`] when it is out of bounds, or
    /// [`Error::Parameter`] when the exact range was already used.
    pub fn ignore_error_range(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        error_type: IgnoreError,
    ) -> Result<&mut Self> {
        Self::check_range(first_row, first_col, last_row, last_col)?;
        if !self
            .ignored_error_ranges
            .insert((first_row, first_col, last_row, last_col))
        {
            return Err(Error::Parameter(format!(
                "ignored-error range {first_row}:{first_col}:{last_row}:{last_col} was previously used"
            )));
        }
        self.ignored_errors
            .entry(error_type)
            .or_default()
            .push(UncheckedRfX {
                rwFirst: first_row,
                rwLast: last_row,
                colFirst: u32::from(first_col),
                colLast: u32::from(last_col),
            });
        Ok(self)
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

    /// Hides rows that contain no data or explicit row options.
    ///
    /// The default is off.
    pub const fn hide_unused_rows(&mut self, enable: bool) -> &mut Self {
        self.hide_unused_rows = enable;
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

    /// Sets the default cell format and dimensions for a detached worksheet.
    ///
    /// A worksheet created independently of a workbook can carry the same
    /// defaults that [`Workbook::set_default_format`](crate::Workbook::set_default_format)
    /// applies. The parent workbook must also be given the same format and
    /// dimensions, otherwise workbook style 0 remains unchanged.
    ///
    /// `row_height` and `col_width` are in pixels; supported column widths are
    /// 56, 64, 72, 80, 96, 104, and 120.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] if any cell or row/column format has
    /// already been added, or if the column width is unsupported. Returns
    /// [`Error::ValueNotRepresentable`] if the row height exceeds Excel's
    /// limit.
    pub fn set_default_format(
        &mut self,
        format: &Format,
        row_height: u32,
        col_width: u32,
    ) -> Result<&mut Self> {
        if !self.formats.is_empty() {
            return Err(Error::Parameter(
                "default format must be set before adding formats".to_owned(),
            ));
        }
        let Some((digit_width, padding)) = default_column_metrics(col_width) else {
            return Err(Error::Parameter(format!(
                "unsupported default column width: {col_width}"
            )));
        };
        validate_row_height(f64::from(row_height) * 0.75)?;

        self.initialize_default_format(
            format,
            row_height,
            (f64::from(col_width) - f64::from(padding)) / f64::from(digit_width),
        );
        Ok(self)
    }

    pub(crate) fn initialize_default_format(
        &mut self,
        format: &Format,
        row_height: u32,
        col_width: f64,
    ) {
        self.default_format = Some(format.clone());
        self.default_row_height = Some(f64::from(row_height) * 0.75);
        self.default_col_width = Some(col_width);
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

    /// Sets the range printed by default.
    ///
    /// A range covering the entire worksheet clears the print area.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when an endpoint is out of range, or
    /// [`Error::RowColumnOrder`] when the first endpoint follows the last.
    pub fn set_print_area(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<&mut Self> {
        Self::check_row(first_row)?;
        Self::check_col(first_col)?;
        Self::check_row(last_row)?;
        Self::check_col(last_col)?;
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        self.print_area =
            if first_row == 0 && first_col == 0 && last_row == MAX_ROW && last_col == MAX_COL {
                None
            } else {
                Some((first_row, first_col, last_row, last_col))
            };
        Ok(self)
    }

    /// Repeats rows at the top of each printed page.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a row is above [`MAX_ROW`], or
    /// [`Error::RowColumnOrder`] when the first row follows the last.
    pub fn set_repeat_rows(&mut self, first_row: RowNum, last_row: RowNum) -> Result<&mut Self> {
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        if first_row > last_row {
            return Err(Error::RowColumnOrder);
        }
        self.repeat_rows = Some((first_row, last_row));
        Ok(self)
    }

    /// Repeats columns at the left of each printed page.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a column is above [`MAX_COL`], or
    /// [`Error::RowColumnOrder`] when the first column follows the last.
    pub fn set_repeat_columns(&mut self, first_col: ColNum, last_col: ColNum) -> Result<&mut Self> {
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;
        if first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        self.repeat_columns = Some((first_col, last_col));
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

    /// Applies a cell format and a border format to an inclusive rectangular
    /// range without changing values.
    ///
    /// The cell format applies to every cell. The border format supplies only
    /// top, bottom, left, and right border styles and colors for the range's
    /// outer edges. Interior borders and diagonal borders come from
    /// `cell_format`.
    ///
    /// Missing cells become formatted blank cells.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnOrder`] when the range is reversed, or
    /// [`Error::RowColumnLimit`] when it is out of bounds.
    pub fn set_range_format_with_border(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        cell_format: &Format,
        border_format: &Format,
    ) -> Result<&mut Self> {
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;

        if first_row == last_row && first_col == last_col {
            let format = Self::combined_border_format(
                cell_format,
                border_format,
                &[
                    BorderSide::Top,
                    BorderSide::Bottom,
                    BorderSide::Left,
                    BorderSide::Right,
                ],
            );
            self.set_cell_format(first_row, first_col, &format)?;
        } else if first_row == last_row {
            let left = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Top, BorderSide::Bottom, BorderSide::Left],
            );
            let center = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Top, BorderSide::Bottom],
            );
            let right = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Top, BorderSide::Bottom, BorderSide::Right],
            );
            for col in first_col..=last_col {
                let format = if col == first_col {
                    &left
                } else if col == last_col {
                    &right
                } else {
                    &center
                };
                self.set_cell_format(first_row, col, format)?;
            }
        } else if first_col == last_col {
            let top = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Left, BorderSide::Right, BorderSide::Top],
            );
            let center = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Left, BorderSide::Right],
            );
            let bottom = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Left, BorderSide::Right, BorderSide::Bottom],
            );
            for row in first_row..=last_row {
                let format = if row == first_row {
                    &top
                } else if row == last_row {
                    &bottom
                } else {
                    &center
                };
                self.set_cell_format(row, first_col, format)?;
            }
        } else {
            let top_left = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Top, BorderSide::Left],
            );
            let top_center =
                Self::combined_border_format(cell_format, border_format, &[BorderSide::Top]);
            let top_right = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Top, BorderSide::Right],
            );
            let center_left =
                Self::combined_border_format(cell_format, border_format, &[BorderSide::Left]);
            let center_center = Self::combined_border_format(cell_format, border_format, &[]);
            let center_right =
                Self::combined_border_format(cell_format, border_format, &[BorderSide::Right]);
            let bottom_left = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Bottom, BorderSide::Left],
            );
            let bottom_center =
                Self::combined_border_format(cell_format, border_format, &[BorderSide::Bottom]);
            let bottom_right = Self::combined_border_format(
                cell_format,
                border_format,
                &[BorderSide::Bottom, BorderSide::Right],
            );

            for row in first_row..=last_row {
                for col in first_col..=last_col {
                    let format = if row == first_row {
                        if col == first_col {
                            &top_left
                        } else if col == last_col {
                            &top_right
                        } else {
                            &top_center
                        }
                    } else if row == last_row {
                        if col == first_col {
                            &bottom_left
                        } else if col == last_col {
                            &bottom_right
                        } else {
                            &bottom_center
                        }
                    } else if col == first_col {
                        &center_left
                    } else if col == last_col {
                        &center_right
                    } else {
                        &center_center
                    };
                    self.set_cell_format(row, col, format)?;
                }
            }
        }
        Ok(self)
    }

    fn combined_border_format(
        cell_format: &Format,
        border_format: &Format,
        sides: &[BorderSide],
    ) -> Format {
        let mut format = cell_format.clone();
        for &side in sides {
            format.copy_border_side(border_format, side);
        }
        format
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

    /// Sets the worksheet `AutoFilter` range.
    ///
    /// Excel permits one `AutoFilter` per worksheet; calling this again replaces
    /// the previous range.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a corner is out of range, or
    /// [`Error::RowColumnOrder`] when the first corner is after the last.
    pub fn autofilter(
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
        let range = UncheckedRfX {
            rwFirst: first_row,
            rwLast: last_row,
            colFirst: u32::from(first_col),
            colLast: u32::from(last_col),
        };
        if self
            .tables
            .iter()
            .any(|table| table.table.show_autofilter && rfx_overlaps(table.range, range))
        {
            return Err(Error::AutofilterRangeOverlaps);
        }
        self.autofilter = Some(range);
        self.filter_conditions.clear();
        self.filter_hidden_rows.clear();
        Ok(self)
    }

    /// Adds a table over a cell range.
    ///
    /// The table writes its header and optional total rows. Column headers
    /// default to existing header-cell strings, then `Column1`, `Column2`, and
    /// so on. Total labels, `SUBTOTAL` functions, per-column data formats,
    /// header formats, calculated-column formulas, and table filter conditions
    /// are supported. Table filter column numbers are zero-based within the
    /// table; matching rows outside the condition are hidden at save time.
    ///
    /// # Errors
    ///
    /// Returns a range, table-name, header, total, overlap, or `AutoFilter`
    /// error. Table filters also require a header row and `AutoFilter`, a
    /// column inside the table, and at least one value or comparison.
    pub fn add_table(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        table: &Table,
    ) -> Result<&mut Self> {
        Self::check_range(first_row, first_col, last_row, last_col)?;
        let rows = last_row - first_row + 1;
        let header_rows = u32::from(table.show_header_row);
        let total_rows = u32::from(table.show_total_row);
        if rows <= header_rows + total_rows {
            return Err(Error::TableError(
                "table must have at least one data row".to_owned(),
            ));
        }
        table.validate_name()?;
        if !table.filter_conditions.is_empty() {
            if !table.show_header_row || !table.show_autofilter {
                return Err(Error::Parameter(
                    "table filters require a header row and AutoFilter".to_owned(),
                ));
            }
            let width = last_col - first_col + 1;
            for (&col, condition) in &table.filter_conditions {
                if col >= width {
                    return Err(Error::Parameter(format!(
                        "table filter column {col} is outside the table width {width}"
                    )));
                }
                condition
                    .validate()
                    .map_err(|message| Error::Parameter(message.to_owned()))?;
            }
        }

        let range = UncheckedRfX {
            rwFirst: first_row,
            rwLast: last_row,
            colFirst: u32::from(first_col),
            colLast: u32::from(last_col),
        };
        if self
            .tables
            .iter()
            .any(|existing| rfx_overlaps(existing.range, range))
        {
            return Err(Error::TableRangeOverlaps);
        }
        if table.show_autofilter
            && self
                .autofilter
                .is_some_and(|autofilter| rfx_overlaps(autofilter, range))
        {
            return Err(Error::AutofilterRangeOverlaps);
        }

        let mut stored = table.clone();
        if !stored.filter_conditions.is_empty() {
            let conditions = std::mem::take(&mut stored.filter_conditions);
            for (col, condition) in conditions {
                let col = first_col.checked_add(col).ok_or_else(|| {
                    Error::ValueNotRepresentable("table filter column is out of range")
                })?;
                stored.filter_conditions.insert(col, condition);
            }
        }
        let mut columns = std::mem::take(&mut stored.columns);
        columns.resize(
            usize::from(last_col - first_col + 1),
            crate::table::TableColumn::default(),
        );
        let mut names = std::collections::HashSet::with_capacity(columns.len());
        for (index, column) in columns.iter_mut().enumerate() {
            if column.header.is_empty()
                && table.show_header_row
                && let Some(CellData::String(text) | CellData::RichString(RichString { text, .. })) =
                    self.cells
                        .get(&first_row)
                        .and_then(|cells| cells.get(&(first_col + u16::try_from(index).ok()?)))
                        .map(|cell| &cell.data)
            {
                column.header.clone_from(text);
            }
            if column.header.is_empty() {
                column.header = format!("Column{}", index + 1);
            }
            if column.header.encode_utf16().count() > 255 {
                return Err(Error::TableError(
                    "table column header exceeds 255 characters".to_owned(),
                ));
            }
            if column.total_label.encode_utf16().count() > 8_189 {
                return Err(Error::TableError(
                    "table total label exceeds 8189 characters".to_owned(),
                ));
            }
            if !names.insert(column.header.to_lowercase()) {
                return Err(Error::TableError(format!(
                    "column name '{}' already exists in table",
                    column.header
                )));
            }
            if column.total_label.is_empty()
                && let TableFunction::Custom(formula) = &column.total_function
            {
                if formula
                    .result_text()
                    .is_some_and(|result| result.encode_utf16().count() > MAX_STRING_UTF16)
                {
                    return Err(Error::StringTooLong);
                }
                column.total_formula = Some(compile(formula.as_str())?);
                column.total_result = formula.result_text().map(Into::into);
            }
        }

        let data_first = first_row + header_rows;
        let data_last = last_row - total_rows;
        for (index, column) in columns.iter().enumerate() {
            let col = first_col
                + u16::try_from(index)
                    .map_err(|_| Error::ValueNotRepresentable("too many table columns"))?;
            if table.show_header_row {
                let format = column
                    .header_format
                    .as_ref()
                    .map(|format| self.intern_format(format));
                self.store_with_index(
                    first_row,
                    col,
                    CellData::String(column.header.clone()),
                    format,
                )?;
            }

            let format = column
                .format
                .as_ref()
                .map(|format| self.intern_format(format));
            if total_rows == 1 {
                if !column.total_label.is_empty() {
                    self.store_with_index(
                        last_row,
                        col,
                        CellData::String(column.total_label.clone()),
                        format,
                    )?;
                } else if let Some(function) = column.total_function.subtotal_number() {
                    let result = cached_result(&self.default_formula_result);
                    self.store_with_index(
                        last_row,
                        col,
                        CellData::Formula(FormulaCell {
                            formula: subtotal_formula(function, data_first, data_last, col),
                            result,
                        }),
                        format,
                    )?;
                } else if matches!(column.total_function, TableFunction::Custom(_)) {
                    let result = column.total_result.as_deref().map_or_else(
                        || cached_result(&self.default_formula_result),
                        cached_result,
                    );
                    self.store_with_index(
                        last_row,
                        col,
                        CellData::Formula(FormulaCell {
                            formula: column
                                .total_formula
                                .clone()
                                .expect("custom total formula compiled"),
                            result,
                        }),
                        format,
                    )?;
                }
            }

            if let Some(format) = format {
                for row in data_first..=data_last {
                    self.cells
                        .entry(row)
                        .or_default()
                        .entry(col)
                        .and_modify(|cell| cell.format = Some(format))
                        .or_insert(Cell {
                            data: CellData::Blank,
                            format: Some(format),
                        });
                }
            }
        }

        stored.columns = columns;
        self.tables.push(TableRange {
            table: stored,
            range,
        });
        Ok(self)
    }

    /// Adds a sparkline to one cell.
    ///
    /// The sparkline data range must be one-dimensional. A range without a
    /// worksheet name uses this worksheet.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when the cell is out of range,
    /// [`Error::SparklineError`] when the data range is missing, invalid, or
    /// not one-dimensional.
    pub fn add_sparkline(
        &mut self,
        row: RowNum,
        col: ColNum,
        sparkline: &Sparkline,
    ) -> Result<&mut Self> {
        if row > MAX_ROW || col > MAX_COL {
            return Err(Error::RowColumnLimit);
        }
        let range = sparkline
            .range
            .clone()
            .ok_or_else(|| Error::SparklineError("sparkline data range not set".to_owned()))?;
        range.validate()?;
        if !range.is_1d() {
            return Err(Error::SparklineError(
                "sparkline data range must be a 1D range".to_owned(),
            ));
        }
        self.sparkline_groups.push(SparklineGroup {
            sparkline: sparkline.clone(),
            cells: vec![SparklineCell { row, col, range }],
        });
        Ok(self)
    }

    /// Adds one sparkline group across a row or column.
    ///
    /// `first_row..=last_row` and `first_col..=last_col` must describe either
    /// one row or one column. The data range must be two-dimensional, and its
    /// row or column count must match the target cells. Each target cell gets
    /// the matching slice of the data range.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] or [`Error::RowColumnOrder`] for an
    /// invalid target range, or [`Error::SparklineError`] when the data range
    /// is missing, invalid, not two-dimensional, or does not match the target.
    pub fn add_sparkline_group(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        sparkline: &Sparkline,
    ) -> Result<&mut Self> {
        Self::check_range(first_row, first_col, last_row, last_col)?;
        if first_row != last_row && first_col != last_col {
            return Err(Error::SparklineError(
                "sparkline group target must be one row or one column".to_owned(),
            ));
        }
        let range = sparkline
            .range
            .clone()
            .ok_or_else(|| Error::SparklineError("sparkline data range not set".to_owned()))?;
        range.validate()?;
        if !range.is_2d() {
            return Err(Error::SparklineError(
                "sparkline group data range must be a 2D range".to_owned(),
            ));
        }

        let target_rows = last_row - first_row + 1;
        let target_cols = u32::from(last_col - first_col) + 1;
        let mut cells =
            Vec::with_capacity(usize::try_from(target_rows.max(target_cols)).unwrap_or(0));
        if target_rows > 1 {
            if range.rows() != target_rows {
                return Err(Error::SparklineError(
                    "sparkline group rows do not match the data range".to_owned(),
                ));
            }
            for row in first_row..=last_row {
                cells.push(SparklineCell {
                    row,
                    col: first_col,
                    range: SparklineRange {
                        sheet: range.sheet.clone(),
                        first_row: row - first_row + range.first_row,
                        last_row: row - first_row + range.first_row,
                        first_col: range.first_col,
                        last_col: range.last_col,
                    },
                });
            }
        } else {
            if range.cols() != target_cols {
                return Err(Error::SparklineError(
                    "sparkline group columns do not match the data range".to_owned(),
                ));
            }
            for col in first_col..=last_col {
                cells.push(SparklineCell {
                    row: first_row,
                    col,
                    range: SparklineRange {
                        sheet: range.sheet.clone(),
                        first_row: range.first_row,
                        last_row: range.last_row,
                        first_col: col - first_col + range.first_col,
                        last_col: col - first_col + range.first_col,
                    },
                });
            }
        }

        self.sparkline_groups.push(SparklineGroup {
            sparkline: sparkline.clone(),
            cells,
        });
        Ok(self)
    }

    /// Adds a conditional format over a cell range.
    ///
    /// The rule is evaluated against the range; a differential format is
    /// applied when it matches. If the rule has a non-empty `set_multi_range`
    /// override, those same-sheet areas replace the range passed here.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] or [`Error::RowColumnOrder`] for an
    /// invalid range, or [`Error::Parameter`] when the rule is missing, its
    /// value is invalid, or the multi-range text is malformed or not
    /// same-sheet A1 notation.
    pub fn add_conditional_format<T>(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        conditional_format: &T,
    ) -> Result<&mut Self>
    where
        T: ConditionalFormat + Send + Sync + ?Sized,
    {
        Self::check_range(first_row, first_col, last_row, last_col)?;
        conditional_format.validate()?;
        let range = UncheckedRfX {
            rwFirst: first_row,
            rwLast: last_row,
            colFirst: u32::from(first_col),
            colLast: u32::from(last_col),
        };
        let ranges = RangeList::from_primary(range, conditional_format.multi_range(), 8_192)?;
        self.conditional_formats.push(ConditionalFormatRange::new(
            ranges,
            conditional_format.box_clone(),
        ));
        Ok(self)
    }

    /// Adds a data validation for a cell range.
    ///
    /// Multiple validations can be added; each keeps its own range and options.
    /// A non-empty `set_multi_range` override replaces the range passed here
    /// with the listed same-sheet areas.
    ///
    /// # Errors
    ///
    /// Returns [`Error::RowColumnLimit`] when a corner is out of range,
    /// [`Error::RowColumnOrder`] when the first corner is after the last, or
    /// [`Error::Parameter`] when the multi-range text is malformed or not
    /// same-sheet A1 notation.
    pub fn add_data_validation(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        data_validation: &DataValidation,
    ) -> Result<&mut Self> {
        Self::check_row(first_row)?;
        Self::check_row(last_row)?;
        Self::check_col(first_col)?;
        Self::check_col(last_col)?;
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        if data_validation.is_invalid_any() {
            return Ok(self);
        }

        let range = UncheckedRfX {
            rwFirst: first_row,
            rwLast: last_row,
            colFirst: u32::from(first_col),
            colLast: u32::from(last_col),
        };
        let ranges = RangeList::from_primary(range, data_validation.multi_range(), 8_191)?;
        if let Some(existing) = self
            .data_validations
            .iter_mut()
            .find(|item| item.ranges.as_slice() == ranges.as_slice())
        {
            existing.validation = data_validation.clone();
        } else {
            self.data_validations.push(DataValidationRange {
                ranges,
                validation: data_validation.clone(),
            });
        }
        Ok(self)
    }

    /// Adds or replaces a filter condition for an autofilter column.
    ///
    /// Rows that do not match all worksheet column conditions are hidden when
    /// the workbook is saved. Use [`Self::filter_automatic_off`] to keep the
    /// records but leave worksheet row visibility to Excel. Table filters are
    /// independent and always hide their non-matching data rows.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when no autofilter range exists, the column
    /// is outside that range, or the condition has no filter values.
    pub fn filter_column(
        &mut self,
        col: ColNum,
        filter_condition: &FilterCondition,
    ) -> Result<&mut Self> {
        Self::check_col(col)?;
        let range = self.autofilter.ok_or_else(|| {
            Error::Parameter(
                "the autofilter range must be set before a filter condition".to_owned(),
            )
        })?;
        if u32::from(col) < range.colFirst || u32::from(col) > range.colLast {
            return Err(Error::Parameter(format!(
                "column {col} is outside the autofilter range {}-{}",
                range.colFirst, range.colLast
            )));
        }
        filter_condition
            .validate()
            .map_err(|message| Error::Parameter(message.to_owned()))?;
        self.filter_conditions.insert(col, filter_condition.clone());
        Ok(self)
    }

    /// Disables automatic hidden-row updates for worksheet autofilter
    /// conditions.
    ///
    /// Filter records are still written; only worksheet row visibility is left
    /// to the application opening the workbook. Table filters are independent
    /// and continue to hide their non-matching data rows.
    pub const fn filter_automatic_off(&mut self) -> &mut Self {
        self.filter_automatic_off = true;
        self
    }

    pub(crate) fn apply_filters(&mut self) {
        self.filter_hidden_rows.clear();
        if let Some(range) = self.autofilter
            && !self.filter_automatic_off
            && !self.filter_conditions.is_empty()
        {
            for row in range.rwFirst.saturating_add(1)..=range.rwLast {
                let hidden = self.filter_conditions.iter().any(|(&col, condition)| {
                    let cell = self.cells.get(&row).and_then(|cells| cells.get(&col));
                    !condition.matches(cell.map(|cell| &cell.data))
                });
                if hidden {
                    self.filter_hidden_rows.insert(row);
                }
            }
        }

        for table in &self.tables {
            if !table.table.show_autofilter || table.table.filter_conditions.is_empty() {
                continue;
            }
            let data_first = table.range.rwFirst + u32::from(table.table.show_header_row);
            let data_last = table.range.rwLast - u32::from(table.table.show_total_row);
            for row in data_first..=data_last {
                let hidden = table
                    .table
                    .filter_conditions
                    .iter()
                    .any(|(&col, condition)| {
                        let cell = self.cells.get(&row).and_then(|cells| cells.get(&col));
                        !condition.matches(cell.map(|cell| &cell.data))
                    });
                if hidden {
                    self.filter_hidden_rows.insert(row);
                }
            }
        }
    }

    /// Sets the worksheet background image.
    ///
    /// The image is written to the package as `xl/media/imageN.<ext>` and
    /// referenced from the worksheet. Replacing it before saving keeps only the
    /// last image.
    pub fn insert_background_image(&mut self, image: &Image) -> &mut Self {
        self.background_image = Some(image.clone());
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
        let data = match data {
            CellData::Number(value) if value.is_nan() => {
                CellData::String(self.nan_value.as_deref().unwrap_or("NAN").to_owned())
            }
            CellData::Number(value) if value == f64::INFINITY => {
                CellData::String(self.infinity_value.as_deref().unwrap_or("INF").to_owned())
            }
            CellData::Number(value) if value == f64::NEG_INFINITY => CellData::String(
                self.neg_infinity_value
                    .as_deref()
                    .unwrap_or("-INF")
                    .to_owned(),
            ),
            other => other,
        };
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

    pub(crate) const fn hyperlinks(&self) -> &BTreeMap<(RowNum, ColNum), Url> {
        &self.hyperlinks
    }

    pub(crate) fn merged_ranges(&self) -> &[MergeRange] {
        &self.merged_ranges
    }

    pub(crate) fn tables(&self) -> &[TableRange] {
        &self.tables
    }

    pub(crate) fn tables_mut(&mut self) -> &mut [TableRange] {
        &mut self.tables
    }

    pub(crate) fn sparkline_groups(&self) -> &[SparklineGroup] {
        &self.sparkline_groups
    }

    pub(crate) const fn autofilter_range(&self) -> Option<UncheckedRfX> {
        self.autofilter
    }

    pub(crate) const fn filter_conditions(&self) -> &BTreeMap<ColNum, FilterCondition> {
        &self.filter_conditions
    }

    pub(crate) const fn filter_hidden_rows(&self) -> &BTreeSet<RowNum> {
        &self.filter_hidden_rows
    }

    pub(crate) fn conditional_formats(&self) -> &[ConditionalFormatRange] {
        &self.conditional_formats
    }

    pub(crate) const fn background_image(&self) -> Option<&Image> {
        self.background_image.as_ref()
    }

    pub(crate) const fn has_rich_strings(&self) -> bool {
        self.has_rich_strings
    }

    pub(crate) fn has_formulas(&self) -> bool {
        self.cells.values().any(|row| {
            row.values()
                .any(|cell| matches!(&cell.data, CellData::Formula(_) | CellData::ArrayFormula(_)))
        })
    }

    pub(crate) fn data_validations(&self) -> &[DataValidationRange] {
        &self.data_validations
    }

    pub(crate) const fn print_area(&self) -> Option<(RowNum, ColNum, RowNum, ColNum)> {
        self.print_area
    }

    pub(crate) const fn repeat_rows(&self) -> Option<(RowNum, RowNum)> {
        self.repeat_rows
    }

    pub(crate) const fn repeat_columns(&self) -> Option<(ColNum, ColNum)> {
        self.repeat_columns
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

    fn check_range(
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<()> {
        Self::check_row(first_row)?;
        Self::check_col(first_col)?;
        Self::check_row(last_row)?;
        Self::check_col(last_col)?;
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        Ok(())
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

    pub(crate) const fn default_col_width(&self) -> Option<f64> {
        self.default_col_width
    }

    pub(crate) const fn unused_rows_hidden(&self) -> bool {
        self.hide_unused_rows
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

    pub(crate) fn unprotected_ranges(&self) -> &[UnprotectedRange] {
        &self.unprotected_ranges
    }

    pub(crate) const fn ignored_errors(&self) -> &BTreeMap<IgnoreError, Vec<UncheckedRfX>> {
        &self.ignored_errors
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

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the workbook default format"
)]
pub(crate) const fn default_column_metrics(width: u32) -> Option<(u32, u32)> {
    match width {
        56 => Some((6, 5)),
        64 => Some((7, 5)),
        72 => Some((8, 5)),
        80 => Some((9, 7)),
        96 => Some((11, 7)),
        104 => Some((12, 7)),
        120 => Some((13, 9)),
        _ => None,
    }
}

fn valid_margin(margin: f64) -> bool {
    (0.0..MAX_MARGIN).contains(&margin)
}

#[expect(clippy::redundant_pub_crate, reason = "shared with the sheet tests")]
pub(crate) fn pixels_to_width(pixels: u32) -> f64 {
    let pixels = f64::from(pixels);
    if pixels <= 12.0 {
        pixels / 12.0
    } else {
        (pixels - 5.0) / 7.0
    }
}

fn formula_result_width(result: &FormulaResult) -> u32 {
    match result {
        FormulaResult::Number(value) => 7 * display_len(*value),
        FormulaResult::String(text) => text.lines().map(pixel_width).max().unwrap_or(0),
        FormulaResult::Bool(true) => 31,
        FormulaResult::Bool(false) => 36,
        FormulaResult::Error(_) => 0,
    }
}

fn autofit_cell_width(data: &CellData) -> u32 {
    match data {
        CellData::String(text) => text.lines().map(pixel_width).max().unwrap_or(0),
        CellData::RichString(rich) => rich.text.lines().map(pixel_width).max().unwrap_or(0),
        CellData::Integer(value) => 7 * integer_digits(*value) + if *value < 0 { 7 } else { 0 },
        CellData::Number(value) => 7 * display_len(*value),
        CellData::Formula(formula) => formula_result_width(&formula.result),
        CellData::ArrayFormula(formula) => formula_result_width(&formula.result),
        CellData::Bool(true) => 31,
        CellData::Bool(false) => 36,
        CellData::Date(_)
        | CellData::Time(_)
        | CellData::DateTime(_)
        | CellData::ExcelDateTime(_)
        | CellData::Duration(_) => 68,
        CellData::Blank | CellData::Error(_) => 0,
    }
}

// Approximate Calibri 11 character widths taken from Excel.
fn pixel_width(text: &str) -> u32 {
    if text.chars().count() > 233 {
        return MAX_AUTOFIT_WIDTH_PIXELS;
    }

    let mut width = 0;
    for character in text.chars() {
        width += match character {
            ' ' | '\'' => 3,
            ',' | '.' | ':' | ';' | 'I' | '`' | 'i' | 'j' | 'l' => 4,
            '!' | '(' | ')' | '-' | 'J' | '[' | ']' | 'f' | 'r' | 't' | '{' | '}' => 5,
            '"' | '/' | 'L' | '\\' | 'c' | 's' | 'z' => 6,
            '#'
            | '$'
            | '*'
            | '+'
            | '0'..='9'
            | '<'
            | '='
            | '>'
            | '?'
            | 'E'
            | 'F'
            | 'S'
            | 'T'
            | 'Y'
            | 'Z'
            | '^'
            | '_'
            | 'a'
            | 'g'
            | 'k'
            | 'v'
            | 'x'
            | 'y'
            | '|'
            | '~' => 7,
            'A' | 'D' | 'G' | 'H' | 'U' | 'V' => 9,
            '&' | 'N' | 'O' | 'Q' => 10,
            '%' | 'w' => 11,
            'M' | 'm' => 12,
            '@' | 'W' => 13,
            _ => 8,
        };
    }
    width
}

const fn integer_digits(value: i64) -> u32 {
    let mut value = value.unsigned_abs();
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn display_len(value: f64) -> u32 {
    struct Counter(usize);

    impl std::fmt::Write for Counter {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0 += text.len();
            Ok(())
        }
    }

    use std::fmt::Write as _;

    let mut counter = Counter(0);
    let _ = write!(&mut counter, "{value}");
    u32::try_from(counter.0).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CellData;
    use crate::table::TableColumn;
    use crate::{FormatBorder, FormatDiagonalBorder};
    use rust_xlsb_core::StyleRegistry;
    use rust_xlsb_core::prelude::BorderType;

    fn border_sides(worksheet: &Worksheet, row: RowNum, col: ColNum) -> [BorderType; 4] {
        let format = worksheet
            .cells()
            .get(&row)
            .and_then(|cells| cells.get(&col))
            .and_then(|cell| cell.format)
            .expect("cell format");
        let mut registry = StyleRegistry::new();
        let style = worksheet.formats()[format as usize]
            .to_cell_style(&mut registry)
            .expect("style");
        [
            style.border.blxfTop.border_type.clone(),
            style.border.blxfBottom.border_type.clone(),
            style.border.blxfLeft.border_type.clone(),
            style.border.blxfRight.border_type,
        ]
    }

    fn assert_border(
        worksheet: &Worksheet,
        row: RowNum,
        col: ColNum,
        top: BorderType,
        bottom: BorderType,
        left: BorderType,
        right: BorderType,
    ) {
        assert_eq!(
            border_sides(worksheet, row, col),
            [top, bottom, left, right],
            "border mismatch at ({row}, {col})"
        );
    }

    #[test]
    fn writes_cells_and_styles() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn array_formula_owns_the_range_and_fills_zero_cells() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .write_array_formula(1, 1, 2, 2, "{=SUM(A1:B2)}")
            .expect("array formula");

        let cell = worksheet
            .cells()
            .get(&1)
            .and_then(|row| row.get(&1))
            .expect("top-left cell");
        let CellData::ArrayFormula(formula) = &cell.data else {
            panic!("array formula cell");
        };
        assert_eq!(formula.range.rwFirst, 1);
        assert_eq!(formula.range.rwLast, 2);
        assert_eq!(formula.range.colFirst, 1);
        assert_eq!(formula.range.colLast, 2);
        assert_eq!(formula.result, FormulaResult::Number(0.0));
        assert_eq!(
            formula.formula.rgce,
            [
                0x45, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0xC0, 1, 0xC0, 0x42, 1, 4, 0
            ]
        );

        for row in 1..=2 {
            for col in 1..=2 {
                if row == 1 && col == 1 {
                    continue;
                }
                assert_eq!(
                    worksheet
                        .cells()
                        .get(&row)
                        .and_then(|cells| cells.get(&col))
                        .map(|cell| &cell.data),
                    Some(&CellData::Number(0.0))
                );
            }
        }
    }

    #[test]
    fn array_formula_applies_the_range_format() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .write_array_formula_with_format(
                0,
                0,
                0,
                1,
                Formula::new("=1+1").set_result("2"),
                &Format::new().set_bold(),
            )
            .expect("array formula");

        let top_left = worksheet
            .cells()
            .get(&0)
            .and_then(|row| row.get(&0))
            .expect("top-left");
        let CellData::ArrayFormula(formula) = &top_left.data else {
            panic!("array formula cell");
        };
        assert_eq!(formula.result, FormulaResult::Number(2.0));
        assert_eq!(top_left.format, Some(0));
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&1))
                .and_then(|cell| cell.format),
            Some(0)
        );
    }

    #[test]
    fn array_formula_cached_result_can_be_replaced() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .write_array_formula(0, 0, 1, 0, "=1+1")
            .expect("array formula")
            .set_formula_result(0, 0, "2");

        let cell = worksheet
            .cells()
            .get(&0)
            .and_then(|row| row.get(&0))
            .expect("top-left");
        let CellData::ArrayFormula(formula) = &cell.data else {
            panic!("array formula cell");
        };
        assert_eq!(formula.result, FormulaResult::Number(2.0));
    }

    #[test]
    fn array_formula_rejects_inverted_and_malformed_ranges() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.write_array_formula(1, 0, 0, 1, "=1+1"),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.write_array_formula(0, 1, 1, 0, "=1+1"),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.write_array_formula(0, 0, 0, 0, "=NOPE(1)"),
            Err(Error::FormulaError(_))
        ));
    }

    #[test]
    fn writes_rich_strings_with_cell_and_run_formats() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let bold = Format::new().set_bold();
        let large = Format::new().set_font_size(14.0);

        worksheet
            .write_rich_string_with_format(
                0,
                0,
                &[(&Format::default(), "A"), (&bold, "B"), (&large, "C")],
                &Format::new().set_align(crate::FormatAlign::Center),
            )
            .expect("rich string");

        let cell = worksheet
            .cells()
            .get(&0)
            .and_then(|row| row.get(&0))
            .expect("cell");
        let crate::data::CellData::RichString(rich) = &cell.data else {
            panic!("rich string cell");
        };
        assert_eq!(rich.text, "ABC");
        assert_eq!(
            rich.runs.iter().map(|run| run.ich).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            rich.runs[1].font.bls,
            rust_xlsb_core::prelude::Bold::BLSBOLD
        );
        assert_eq!(rich.runs[2].font.dyHeight, 280);
        assert_eq!(cell.format, Some(0));
    }

    #[test]
    fn rich_strings_reject_empty_and_oversized_input() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let format = Format::default();

        assert!(matches!(
            worksheet.write_rich_string(0, 0, &[]),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.write_rich_string(0, 0, &[(&format, "")]),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.write_rich_string(
                0,
                0,
                &[
                    (&format, "x".repeat(16_384).as_str()),
                    (&format, "y".repeat(16_384).as_str())
                ]
            ),
            Err(Error::StringTooLong)
        ));
        assert!(matches!(
            worksheet.write_rich_string(MAX_ROW + 1, 0, &[(&format, "x")]),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.write_rich_string(
                0,
                0,
                &[(&Format::new().set_font_name("x".repeat(32)), "x")]
            ),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn rejects_bounds_and_long_strings() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        assert_eq!(worksheet.page_breaks().len(), 0);
    }

    #[test]
    fn page_break_limits_do_not_mutate_state() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn autofilter_stores_the_last_valid_range() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert_eq!(worksheet.autofilter_range(), None);

        worksheet.autofilter(1, 2, 5, 4).expect("autofilter");
        assert_eq!(
            worksheet.autofilter_range(),
            Some(UncheckedRfX {
                rwFirst: 1,
                rwLast: 5,
                colFirst: 2,
                colLast: 4,
            })
        );

        worksheet.autofilter(0, 0, 2, 1).expect("replace");
        assert_eq!(
            worksheet.autofilter_range(),
            Some(UncheckedRfX {
                rwFirst: 0,
                rwLast: 2,
                colFirst: 0,
                colLast: 1,
            })
        );
    }

    #[test]
    fn autofilter_rejects_invalid_ranges_without_replacing_state() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.autofilter(1, 1, 2, 2).expect("initial range");

        assert!(matches!(
            worksheet.autofilter(2, 0, 1, 0),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.autofilter(0, 0, MAX_ROW + 1, 0),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(
            worksheet.autofilter_range(),
            Some(UncheckedRfX {
                rwFirst: 1,
                rwLast: 2,
                colFirst: 1,
                colLast: 2,
            })
        );
    }

    #[test]
    fn filter_conditions_require_an_autofilter_and_a_column_in_range() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let condition = FilterCondition::new().add_list_filter("East");

        assert!(matches!(
            worksheet.filter_column(0, &condition),
            Err(Error::Parameter(_))
        ));

        worksheet.autofilter(0, 1, 4, 2).expect("autofilter");
        assert!(matches!(
            worksheet.filter_column(0, &condition),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.filter_column(1, &FilterCondition::new()),
            Err(Error::Parameter(_))
        ));
        worksheet
            .filter_column(1, &condition)
            .expect("valid filter");

        let replacement = FilterCondition::new().add_list_filter("West");
        worksheet
            .filter_column(1, &replacement)
            .expect("replacement");
        assert_eq!(worksheet.filter_conditions().get(&1), Some(&replacement));

        worksheet.autofilter(0, 1, 4, 2).expect("new range");
        assert!(worksheet.filter_conditions().is_empty());
    }

    #[test]
    fn list_filters_hide_nonmatching_rows_and_can_be_disabled() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.autofilter(0, 0, 3, 0).expect("autofilter");
        worksheet.write_string(0, 0, "Region").expect("header");
        worksheet.write_string(1, 0, "East").expect("east");
        worksheet.write_string(2, 0, "West").expect("west");
        worksheet
            .filter_column(0, &FilterCondition::new().add_list_filter("East"))
            .expect("filter");

        worksheet.apply_filters();
        assert_eq!(worksheet.filter_hidden_rows(), &[2, 3].into());

        worksheet.filter_automatic_off();
        worksheet.apply_filters();
        assert!(worksheet.filter_hidden_rows().is_empty());
    }

    #[test]
    fn custom_filters_hide_rows_using_and_or_and_blanks() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.autofilter(0, 0, 3, 0).expect("autofilter");
        for (row, value) in [(1, 3000), (2, 5000), (3, 9000)] {
            worksheet.write_number(row, 0, value).expect("number");
        }
        worksheet
            .filter_column(
                0,
                &FilterCondition::new()
                    .add_custom_filter(crate::FilterCriteria::GreaterThanOrEqualTo, 4000_i32)
                    .add_custom_filter(crate::FilterCriteria::LessThanOrEqualTo, 8000_i32),
            )
            .expect("between");
        worksheet.apply_filters();
        assert_eq!(worksheet.filter_hidden_rows(), &[1, 3].into());

        worksheet
            .filter_column(
                0,
                &FilterCondition::new()
                    .add_custom_filter(crate::FilterCriteria::EqualTo, 3000_i32)
                    .add_custom_filter(crate::FilterCriteria::EqualTo, 9000_i32)
                    .add_custom_boolean_or(),
            )
            .expect("or");
        worksheet.apply_filters();
        assert_eq!(worksheet.filter_hidden_rows(), &[2].into());
    }

    #[test]
    fn temporal_and_boolean_filters_hide_nonmatching_rows() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.autofilter(0, 0, 3, 1).expect("autofilter");
        worksheet.write_string(0, 0, "Date").expect("header");
        worksheet.write_string(0, 1, "Active").expect("header");
        for (row, day) in [(1, 1), (2, 2), (3, 3)] {
            let date = chrono::NaiveDate::from_ymd_opt(2024, 1, day).expect("date");
            worksheet.write_date(row, 0, date).expect("date");
        }
        worksheet.write_boolean(1, 1, true).expect("bool");
        worksheet.write_boolean(2, 1, false).expect("bool");
        worksheet.write_boolean(3, 1, true).expect("bool");

        worksheet
            .filter_column(1, &FilterCondition::new().add_list_filter("TRUE"))
            .expect("boolean filter");
        worksheet.apply_filters();
        assert_eq!(worksheet.filter_hidden_rows(), &[2].into());

        worksheet
            .filter_column(0, &FilterCondition::new().add_list_filter(45292_i32))
            .expect("date filter");
        worksheet.apply_filters();
        assert_eq!(worksheet.filter_hidden_rows(), &[2, 3].into());
    }

    #[test]
    fn merge_range_writes_string_and_formatted_blanks() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_range_format(2, 0, 1, 0, &Format::new()),
            Err(Error::RowColumnOrder)
        ));
    }

    #[test]
    fn write_row_keeps_the_cells_before_the_failure() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.set_column_range_width(2, 1, 10.0),
            Err(Error::RowColumnOrder)
        ));
    }

    #[test]
    fn tab_state_follows_visibility_rules() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn non_finite_numbers_become_display_strings() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.write(0, 0, f64::NAN).expect("nan");
        worksheet.write(1, 0, f64::INFINITY).expect("infinity");
        worksheet
            .write(2, 0, f64::NEG_INFINITY)
            .expect("negative infinity");

        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("NAN".to_owned()))
        );
        assert_eq!(
            worksheet
                .cells()
                .get(&1)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("INF".to_owned()))
        );
        assert_eq!(
            worksheet
                .cells()
                .get(&2)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("-INF".to_owned()))
        );
    }

    #[test]
    fn custom_non_finite_strings_preserve_formats() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.set_nan_value("NaN");
        worksheet.set_infinity_value("Infinity");
        worksheet.set_neg_infinity_value("NegInfinity");
        worksheet
            .write_with_format(0, 0, f64::NAN, &Format::new().set_bold())
            .expect("nan");

        let cell = worksheet
            .cells()
            .get(&0)
            .and_then(|row| row.get(&0))
            .expect("cell");
        assert_eq!(cell.data, CellData::String("NaN".to_owned()));
        assert_eq!(cell.format, Some(0));

        worksheet.write(1, 0, f64::INFINITY).expect("infinity");
        worksheet
            .write(2, 0, f64::NEG_INFINITY)
            .expect("negative infinity");
        assert_eq!(
            worksheet
                .cells()
                .get(&1)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("Infinity".to_owned()))
        );
        assert_eq!(
            worksheet
                .cells()
                .get(&2)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("NegInfinity".to_owned()))
        );
    }

    #[test]
    fn non_finite_display_strings_obey_the_string_limit() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.set_nan_value("x".repeat(MAX_STRING_UTF16 + 1));

        assert!(matches!(
            worksheet.write(0, 0, f64::NAN),
            Err(Error::StringTooLong)
        ));
    }

    #[test]
    fn range_border_positions_follow_the_rectangle() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let cell_format = Format::new()
            .set_bold()
            .set_background_color(Color::Yellow)
            .set_border_diagonal(FormatBorder::Thin)
            .set_border_diagonal_color(Color::Blue)
            .set_border_diagonal_type(FormatDiagonalBorder::BorderUp);
        let border_format = Format::new()
            .set_border(FormatBorder::Medium)
            .set_border_color(Color::Red);
        worksheet
            .set_range_format_with_border(0, 0, 2, 2, &cell_format, &border_format)
            .expect("range");

        let medium = || BorderType::Medium;
        let none = || BorderType::None;
        assert_border(&worksheet, 0, 0, medium(), none(), medium(), none());
        assert_border(&worksheet, 0, 1, medium(), none(), none(), none());
        assert_border(&worksheet, 0, 2, medium(), none(), none(), medium());
        assert_border(&worksheet, 1, 0, none(), none(), medium(), none());
        assert_border(&worksheet, 1, 1, none(), none(), none(), none());
        assert_border(&worksheet, 1, 2, none(), none(), none(), medium());
        assert_border(&worksheet, 2, 0, none(), medium(), medium(), none());
        assert_border(&worksheet, 2, 1, none(), medium(), none(), none());
        assert_border(&worksheet, 2, 2, none(), medium(), none(), medium());

        let format = worksheet
            .cells()
            .get(&1)
            .and_then(|cells| cells.get(&1))
            .and_then(|cell| cell.format)
            .expect("center format");
        let mut registry = StyleRegistry::new();
        let style = worksheet.formats()[format as usize]
            .to_cell_style(&mut registry)
            .expect("style");
        assert_eq!(style.font.bls, rust_xlsb_core::prelude::Bold::BLSBOLD);
        assert_eq!(style.fill.fls, 1);
        assert!(style.border.fBdrDiagUp);
        assert_eq!(style.border.blxfDiag.border_type, BorderType::Thin);
        let format = worksheet
            .cells()
            .get(&0)
            .and_then(|cells| cells.get(&0))
            .and_then(|cell| cell.format)
            .expect("corner format");
        let style = worksheet.formats()[format as usize]
            .to_cell_style(&mut registry)
            .expect("style");
        assert_eq!(
            style.border.blxfTop.color,
            Color::Red.to_brt_color().expect("red")
        );
    }

    #[test]
    fn range_border_handles_single_cell_row_and_column() {
        let border = Format::new().set_border(FormatBorder::Thin);
        let medium = || BorderType::Thin;
        let none = || BorderType::None;

        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .set_range_format_with_border(1, 1, 1, 1, &Format::new(), &border)
            .expect("cell");
        assert_border(&worksheet, 1, 1, medium(), medium(), medium(), medium());

        worksheet
            .set_range_format_with_border(3, 1, 3, 3, &Format::new(), &border)
            .expect("row");
        assert_border(&worksheet, 3, 1, medium(), medium(), medium(), none());
        assert_border(&worksheet, 3, 2, medium(), medium(), none(), none());
        assert_border(&worksheet, 3, 3, medium(), medium(), none(), medium());

        worksheet
            .set_range_format_with_border(5, 1, 7, 1, &Format::new(), &border)
            .expect("column");
        assert_border(&worksheet, 5, 1, medium(), none(), medium(), medium());
        assert_border(&worksheet, 6, 1, none(), none(), medium(), medium());
        assert_border(&worksheet, 7, 1, none(), medium(), medium(), medium());
    }

    #[test]
    fn range_border_rejects_invalid_ranges_before_mutating() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let format = Format::new();
        let border = Format::new().set_border(FormatBorder::Thin);

        assert!(matches!(
            worksheet.set_range_format_with_border(2, 0, 1, 0, &format, &border),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.set_range_format_with_border(0, 0, MAX_ROW + 1, 0, &format, &border),
            Err(Error::RowColumnLimit)
        ));
        assert!(worksheet.cells().is_empty());
    }

    #[test]
    fn print_area_validates_and_clears_the_full_sheet() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.set_print_area(1, 2, 3, 4).expect("area");
        assert_eq!(worksheet.print_area(), Some((1, 2, 3, 4)));

        assert!(matches!(
            worksheet.set_print_area(3, 0, 1, 0),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.set_print_area(0, 0, MAX_ROW + 1, 0),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(worksheet.print_area(), Some((1, 2, 3, 4)));

        worksheet
            .set_print_area(0, 0, MAX_ROW, MAX_COL)
            .expect("full sheet");
        assert_eq!(worksheet.print_area(), None);
    }

    #[test]
    fn repeat_rows_and_columns_are_independent() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.set_repeat_rows(0, 1).expect("rows");
        worksheet.set_repeat_columns(1, 2).expect("columns");
        assert_eq!(worksheet.repeat_rows(), Some((0, 1)));
        assert_eq!(worksheet.repeat_columns(), Some((1, 2)));

        worksheet.set_repeat_rows(3, 4).expect("new rows");
        assert_eq!(worksheet.repeat_rows(), Some((3, 4)));
        assert_eq!(worksheet.repeat_columns(), Some((1, 2)));

        assert!(matches!(
            worksheet.set_repeat_rows(2, 1),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.set_repeat_columns(0, MAX_COL + 1),
            Err(Error::RowColumnLimit)
        ));
        assert_eq!(worksheet.repeat_rows(), Some((3, 4)));
        assert_eq!(worksheet.repeat_columns(), Some((1, 2)));
    }

    #[test]
    fn hyperlinks_validate_before_storing() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());

        assert!(matches!(
            worksheet.write_url(0, 0, "not a url"),
            Err(Error::UnknownUrlType(_))
        ));
        assert!(worksheet.cells().is_empty());
        assert!(worksheet.hyperlinks().is_empty());

        assert!(matches!(
            worksheet.write_url(MAX_ROW + 1, 0, "https://example.com"),
            Err(Error::RowColumnLimit)
        ));
        assert!(worksheet.cells().is_empty());
        assert!(worksheet.hyperlinks().is_empty());
    }

    #[test]
    fn hyperlink_variants_store_text_tooltip_and_format() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .write_url_with_options(0, 0, "https://example.com", "Example", "Open example", None)
            .expect("url");
        let link = worksheet.hyperlinks().get(&(0, 0)).expect("hyperlink");
        assert_eq!(link.text, "Example");
        assert_eq!(link.tooltip, "Open example");
        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::String("Example".to_owned()))
        );

        let format = worksheet
            .cells()
            .get(&0)
            .and_then(|row| row.get(&0))
            .and_then(|cell| cell.format)
            .expect("default hyperlink format");
        let mut registry = StyleRegistry::new();
        let style = worksheet.formats()[format as usize]
            .to_cell_style(&mut registry)
            .expect("style");
        assert_eq!(style.font.uls, 1);
        assert_eq!(
            style.font.brtColor,
            Color::RGB(0x0005_63C1)
                .to_brt_color()
                .expect("hyperlink color")
        );

        worksheet
            .write_url_with_format(1, 0, "https://example.com", &Format::new().set_bold())
            .expect("formatted url");
        let format = worksheet
            .cells()
            .get(&1)
            .and_then(|row| row.get(&0))
            .and_then(|cell| cell.format)
            .expect("formatted hyperlink");
        assert_eq!(
            worksheet.formats()[format as usize],
            Format::new().set_bold()
        );
    }

    #[test]
    fn default_dimensions_convert_pixels() {
        let mut worksheet = Worksheet::new();
        worksheet.initialize_default_format(&Format::new(), 24, 8.0);

        assert_eq!(worksheet.default_row_height(), Some(18.0));
        assert_eq!(worksheet.default_col_width(), Some(8.0));
    }

    #[test]
    fn worksheet_default_format_sets_dimensions_and_hyperlink_style() {
        let mut worksheet = Worksheet::new();
        worksheet
            .write(0, 0, "unformatted data")
            .expect("unformatted write");
        worksheet
            .set_default_format(&Format::new().set_bold(), 24, 80)
            .expect("sets default format");

        assert_eq!(worksheet.default_row_height(), Some(18.0));
        assert_eq!(worksheet.default_col_width(), Some(73.0 / 9.0));

        worksheet
            .write_url(1, 0, "https://example.com")
            .expect("writes url");
        let cell = &worksheet.cells()[&1][&0];
        let style = &worksheet.formats()[usize::try_from(cell.format.expect("format")).unwrap()];
        assert_eq!(style, &Format::new().set_bold().set_hyperlink());
    }

    #[test]
    fn worksheet_default_format_must_precede_formats() {
        let mut worksheet = Worksheet::new();
        worksheet
            .write_with_format(0, 0, "x", &Format::new().set_bold())
            .expect("formatted write");

        assert!(matches!(
            worksheet.set_default_format(&Format::new(), 24, 80),
            Err(Error::Parameter(_))
        ));
    }

    #[test]
    fn worksheet_default_format_validates_dimensions() {
        let mut worksheet = Worksheet::new();

        assert!(matches!(
            worksheet.set_default_format(&Format::new(), 24, 65),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.set_default_format(&Format::new(), 547, 80),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn autofit_sizes_columns_from_cell_data() {
        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "Hello").expect("string");
        worksheet.write(0, 1, 123_456).expect("integer");
        worksheet.write(0, 2, true).expect("boolean");
        worksheet.write(1, 0, "Hello World").expect("longer string");
        worksheet.write(2, 0, "short").expect("shorter string");

        worksheet.autofit();

        assert_eq!(worksheet.column_options()[&0].width, Some(76.0 / 7.0));
        assert_eq!(worksheet.column_options()[&1].width, Some(44.0 / 7.0));
        assert_eq!(worksheet.column_options()[&2].width, Some(33.0 / 7.0));
        assert!(worksheet.column_options()[&0].best_fit);
    }

    #[test]
    fn autofit_preserves_wider_manual_widths_and_widens_narrow_ones() {
        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "Hello").expect("cell");
        worksheet.set_column_width(0, 20).expect("manual");
        worksheet.write(0, 1, "Hello").expect("cell");
        worksheet.set_column_width(1, 2).expect("narrow");

        worksheet.autofit();

        assert_eq!(worksheet.column_options()[&0].width, Some(20.0));
        assert_eq!(worksheet.column_options()[&1].width, Some(5.0));
        assert!(!worksheet.column_options()[&0].best_fit);
        assert!(worksheet.column_options()[&1].best_fit);
    }

    #[test]
    fn autofit_honors_row_and_width_limits() {
        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "short").expect("cell");
        worksheet.write(10, 0, "Much longer text").expect("cell");
        worksheet.set_autofit_max_row(10);
        worksheet.autofit();
        assert_eq!(worksheet.column_options()[&0].width, Some(34.0 / 7.0));

        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "MMMMMMMMMMMMMMMMMMMM").expect("cell");
        worksheet.set_autofit_max_width(100);
        worksheet.autofit();
        assert_eq!(worksheet.column_options()[&0].width, Some(95.0 / 7.0));
    }

    #[test]
    fn autofit_handles_multiline_autofilter_and_other_types() {
        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "short\nHello World").expect("cell");
        worksheet.write(0, 1, false).expect("boolean");
        worksheet
            .write(0, 2, chrono::NaiveDate::from_ymd_opt(2026, 11, 18).unwrap())
            .expect("date");
        worksheet
            .write(0, 3, crate::ExcelDateTime::from_ymd(2026, 11, 18).unwrap())
            .expect("native date");
        worksheet.autofit();
        assert_eq!(worksheet.column_options()[&0].width, Some(76.0 / 7.0));
        assert_eq!(worksheet.column_options()[&1].width, Some(38.0 / 7.0));
        assert_eq!(worksheet.column_options()[&2].width, Some(10.0));
        assert_eq!(worksheet.column_options()[&3].width, Some(10.0));

        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "A").expect("header");
        worksheet.autofilter(0, 0, 0, 0).expect("autofilter");
        worksheet.autofit();
        assert_eq!(worksheet.column_options()[&0].width, Some(27.0 / 7.0));
    }

    #[test]
    fn autofit_handles_float_and_integer_extremes() {
        let mut worksheet = Worksheet::new();
        let number = 1.234_567_890_123_456_7_f64;
        worksheet.write(0, 0, number).expect("number");
        worksheet.write(0, 1, i64::MIN).expect("integer");
        worksheet.write(0, 2, -123).expect("negative integer");
        worksheet.autofit();

        let digits = u32::try_from(number.to_string().len()).unwrap();
        assert_eq!(
            worksheet.column_options()[&0].width,
            Some(pixels_to_width(7 * digits + 7))
        );
        assert_eq!(
            worksheet.column_options()[&1].width,
            Some(pixels_to_width(147))
        );
        assert_eq!(
            worksheet.column_options()[&2].width,
            Some(pixels_to_width(35))
        );
    }

    #[test]
    fn autofit_ignores_blank_columns() {
        let mut worksheet = Worksheet::new();
        worksheet
            .set_cell_format(0, 0, &Format::new().set_bold())
            .expect("blank");
        worksheet.autofit();
        assert!(!worksheet.column_options().contains_key(&0));

        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "Hello").expect("cell");
        worksheet.set_autofit_max_width(0);
        worksheet.autofit();
        assert!(!worksheet.column_options().contains_key(&0));
    }

    #[test]
    #[allow(deprecated)]
    fn autofit_to_max_width_delegates_to_limit_and_autofit() {
        let mut worksheet = Worksheet::new();
        worksheet.write(0, 0, "MMMMMMMMMMMMMMMMMMMM").expect("cell");
        worksheet.autofit_to_max_width(100);
        assert_eq!(worksheet.column_options()[&0].width, Some(95.0 / 7.0));
    }

    #[test]
    fn default_hyperlink_style_keeps_workbook_default_format() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.initialize_default_format(&Format::new().set_bold(), 24, 8.0);
        worksheet
            .write_url(0, 0, "https://example.com")
            .expect("writes url");

        let cell = &worksheet.cells()[&0][&0];
        let style = &worksheet.formats()[usize::try_from(cell.format.expect("format")).unwrap()];
        assert_eq!(style, &Format::new().set_bold().set_hyperlink());
    }

    #[test]
    fn default_row_height_validates_and_converts_pixels() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.set_default_row_height(20.0).expect("height");
        assert_eq!(worksheet.default_row_height(), Some(20.0));

        worksheet.set_default_row_height_pixels(32).expect("pixels");
        assert_eq!(worksheet.default_row_height(), Some(24.0));

        assert!(matches!(
            worksheet.set_default_row_height(409.6),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn unprotected_ranges_get_default_names_and_password_hashes() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.unprotect_range(0, 0, 1, 1).expect("first range");
        worksheet
            .unprotect_range_with_options(2, 0, 2, 1, "Editable", "abc123")
            .expect("named range");

        assert_eq!(worksheet.unprotected_ranges().len(), 2);
        assert_eq!(worksheet.unprotected_ranges()[0].name, "Range1");
        assert_eq!(worksheet.unprotected_ranges()[0].password_hash, 0);
        assert_eq!(worksheet.unprotected_ranges()[1].name, "Editable");
        assert_eq!(worksheet.unprotected_ranges()[1].password_hash, 0xC58F);
        assert_eq!(
            worksheet.unprotected_ranges()[1].range,
            UncheckedRfX {
                rwFirst: 2,
                rwLast: 2,
                colFirst: 0,
                colLast: 1,
            }
        );
    }

    #[test]
    fn unprotected_ranges_validate_limits_order_and_name_length() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.unprotect_range(MAX_ROW + 1, 0, 0, 0),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.unprotect_range(2, 0, 1, 0),
            Err(Error::RowColumnOrder)
        ));
        let name = "x".repeat(256);
        assert!(matches!(
            worksheet.unprotect_range_with_options(0, 0, 0, 0, &name, ""),
            Err(Error::Parameter(_))
        ));
        assert_eq!(worksheet.unprotected_ranges().len(), 0);
    }

    #[test]
    fn ignored_errors_group_ranges_and_reject_exact_duplicates() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .ignore_error_range(0, 0, 1, 1, IgnoreError::NumberStoredAsText)
            .expect("first");
        worksheet
            .ignore_error(2, 2, IgnoreError::NumberStoredAsText)
            .expect("second");
        worksheet
            .ignore_error(3, 3, IgnoreError::FormulaError)
            .expect("formula");

        assert_eq!(worksheet.ignored_errors().len(), 2);
        assert_eq!(
            worksheet
                .ignored_errors()
                .get(&IgnoreError::NumberStoredAsText)
                .map(Vec::len),
            Some(2)
        );
        assert!(matches!(
            worksheet.ignore_error_range(0, 0, 1, 1, IgnoreError::FormulaError),
            Err(Error::Parameter(_))
        ));
        assert_eq!(
            worksheet
                .ignored_errors()
                .get(&IgnoreError::FormulaError)
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn ignored_errors_validate_range_limits_and_order() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.ignore_error(0, MAX_COL + 1, IgnoreError::FormulaError),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.ignore_error_range(1, 0, 0, 0, IgnoreError::FormulaError),
            Err(Error::RowColumnOrder)
        ));
        assert!(worksheet.ignored_errors().is_empty());
    }

    #[test]
    fn data_validations_store_ranges_and_clone_input() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let validation =
            DataValidation::new().allow_whole_number(crate::DataValidationRule::Between(1, 10));
        worksheet
            .add_data_validation(1, 2, 3, 4, &validation)
            .expect("validation");
        worksheet
            .add_data_validation(5, 0, 5, 0, &validation)
            .expect("second validation");
        worksheet
            .add_data_validation(5, 0, 5, 0, &DataValidation::new().allow_any_value())
            .expect("empty any is ignored");

        assert_eq!(worksheet.data_validations().len(), 2);
        assert_eq!(
            worksheet.data_validations()[0].ranges.as_slice()[0],
            UncheckedRfX {
                rwFirst: 1,
                rwLast: 3,
                colFirst: 2,
                colLast: 4,
            }
        );

        worksheet
            .add_data_validation(
                1,
                2,
                3,
                4,
                &DataValidation::new()
                    .allow_list_strings(&["x"])
                    .expect("list"),
            )
            .expect("replaces same range");
        assert_eq!(worksheet.data_validations().len(), 2);
        assert_eq!(
            worksheet.data_validations()[0]
                .validation
                .record_ranges(worksheet.data_validations()[0].ranges.as_slice())
                .expect("record")
                .val_type(),
            3
        );
    }

    #[test]
    fn data_validations_store_multi_ranges_and_replace_matching_lists() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let first = DataValidation::new()
            .allow_whole_number(crate::DataValidationRule::GreaterThan(0))
            .set_multi_range("B3:D6 I3:K6");
        worksheet
            .add_data_validation(0, 0, 0, 0, &first)
            .expect("multi-range validation");
        assert_eq!(
            worksheet.data_validations()[0].ranges.as_slice(),
            &[
                UncheckedRfX {
                    rwFirst: 2,
                    rwLast: 5,
                    colFirst: 1,
                    colLast: 3,
                },
                UncheckedRfX {
                    rwFirst: 2,
                    rwLast: 5,
                    colFirst: 8,
                    colLast: 10,
                },
            ]
        );

        let replacement = DataValidation::new()
            .allow_whole_number(crate::DataValidationRule::LessThan(10))
            .set_multi_range("B3:D6 I3:K6");
        worksheet
            .add_data_validation(10, 10, 10, 10, &replacement)
            .expect("replacement");
        assert_eq!(worksheet.data_validations().len(), 1);

        let before = worksheet.data_validations().len();
        assert!(
            worksheet
                .add_data_validation(
                    0,
                    0,
                    0,
                    0,
                    &DataValidation::new()
                        .allow_whole_number(crate::DataValidationRule::GreaterThan(0))
                        .set_multi_range("B2:A1")
                )
                .is_err()
        );
        assert_eq!(worksheet.data_validations().len(), before);
    }

    #[test]
    fn data_validations_validate_range_limits_and_order() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let validation = DataValidation::new().allow_any_value();
        assert!(matches!(
            worksheet.add_data_validation(MAX_ROW + 1, 0, MAX_ROW + 1, 0, &validation),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            worksheet.add_data_validation(1, 0, 0, 0, &validation),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.add_data_validation(0, 0, 0, MAX_COL + 1, &validation),
            Err(Error::RowColumnLimit)
        ));
        assert!(worksheet.data_validations().is_empty());
    }

    #[test]
    fn add_conditional_format_stores_and_validates_rules() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let rule = crate::ConditionalFormatCell::new()
            .set_rule(crate::ConditionalFormatCellRule::GreaterThanOrEqualTo(50));
        worksheet
            .add_conditional_format(1, 2, 4, 3, &rule)
            .expect("conditional format");
        assert_eq!(worksheet.conditional_formats().len(), 1);
        let stored = &worksheet.conditional_formats()[0];
        let range = stored.ranges.as_slice()[0];
        assert_eq!(range.rwFirst, 1);
        assert_eq!(range.rwLast, 4);
        assert_eq!(range.colFirst, 2);
        assert_eq!(range.colLast, 3);
        assert_eq!(stored.begin_rule(1, u32::MAX).expect("rule").iParam, 7);

        assert!(matches!(
            worksheet.add_conditional_format(0, 0, 1, 0, &crate::ConditionalFormatCell::new()),
            Err(Error::Parameter(_))
        ));
        let bad_number = crate::ConditionalFormatCell::new()
            .set_rule(crate::ConditionalFormatCellRule::EqualTo(f64::NAN));
        assert!(matches!(
            worksheet.add_conditional_format(0, 0, 1, 0, &bad_number),
            Err(Error::Parameter(_))
        ));
        let empty = crate::ConditionalFormatCell::new()
            .set_rule(crate::ConditionalFormatCellRule::EqualTo(""));
        assert!(matches!(
            worksheet.add_conditional_format(0, 0, 1, 0, &empty),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.add_conditional_format(2, 0, 1, 0, &rule),
            Err(Error::RowColumnOrder)
        ));
    }

    #[test]
    fn conditional_formats_store_multi_ranges_and_use_the_first_for_formulas() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let rule = crate::ConditionalFormatFormula::new()
            .set_rule("=A1")
            .set_multi_range("$I$3:$K$6,$B$9:$D$12");
        worksheet
            .add_conditional_format(0, 0, 0, 0, &rule)
            .expect("multi-range conditional format");

        let stored = &worksheet.conditional_formats()[0];
        assert_eq!(stored.ranges.as_slice().len(), 2);
        assert_eq!(stored.ranges.as_slice()[0].rwFirst, 2);
        assert_eq!(stored.ranges.as_slice()[0].colFirst, 8);
        assert_eq!(stored.ranges.as_slice()[1].rwFirst, 8);
        assert_eq!(stored.ranges.as_slice()[1].colFirst, 1);

        let record = stored.begin_rule(1, u32::MAX).expect("rule");
        let formula = record.formula1.expect("formula");
        let expected = crate::formula::compile_relative("=A1", 2, 1).expect("compiled");
        assert_eq!(formula.rgce, expected.rgce);

        let before = worksheet.conditional_formats().len();
        assert!(
            worksheet
                .add_conditional_format(0, 0, 0, 0, &rule.set_multi_range("B2:A1"))
                .is_err()
        );
        assert_eq!(worksheet.conditional_formats().len(), before);
    }

    #[test]
    fn add_table_writes_headers_and_uses_existing_header_cells() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.write_string(1, 0, "Existing").expect("header");
        worksheet
            .write_rich_string(1, 2, &[(&Format::new().set_bold(), "Rich")])
            .expect("rich header");
        let table = Table::new();
        worksheet.add_table(1, 0, 3, 2, &table).expect("table");

        assert_eq!(
            worksheet.cells()[&1][&0].data,
            CellData::String("Existing".to_owned())
        );
        assert_eq!(
            worksheet.cells()[&1][&1].data,
            CellData::String("Column2".to_owned())
        );
        assert_eq!(
            worksheet.cells()[&1][&2].data,
            CellData::String("Rich".to_owned())
        );
        assert_eq!(worksheet.tables()[0].table.columns[1].header, "Column2");
        assert_eq!(worksheet.tables()[0].table.columns[2].header, "Rich");
    }

    #[test]
    fn add_table_fills_missing_columns_by_position() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let table = Table::new().set_columns(&[TableColumn::new().set_header("Only")]);
        worksheet.add_table(0, 0, 2, 1, &table).expect("table");
        assert_eq!(worksheet.tables()[0].table.columns[0].header, "Only");
        assert_eq!(worksheet.tables()[0].table.columns[1].header, "Column2");
    }

    #[test]
    fn add_table_validates_and_translates_filter_columns() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let filter = FilterCondition::new().add_list_filter("East");
        assert!(matches!(
            worksheet.add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_header_row(false).filter_column(0, &filter)
            ),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_autofilter(false).filter_column(0, &filter)
            ),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.add_table(0, 0, 2, 1, &Table::new().filter_column(2, &filter)),
            Err(Error::Parameter(_))
        ));
        assert!(matches!(
            worksheet.add_table(
                0,
                0,
                2,
                1,
                &Table::new().filter_column(0, &FilterCondition::new())
            ),
            Err(Error::Parameter(_))
        ));
        let long = "x".repeat(256);
        assert!(matches!(
            worksheet.add_table(
                0,
                0,
                2,
                1,
                &Table::new()
                    .filter_column(0, &FilterCondition::new().add_list_filter(long.as_str()))
            ),
            Err(Error::Parameter(_))
        ));
        assert!(worksheet.tables().is_empty());

        let table = Table::new().filter_column(1, &filter).set_columns(&[
            TableColumn::new().set_header("Region"),
            TableColumn::new().set_header("Sales"),
        ]);
        worksheet.add_table(0, 3, 2, 4, &table).expect("table");
        assert_eq!(
            worksheet.tables()[0]
                .table
                .filter_conditions
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [4]
        );
    }

    #[test]
    fn add_table_uses_custom_columns_and_skips_headers_when_disabled() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let table = Table::new()
            .set_columns(&[
                TableColumn::new().set_header("Name"),
                TableColumn::new().set_header("Value"),
            ])
            .set_header_row(false);
        worksheet.add_table(2, 0, 4, 1, &table).expect("table");
        assert!(!worksheet.cells().contains_key(&2));
        assert_eq!(worksheet.tables()[0].table.columns[0].header, "Name");
    }

    #[test]
    fn add_table_writes_total_labels_subtotals_and_formats() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let data_format = Format::new().set_bold();
        let table = Table::new().set_total_row(true).set_columns(&[
            TableColumn::new()
                .set_header("Name")
                .set_total_label("Totals"),
            TableColumn::new()
                .set_header("Q1")
                .set_total_function(TableFunction::Sum)
                .set_format(&data_format),
        ]);
        worksheet.add_table(0, 0, 4, 1, &table).expect("table");

        assert_eq!(
            worksheet.cells()[&4][&0].data,
            CellData::String("Totals".to_owned())
        );
        let total = &worksheet.cells()[&4][&1];
        let CellData::Formula(formula) = &total.data else {
            panic!("expected total formula");
        };
        assert_eq!(formula.formula, subtotal_formula(109, 1, 3, 1));
        assert_eq!(formula.result, crate::data::FormulaResult::Number(0.0));
        assert!(total.format.is_some());
        for row in 1..=3 {
            assert_eq!(worksheet.cells()[&row][&1].format, total.format);
        }
        assert!(worksheet.tables()[0].table.show_total_row);
        assert_eq!(worksheet.tables()[0].table.columns[0].total_label, "Totals");
        assert_eq!(
            worksheet.tables()[0].table.columns[1].total_function,
            TableFunction::Sum
        );
    }

    #[test]
    fn add_table_writes_header_formats_and_custom_totals() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        let header_format = Format::new().set_bold();
        let custom = Formula::new("=SUM(A2:A3)").set_result("6");
        let table = Table::new()
            .set_total_row(true)
            .set_columns(&[TableColumn::new()
                .set_header("Value")
                .set_header_format(&header_format)
                .set_total_function(TableFunction::Custom(custom))]);
        worksheet.add_table(0, 0, 3, 0, &table).expect("table");

        assert!(worksheet.cells()[&0][&0].format.is_some());
        let total = &worksheet.cells()[&3][&0];
        let CellData::Formula(formula) = &total.data else {
            panic!("expected custom total formula");
        };
        assert_eq!(formula.formula, compile("=SUM(A2:A3)").expect("compiled"));
        assert_eq!(formula.result, crate::data::FormulaResult::Number(6.0));
        assert!(
            worksheet.tables()[0].table.columns[0]
                .total_formula
                .is_some()
        );
        assert_eq!(
            worksheet.tables()[0].table.columns[0]
                .total_result
                .as_deref(),
            Some("6")
        );
    }

    #[test]
    fn add_table_rejects_total_rows_without_data_rows_and_long_labels() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.add_table(0, 0, 0, 0, &Table::new().set_total_row(true)),
            Err(Error::TableError(_))
        ));
        assert!(matches!(
            worksheet.add_table(0, 0, 1, 0, &Table::new().set_total_row(true)),
            Err(Error::TableError(_))
        ));
        let long_label = Table::new()
            .set_total_row(true)
            .set_columns(&[TableColumn::new().set_total_label("x".repeat(8_190))]);
        assert!(matches!(
            worksheet.add_table(0, 0, 2, 0, &long_label),
            Err(Error::TableError(_))
        ));
        let bad_formula = Table::new()
            .set_total_row(true)
            .set_columns(&[TableColumn::new()
                .set_total_function(TableFunction::Custom(Formula::new("=Sheet2!A1")))]);
        assert!(matches!(
            worksheet.add_table(0, 0, 2, 0, &bad_formula),
            Err(Error::FormulaError(_))
        ));
        assert!(worksheet.tables().is_empty());
        assert!(worksheet.cells().is_empty());
    }

    #[test]
    fn add_table_rejects_invalid_shapes_headers_names_and_overlaps() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        assert!(matches!(
            worksheet.add_table(0, 0, 0, 0, &Table::new()),
            Err(Error::TableError(_))
        ));
        assert!(matches!(
            worksheet.add_table(1, 1, 0, 0, &Table::new()),
            Err(Error::RowColumnOrder)
        ));
        assert!(matches!(
            worksheet.add_table(0, 0, 1, 0, &Table::new().set_name("_xlnm.Bad")),
            Err(Error::TableError(_))
        ));
        assert!(matches!(
            worksheet.add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_columns(&[
                    TableColumn::new().set_header("Name"),
                    TableColumn::new().set_header("name"),
                ])
            ),
            Err(Error::TableError(_))
        ));

        worksheet
            .add_table(0, 0, 2, 1, &Table::new())
            .expect("table");
        assert!(matches!(
            worksheet.add_table(1, 1, 3, 2, &Table::new()),
            Err(Error::TableRangeOverlaps)
        ));
    }

    #[test]
    fn table_and_worksheet_autofilters_cannot_overlap() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet.autofilter(0, 0, 3, 1).expect("autofilter");
        assert!(matches!(
            worksheet.add_table(1, 1, 4, 2, &Table::new()),
            Err(Error::AutofilterRangeOverlaps)
        ));

        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .add_table(0, 0, 3, 1, &Table::new())
            .expect("table");
        assert!(matches!(
            worksheet.autofilter(1, 1, 4, 2),
            Err(Error::AutofilterRangeOverlaps)
        ));

        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .add_table(0, 0, 3, 1, &Table::new().set_autofilter(false))
            .expect("table");
        worksheet
            .autofilter(1, 1, 4, 2)
            .expect("filter-less table does not block");
    }

    #[test]
    fn protected_range_names_are_unique_and_defaults_skip_collisions() {
        let mut worksheet = Worksheet::new_named("Sheet1".to_owned());
        worksheet
            .unprotect_range_with_options(0, 0, 0, 0, "Range2", "")
            .expect("explicit");
        worksheet
            .unprotect_range(1, 0, 1, 0)
            .expect("default skips Range2");
        assert_eq!(worksheet.unprotected_ranges()[1].name, "Range3");

        assert!(matches!(
            worksheet.unprotect_range_with_options(2, 0, 2, 0, "Range3", ""),
            Err(Error::Parameter(_))
        ));
        assert_eq!(worksheet.unprotected_ranges().len(), 2);
    }
}
