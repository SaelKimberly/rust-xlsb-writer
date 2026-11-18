//! Cell-value conditional formatting.

use std::borrow::Cow;

use rust_xlsb_core::prelude::{CFParsedFormula, UncheckedRfX};

use crate::{Error, Format, Result};

/// A value accepted by a cell conditional-format rule.
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionalFormatValue {
    /// A finite numeric value.
    Number(f64),
    /// A text value.
    String(String),
}

/// Converts a rule value into the conditional-format value model.
pub trait IntoConditionalFormatValue {
    /// Converts the value.
    fn new_value(self) -> ConditionalFormatValue;
}

macro_rules! impl_number_value {
    ($($t:ty),* $(,)?) => {
        $(
            #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
            impl IntoConditionalFormatValue for $t {
                fn new_value(self) -> ConditionalFormatValue {
                    ConditionalFormatValue::Number(self as f64)
                }
            }
        )*
    };
}

impl_number_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl IntoConditionalFormatValue for ConditionalFormatValue {
    fn new_value(self) -> ConditionalFormatValue {
        self
    }
}

impl IntoConditionalFormatValue for &str {
    fn new_value(self) -> ConditionalFormatValue {
        ConditionalFormatValue::String(self.to_owned())
    }
}

impl IntoConditionalFormatValue for String {
    fn new_value(self) -> ConditionalFormatValue {
        ConditionalFormatValue::String(self)
    }
}

impl IntoConditionalFormatValue for &String {
    fn new_value(self) -> ConditionalFormatValue {
        ConditionalFormatValue::String(self.clone())
    }
}

impl IntoConditionalFormatValue for Cow<'_, str> {
    fn new_value(self) -> ConditionalFormatValue {
        ConditionalFormatValue::String(self.into_owned())
    }
}

/// A cell-value conditional-format comparison.
#[derive(Clone, Debug)]
pub enum ConditionalFormatCellRule<T> {
    /// Cells equal to the target.
    EqualTo(T),
    /// Cells not equal to the target.
    NotEqualTo(T),
    /// Cells greater than the target.
    GreaterThan(T),
    /// Cells greater than or equal to the target.
    GreaterThanOrEqualTo(T),
    /// Cells less than the target.
    LessThan(T),
    /// Cells less than or equal to the target.
    LessThanOrEqualTo(T),
    /// Cells between the two targets.
    Between(T, T),
    /// Cells outside the two targets.
    NotBetween(T, T),
}

#[derive(Clone, Debug)]
pub enum InternalRule {
    Between(ConditionalFormatValue, ConditionalFormatValue),
    NotBetween(ConditionalFormatValue, ConditionalFormatValue),
    EqualTo(ConditionalFormatValue),
    NotEqualTo(ConditionalFormatValue),
    GreaterThan(ConditionalFormatValue),
    LessThan(ConditionalFormatValue),
    GreaterThanOrEqualTo(ConditionalFormatValue),
    LessThanOrEqualTo(ConditionalFormatValue),
}

impl InternalRule {
    const fn operator(&self) -> u32 {
        match self {
            Self::Between(..) => 1,
            Self::NotBetween(..) => 2,
            Self::EqualTo(_) => 3,
            Self::NotEqualTo(_) => 4,
            Self::GreaterThan(_) => 5,
            Self::LessThan(_) => 6,
            Self::GreaterThanOrEqualTo(_) => 7,
            Self::LessThanOrEqualTo(_) => 8,
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Between(first, second) | Self::NotBetween(first, second) => {
                validate_value(first)?;
                validate_value(second)
            }
            Self::EqualTo(value)
            | Self::NotEqualTo(value)
            | Self::GreaterThan(value)
            | Self::LessThan(value)
            | Self::GreaterThanOrEqualTo(value)
            | Self::LessThanOrEqualTo(value) => validate_value(value),
        }
    }

    fn formulas(&self) -> Result<(CFParsedFormula, Option<CFParsedFormula>)> {
        let formula = |value: &ConditionalFormatValue| value_formula(value);
        match self {
            Self::Between(first, second) | Self::NotBetween(first, second) => {
                Ok((formula(first)?, Some(formula(second)?)))
            }
            Self::EqualTo(value)
            | Self::NotEqualTo(value)
            | Self::GreaterThan(value)
            | Self::LessThan(value)
            | Self::GreaterThanOrEqualTo(value)
            | Self::LessThanOrEqualTo(value) => Ok((formula(value)?, None)),
        }
    }
}

fn validate_value(value: &ConditionalFormatValue) -> Result<()> {
    match value {
        ConditionalFormatValue::Number(value) if !value.is_finite() => Err(Error::Parameter(
            "conditional-format value must be finite".to_owned(),
        )),
        ConditionalFormatValue::String(text) => {
            let length = text.encode_utf16().count();
            if length == 0 || length > 255 {
                Err(Error::Parameter(
                    "conditional-format string value must be 1..=255 characters".to_owned(),
                ))
            } else {
                Ok(())
            }
        }
        ConditionalFormatValue::Number(_) => Ok(()),
    }
}

fn value_formula(value: &ConditionalFormatValue) -> Result<CFParsedFormula> {
    validate_value(value)?;
    let rgce = match value {
        ConditionalFormatValue::Number(value) => {
            let mut rgce = Vec::with_capacity(9);
            rgce.push(0x1F);
            rgce.extend_from_slice(&value.to_le_bytes());
            rgce
        }
        ConditionalFormatValue::String(text) => {
            let length = text.encode_utf16().count();
            let mut rgce = Vec::with_capacity(3 + length * 2);
            rgce.push(0x17);
            rgce.extend_from_slice(&u16::try_from(length).expect("<=255").to_le_bytes());
            for unit in text.encode_utf16() {
                rgce.extend_from_slice(&unit.to_le_bytes());
            }
            rgce
        }
    };
    Ok(CFParsedFormula {
        cce: u32::try_from(rgce.len())
            .map_err(|_| Error::ValueNotRepresentable("conditional-format formula too long"))?,
        rgce,
        cb: 0,
        rgcb: Vec::new(),
    })
}

/// A cell-value conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatCell {
    rule: Option<InternalRule>,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatCell {
    /// Creates an empty cell conditional format. A rule is required.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rule: None,
            format: None,
            stop_if_true: false,
        }
    }

    /// Sets the comparison rule.
    #[must_use]
    pub fn set_rule<T: IntoConditionalFormatValue>(
        mut self,
        rule: ConditionalFormatCellRule<T>,
    ) -> Self {
        let rule = match rule {
            ConditionalFormatCellRule::EqualTo(value) => InternalRule::EqualTo(value.new_value()),
            ConditionalFormatCellRule::NotEqualTo(value) => {
                InternalRule::NotEqualTo(value.new_value())
            }
            ConditionalFormatCellRule::GreaterThan(value) => {
                InternalRule::GreaterThan(value.new_value())
            }
            ConditionalFormatCellRule::GreaterThanOrEqualTo(value) => {
                InternalRule::GreaterThanOrEqualTo(value.new_value())
            }
            ConditionalFormatCellRule::LessThan(value) => InternalRule::LessThan(value.new_value()),
            ConditionalFormatCellRule::LessThanOrEqualTo(value) => {
                InternalRule::LessThanOrEqualTo(value.new_value())
            }
            ConditionalFormatCellRule::Between(first, second) => {
                InternalRule::Between(first.new_value(), second.new_value())
            }
            ConditionalFormatCellRule::NotBetween(first, second) => {
                InternalRule::NotBetween(first.new_value(), second.new_value())
            }
        };
        self.rule = Some(rule);
        self
    }

    /// Sets the differential format applied when the rule matches.
    #[must_use]
    pub fn set_format(mut self, format: impl Into<Format>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Stops evaluating lower-priority rules when this rule matches.
    #[must_use]
    pub const fn set_stop_if_true(mut self, enable: bool) -> Self {
        self.stop_if_true = enable;
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.rule()?.validate()
    }

    pub(crate) fn rule(&self) -> Result<&InternalRule> {
        self.rule
            .as_ref()
            .ok_or_else(|| Error::Parameter("conditional-format rule must be set".to_owned()))
    }
}

impl Default for ConditionalFormatCell {
    fn default() -> Self {
        Self::new()
    }
}

/// One worksheet conditional format and its target range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatRange {
    pub(crate) range: UncheckedRfX,
    pub(crate) conditional_format: ConditionalFormatCell,
}

impl ConditionalFormatRange {
    pub(crate) fn operator(&self) -> Result<u32> {
        Ok(self.conditional_format.rule()?.operator())
    }

    pub(crate) fn formulas(&self) -> Result<(CFParsedFormula, Option<CFParsedFormula>)> {
        self.conditional_format.rule()?.formulas()
    }

    pub(crate) const fn format(&self) -> Option<&Format> {
        self.conditional_format.format.as_ref()
    }

    pub(crate) const fn stop_if_true(&self) -> bool {
        self.conditional_format.stop_if_true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_numeric_and_string_formulas() {
        let (number, second) = InternalRule::GreaterThan(ConditionalFormatValue::Number(1.5))
            .formulas()
            .expect("formula");
        assert_eq!(number.rgce[0], 0x1F);
        assert!(second.is_none());

        let (text, _) = InternalRule::EqualTo(ConditionalFormatValue::String("A".to_owned()))
            .formulas()
            .expect("formula");
        assert_eq!(text.rgce, [0x17, 1, 0, 0x41, 0]);
    }

    #[test]
    fn rejects_empty_and_non_finite_values() {
        assert!(value_formula(&ConditionalFormatValue::String(String::new())).is_err());
        assert!(value_formula(&ConditionalFormatValue::Number(f64::NAN)).is_err());
    }
}
