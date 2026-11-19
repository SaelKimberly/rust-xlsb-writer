//! Worksheet data validation and its MS-XLSB `DVALS` record.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use rust_xlsb_core::prelude::{
    BrtDVal, DValStrings, NameParsedFormula, UncheckedRfX, XlNullableWideString, XlStringExt,
};

use crate::data::IntoExcelDateTime;
use crate::datetime::ExcelDateTime;
use crate::range_list::RangeList;
use crate::{Error, Formula, Result};

/// A data-validation comparison rule.
#[derive(Clone)]
pub enum DataValidationRule<T: IntoDataValidationValue> {
    /// Values equal to the target.
    EqualTo(T),
    /// Values not equal to the target.
    NotEqualTo(T),
    /// Values greater than the target.
    GreaterThan(T),
    /// Values greater than or equal to the target.
    GreaterThanOrEqualTo(T),
    /// Values less than the target.
    LessThan(T),
    /// Values less than or equal to the target.
    LessThanOrEqualTo(T),
    /// Values between the two targets.
    Between(T, T),
    /// Values outside the two targets.
    NotBetween(T, T),
}

impl<T: IntoDataValidationValue> DataValidationRule<T> {
    fn into_internal_rule(self) -> DataValidationRuleInternal {
        let literal = |value: T| DataValidationOperand::Literal(value.to_string_value());
        match self {
            Self::EqualTo(value) => DataValidationRuleInternal::EqualTo(literal(value)),
            Self::NotEqualTo(value) => DataValidationRuleInternal::NotEqualTo(literal(value)),
            Self::GreaterThan(value) => DataValidationRuleInternal::GreaterThan(literal(value)),
            Self::GreaterThanOrEqualTo(value) => {
                DataValidationRuleInternal::GreaterThanOrEqualTo(literal(value))
            }
            Self::LessThan(value) => DataValidationRuleInternal::LessThan(literal(value)),
            Self::LessThanOrEqualTo(value) => {
                DataValidationRuleInternal::LessThanOrEqualTo(literal(value))
            }
            Self::Between(first, last) => {
                DataValidationRuleInternal::Between(literal(first), literal(last))
            }
            Self::NotBetween(first, last) => {
                DataValidationRuleInternal::NotBetween(literal(first), literal(last))
            }
        }
    }
}

impl DataValidationRule<Formula> {
    fn into_formula_rule(self) -> DataValidationRuleInternal {
        let formula = |value: Formula| DataValidationOperand::Formula(value);
        match self {
            Self::EqualTo(value) => DataValidationRuleInternal::EqualTo(formula(value)),
            Self::NotEqualTo(value) => DataValidationRuleInternal::NotEqualTo(formula(value)),
            Self::GreaterThan(value) => DataValidationRuleInternal::GreaterThan(formula(value)),
            Self::GreaterThanOrEqualTo(value) => {
                DataValidationRuleInternal::GreaterThanOrEqualTo(formula(value))
            }
            Self::LessThan(value) => DataValidationRuleInternal::LessThan(formula(value)),
            Self::LessThanOrEqualTo(value) => {
                DataValidationRuleInternal::LessThanOrEqualTo(formula(value))
            }
            Self::Between(first, last) => {
                DataValidationRuleInternal::Between(formula(first), formula(last))
            }
            Self::NotBetween(first, last) => {
                DataValidationRuleInternal::NotBetween(formula(first), formula(last))
            }
        }
    }
}

/// Maps a value type into the string used by a validation formula.
pub trait IntoDataValidationValue {
    /// Returns the value as an Excel formula literal.
    fn to_string_value(&self) -> String;
}

impl IntoDataValidationValue for i32 {
    fn to_string_value(&self) -> String {
        self.to_string()
    }
}

impl IntoDataValidationValue for u32 {
    fn to_string_value(&self) -> String {
        self.to_string()
    }
}

impl IntoDataValidationValue for f64 {
    fn to_string_value(&self) -> String {
        self.to_string()
    }
}

impl IntoDataValidationValue for Formula {
    fn to_string_value(&self) -> String {
        self.as_str().to_owned()
    }
}

impl IntoDataValidationValue for ExcelDateTime {
    fn to_string_value(&self) -> String {
        self.to_excel().to_string()
    }
}

impl IntoDataValidationValue for &ExcelDateTime {
    fn to_string_value(&self) -> String {
        self.to_excel().to_string()
    }
}

impl IntoDataValidationValue for NaiveDate {
    fn to_string_value(&self) -> String {
        chrono_date(*self).to_string()
    }
}

impl IntoDataValidationValue for &NaiveDate {
    fn to_string_value(&self) -> String {
        chrono_date(**self).to_string()
    }
}

impl IntoDataValidationValue for NaiveTime {
    fn to_string_value(&self) -> String {
        chrono_time(*self).to_string()
    }
}

impl IntoDataValidationValue for &NaiveTime {
    fn to_string_value(&self) -> String {
        chrono_time(**self).to_string()
    }
}

impl IntoDataValidationValue for NaiveDateTime {
    fn to_string_value(&self) -> String {
        chrono_datetime(*self).to_string()
    }
}

impl IntoDataValidationValue for &NaiveDateTime {
    fn to_string_value(&self) -> String {
        chrono_datetime(**self).to_string()
    }
}

#[allow(clippy::cast_precision_loss)]
fn chrono_date(value: NaiveDate) -> f64 {
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 31).expect("Excel epoch is a valid date");
    let mut serial = (value - epoch).num_days() as f64;
    if serial > 59.0 {
        serial += 1.0;
    }
    serial
}

#[allow(clippy::cast_precision_loss)]
fn chrono_time(value: NaiveTime) -> f64 {
    let midnight = NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is a valid time");
    (value - midnight).num_milliseconds() as f64 / 86_400_000.0
}

fn chrono_datetime(value: NaiveDateTime) -> f64 {
    chrono_date(value.date()) + chrono_time(value.time())
}

/// The style of the error dialog shown by a validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataValidationErrorStyle {
    /// A stop dialog.
    Stop,
    /// A warning dialog.
    Warning,
    /// An information dialog.
    Information,
}

#[derive(Clone)]
enum DataValidationType {
    Any,
    Whole,
    Decimal,
    List,
    Date,
    Time,
    TextLength,
    Custom,
}

#[derive(Clone)]
enum DataValidationOperand {
    Literal(String),
    Formula(Formula),
}

#[derive(Clone)]
enum DataValidationRuleInternal {
    EqualTo(DataValidationOperand),
    NotEqualTo(DataValidationOperand),
    GreaterThan(DataValidationOperand),
    GreaterThanOrEqualTo(DataValidationOperand),
    LessThan(DataValidationOperand),
    LessThanOrEqualTo(DataValidationOperand),
    Between(DataValidationOperand, DataValidationOperand),
    NotBetween(DataValidationOperand, DataValidationOperand),
    ListSource(DataValidationOperand),
    CustomFormula(Formula),
}

/// A worksheet cell data validation.
#[derive(Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "options map to independent MS-XLSB validation flags"
)]
pub struct DataValidation {
    validation_type: DataValidationType,
    rule: DataValidationRuleInternal,
    ignore_blank: bool,
    show_input_message: bool,
    show_error_message: bool,
    show_dropdown: bool,
    input_title: String,
    error_title: String,
    input_message: String,
    error_message: String,
    error_style: DataValidationErrorStyle,
    multi_range: Option<Box<str>>,
}

impl Default for DataValidation {
    fn default() -> Self {
        Self::new()
    }
}

impl DataValidation {
    /// Creates an “Any value” validation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            validation_type: DataValidationType::Any,
            rule: DataValidationRuleInternal::EqualTo(
                DataValidationOperand::Literal(String::new()),
            ),
            ignore_blank: true,
            show_input_message: true,
            show_error_message: true,
            show_dropdown: true,
            input_title: String::new(),
            error_title: String::new(),
            input_message: String::new(),
            error_message: String::new(),
            error_style: DataValidationErrorStyle::Stop,
            multi_range: None,
        }
    }

    /// Restricts input to whole numbers matching `rule`.
    #[must_use]
    pub fn allow_whole_number(mut self, rule: DataValidationRule<i32>) -> Self {
        self.rule = rule.into_internal_rule();
        self.validation_type = DataValidationType::Whole;
        self
    }

    /// Restricts input to whole numbers matching a formula `rule`.
    #[must_use]
    pub fn allow_whole_number_formula(mut self, rule: DataValidationRule<Formula>) -> Self {
        self.rule = rule.into_formula_rule();
        self.validation_type = DataValidationType::Whole;
        self
    }

    /// Restricts input to decimal numbers matching `rule`.
    #[must_use]
    pub fn allow_decimal_number(mut self, rule: DataValidationRule<f64>) -> Self {
        self.rule = rule.into_internal_rule();
        self.validation_type = DataValidationType::Decimal;
        self
    }

    /// Restricts input to decimal numbers matching a formula `rule`.
    #[must_use]
    pub fn allow_decimal_number_formula(mut self, rule: DataValidationRule<Formula>) -> Self {
        self.rule = rule.into_formula_rule();
        self.validation_type = DataValidationType::Decimal;
        self
    }

    /// Restricts input to dates matching `rule`.
    #[must_use]
    pub fn allow_date(
        mut self,
        rule: DataValidationRule<impl IntoExcelDateTime + IntoDataValidationValue>,
    ) -> Self {
        self.rule = rule.into_internal_rule();
        self.validation_type = DataValidationType::Date;
        self
    }

    /// Restricts input to dates matching a formula `rule`.
    #[must_use]
    pub fn allow_date_formula(mut self, rule: DataValidationRule<Formula>) -> Self {
        self.rule = rule.into_formula_rule();
        self.validation_type = DataValidationType::Date;
        self
    }

    /// Restricts input to times matching `rule`.
    #[must_use]
    pub fn allow_time(
        mut self,
        rule: DataValidationRule<impl IntoExcelDateTime + IntoDataValidationValue>,
    ) -> Self {
        self.rule = rule.into_internal_rule();
        self.validation_type = DataValidationType::Time;
        self
    }

    /// Restricts input to times matching a formula `rule`.
    #[must_use]
    pub fn allow_time_formula(mut self, rule: DataValidationRule<Formula>) -> Self {
        self.rule = rule.into_formula_rule();
        self.validation_type = DataValidationType::Time;
        self
    }

    /// Restricts input to text whose length matches `rule`.
    #[must_use]
    pub fn allow_text_length(mut self, rule: DataValidationRule<u32>) -> Self {
        self.rule = rule.into_internal_rule();
        self.validation_type = DataValidationType::TextLength;
        self
    }

    /// Restricts input to text whose length matches a formula `rule`.
    #[must_use]
    pub fn allow_text_length_formula(mut self, rule: DataValidationRule<Formula>) -> Self {
        self.rule = rule.into_formula_rule();
        self.validation_type = DataValidationType::TextLength;
        self
    }

    /// Restricts input to one of the supplied strings.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the joined list exceeds Excel's
    /// 255-character limit.
    pub fn allow_list_strings(mut self, list: &[impl AsRef<str>]) -> Result<Self> {
        let capacity = list
            .iter()
            .map(|item| {
                let text = item.as_ref();
                text.len() + text.matches('"').count()
            })
            .sum::<usize>()
            + list.len().saturating_sub(1);
        let mut joined = String::with_capacity(capacity);
        for (index, item) in list.iter().enumerate() {
            if index != 0 {
                joined.push(',');
            }
            for ch in item.as_ref().chars() {
                if ch == '"' {
                    joined.push('"');
                }
                joined.push(ch);
            }
        }

        let length = utf16_len(&joined);
        if length > 255 {
            return Err(Error::Parameter(format!(
                "validation list length '{length}' including commas is greater than Excel's limit \
                 of 255 characters"
            )));
        }

        self.rule = DataValidationRuleInternal::ListSource(DataValidationOperand::Literal(joined));
        self.validation_type = DataValidationType::List;
        Ok(self)
    }

    /// Restricts input to a list from a formula.
    #[must_use]
    pub fn allow_list_formula(mut self, list: Formula) -> Self {
        self.rule = DataValidationRuleInternal::ListSource(DataValidationOperand::Formula(list));
        self.validation_type = DataValidationType::List;
        self
    }

    /// Restricts input to values accepted by a custom formula.
    #[must_use]
    pub fn allow_custom(mut self, rule: Formula) -> Self {
        self.rule = DataValidationRuleInternal::CustomFormula(rule);
        self.validation_type = DataValidationType::Custom;
        self
    }

    /// Removes any value restriction while keeping prompt and error options.
    #[must_use]
    pub fn allow_any_value(mut self) -> Self {
        self.rule =
            DataValidationRuleInternal::EqualTo(DataValidationOperand::Literal(String::new()));
        self.validation_type = DataValidationType::Any;
        self
    }

    /// Sets whether blank input is accepted.
    #[must_use]
    pub const fn ignore_blank(mut self, enable: bool) -> Self {
        self.ignore_blank = enable;
        self
    }

    /// Sets whether a list validation shows its dropdown.
    #[must_use]
    pub const fn show_dropdown(mut self, enable: bool) -> Self {
        self.show_dropdown = enable;
        self
    }

    /// Sets whether the input prompt is shown.
    #[must_use]
    pub const fn show_input_message(mut self, enable: bool) -> Self {
        self.show_input_message = enable;
        self
    }

    /// Sets the input prompt title.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the title exceeds 32 UTF-16 code
    /// units.
    pub fn set_input_title(mut self, text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        check_utf16_len(&text, 32, "input title")?;
        self.input_title = text;
        Ok(self)
    }

    /// Sets the input prompt message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the message exceeds 255 UTF-16 code
    /// units.
    pub fn set_input_message(mut self, text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        check_utf16_len(&text, 255, "input message")?;
        self.input_message = text;
        Ok(self)
    }

    /// Sets whether the error alert is shown.
    #[must_use]
    pub const fn show_error_message(mut self, enable: bool) -> Self {
        self.show_error_message = enable;
        self
    }

    /// Sets the error alert title.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the title exceeds 32 UTF-16 code
    /// units.
    pub fn set_error_title(mut self, text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        check_utf16_len(&text, 32, "error title")?;
        self.error_title = text;
        Ok(self)
    }

    /// Sets the error alert message.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parameter`] when the message exceeds 225 UTF-16 code
    /// units.
    pub fn set_error_message(mut self, text: impl Into<String>) -> Result<Self> {
        let text = text.into();
        check_utf16_len(&text, 225, "error message")?;
        self.error_message = text;
        Ok(self)
    }

    /// Sets the error alert style.
    #[must_use]
    pub const fn set_error_style(mut self, error_style: DataValidationErrorStyle) -> Self {
        self.error_style = error_style;
        self
    }

    pub(crate) const fn is_invalid_any(&self) -> bool {
        matches!(self.validation_type, DataValidationType::Any)
            && self.input_title.is_empty()
            && self.input_message.is_empty()
            && self.error_title.is_empty()
            && self.error_message.is_empty()
    }

    /// Sets a same-sheet multi-range override for this validation.
    #[must_use]
    pub fn set_multi_range(mut self, range: impl Into<String>) -> Self {
        let range = range.into();
        self.multi_range = (!range.trim().is_empty()).then(|| range.into_boxed_str());
        self
    }

    pub(crate) fn multi_range(&self) -> Option<&str> {
        self.multi_range.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn record(&self, range: UncheckedRfX) -> Result<BrtDVal> {
        self.record_ranges(&[range])
    }

    pub(crate) fn record_ranges(&self, ranges: &[UncheckedRfX]) -> Result<BrtDVal> {
        let first = ranges
            .first()
            .copied()
            .ok_or_else(|| Error::Parameter("data validation requires a range".to_owned()))?;
        let range_count = u32::try_from(ranges.len())
            .map_err(|_| Error::ValueNotRepresentable("too many data-validation ranges"))?;
        let (val_type, operator, formula1, formula2) = self.formulas(&first)?;
        let mut flags = u32::from(val_type) | (u32::from(operator) << 20);
        flags |= match self.error_style {
            DataValidationErrorStyle::Stop => 0,
            DataValidationErrorStyle::Warning => 1 << 4,
            DataValidationErrorStyle::Information => 2 << 4,
        };
        if self.ignore_blank {
            flags |= BrtDVal::F_ALLOW_BLANK;
        }
        if matches!(self.validation_type, DataValidationType::List) && !self.show_dropdown {
            flags |= BrtDVal::F_SUPPRESS_COMBO;
        }
        if self.show_input_message {
            flags |= BrtDVal::F_SHOW_INPUT_MSG;
        }
        if self.show_error_message {
            flags |= BrtDVal::F_SHOW_ERROR_MSG;
        }

        Ok(BrtDVal {
            flags,
            sqrfx: rust_xlsb_core::prelude::UncheckedSqRfX {
                crfx: range_count,
                rgRefx: ranges.to_vec(),
            },
            strings: DValStrings {
                strErrorTitle: nullable_string(&self.error_title),
                strError: nullable_string(&self.error_message),
                strPromptTitle: nullable_string(&self.input_title),
                strPrompt: nullable_string(&self.input_message),
            },
            formula1,
            formula2,
        })
    }

    fn formulas(
        &self,
        range: &UncheckedRfX,
    ) -> Result<(u8, u8, NameParsedFormula, NameParsedFormula)> {
        match &self.validation_type {
            DataValidationType::Any => Ok((
                0,
                0,
                NameParsedFormula::default(),
                NameParsedFormula::default(),
            )),
            DataValidationType::List => {
                let DataValidationRuleInternal::ListSource(list) = &self.rule else {
                    return Err(unsupported_rule());
                };
                Ok((
                    3,
                    0,
                    list_formula(list, range)?,
                    NameParsedFormula::default(),
                ))
            }
            DataValidationType::Custom => {
                let DataValidationRuleInternal::CustomFormula(formula) = &self.rule else {
                    return Err(unsupported_rule());
                };
                Ok((
                    7,
                    0,
                    formula_formula(formula, range)?,
                    NameParsedFormula::default(),
                ))
            }
            DataValidationType::Whole => self.numeric_rule(1, range),
            DataValidationType::Decimal => self.numeric_rule(2, range),
            DataValidationType::Date => self.numeric_rule(4, range),
            DataValidationType::Time => self.numeric_rule(5, range),
            DataValidationType::TextLength => self.numeric_rule(6, range),
        }
    }

    fn numeric_rule(
        &self,
        val_type: u8,
        range: &UncheckedRfX,
    ) -> Result<(u8, u8, NameParsedFormula, NameParsedFormula)> {
        let (operator, first, second): (
            u8,
            &DataValidationOperand,
            Option<&DataValidationOperand>,
        ) = match &self.rule {
            DataValidationRuleInternal::EqualTo(value) => (2, value, None),
            DataValidationRuleInternal::NotEqualTo(value) => (3, value, None),
            DataValidationRuleInternal::GreaterThan(value) => (4, value, None),
            DataValidationRuleInternal::GreaterThanOrEqualTo(value) => (6, value, None),
            DataValidationRuleInternal::LessThan(value) => (5, value, None),
            DataValidationRuleInternal::LessThanOrEqualTo(value) => (7, value, None),
            DataValidationRuleInternal::Between(first, second) => (0, first, Some(second)),
            DataValidationRuleInternal::NotBetween(first, second) => (1, first, Some(second)),
            DataValidationRuleInternal::ListSource(_)
            | DataValidationRuleInternal::CustomFormula(_) => {
                return Err(unsupported_rule());
            }
        };

        Ok((
            val_type,
            operator,
            operand_formula(first, range)?,
            match second {
                Some(value) => operand_formula(value, range)?,
                None => NameParsedFormula::default(),
            },
        ))
    }
}

fn unsupported_rule() -> Error {
    Error::Parameter("data validation type does not match its rule".to_owned())
}

fn check_utf16_len(text: &str, max: usize, name: &str) -> Result<()> {
    let length = utf16_len(text);
    if length > max {
        return Err(Error::Parameter(format!(
            "validation {name} length '{length}' is greater than Excel's limit of {max} characters"
        )));
    }
    Ok(())
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

fn nullable_string(text: &str) -> XlNullableWideString {
    XlNullableWideString::construct((!text.is_empty()).then_some(text))
}

fn formula_formula(formula: &Formula, range: &UncheckedRfX) -> Result<NameParsedFormula> {
    let first_col = u16::try_from(range.colFirst)
        .map_err(|_| Error::ValueNotRepresentable("validation column"))?;
    let parsed = crate::formula::compile_relative(formula.as_str(), range.rwFirst, first_col)?;
    Ok(NameParsedFormula {
        cce: parsed.cce,
        rgce: parsed.rgce,
        cb: parsed.cb,
        rgcb: parsed.rgcb,
    })
}

fn operand_formula(
    operand: &DataValidationOperand,
    range: &UncheckedRfX,
) -> Result<NameParsedFormula> {
    match operand {
        DataValidationOperand::Literal(value) => number_formula(value),
        DataValidationOperand::Formula(formula) => formula_formula(formula, range),
    }
}

fn list_formula(
    operand: &DataValidationOperand,
    range: &UncheckedRfX,
) -> Result<NameParsedFormula> {
    match operand {
        DataValidationOperand::Literal(value) => string_formula(value),
        DataValidationOperand::Formula(formula) => formula_formula(formula, range),
    }
}

fn number_formula(value: &str) -> Result<NameParsedFormula> {
    let value: f64 = value
        .parse()
        .map_err(|_| Error::Parameter(format!("validation value '{value}' is not a number")))?;
    if !value.is_finite() {
        return Err(Error::Parameter(format!(
            "validation value '{value}' must be finite"
        )));
    }

    let mut rgce = Vec::with_capacity(9);
    rgce.push(0x1F);
    rgce.extend_from_slice(&value.to_le_bytes());
    Ok(NameParsedFormula {
        cce: u32::try_from(rgce.len())
            .map_err(|_| Error::ValueNotRepresentable("validation formula too long"))?,
        rgce,
        ..NameParsedFormula::default()
    })
}

fn string_formula(quoted: &str) -> Result<NameParsedFormula> {
    let mut rgce = Vec::with_capacity(3 + quoted.len().saturating_mul(2));
    rgce.push(0x17);
    rgce.extend_from_slice(&[0, 0]);
    let mut count = 0_u16;
    let mut chars = quoted.chars();
    let mut units = [0_u16; 2];

    while let Some(ch) = chars.next() {
        if ch == '"' && chars.next() != Some('"') {
            return Err(Error::Parameter(
                "validation list contains an unescaped quote".to_owned(),
            ));
        }
        for unit in ch.encode_utf16(&mut units) {
            count = count
                .checked_add(1)
                .ok_or(Error::ValueNotRepresentable("validation list too long"))?;
            rgce.extend_from_slice(&unit.to_le_bytes());
        }
        if count > 255 {
            return Err(Error::Parameter(
                "validation list exceeds 255 UTF-16 characters".to_owned(),
            ));
        }
    }

    rgce[1..3].copy_from_slice(&count.to_le_bytes());
    Ok(NameParsedFormula {
        cce: u32::try_from(rgce.len())
            .map_err(|_| Error::ValueNotRepresentable("validation formula too long"))?,
        rgce,
        ..NameParsedFormula::default()
    })
}

/// One worksheet validation and its target range.
#[derive(Clone)]
pub struct DataValidationRange {
    pub(crate) ranges: RangeList,
    pub(crate) validation: DataValidation,
}

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use rust_xlsb_core::Checked;

    fn range() -> UncheckedRfX {
        UncheckedRfX {
            rwFirst: 1,
            rwLast: 2,
            colFirst: 3,
            colLast: 3,
        }
    }

    #[test]
    fn formula_comparisons_compile_to_formula_operands() {
        let validation = DataValidation::new().allow_whole_number_formula(
            DataValidationRule::Between(crate::Formula::new("=D1"), crate::Formula::new("=D2")),
        );
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 1);
        assert_eq!(record.operator(), 0);
        assert_eq!(
            record.formula1.rgce,
            [0x4C, 0xFF, 0xFF, 0x0F, 0x00, 0, 0xC0]
        );
        assert_eq!(record.formula2.rgce, [0x4C, 0, 0, 0, 0, 0, 0xC0]);
    }

    #[test]
    fn every_formula_comparison_keeps_its_validation_type() {
        let formula = crate::Formula::new("=D1");
        for (validation, val_type) in [
            (
                DataValidation::new()
                    .allow_whole_number_formula(DataValidationRule::EqualTo(formula.clone())),
                1,
            ),
            (
                DataValidation::new()
                    .allow_decimal_number_formula(DataValidationRule::EqualTo(formula.clone())),
                2,
            ),
            (
                DataValidation::new()
                    .allow_date_formula(DataValidationRule::EqualTo(formula.clone())),
                4,
            ),
            (
                DataValidation::new()
                    .allow_time_formula(DataValidationRule::EqualTo(formula.clone())),
                5,
            ),
            (
                DataValidation::new()
                    .allow_text_length_formula(DataValidationRule::EqualTo(formula)),
                6,
            ),
        ] {
            let record = validation.record(range()).expect("record");
            assert_eq!(record.val_type(), val_type);
            assert_eq!(
                record.formula1.rgce,
                [0x4C, 0xFF, 0xFF, 0x0F, 0x00, 0, 0xC0]
            );
        }
    }

    #[test]
    fn list_and_custom_formulas_use_compiled_tokens() {
        let list = DataValidation::new().allow_list_formula(crate::Formula::new("=B1:B9"));
        let record = list.record(range()).expect("list formula");
        assert_eq!(record.val_type(), 3);
        assert_eq!(record.operator(), 0);
        assert_eq!(
            record.formula1.rgce,
            [
                0x4D, 0xFF, 0xFF, 0x0F, 0x00, 0x07, 0, 0, 0, 0xFE, 0xFF, 0xFE, 0xFF
            ]
        );

        let custom = DataValidation::new().allow_custom(crate::Formula::new("=A1>0"));
        let record = custom.record(range()).expect("custom formula");
        assert_eq!(record.val_type(), 7);
        assert_eq!(record.operator(), 0);
        assert_eq!(
            record.formula1.rgce,
            [0x4C, 0xFF, 0xFF, 0x0F, 0x00, 0xFD, 0xFF, 0x1E, 0, 0, 0x0D]
        );
    }

    #[test]
    fn formula_rules_reject_empty_and_cross_sheet_formulas() {
        assert!(
            DataValidation::new()
                .allow_custom(crate::Formula::new(""))
                .record(range())
                .is_err()
        );
        assert!(
            DataValidation::new()
                .allow_whole_number_formula(DataValidationRule::EqualTo(crate::Formula::new(
                    "=Sheet2!A1"
                )))
                .record(range())
                .is_err()
        );
    }

    #[test]
    fn multi_range_records_keep_every_area_and_the_first_formula_base() {
        let validation = DataValidation::new()
            .allow_whole_number(DataValidationRule::GreaterThan(0))
            .set_multi_range("$B$3:$D$6,$I$3:$K$6");
        let ranges = crate::range_list::RangeList::parse(
            validation.multi_range().expect("multi range"),
            8_191,
        )
        .expect("parses");
        let record = validation.record_ranges(ranges.as_slice()).expect("record");

        assert_eq!(record.sqrfx.crfx, 2);
        assert_eq!(record.sqrfx.rgRefx.as_slice(), ranges.as_slice());
        assert_eq!(record.val_type(), 1);

        let custom = DataValidation::new()
            .allow_custom(crate::Formula::new("=A1>0"))
            .set_multi_range("C3 D4");
        let ranges =
            crate::range_list::RangeList::parse(custom.multi_range().expect("multi range"), 8_191)
                .expect("parses");
        let record = custom.record_ranges(ranges.as_slice()).expect("record");
        assert_eq!(
            record.formula1.rgce,
            [0x4C, 0xFE, 0xFF, 0x0F, 0x00, 0xFE, 0xFF, 0x1E, 0, 0, 0x0D]
        );
    }

    #[test]
    fn empty_multi_range_clears_the_override() {
        let validation = DataValidation::new().set_multi_range("   ");
        assert!(validation.multi_range().is_none());
    }

    #[test]
    fn numeric_rules_encode_operators_and_values() {
        let validation = DataValidation::new()
            .allow_whole_number(DataValidationRule::Between(1, 10))
            .ignore_blank(false);
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 1);
        assert_eq!(record.operator(), 0);
        assert_eq!(record.flags & BrtDVal::F_ALLOW_BLANK, 0);
        assert_eq!(record.formula1.rgce[0], 0x1F);
        assert_eq!(record.formula2.rgce[0], 0x1F);

        let validation = DataValidation::new()
            .allow_decimal_number(DataValidationRule::GreaterThanOrEqualTo(1.5));
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 2);
        assert_eq!(record.operator(), 6);
        assert_eq!(record.formula2.rgce, Vec::<u8>::new());
    }

    #[test]
    fn list_escapes_quotes_and_encodes_ptgstr() {
        let validation = DataValidation::new()
            .allow_list_strings(&["A\"B", "C"])
            .expect("list");
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 3);
        assert_eq!(record.formula1.rgce[0], 0x17);
        assert_eq!(&record.formula1.rgce[1..3], &5_u16.to_le_bytes());
        let units = record.formula1.rgce[3..]
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        assert_eq!(String::from_utf16(&units).expect("utf16"), "A\"B,C");
    }

    #[test]
    fn list_length_and_titles_are_limited() {
        let too_long = "x".repeat(256);
        assert!(
            DataValidation::new()
                .allow_list_strings(&[too_long])
                .is_err()
        );
        assert!(
            DataValidation::new()
                .set_input_title("x".repeat(33))
                .is_err()
        );
        assert!(
            DataValidation::new()
                .set_error_message("x".repeat(226))
                .is_err()
        );
        assert!(
            DataValidation::new()
                .set_input_message("x".repeat(255))
                .is_ok()
        );
    }

    #[test]
    fn chrono_dates_and_times_use_excel_serials() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("date");
        let validation = DataValidation::new().allow_date(DataValidationRule::EqualTo(date));
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 4);
        assert_eq!(
            f64::from_le_bytes(record.formula1.rgce[1..9].try_into().expect("8 bytes")),
            chrono_date(date)
        );

        let time = NaiveTime::from_hms_micro_opt(23, 59, 59, 999_500).expect("time");
        let validation = DataValidation::new().allow_time(DataValidationRule::EqualTo(time));
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 5);
        assert_eq!(
            f64::from_le_bytes(record.formula1.rgce[1..9].try_into().expect("8 bytes")),
            chrono_time(time)
        );

        let old = NaiveDate::from_ymd_opt(1850, 1, 1).expect("date");
        let validation = DataValidation::new().allow_date(DataValidationRule::EqualTo(old));
        let record = validation.record(range()).expect("pre-1900 date");
        assert!(f64::from_le_bytes(record.formula1.rgce[1..9].try_into().expect("8 bytes")) < 0.0);
    }

    #[test]
    fn dates_use_excel_serials() {
        let date = ExcelDateTime::from_ymd(2025, 1, 1).expect("date");
        let validation = DataValidation::new()
            .allow_date(DataValidationRule::Between(date.clone(), date.clone()));
        let record = validation.record(range()).expect("record");
        assert_eq!(record.val_type(), 4);
        assert_eq!(
            f64::from_le_bytes(record.formula1.rgce[1..9].try_into().expect("8 bytes")),
            date.to_excel()
        );
    }

    #[test]
    fn any_without_text_is_ignored_but_any_with_text_writes() {
        let empty = DataValidation::new();
        assert!(empty.is_invalid_any());

        let with_prompt = DataValidation::new()
            .set_input_message("Pick a value")
            .expect("message");
        assert!(!with_prompt.is_invalid_any());
        let record = with_prompt.record(range()).expect("record");
        assert_eq!(record.val_type(), 0);
        assert_eq!(record.formula1.rgce, Vec::<u8>::new());
        assert_eq!(
            record.strings.strPrompt.to_optional_string().as_deref(),
            Some("Pick a value")
        );
    }

    #[test]
    fn records_are_valid_before_output() {
        let validation = DataValidation::new().allow_text_length(DataValidationRule::EqualTo(5));
        let record = validation.record(range()).expect("record");
        record.check().expect("valid record");
        record.to_bytes().expect("serializes");
    }
}
