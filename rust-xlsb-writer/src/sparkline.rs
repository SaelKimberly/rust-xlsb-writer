//! Sparkline values and grouped range expansion.

use std::fmt;
use std::sync::Arc;

use crate::{Color, Error, Result};

/// The kind of sparkline to draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SparklineType {
    /// A line sparkline.
    #[default]
    Line,
    /// A column sparkline.
    Column,
    /// A positive/negative win/loss sparkline.
    WinLose,
}

/// How empty cells are plotted in a sparkline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartEmptyCells {
    /// Plot empty cells as gaps.
    #[default]
    Gaps,
    /// Plot empty cells as zero.
    Zero,
    /// Connect empty cells to the previous point.
    Connected,
}

impl ChartEmptyCells {
    pub(crate) const fn record_value(self) -> u8 {
        match self {
            Self::Gaps => 1,
            Self::Zero => 0,
            Self::Connected => 2,
        }
    }
}

impl fmt::Display for ChartEmptyCells {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Gaps => "gap",
            Self::Zero => "zero",
            Self::Connected => "span",
        })
    }
}

/// A sparkline data range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparklineRange {
    pub(crate) sheet: Option<Arc<str>>,
    pub(crate) first_row: u32,
    pub(crate) first_col: u16,
    pub(crate) last_row: u32,
    pub(crate) last_col: u16,
}

impl SparklineRange {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.first_row > self.last_row || self.first_col > self.last_col {
            return Err(Error::SparklineError(
                "sparkline range start is after its end".to_owned(),
            ));
        }
        if self.last_row > crate::worksheet::MAX_ROW || self.last_col > crate::worksheet::MAX_COL {
            return Err(Error::SparklineError(
                "sparkline range is outside the worksheet".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) const fn is_1d(&self) -> bool {
        self.first_row == self.last_row || self.first_col == self.last_col
    }

    pub(crate) const fn is_2d(&self) -> bool {
        self.first_row != self.last_row && self.first_col != self.last_col
    }

    pub(crate) const fn rows(&self) -> u32 {
        self.last_row - self.first_row + 1
    }

    pub(crate) const fn cols(&self) -> u32 {
        (self.last_col - self.first_col) as u32 + 1
    }
}

impl From<(&str, u32, u16, u32, u16)> for SparklineRange {
    fn from((sheet, first_row, first_col, last_row, last_col): (&str, u32, u16, u32, u16)) -> Self {
        Self {
            sheet: Some(Arc::from(sheet)),
            first_row,
            first_col,
            last_row,
            last_col,
        }
    }
}

impl From<(u32, u16, u32, u16)> for SparklineRange {
    fn from((first_row, first_col, last_row, last_col): (u32, u16, u32, u16)) -> Self {
        Self {
            sheet: None,
            first_row,
            first_col,
            last_row,
            last_col,
        }
    }
}

/// A sparkline configuration.
///
/// The data range is set with [`Self::set_range`]. A single sparkline uses a
/// one-dimensional range; [`Worksheet::add_sparkline_group`](crate::Worksheet::add_sparkline_group)
/// uses a two-dimensional range whose row or column count matches the target
/// range.
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "sparkline options are independent public switches"
)]
pub struct Sparkline {
    pub(crate) range: Option<SparklineRange>,
    pub(crate) kind: SparklineType,
    pub(crate) series_color: Color,
    pub(crate) negative_points_color: Color,
    pub(crate) axis_color: Color,
    pub(crate) markers_color: Color,
    pub(crate) first_point_color: Color,
    pub(crate) last_point_color: Color,
    pub(crate) high_point_color: Color,
    pub(crate) low_point_color: Color,
    pub(crate) show_high_point: bool,
    pub(crate) show_low_point: bool,
    pub(crate) show_first_point: bool,
    pub(crate) show_last_point: bool,
    pub(crate) show_negative_points: bool,
    pub(crate) show_markers: bool,
    pub(crate) show_axis: bool,
    pub(crate) show_hidden_data: bool,
    pub(crate) show_right_to_left: bool,
    pub(crate) show_empty_cells_as: ChartEmptyCells,
    pub(crate) line_weight: Option<f64>,
    pub(crate) custom_min: Option<f64>,
    pub(crate) custom_max: Option<f64>,
    pub(crate) group_max: bool,
    pub(crate) group_min: bool,
}

impl Sparkline {
    /// Creates a default line sparkline.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            range: None,
            kind: SparklineType::Line,
            series_color: Color::Theme(4, 5),
            negative_points_color: Color::Theme(5, 0),
            axis_color: Color::Black,
            markers_color: Color::Theme(4, 5),
            first_point_color: Color::Theme(4, 3),
            last_point_color: Color::Theme(4, 3),
            high_point_color: Color::Theme(4, 0),
            low_point_color: Color::Theme(4, 0),
            show_high_point: false,
            show_low_point: false,
            show_first_point: false,
            show_last_point: false,
            show_negative_points: false,
            show_markers: false,
            show_axis: false,
            show_hidden_data: false,
            show_right_to_left: false,
            show_empty_cells_as: ChartEmptyCells::Gaps,
            line_weight: None,
            custom_min: None,
            custom_max: None,
            group_max: false,
            group_min: false,
        }
    }

    /// Sets the data range to plot.
    #[must_use]
    pub fn set_range(mut self, range: impl Into<SparklineRange>) -> Self {
        self.range = Some(range.into());
        self
    }

    /// Sets the sparkline type.
    #[must_use]
    pub const fn set_type(mut self, sparkline_type: SparklineType) -> Self {
        self.kind = sparkline_type;
        self
    }

    /// Shows the highest data point.
    #[must_use]
    pub const fn show_high_point(mut self, enable: bool) -> Self {
        self.show_high_point = enable;
        self
    }

    /// Shows the lowest data point.
    #[must_use]
    pub const fn show_low_point(mut self, enable: bool) -> Self {
        self.show_low_point = enable;
        self
    }

    /// Shows the first data point.
    #[must_use]
    pub const fn show_first_point(mut self, enable: bool) -> Self {
        self.show_first_point = enable;
        self
    }

    /// Shows the last data point.
    #[must_use]
    pub const fn show_last_point(mut self, enable: bool) -> Self {
        self.show_last_point = enable;
        self
    }

    /// Highlights negative data points.
    #[must_use]
    pub const fn show_negative_points(mut self, enable: bool) -> Self {
        self.show_negative_points = enable;
        self
    }

    /// Shows markers on a line sparkline.
    #[must_use]
    pub const fn show_markers(mut self, enable: bool) -> Self {
        self.show_markers = enable;
        self
    }

    /// Shows the horizontal axis.
    #[must_use]
    pub const fn show_axis(mut self, enable: bool) -> Self {
        self.show_axis = enable;
        self
    }

    /// Plots data from hidden cells.
    #[must_use]
    pub const fn show_hidden_data(mut self, enable: bool) -> Self {
        self.show_hidden_data = enable;
        self
    }

    /// Sets how empty cells are plotted.
    #[must_use]
    pub const fn show_empty_cells_as(mut self, option: ChartEmptyCells) -> Self {
        self.show_empty_cells_as = option;
        self
    }

    /// Reverses the sparkline direction.
    #[must_use]
    pub const fn set_right_to_left(mut self, enable: bool) -> Self {
        self.show_right_to_left = enable;
        self
    }

    /// Sets the series color.
    #[must_use]
    pub fn set_sparkline_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.series_color = color;
        }
        self
    }

    /// Sets the highest-point color and enables the marker.
    #[must_use]
    pub fn set_high_point_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.high_point_color = color;
            self.show_high_point = true;
        }
        self
    }

    /// Sets the lowest-point color and enables the marker.
    #[must_use]
    pub fn set_low_point_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.low_point_color = color;
            self.show_low_point = true;
        }
        self
    }

    /// Sets the first-point color and enables the marker.
    #[must_use]
    pub fn set_first_point_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.first_point_color = color;
            self.show_first_point = true;
        }
        self
    }

    /// Sets the last-point color and enables the marker.
    #[must_use]
    pub fn set_last_point_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.last_point_color = color;
            self.show_last_point = true;
        }
        self
    }

    /// Sets the negative-point color and enables highlighting.
    #[must_use]
    pub fn set_negative_points_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.negative_points_color = color;
            self.show_negative_points = true;
        }
        self
    }

    /// Sets the marker color and enables markers.
    #[must_use]
    pub fn set_markers_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.markers_color = color;
            self.show_markers = true;
        }
        self
    }

    /// Sets the sparkline line weight in points.
    #[must_use]
    pub fn set_line_weight<T: Into<f64>>(mut self, weight: T) -> Self {
        let weight = weight.into();
        if weight.is_finite() {
            self.line_weight = Some(weight.clamp(0.0, 1584.0));
        }
        self
    }

    /// Sets a custom vertical-axis maximum.
    #[must_use]
    pub fn set_custom_max<T: Into<f64>>(mut self, max: T) -> Self {
        self.custom_max = Some(max.into());
        self.group_max = false;
        self
    }

    /// Sets a custom vertical-axis minimum.
    #[must_use]
    pub fn set_custom_min<T: Into<f64>>(mut self, min: T) -> Self {
        self.custom_min = Some(min.into());
        self.group_min = false;
        self
    }

    /// Uses the group maximum for the vertical axis.
    #[must_use]
    pub const fn set_group_max(mut self, enable: bool) -> Self {
        self.group_max = enable;
        if enable {
            self.custom_max = None;
        }
        self
    }

    /// Uses the group minimum for the vertical axis.
    #[must_use]
    pub const fn set_group_min(mut self, enable: bool) -> Self {
        self.group_min = enable;
        if enable {
            self.custom_min = None;
        }
        self
    }
}

impl Default for Sparkline {
    fn default() -> Self {
        Self::new()
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "package and worksheet serialization share these records"
)]
pub(crate) struct SparklineGroup {
    pub(crate) sparkline: Sparkline,
    pub(crate) cells: Vec<SparklineCell>,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "package and worksheet serialization share these records"
)]
pub(crate) struct SparklineCell {
    pub(crate) row: u32,
    pub(crate) col: u16,
    pub(crate) range: SparklineRange,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "package and worksheet serialization share these records"
)]
pub(crate) struct SparklineSaveGroup<'a> {
    pub(crate) sparkline: &'a Sparkline,
    pub(crate) cells: Vec<SparklineSaveCell>,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "package and worksheet serialization share these records"
)]
pub(crate) struct SparklineSaveCell {
    pub(crate) row: u32,
    pub(crate) col: u16,
    pub(crate) rgce: [u8; 15],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Worksheet;

    #[test]
    fn single_sparkline_uses_its_own_cell_and_resolves_the_sheet() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        let sparkline = Sparkline::new().set_range((0, 0, 0, 4));
        sheet.add_sparkline(0, 5, &sparkline).expect("sparkline");
        let group = &sheet.sparkline_groups()[0];
        assert_eq!(group.cells.len(), 1);
        assert_eq!((group.cells[0].row, group.cells[0].col), (0, 5));
        assert_eq!(group.cells[0].range.sheet, None);
        assert_eq!(group.cells[0].range.first_row, 0);
        assert_eq!(group.cells[0].range.last_col, 4);
    }

    #[test]
    fn grouped_sparkline_expands_rows_and_columns() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        let column_group = Sparkline::new().set_range(("Sheet1", 1, 0, 3, 4));
        sheet
            .add_sparkline_group(0, 5, 2, 5, &column_group)
            .expect("column group");
        let group = &sheet.sparkline_groups()[0];
        assert_eq!(group.cells.len(), 3);
        assert_eq!(group.cells[0].range.first_row, 1);
        assert_eq!(group.cells[2].range.first_row, 3);
        assert_eq!(group.cells[0].range.first_col, 0);
        assert_eq!(group.cells[0].range.last_col, 4);

        let row_group = Sparkline::new().set_range(("Sheet1", 0, 0, 2, 2));
        sheet
            .add_sparkline_group(4, 1, 4, 3, &row_group)
            .expect("row group");
        let group = &sheet.sparkline_groups()[1];
        assert_eq!(group.cells.len(), 3);
        assert_eq!(group.cells[0].range.first_col, 0);
        assert_eq!(group.cells[2].range.first_col, 2);
        assert_eq!(group.cells[0].range.first_row, 0);
        assert_eq!(group.cells[0].range.last_row, 2);
    }

    #[test]
    fn sparkline_ranges_and_groups_are_validated() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        let missing = Sparkline::new();
        assert!(matches!(
            sheet.add_sparkline(0, 0, &missing),
            Err(Error::SparklineError(_))
        ));

        let two_d = Sparkline::new().set_range((0, 0, 1, 1));
        assert!(matches!(
            sheet.add_sparkline(0, 5, &two_d),
            Err(Error::SparklineError(_))
        ));

        let one_d = Sparkline::new().set_range((0, 0, 0, 4));
        assert!(matches!(
            sheet.add_sparkline_group(0, 5, 2, 5, &one_d),
            Err(Error::SparklineError(_))
        ));

        let mismatch = Sparkline::new().set_range((0, 0, 3, 4));
        assert!(matches!(
            sheet.add_sparkline_group(0, 5, 2, 5, &mismatch),
            Err(Error::SparklineError(_))
        ));

        assert!(matches!(
            sheet.add_sparkline_group(0, 5, 1, 6, &mismatch),
            Err(Error::SparklineError(_))
        ));

        let clamped = Sparkline::new()
            .set_line_weight(f64::NAN)
            .set_line_weight(2_000.0);
        assert_eq!(clamped.line_weight, Some(1584.0));
        let clamped = Sparkline::new().set_line_weight(-2.0);
        assert_eq!(clamped.line_weight, Some(0.0));
    }
    #[test]
    fn detached_sparkline_uses_the_assigned_sheet_name() {
        let mut workbook = crate::Workbook::new();
        let mut sheet = Worksheet::new();
        let sparkline = Sparkline::new().set_range((0, 0, 0, 4));
        sheet.add_sparkline(0, 5, &sparkline).expect("sparkline");
        workbook.push_worksheet(sheet);
        workbook.save_to_buffer().expect("saves");
    }
}
