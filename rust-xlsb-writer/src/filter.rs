//! Worksheet autofilter conditions and matching.

use crate::data::CellData;

/// Comparison used by a custom autofilter condition.
///
/// String wildcard criteria (`BeginsWith`, `Contains`, and their negations)
/// are encoded as equal/not-equal comparisons with Excel wildcards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterCriteria {
    /// Equal to the filter value.
    EqualTo,
    /// Not equal to the filter value.
    NotEqualTo,
    /// Greater than the filter value.
    GreaterThan,
    /// Greater than or equal to the filter value.
    GreaterThanOrEqualTo,
    /// Less than the filter value.
    LessThan,
    /// Less than or equal to the filter value.
    LessThanOrEqualTo,
    /// Begins with the filter string.
    BeginsWith,
    /// Does not begin with the filter string.
    DoesNotBeginWith,
    /// Ends with the filter string.
    EndsWith,
    /// Does not end with the filter string.
    DoesNotEndWith,
    /// Contains the filter string.
    Contains,
    /// Does not contain the filter string.
    DoesNotContain,
}

impl FilterCriteria {
    pub(crate) const fn operator(self) -> u8 {
        match self {
            Self::EqualTo | Self::BeginsWith | Self::EndsWith | Self::Contains => 0x02,
            Self::NotEqualTo
            | Self::DoesNotBeginWith
            | Self::DoesNotEndWith
            | Self::DoesNotContain => 0x05,
            Self::LessThan => 0x01,
            Self::LessThanOrEqualTo => 0x03,
            Self::GreaterThan => 0x04,
            Self::GreaterThanOrEqualTo => 0x06,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterDataType {
    String,
    Number,
}

/// One value and comparison in a filter condition.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterData {
    pub(crate) data_type: FilterDataType,
    pub(crate) string: String,
    pub(crate) normalized: String,
    pub(crate) number: f64,
    pub(crate) criteria: FilterCriteria,
}

impl FilterData {
    /// Creates string filter data.
    #[must_use]
    pub fn new_string_and_criteria(value: &str, criteria: FilterCriteria) -> Self {
        Self {
            data_type: FilterDataType::String,
            string: value.to_owned(),
            normalized: normalize(value),
            number: 0.0,
            criteria,
        }
    }

    /// Creates numeric filter data.
    #[must_use]
    pub fn new_number_and_criteria(value: f64, criteria: FilterCriteria) -> Self {
        let string = value.to_string();
        Self {
            data_type: FilterDataType::Number,
            normalized: normalize(&string),
            string,
            number: value,
            criteria,
        }
    }

    pub(crate) fn value(&self) -> String {
        let value = &self.string;
        match self.criteria {
            FilterCriteria::EndsWith | FilterCriteria::DoesNotEndWith => format!("*{value}"),
            FilterCriteria::Contains | FilterCriteria::DoesNotContain => format!("*{value}*"),
            FilterCriteria::BeginsWith | FilterCriteria::DoesNotBeginWith => format!("{value}*"),
            _ => value.clone(),
        }
    }

    pub(crate) const fn is_number(&self) -> bool {
        matches!(self.data_type, FilterDataType::Number)
    }
}

/// Converts a value into autofilter data.
pub trait IntoFilterData {
    /// Builds filter data with the requested comparison.
    fn new_filter_data(&self, criteria: FilterCriteria) -> FilterData;
}

impl IntoFilterData for f64 {
    fn new_filter_data(&self, criteria: FilterCriteria) -> FilterData {
        FilterData::new_number_and_criteria(*self, criteria)
    }
}

impl IntoFilterData for i32 {
    fn new_filter_data(&self, criteria: FilterCriteria) -> FilterData {
        FilterData::new_number_and_criteria(f64::from(*self), criteria)
    }
}

impl IntoFilterData for &str {
    fn new_filter_data(&self, criteria: FilterCriteria) -> FilterData {
        FilterData::new_string_and_criteria(self, criteria)
    }
}

/// Filter rules for one autofilter column.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterCondition {
    pub(crate) is_list_filter: bool,
    pub(crate) apply_logical_or: bool,
    pub(crate) should_match_blanks: bool,
    pub(crate) list: Vec<FilterData>,
    pub(crate) custom1: Option<FilterData>,
    pub(crate) custom2: Option<FilterData>,
}

impl Default for FilterCondition {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterCondition {
    /// Creates an empty filter condition.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_list_filter: true,
            apply_logical_or: false,
            should_match_blanks: false,
            list: Vec::new(),
            custom1: None,
            custom2: None,
        }
    }

    /// Adds a list-filter value; repeated values are combined with OR.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "public signature matches rust_xlsxwriter"
    )]
    pub fn add_list_filter<T: IntoFilterData>(mut self, value: T) -> Self {
        self.list
            .push(value.new_filter_data(FilterCriteria::EqualTo));
        self.is_list_filter = true;
        self
    }

    /// Includes blank cells in a list filter.
    #[must_use]
    pub const fn add_list_blanks_filter(mut self) -> Self {
        self.should_match_blanks = true;
        self.is_list_filter = true;
        self
    }

    /// Adds a custom comparison. Excel allows at most two per column.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "public signature matches rust_xlsxwriter"
    )]
    pub fn add_custom_filter<T: IntoFilterData>(
        mut self,
        criteria: FilterCriteria,
        value: T,
    ) -> Self {
        let data = value.new_filter_data(criteria);
        if self.custom1.is_none() {
            self.custom1 = Some(data);
        } else if self.custom2.is_none() {
            self.custom2 = Some(data);
        }
        self.is_list_filter = false;
        self
    }

    /// Combines two custom comparisons with OR instead of AND.
    #[must_use]
    pub const fn add_custom_boolean_or(mut self) -> Self {
        self.apply_logical_or = true;
        self
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.list.is_empty() && self.custom1.is_none() && !self.should_match_blanks
    }

    pub(crate) fn matches(&self, cell: Option<&CellData>) -> bool {
        if self.is_list_filter {
            self.matches_list(cell)
        } else {
            self.matches_custom(cell)
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::float_cmp,
        reason = "Excel filters compare stored IEEE-754 values exactly"
    )]
    fn matches_list(&self, cell: Option<&CellData>) -> bool {
        let Some(cell) = cell else {
            return self.should_match_blanks;
        };
        match cell {
            CellData::String(text) | CellData::RichString(crate::data::RichString { text, .. }) => {
                let key = normalize(text);
                self.list.iter().any(|filter| match filter.criteria {
                    FilterCriteria::EndsWith | FilterCriteria::DoesNotEndWith => {
                        key.ends_with(&filter.normalized)
                    }
                    FilterCriteria::Contains | FilterCriteria::DoesNotContain => {
                        key.contains(&filter.normalized)
                    }
                    FilterCriteria::BeginsWith | FilterCriteria::DoesNotBeginWith => {
                        key.starts_with(&filter.normalized)
                    }
                    _ => key == filter.normalized,
                }) || (self.should_match_blanks && key.is_empty())
            }
            CellData::Integer(value) => {
                let value = *value as f64;
                self.list
                    .iter()
                    .any(|filter| filter.is_number() && value == filter.number)
            }
            CellData::Number(value) => self
                .list
                .iter()
                .any(|filter| filter.is_number() && *value == filter.number),
            CellData::Blank => self.should_match_blanks,
            _ => false,
        }
    }

    fn matches_custom(&self, cell: Option<&CellData>) -> bool {
        let first = self.custom1.as_ref().is_some_and(|f| f.matches(cell));
        let Some(second) = self.custom2.as_ref() else {
            return first;
        };
        let second = second.matches(cell);
        if self.apply_logical_or {
            first || second
        } else {
            first && second
        }
    }
}

impl FilterData {
    #[expect(
        clippy::cast_precision_loss,
        reason = "Excel filters compare stored IEEE-754 values exactly"
    )]
    fn matches(&self, cell: Option<&CellData>) -> bool {
        let Some(cell) = cell else {
            return false;
        };
        match cell {
            CellData::String(text) | CellData::RichString(crate::data::RichString { text, .. }) => {
                let key = normalize(text);
                match self.criteria {
                    FilterCriteria::EqualTo => key == self.normalized,
                    FilterCriteria::NotEqualTo => key != self.normalized,
                    FilterCriteria::LessThan => key < self.normalized,
                    FilterCriteria::LessThanOrEqualTo => key <= self.normalized,
                    FilterCriteria::GreaterThan => key > self.normalized,
                    FilterCriteria::GreaterThanOrEqualTo => key >= self.normalized,
                    FilterCriteria::EndsWith => key.ends_with(&self.normalized),
                    FilterCriteria::DoesNotEndWith => !key.ends_with(&self.normalized),
                    FilterCriteria::BeginsWith => key.starts_with(&self.normalized),
                    FilterCriteria::DoesNotBeginWith => !key.starts_with(&self.normalized),
                    FilterCriteria::Contains => key.contains(&self.normalized),
                    FilterCriteria::DoesNotContain => !key.contains(&self.normalized),
                }
            }
            CellData::Integer(value) if self.is_number() => {
                compare_number(*value as f64, self.number, self.criteria)
            }
            CellData::Number(value) if self.is_number() => {
                compare_number(*value, self.number, self.criteria)
            }
            CellData::Blank => false,
            _ => self.is_non_blank_filter(),
        }
    }

    fn is_non_blank_filter(&self) -> bool {
        self.criteria == FilterCriteria::NotEqualTo && self.string == " "
    }
}

fn normalize(text: &str) -> String {
    text.trim().to_lowercase()
}

#[expect(
    clippy::float_cmp,
    reason = "Excel filters compare stored IEEE-754 values exactly"
)]
fn compare_number(value: f64, target: f64, criteria: FilterCriteria) -> bool {
    match criteria {
        FilterCriteria::EqualTo => value == target,
        FilterCriteria::NotEqualTo => value != target,
        FilterCriteria::LessThan => value < target,
        FilterCriteria::LessThanOrEqualTo => value <= target,
        FilterCriteria::GreaterThan => value > target,
        FilterCriteria::GreaterThanOrEqualTo => value >= target,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use rust_xlsb_core::prelude::BrtFont;

    use crate::data::{RichRun, RichString};

    use super::*;

    fn rich_text(text: &str) -> CellData {
        CellData::RichString(RichString {
            text: text.to_owned(),
            runs: vec![RichRun {
                ich: 0,
                font: BrtFont::default(),
            }],
        })
    }

    #[test]
    fn list_filters_match_strings_numbers_and_blanks() {
        let condition = FilterCondition::new()
            .add_list_filter(" East ")
            .add_list_filter(3000_i32);
        assert!(condition.matches(Some(&CellData::String("east".to_owned()))));
        assert!(condition.matches(Some(&rich_text("east"))));
        assert!(condition.matches(Some(&CellData::Integer(3000))));
        assert!(condition.matches(Some(&CellData::Number(3000.0))));
        assert!(condition.matches(Some(&CellData::String("3000".to_owned()))));
        assert!(!condition.matches(Some(&CellData::String("west".to_owned()))));
        assert!(!condition.matches(None));

        let blanks = FilterCondition::new().add_list_blanks_filter();
        assert!(blanks.matches(None));
        assert!(blanks.matches(Some(&CellData::Blank)));
        assert!(blanks.matches(Some(&CellData::String("  ".to_owned()))));
    }

    #[test]
    fn custom_filters_compare_and_combine() {
        let between = FilterCondition::new()
            .add_custom_filter(FilterCriteria::GreaterThanOrEqualTo, 4000_i32)
            .add_custom_filter(FilterCriteria::LessThanOrEqualTo, 8000_i32);
        assert!(between.matches(Some(&CellData::Integer(5000))));
        assert!(!between.matches(Some(&CellData::Integer(9000))));

        let either = FilterCondition::new()
            .add_custom_filter(FilterCriteria::EqualTo, "East")
            .add_custom_filter(FilterCriteria::EqualTo, "West")
            .add_custom_boolean_or();
        assert!(either.matches(Some(&CellData::String("west".to_owned()))));
        assert!(either.matches(Some(&rich_text("west"))));
        assert!(!either.matches(Some(&CellData::String("north".to_owned()))));
    }

    #[test]
    fn custom_numeric_filters_also_compare_numeric_strings() {
        let equal = FilterCondition::new().add_custom_filter(FilterCriteria::EqualTo, 4000_i32);
        assert!(equal.matches(Some(&CellData::String("4000".to_owned()))));
        assert!(!equal.matches(Some(&CellData::String(String::new()))));

        let not_equal =
            FilterCondition::new().add_custom_filter(FilterCriteria::NotEqualTo, 4000_i32);
        assert!(not_equal.matches(Some(&CellData::String(String::new()))));
    }

    #[test]
    fn custom_string_wildcards_are_matched() {
        let condition = FilterCondition::new().add_custom_filter(FilterCriteria::Contains, "ast");
        assert!(condition.matches(Some(&CellData::String("East".to_owned()))));
        assert!(condition.matches(Some(&rich_text("East"))));
        assert!(!condition.matches(Some(&CellData::String("West".to_owned()))));
    }

    #[test]
    fn non_blank_custom_filter_matches_present_values() {
        let condition = FilterCondition::new().add_custom_filter(FilterCriteria::NotEqualTo, " ");
        assert!(!condition.matches(None));
        assert!(!condition.matches(Some(&CellData::Blank)));
        assert!(condition.matches(Some(&CellData::Integer(1))));
        assert!(condition.matches(Some(&CellData::String("x".to_owned()))));
        assert!(condition.matches(Some(&rich_text("x"))));
    }
}
