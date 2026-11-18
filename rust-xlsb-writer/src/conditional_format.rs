//! Cell-value conditional formatting.

use std::borrow::Cow;

use rust_xlsb_core::prelude::{
    BrtBeginCFRule, BrtBeginColorScale, BrtBeginDatabar, BrtBeginIconSet, BrtCFVO, BrtColor,
    BrtEndColorScale, BrtEndDatabar, BrtEndIconSet, CF_TEMPLATE_ABOVE_AVERAGE,
    CF_TEMPLATE_BELOW_AVERAGE, CF_TEMPLATE_CONTAINS_BLANKS, CF_TEMPLATE_CONTAINS_ERRORS,
    CF_TEMPLATE_CONTAINS_NO_BLANKS, CF_TEMPLATE_CONTAINS_NO_ERRORS, CF_TEMPLATE_CONTAINS_TEXT,
    CF_TEMPLATE_DATABAR, CF_TEMPLATE_DUPLICATE_VALUES, CF_TEMPLATE_EQUAL_ABOVE_AVERAGE,
    CF_TEMPLATE_EQUAL_BELOW_AVERAGE, CF_TEMPLATE_EXPR, CF_TEMPLATE_FILTER, CF_TEMPLATE_GRADIENT,
    CF_TEMPLATE_MULTISTATE, CF_TEMPLATE_UNIQUE_VALUES, CF_TEXTOPER_BEGINS_WITH,
    CF_TEXTOPER_CONTAINS, CF_TEXTOPER_ENDS_WITH, CF_TEXTOPER_NOT_CONTAINS, CF_TYPE_CELL_IS,
    CF_TYPE_DATABAR, CF_TYPE_EXPRESSION, CF_TYPE_FILTER, CF_TYPE_GRADIENT, CF_TYPE_MULTISTATE,
    CFParsedFormula, CFVO_MAX, CFVO_MIN, CFVO_NUM, CFVO_PERCENT, CFVO_PERCENTILE, UncheckedRfX,
    XlNullableWideString, XlStringExt,
};
use rust_xlsb_core::push_record;

use crate::{Color, Error, Format, Result};

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
            let mut rgce = Vec::with_capacity(3 + text.len() * 2);
            push_formula_string(&mut rgce, text)?;
            rgce
        }
    };
    parsed_formula(rgce)
}

/// A conditional-format rule accepted by
/// [`Worksheet::add_conditional_format`](crate::Worksheet::add_conditional_format).
pub trait ConditionalFormat {
    /// Validates the rule.
    #[doc(hidden)]
    fn validate(&self) -> Result<()>;

    /// Returns the differential format, if one was set.
    #[doc(hidden)]
    fn dxf_format(&self) -> Option<&Format>;

    /// Builds the rule record for the given priority and DXF id.
    #[doc(hidden)]
    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule>;

    /// Writes records nested between `BrtBeginCFRule` and `BrtEndCFRule`.
    #[doc(hidden)]
    fn write_body(&self, _out: &mut Vec<u8>) -> Result<()> {
        Ok(())
    }

    /// Clones the rule into a boxed trait object.
    #[doc(hidden)]
    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync>;
}

const fn stop_flag(enabled: bool) -> u16 {
    if enabled {
        BrtBeginCFRule::F_STOP_TRUE
    } else {
        0
    }
}

const PTG_REF_N: u8 = 0x4C;
const PTG_STR: u8 = 0x17;
const PTG_INT: u8 = 0x1E;
const PTG_FUNC: u8 = 0x41;
const PTG_FUNC_VAR: u8 = 0x42;
const PTG_EQ: u8 = 0x0B;
const PTG_GT: u8 = 0x0D;

const FUNC_ISERROR: u16 = 0x0003;
const FUNC_LEN: u16 = 0x0020;
const FUNC_NOT: u16 = 0x0026;
const FUNC_SEARCH: u16 = 0x0052;
const FUNC_LEFT: u16 = 0x0073;
const FUNC_RIGHT: u16 = 0x0074;
const FUNC_TRIM: u16 = 0x0076;

fn push_relative_ref(out: &mut Vec<u8>) {
    // PtgRefN to the current cell: zero row offset, relative row and column.
    out.extend_from_slice(&[PTG_REF_N, 0, 0, 0, 0, 0x00, 0xC0]);
}

fn push_formula_string(out: &mut Vec<u8>, text: &str) -> Result<()> {
    let length = text.encode_utf16().count();
    if length > 255 {
        return Err(Error::Parameter(
            "conditional-format text is too long".to_owned(),
        ));
    }
    out.push(PTG_STR);
    out.extend_from_slice(&u16::try_from(length).expect("<=255").to_le_bytes());
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    Ok(())
}

fn push_formula_int(out: &mut Vec<u8>, value: u16) {
    out.push(PTG_INT);
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_formula_func(out: &mut Vec<u8>, function: u16) {
    out.push(PTG_FUNC);
    out.extend_from_slice(&function.to_le_bytes());
}

fn push_formula_func_var(out: &mut Vec<u8>, params: u8, function: u16) {
    out.push(PTG_FUNC_VAR);
    out.push(params);
    out.extend_from_slice(&function.to_le_bytes());
}

fn parsed_formula(rgce: Vec<u8>) -> Result<CFParsedFormula> {
    let cce = u32::try_from(rgce.len())
        .map_err(|_| Error::Parameter("conditional-format formula is too long".to_owned()))?;
    Ok(CFParsedFormula {
        cce,
        rgce,
        cb: 0,
        rgcb: Vec::new(),
    })
}

fn text_formula(rule: &ConditionalFormatTextRule) -> Result<CFParsedFormula> {
    let mut rgce = Vec::with_capacity(48);
    match rule {
        ConditionalFormatTextRule::Contains(text) => {
            push_formula_string(&mut rgce, text)?;
            push_relative_ref(&mut rgce);
            push_formula_func_var(&mut rgce, 2, FUNC_SEARCH);
            push_formula_func(&mut rgce, FUNC_ISERROR);
            push_formula_func(&mut rgce, FUNC_NOT);
        }
        ConditionalFormatTextRule::DoesNotContain(text) => {
            push_formula_string(&mut rgce, text)?;
            push_relative_ref(&mut rgce);
            push_formula_func_var(&mut rgce, 2, FUNC_SEARCH);
            push_formula_func(&mut rgce, FUNC_ISERROR);
        }
        ConditionalFormatTextRule::BeginsWith(text) => {
            let length = u16::try_from(text.encode_utf16().count())
                .map_err(|_| Error::Parameter("conditional-format text is too long".to_owned()))?;
            push_relative_ref(&mut rgce);
            push_formula_int(&mut rgce, length);
            push_formula_func_var(&mut rgce, 2, FUNC_LEFT);
            push_formula_string(&mut rgce, text)?;
            rgce.push(PTG_EQ);
        }
        ConditionalFormatTextRule::EndsWith(text) => {
            let length = u16::try_from(text.encode_utf16().count())
                .map_err(|_| Error::Parameter("conditional-format text is too long".to_owned()))?;
            push_relative_ref(&mut rgce);
            push_formula_int(&mut rgce, length);
            push_formula_func_var(&mut rgce, 2, FUNC_RIGHT);
            push_formula_string(&mut rgce, text)?;
            rgce.push(PTG_EQ);
        }
    }
    parsed_formula(rgce)
}

fn blank_formula(inverted: bool) -> Result<CFParsedFormula> {
    let mut rgce = Vec::with_capacity(20);
    push_relative_ref(&mut rgce);
    push_formula_func(&mut rgce, FUNC_TRIM);
    push_formula_func(&mut rgce, FUNC_LEN);
    push_formula_int(&mut rgce, 0);
    rgce.push(if inverted { PTG_GT } else { PTG_EQ });
    parsed_formula(rgce)
}

fn error_formula(inverted: bool) -> Result<CFParsedFormula> {
    let mut rgce = Vec::with_capacity(16);
    push_relative_ref(&mut rgce);
    push_formula_func(&mut rgce, FUNC_ISERROR);
    if inverted {
        push_formula_func(&mut rgce, FUNC_NOT);
    }
    parsed_formula(rgce)
}

fn formula_rule(
    template: u32,
    parameter: u32,
    dxf_id: u32,
    priority: i32,
    stop_if_true: bool,
    str_param: XlNullableWideString,
    formula: CFParsedFormula,
) -> BrtBeginCFRule {
    BrtBeginCFRule {
        iType: CF_TYPE_EXPRESSION,
        iTemplate: template,
        dxfId: dxf_id,
        iPri: priority,
        iParam: parameter,
        flags: stop_flag(stop_if_true),
        cbFmla1: 1,
        strParam: str_param,
        formula1: Some(formula),
        ..BrtBeginCFRule::default()
    }
}

/// A text conditional-format comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionalFormatTextRule {
    /// Matches cells containing the text.
    Contains(String),
    /// Matches cells not containing the text.
    DoesNotContain(String),
    /// Matches cells beginning with the text.
    BeginsWith(String),
    /// Matches cells ending with the text.
    EndsWith(String),
}

impl ConditionalFormatTextRule {
    const fn parameter(&self) -> u32 {
        match self {
            Self::Contains(_) => CF_TEXTOPER_CONTAINS,
            Self::DoesNotContain(_) => CF_TEXTOPER_NOT_CONTAINS,
            Self::BeginsWith(_) => CF_TEXTOPER_BEGINS_WITH,
            Self::EndsWith(_) => CF_TEXTOPER_ENDS_WITH,
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Contains(text)
            | Self::DoesNotContain(text)
            | Self::BeginsWith(text)
            | Self::EndsWith(text) => text,
        }
    }
}

/// A text conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatText {
    rule: Option<ConditionalFormatTextRule>,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatText {
    /// Creates an empty text rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rule: None,
            format: None,
            stop_if_true: false,
        }
    }

    /// Sets the text comparison rule.
    #[must_use]
    pub fn set_rule(mut self, rule: ConditionalFormatTextRule) -> Self {
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
}

impl Default for ConditionalFormatText {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatText {
    fn validate(&self) -> Result<()> {
        let Some(rule) = &self.rule else {
            return Err(Error::Parameter(
                "text conditional format requires a rule".to_owned(),
            ));
        };
        let length = rule.text().encode_utf16().count();
        if !(1..=255).contains(&length) {
            return Err(Error::Parameter(
                "text conditional format requires 1..=255 UTF-16 units".to_owned(),
            ));
        }
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        self.validate()?;
        let rule = self
            .rule
            .as_ref()
            .expect("validated text conditional format has a rule");
        let formula = text_formula(rule)?;
        Ok(formula_rule(
            CF_TEMPLATE_CONTAINS_TEXT,
            rule.parameter(),
            dxf_id,
            priority,
            self.stop_if_true,
            XlNullableWideString::construct(Some(rule.text())),
            formula,
        ))
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A blank or non-blank conditional format.
#[derive(Clone, Debug)]
pub struct ConditionalFormatBlank {
    is_inverted: bool,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatBlank {
    /// Creates a blank-cell rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_inverted: false,
            format: None,
            stop_if_true: false,
        }
    }

    /// Matches non-blank cells instead of blank cells.
    #[must_use]
    pub const fn invert(mut self) -> Self {
        self.is_inverted = true;
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
}

impl Default for ConditionalFormatBlank {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatBlank {
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(formula_rule(
            if self.is_inverted {
                CF_TEMPLATE_CONTAINS_NO_BLANKS
            } else {
                CF_TEMPLATE_CONTAINS_BLANKS
            },
            0,
            dxf_id,
            priority,
            self.stop_if_true,
            XlNullableWideString::construct(None::<&str>),
            blank_formula(self.is_inverted)?,
        ))
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// An error or non-error conditional format.
#[derive(Clone, Debug)]
pub struct ConditionalFormatError {
    is_inverted: bool,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatError {
    /// Creates an error-cell rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_inverted: false,
            format: None,
            stop_if_true: false,
        }
    }

    /// Matches non-error cells instead of error cells.
    #[must_use]
    pub const fn invert(mut self) -> Self {
        self.is_inverted = true;
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
}

impl Default for ConditionalFormatError {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatError {
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(formula_rule(
            if self.is_inverted {
                CF_TEMPLATE_CONTAINS_NO_ERRORS
            } else {
                CF_TEMPLATE_CONTAINS_ERRORS
            },
            0,
            dxf_id,
            priority,
            self.stop_if_true,
            XlNullableWideString::construct(None::<&str>),
            error_formula(self.is_inverted)?,
        ))
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
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

    fn validate_rule(&self) -> Result<()> {
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

impl ConditionalFormat for ConditionalFormatCell {
    fn validate(&self) -> Result<()> {
        self.validate_rule()
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        let (formula1, formula2) = self.rule()?.formulas()?;
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_CELL_IS,
            iTemplate: CF_TEMPLATE_EXPR,
            dxfId: dxf_id,
            iPri: priority,
            iParam: self.rule()?.operator(),
            flags: stop_flag(self.stop_if_true),
            cbFmla1: formula1.cce,
            cbFmla2: formula2.as_ref().map_or(0, |formula| formula.cce),
            formula1: Some(formula1),
            formula2,
            ..BrtBeginCFRule::default()
        })
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A top/bottom conditional-format rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionalFormatTopRule {
    /// Cells in the top `N` values.
    Top(u16),
    /// Cells in the bottom `N` values.
    Bottom(u16),
    /// Cells in the top `N` percent.
    TopPercent(u16),
    /// Cells in the bottom `N` percent.
    BottomPercent(u16),
}

/// A top/bottom conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatTop {
    rule: ConditionalFormatTopRule,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatTop {
    /// Creates a top-10 rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rule: ConditionalFormatTopRule::Top(10),
            format: None,
            stop_if_true: false,
        }
    }

    /// Sets the top/bottom rule.
    #[must_use]
    pub const fn set_rule(mut self, rule: ConditionalFormatTopRule) -> Self {
        self.rule = rule;
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

    fn validate_rule(&self) -> Result<()> {
        let (value, maximum, kind) = match self.rule {
            ConditionalFormatTopRule::Top(value) | ConditionalFormatTopRule::Bottom(value) => {
                (value, 1000, "rank")
            }
            ConditionalFormatTopRule::TopPercent(value)
            | ConditionalFormatTopRule::BottomPercent(value) => (value, 100, "percent"),
        };
        if value == 0 || value > maximum {
            return Err(Error::Parameter(format!(
                "conditional-format top/bottom {kind} must be 1..={maximum}"
            )));
        }
        Ok(())
    }
}

impl Default for ConditionalFormatTop {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatTop {
    fn validate(&self) -> Result<()> {
        self.validate_rule()
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        self.validate_rule()?;
        let (rank, bottom, percent) = match self.rule {
            ConditionalFormatTopRule::Top(rank) => (rank, false, false),
            ConditionalFormatTopRule::Bottom(rank) => (rank, true, false),
            ConditionalFormatTopRule::TopPercent(rank) => (rank, false, true),
            ConditionalFormatTopRule::BottomPercent(rank) => (rank, true, true),
        };
        let mut flags = stop_flag(self.stop_if_true);
        if bottom {
            flags |= BrtBeginCFRule::F_BOTTOM;
        }
        if percent {
            flags |= BrtBeginCFRule::F_PERCENT;
        }
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_FILTER,
            iTemplate: CF_TEMPLATE_FILTER,
            dxfId: dxf_id,
            iPri: priority,
            iParam: u32::from(rank),
            flags,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// An average conditional-format rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionalFormatAverageRule {
    /// Cells above the range average. This is the default.
    AboveAverage,
    /// Cells below the range average.
    BelowAverage,
    /// Cells equal to or above the range average.
    EqualOrAboveAverage,
    /// Cells equal to or below the range average.
    EqualOrBelowAverage,
    /// Cells one standard deviation above the range average.
    OneStandardDeviationAbove,
    /// Cells one standard deviation below the range average.
    OneStandardDeviationBelow,
    /// Cells two standard deviations above the range average.
    TwoStandardDeviationsAbove,
    /// Cells two standard deviations below the range average.
    TwoStandardDeviationsBelow,
    /// Cells three standard deviations above the range average.
    ThreeStandardDeviationsAbove,
    /// Cells three standard deviations below the range average.
    ThreeStandardDeviationsBelow,
}

impl ConditionalFormatAverageRule {
    const fn template(self) -> u32 {
        match self {
            Self::EqualOrAboveAverage => CF_TEMPLATE_EQUAL_ABOVE_AVERAGE,
            Self::EqualOrBelowAverage => CF_TEMPLATE_EQUAL_BELOW_AVERAGE,
            Self::AboveAverage
            | Self::OneStandardDeviationAbove
            | Self::TwoStandardDeviationsAbove
            | Self::ThreeStandardDeviationsAbove => CF_TEMPLATE_ABOVE_AVERAGE,
            Self::BelowAverage
            | Self::OneStandardDeviationBelow
            | Self::TwoStandardDeviationsBelow
            | Self::ThreeStandardDeviationsBelow => CF_TEMPLATE_BELOW_AVERAGE,
        }
    }

    const fn above(self) -> bool {
        matches!(
            self,
            Self::AboveAverage
                | Self::EqualOrAboveAverage
                | Self::OneStandardDeviationAbove
                | Self::TwoStandardDeviationsAbove
                | Self::ThreeStandardDeviationsAbove
        )
    }

    const fn parameter(self) -> u32 {
        match self {
            Self::OneStandardDeviationAbove | Self::OneStandardDeviationBelow => 1,
            Self::TwoStandardDeviationsAbove | Self::TwoStandardDeviationsBelow => 2,
            Self::ThreeStandardDeviationsAbove | Self::ThreeStandardDeviationsBelow => 3,
            _ => 0,
        }
    }
}

/// An average conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatAverage {
    rule: ConditionalFormatAverageRule,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatAverage {
    /// Creates an above-average rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rule: ConditionalFormatAverageRule::AboveAverage,
            format: None,
            stop_if_true: false,
        }
    }

    /// Sets the average rule.
    #[must_use]
    pub const fn set_rule(mut self, rule: ConditionalFormatAverageRule) -> Self {
        self.rule = rule;
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
}

impl Default for ConditionalFormatAverage {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatAverage {
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        let mut flags = stop_flag(self.stop_if_true);
        if self.rule.above() {
            flags |= BrtBeginCFRule::F_ABOVE;
        }
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: self.rule.template(),
            dxfId: dxf_id,
            iPri: priority,
            iParam: self.rule.parameter(),
            flags,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A duplicate/unique conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatDuplicate {
    is_inverted: bool,
    pub(crate) format: Option<Format>,
    pub(crate) stop_if_true: bool,
}

impl ConditionalFormatDuplicate {
    /// Creates a duplicate-values rule.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_inverted: false,
            format: None,
            stop_if_true: false,
        }
    }

    /// Inverts the rule to match unique values instead of duplicates.
    #[must_use]
    pub const fn invert(mut self) -> Self {
        self.is_inverted = true;
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
}

impl Default for ConditionalFormatDuplicate {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatDuplicate {
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        self.format.as_ref()
    }

    fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: if self.is_inverted {
                CF_TEMPLATE_UNIQUE_VALUES
            } else {
                CF_TEMPLATE_DUPLICATE_VALUES
            },
            dxfId: dxf_id,
            iPri: priority,
            iParam: 0,
            flags: stop_flag(self.stop_if_true),
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A conditional-format threshold type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionalFormatType {
    /// The lowest value in the range.
    Lowest,
    /// A literal number.
    Number,
    /// A percentage between the range minimum and maximum.
    Percent,
    /// A percentile of the range.
    Percentile,
    /// The highest value in the range.
    Highest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThresholdPosition {
    Minimum,
    Midpoint,
    Maximum,
}

#[derive(Clone, Debug)]
struct Threshold {
    kind: ConditionalFormatType,
    value: ConditionalFormatValue,
}

impl Threshold {
    const fn lowest() -> Self {
        Self {
            kind: ConditionalFormatType::Lowest,
            value: ConditionalFormatValue::Number(0.0),
        }
    }

    const fn highest() -> Self {
        Self {
            kind: ConditionalFormatType::Highest,
            value: ConditionalFormatValue::Number(0.0),
        }
    }

    const fn percentile(value: f64) -> Self {
        Self {
            kind: ConditionalFormatType::Percentile,
            value: ConditionalFormatValue::Number(value),
        }
    }

    const fn new(kind: ConditionalFormatType, value: ConditionalFormatValue) -> Self {
        Self { kind, value }
    }

    fn number_value(&self) -> Result<f64> {
        match &self.value {
            ConditionalFormatValue::Number(value) if value.is_finite() => Ok(*value),
            ConditionalFormatValue::Number(_) => Err(Error::Parameter(
                "conditional-format threshold must be finite".to_owned(),
            )),
            ConditionalFormatValue::String(_) => Err(Error::Parameter(
                "conditional-format threshold must be numeric".to_owned(),
            )),
        }
    }

    fn cfvo(&self) -> Result<BrtCFVO> {
        let (i_type, value) = match self.kind {
            ConditionalFormatType::Lowest => (CFVO_MIN, 0.0),
            ConditionalFormatType::Highest => (CFVO_MAX, 0.0),
            ConditionalFormatType::Number => (CFVO_NUM, self.number_value()?),
            ConditionalFormatType::Percent | ConditionalFormatType::Percentile => {
                let value = self.number_value()?;
                if !(0.0..=100.0).contains(&value) {
                    return Err(Error::Parameter(
                        "conditional-format percentage must be in 0..=100".to_owned(),
                    ));
                }
                let i_type = if self.kind == ConditionalFormatType::Percent {
                    CFVO_PERCENT
                } else {
                    CFVO_PERCENTILE
                };
                (i_type, value)
            }
        };

        Ok(BrtCFVO {
            iType: i_type,
            numParam: value.into(),
            ..BrtCFVO::default()
        })
    }

    fn icon_cfvo(&self) -> Result<BrtCFVO> {
        let (i_type, value) = match self.kind {
            ConditionalFormatType::Number => (CFVO_NUM, self.number_value()?),
            ConditionalFormatType::Percent => {
                let value = self.number_value()?;
                if !(0.0..=100.0).contains(&value) {
                    return Err(Error::Parameter(
                        "icon-set percentage must be in 0..=100".to_owned(),
                    ));
                }
                (CFVO_PERCENT, value)
            }
            ConditionalFormatType::Percentile => {
                let value = self.number_value()?;
                if !(0.0..=100.0).contains(&value) {
                    return Err(Error::Parameter(
                        "icon-set percentile must be in 0..=100".to_owned(),
                    ));
                }
                (CFVO_PERCENTILE, value)
            }
            ConditionalFormatType::Lowest | ConditionalFormatType::Highest => {
                return Err(Error::Parameter(
                    "icon-set thresholds must use a number, percent, or percentile".to_owned(),
                ));
            }
        };

        Ok(BrtCFVO {
            iType: i_type,
            numParam: value.into(),
            fSaveGTE: 1,
            fGTE: 1,
            ..BrtCFVO::default()
        })
    }

    fn validate(&self, position: ThresholdPosition) -> Result<()> {
        if position == ThresholdPosition::Minimum && self.kind == ConditionalFormatType::Highest {
            return Err(Error::Parameter(
                "conditional-format minimum cannot use the highest threshold".to_owned(),
            ));
        }
        if position == ThresholdPosition::Maximum && self.kind == ConditionalFormatType::Lowest {
            return Err(Error::Parameter(
                "conditional-format maximum cannot use the lowest threshold".to_owned(),
            ));
        }
        if position == ThresholdPosition::Midpoint
            && matches!(
                self.kind,
                ConditionalFormatType::Lowest | ConditionalFormatType::Highest
            )
        {
            return Err(Error::Parameter(
                "conditional-format midpoint must use a numeric threshold".to_owned(),
            ));
        }
        self.cfvo().map(|_| ())
    }
}

fn conditional_color(color: Color) -> Result<BrtColor> {
    if matches!(color, Color::Default | Color::Automatic) {
        return Err(Error::Parameter(
            "conditional-format colors must be explicit".to_owned(),
        ));
    }
    color.to_brt_color()
}

fn write_scale_body(
    thresholds: &[&Threshold],
    colors: &[BrtColor],
    out: &mut Vec<u8>,
) -> Result<()> {
    push_record(out, &BrtBeginColorScale::default())?;
    for threshold in thresholds {
        push_record(out, &threshold.cfvo()?)?;
    }
    for color in colors {
        push_record(out, color)?;
    }
    push_record(out, &BrtEndColorScale::default())?;
    Ok(())
}

const TWO_COLOR_MIN: Color = Color::RGB(0x00FF_EF9C);
const TWO_COLOR_MAX: Color = Color::RGB(0x0063_BE7B);
const THREE_COLOR_MIN: Color = Color::RGB(0x00F8_696B);
const THREE_COLOR_MID: Color = Color::RGB(0x00FF_EB84);
const THREE_COLOR_MAX: Color = Color::RGB(0x0063_BE7B);

/// A two-color scale conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormat2ColorScale {
    min: Threshold,
    max: Threshold,
    min_color: Color,
    max_color: Color,
    stop_if_true: bool,
}

impl ConditionalFormat2ColorScale {
    /// Creates a two-color scale with Excel's default yellow-to-green colors.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min: Threshold::lowest(),
            max: Threshold::highest(),
            min_color: TWO_COLOR_MIN,
            max_color: TWO_COLOR_MAX,
            stop_if_true: false,
        }
    }

    /// Sets the minimum threshold type and value.
    #[must_use]
    pub fn set_minimum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.min = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the maximum threshold type and value.
    #[must_use]
    pub fn set_maximum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.max = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the minimum color.
    #[must_use]
    pub fn set_minimum_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.min_color = color;
        }
        self
    }

    /// Sets the maximum color.
    #[must_use]
    pub fn set_maximum_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.max_color = color;
        }
        self
    }

    /// Sets the stop-if-true option. Color scales cannot use it.
    #[must_use]
    pub const fn set_stop_if_true(mut self, enable: bool) -> Self {
        self.stop_if_true = enable;
        self
    }
}

impl Default for ConditionalFormat2ColorScale {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormat2ColorScale {
    fn validate(&self) -> Result<()> {
        if self.stop_if_true {
            return Err(Error::Parameter(
                "color scales cannot set stop_if_true".to_owned(),
            ));
        }
        self.min.validate(ThresholdPosition::Minimum)?;
        self.max.validate(ThresholdPosition::Maximum)?;
        conditional_color(self.min_color)?;
        conditional_color(self.max_color)?;
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        None
    }

    fn begin_rule(&self, priority: i32, _dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_GRADIENT,
            iTemplate: CF_TEMPLATE_GRADIENT,
            dxfId: u32::MAX,
            iPri: priority,
            iParam: 0,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        let colors = [
            conditional_color(self.min_color)?,
            conditional_color(self.max_color)?,
        ];
        write_scale_body(&[&self.min, &self.max], &colors, out)
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A three-color scale conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormat3ColorScale {
    min: Threshold,
    mid: Threshold,
    max: Threshold,
    min_color: Color,
    mid_color: Color,
    max_color: Color,
    stop_if_true: bool,
}

impl ConditionalFormat3ColorScale {
    /// Creates a three-color scale with Excel's default red-yellow-green colors.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min: Threshold::lowest(),
            mid: Threshold::percentile(50.0),
            max: Threshold::highest(),
            min_color: THREE_COLOR_MIN,
            mid_color: THREE_COLOR_MID,
            max_color: THREE_COLOR_MAX,
            stop_if_true: false,
        }
    }

    /// Sets the minimum threshold type and value.
    #[must_use]
    pub fn set_minimum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.min = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the midpoint threshold type and value.
    #[must_use]
    pub fn set_midpoint(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.mid = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the maximum threshold type and value.
    #[must_use]
    pub fn set_maximum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.max = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the minimum color.
    #[must_use]
    pub fn set_minimum_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.min_color = color;
        }
        self
    }

    /// Sets the midpoint color.
    #[must_use]
    pub fn set_midpoint_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.mid_color = color;
        }
        self
    }

    /// Sets the maximum color.
    #[must_use]
    pub fn set_maximum_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.max_color = color;
        }
        self
    }

    /// Sets the stop-if-true option. Color scales cannot use it.
    #[must_use]
    pub const fn set_stop_if_true(mut self, enable: bool) -> Self {
        self.stop_if_true = enable;
        self
    }
}

impl Default for ConditionalFormat3ColorScale {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormat3ColorScale {
    fn validate(&self) -> Result<()> {
        if self.stop_if_true {
            return Err(Error::Parameter(
                "color scales cannot set stop_if_true".to_owned(),
            ));
        }
        self.min.validate(ThresholdPosition::Minimum)?;
        self.mid.validate(ThresholdPosition::Midpoint)?;
        self.max.validate(ThresholdPosition::Maximum)?;
        conditional_color(self.min_color)?;
        conditional_color(self.mid_color)?;
        conditional_color(self.max_color)?;
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        None
    }

    fn begin_rule(&self, priority: i32, _dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_GRADIENT,
            iTemplate: CF_TEMPLATE_GRADIENT,
            dxfId: u32::MAX,
            iPri: priority,
            iParam: 0,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        let colors = [
            conditional_color(self.min_color)?,
            conditional_color(self.mid_color)?,
            conditional_color(self.max_color)?,
        ];
        write_scale_body(&[&self.min, &self.mid, &self.max], &colors, out)
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

const DATA_BAR_COLOR: Color = Color::RGB(0x0063_8EC6);

/// A data-bar conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatDataBar {
    min: Threshold,
    max: Threshold,
    fill_color: Color,
    bar_only: bool,
}

impl ConditionalFormatDataBar {
    /// Creates a data bar with Excel's automatic minimum and maximum and blue fill.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min: Threshold::lowest(),
            max: Threshold::highest(),
            fill_color: DATA_BAR_COLOR,
            bar_only: false,
        }
    }

    /// Sets the minimum threshold type and value.
    #[must_use]
    pub fn set_minimum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.min = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the maximum threshold type and value.
    #[must_use]
    pub fn set_maximum(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.max = Threshold::new(rule_type, value.new_value());
        self
    }

    /// Sets the data-bar fill color.
    #[must_use]
    pub fn set_fill_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        if color.is_valid() {
            self.fill_color = color;
        }
        self
    }

    /// Hides the cell value and shows only the data bar.
    #[must_use]
    pub const fn set_bar_only(mut self, enable: bool) -> Self {
        self.bar_only = enable;
        self
    }
}

impl Default for ConditionalFormatDataBar {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatDataBar {
    fn validate(&self) -> Result<()> {
        self.min.validate(ThresholdPosition::Minimum)?;
        self.max.validate(ThresholdPosition::Maximum)?;
        conditional_color(self.fill_color)?;
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        None
    }

    fn begin_rule(&self, priority: i32, _dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_DATABAR,
            iTemplate: CF_TEMPLATE_DATABAR,
            dxfId: u32::MAX,
            iPri: priority,
            iParam: 0,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        push_record(
            out,
            &BrtBeginDatabar {
                fShowValue: u8::from(!self.bar_only),
                ..BrtBeginDatabar::default()
            },
        )?;
        push_record(out, &self.min.cfvo()?)?;
        push_record(out, &self.max.cfvo()?)?;
        push_record(out, &conditional_color(self.fill_color)?)?;
        push_record(out, &BrtEndDatabar::default())?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// A standard Excel icon set.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ConditionalFormatIconType {
    /// Three arrows: up, sideways, down.
    ThreeArrows,
    /// Three gray arrows: up, sideways, down.
    ThreeArrowsGray,
    /// Three flags: red, yellow, green.
    ThreeFlags,
    /// Three rounded traffic lights.
    ThreeTrafficLights,
    /// Three traffic lights with a square rim.
    ThreeTrafficLightsWithRim,
    /// Three signs: circle, triangle, diamond.
    ThreeSigns,
    /// Three circled symbols: tick, exclamation, cross.
    ThreeSymbolsCircled,
    /// Three symbols: tick, exclamation, cross.
    ThreeSymbols,
    /// Four arrows: up, diagonal up, diagonal down, down.
    FourArrows,
    /// Four gray arrows: up, diagonal up, diagonal down, down.
    FourArrowsGray,
    /// Four circles from red to black.
    FourRedToBlack,
    /// Four histogram ratings.
    FourHistograms,
    /// Four traffic lights.
    FourTrafficLights,
    /// Five arrows: up, diagonal up, sideways, diagonal down, down.
    FiveArrows,
    /// Five gray arrows.
    FiveArrowsGray,
    /// Five histogram ratings.
    FiveHistograms,
    /// Five quadrants.
    FiveQuadrants,
}

impl ConditionalFormatIconType {
    const fn count(self) -> usize {
        match self {
            Self::ThreeArrows
            | Self::ThreeArrowsGray
            | Self::ThreeFlags
            | Self::ThreeTrafficLights
            | Self::ThreeTrafficLightsWithRim
            | Self::ThreeSigns
            | Self::ThreeSymbolsCircled
            | Self::ThreeSymbols => 3,
            Self::FourArrows
            | Self::FourArrowsGray
            | Self::FourRedToBlack
            | Self::FourHistograms
            | Self::FourTrafficLights => 4,
            Self::FiveArrows
            | Self::FiveArrowsGray
            | Self::FiveHistograms
            | Self::FiveQuadrants => 5,
        }
    }

    const fn kpi(self) -> u32 {
        match self {
            Self::ThreeArrows => 0x00,
            Self::ThreeArrowsGray => 0x01,
            Self::ThreeFlags => 0x02,
            Self::ThreeTrafficLights => 0x03,
            Self::ThreeTrafficLightsWithRim => 0x04,
            Self::ThreeSigns => 0x05,
            Self::ThreeSymbolsCircled => 0x06,
            Self::ThreeSymbols => 0x07,
            Self::FourArrows => 0x08,
            Self::FourArrowsGray => 0x09,
            Self::FourRedToBlack => 0x0A,
            Self::FourHistograms => 0x0B,
            Self::FourTrafficLights => 0x0C,
            Self::FiveArrows => 0x0D,
            Self::FiveArrowsGray => 0x0E,
            Self::FiveHistograms => 0x0F,
            Self::FiveQuadrants => 0x10,
        }
    }

    fn default_icons(self) -> Vec<ConditionalFormatCustomIcon> {
        let values: &[f64] = match self.count() {
            3 => &[0.0, 33.0, 67.0],
            4 => &[0.0, 25.0, 50.0, 75.0],
            _ => &[0.0, 20.0, 40.0, 60.0, 80.0],
        };
        let mut icons = Vec::with_capacity(values.len());
        icons.extend(values.iter().map(|&value| ConditionalFormatCustomIcon {
            threshold: Threshold::new(
                ConditionalFormatType::Percent,
                ConditionalFormatValue::Number(value),
            ),
        }));
        icons
    }
}

/// One icon threshold in a [`ConditionalFormatIconSet`].
#[derive(Clone, Debug)]
pub struct ConditionalFormatCustomIcon {
    threshold: Threshold,
}

impl ConditionalFormatCustomIcon {
    /// Creates the default lowest threshold, `Percent(0)`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            threshold: Threshold::new(
                ConditionalFormatType::Percent,
                ConditionalFormatValue::Number(0.0),
            ),
        }
    }

    /// Sets the threshold type and value.
    #[must_use]
    pub fn set_rule(
        mut self,
        rule_type: ConditionalFormatType,
        value: impl IntoConditionalFormatValue,
    ) -> Self {
        self.threshold = Threshold::new(rule_type, value.new_value());
        self
    }
}

impl Default for ConditionalFormatCustomIcon {
    fn default() -> Self {
        Self::new()
    }
}

/// An icon-set conditional format applied to a worksheet range.
#[derive(Clone, Debug)]
pub struct ConditionalFormatIconSet {
    icon_type: ConditionalFormatIconType,
    is_reversed: bool,
    icon_only: bool,
    icons: Vec<ConditionalFormatCustomIcon>,
}

impl ConditionalFormatIconSet {
    /// Creates an icon set with three traffic lights and Excel's defaults.
    #[must_use]
    pub fn new() -> Self {
        let icon_type = ConditionalFormatIconType::ThreeTrafficLights;
        Self {
            icon_type,
            is_reversed: false,
            icon_only: false,
            icons: icon_type.default_icons(),
        }
    }

    /// Sets the icon type and restores that set's default thresholds.
    #[must_use]
    pub fn set_icon_type(mut self, icon_type: ConditionalFormatIconType) -> Self {
        self.icon_type = icon_type;
        self.icons = icon_type.default_icons();
        self
    }

    /// Reverses the icon order.
    #[must_use]
    pub const fn reverse_icons(mut self, enable: bool) -> Self {
        self.is_reversed = enable;
        self
    }

    /// Hides cell values and shows only the icons.
    #[must_use]
    pub const fn show_icons_only(mut self, enable: bool) -> Self {
        self.icon_only = enable;
        self
    }

    /// Sets custom thresholds. The first is always forced to `Percent(0)`.
    #[must_use]
    pub fn set_icons(mut self, icons: &[ConditionalFormatCustomIcon]) -> Self {
        let mut icons = icons.to_vec();
        if let Some(first) = icons.first_mut() {
            first.threshold = Threshold::new(
                ConditionalFormatType::Percent,
                ConditionalFormatValue::Number(0.0),
            );
        }
        self.icons = icons;
        self
    }
}

impl Default for ConditionalFormatIconSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalFormat for ConditionalFormatIconSet {
    fn validate(&self) -> Result<()> {
        let expected = self.icon_type.count();
        if self.icons.len() != expected {
            return Err(Error::Parameter(format!(
                "icon-set rule needs {expected} thresholds, found {}",
                self.icons.len()
            )));
        }
        for icon in &self.icons {
            icon.threshold.icon_cfvo()?;
        }
        Ok(())
    }

    fn dxf_format(&self) -> Option<&Format> {
        None
    }

    fn begin_rule(&self, priority: i32, _dxf_id: u32) -> Result<BrtBeginCFRule> {
        Ok(BrtBeginCFRule {
            iType: CF_TYPE_MULTISTATE,
            iTemplate: CF_TEMPLATE_MULTISTATE,
            dxfId: u32::MAX,
            iPri: priority,
            iParam: 0,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        })
    }

    fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        let mut flags = 0;
        if self.icon_only {
            flags |= BrtBeginIconSet::F_ICON;
        }
        if self.is_reversed {
            flags |= BrtBeginIconSet::F_REVERSE;
        }
        push_record(
            out,
            &BrtBeginIconSet {
                iSet: self.icon_type.kpi(),
                flags,
            },
        )?;
        for icon in &self.icons {
            push_record(out, &icon.threshold.icon_cfvo()?)?;
        }
        push_record(out, &BrtEndIconSet::default())?;
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn ConditionalFormat + Send + Sync> {
        Box::new(self.clone())
    }
}

/// One worksheet conditional format and its target range.
pub struct ConditionalFormatRange {
    pub(crate) range: UncheckedRfX,
    conditional_format: Box<dyn ConditionalFormat + Send + Sync>,
}

impl ConditionalFormatRange {
    pub(crate) fn new(
        range: UncheckedRfX,
        conditional_format: Box<dyn ConditionalFormat + Send + Sync>,
    ) -> Self {
        Self {
            range,
            conditional_format,
        }
    }

    pub(crate) fn begin_rule(&self, priority: i32, dxf_id: u32) -> Result<BrtBeginCFRule> {
        self.conditional_format.begin_rule(priority, dxf_id)
    }

    pub(crate) fn write_body(&self, out: &mut Vec<u8>) -> Result<()> {
        self.conditional_format.write_body(out)
    }

    pub(crate) fn dxf_format(&self) -> Option<&Format> {
        self.conditional_format.dxf_format()
    }
}

#[cfg(test)]
mod tests {
    use deku::DekuContainerRead;
    use rust_xlsb_core::{BiffHead, KnownID};

    use super::*;

    fn records(data: &[u8]) -> Vec<(KnownID, &[u8])> {
        let mut result = Vec::new();
        let mut pos = 0;
        while pos < data.len() {
            let (used, head) = BiffHead::from_data(&data[pos..]).expect("record header");
            let start = pos + used;
            let end = start + head.size() as usize;
            result.push((head.known_id().expect("known record"), &data[start..end]));
            pos = end;
        }
        result
    }
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

    #[test]
    fn top_rules_map_rank_and_filter_flags() {
        for (rule, rank, bottom, percent) in [
            (ConditionalFormatTopRule::Top(10), 10, false, false),
            (ConditionalFormatTopRule::Bottom(9), 9, true, false),
            (ConditionalFormatTopRule::TopPercent(25), 25, false, true),
            (ConditionalFormatTopRule::BottomPercent(5), 5, true, true),
        ] {
            let record = ConditionalFormatTop::new()
                .set_rule(rule)
                .set_stop_if_true(true)
                .begin_rule(2, u32::MAX)
                .expect("rule");
            assert_eq!(record.iType, CF_TYPE_FILTER);
            assert_eq!(record.iTemplate, CF_TEMPLATE_FILTER);
            assert_eq!(record.iPri, 2);
            assert_eq!(record.iParam, rank);
            assert_eq!(record.cbFmla1, 0);
            assert!(record.formula1.is_none());
            assert!(record.stop_if_true());
            assert_eq!(record.flags & BrtBeginCFRule::F_BOTTOM != 0, bottom);
            assert_eq!(record.flags & BrtBeginCFRule::F_PERCENT != 0, percent);
        }
    }

    #[test]
    fn average_rules_map_templates_and_above_flag() {
        for (rule, template, parameter, above) in [
            (
                ConditionalFormatAverageRule::AboveAverage,
                CF_TEMPLATE_ABOVE_AVERAGE,
                0,
                true,
            ),
            (
                ConditionalFormatAverageRule::BelowAverage,
                CF_TEMPLATE_BELOW_AVERAGE,
                0,
                false,
            ),
            (
                ConditionalFormatAverageRule::EqualOrAboveAverage,
                CF_TEMPLATE_EQUAL_ABOVE_AVERAGE,
                0,
                true,
            ),
            (
                ConditionalFormatAverageRule::EqualOrBelowAverage,
                CF_TEMPLATE_EQUAL_BELOW_AVERAGE,
                0,
                false,
            ),
            (
                ConditionalFormatAverageRule::OneStandardDeviationAbove,
                CF_TEMPLATE_ABOVE_AVERAGE,
                1,
                true,
            ),
            (
                ConditionalFormatAverageRule::OneStandardDeviationBelow,
                CF_TEMPLATE_BELOW_AVERAGE,
                1,
                false,
            ),
            (
                ConditionalFormatAverageRule::TwoStandardDeviationsAbove,
                CF_TEMPLATE_ABOVE_AVERAGE,
                2,
                true,
            ),
            (
                ConditionalFormatAverageRule::TwoStandardDeviationsBelow,
                CF_TEMPLATE_BELOW_AVERAGE,
                2,
                false,
            ),
            (
                ConditionalFormatAverageRule::ThreeStandardDeviationsAbove,
                CF_TEMPLATE_ABOVE_AVERAGE,
                3,
                true,
            ),
            (
                ConditionalFormatAverageRule::ThreeStandardDeviationsBelow,
                CF_TEMPLATE_BELOW_AVERAGE,
                3,
                false,
            ),
        ] {
            let record = ConditionalFormatAverage::new()
                .set_rule(rule)
                .begin_rule(1, 0)
                .expect("rule");
            assert_eq!(record.iType, CF_TYPE_EXPRESSION);
            assert_eq!(record.iTemplate, template);
            assert_eq!(record.iParam, parameter);
            assert_eq!(record.flags & BrtBeginCFRule::F_ABOVE != 0, above);
            assert_eq!(record.cbFmla1, 0);
            assert!(record.formula1.is_none());
        }
    }

    #[test]
    fn duplicate_rules_support_unique_inversion() {
        let duplicate = ConditionalFormatDuplicate::new()
            .begin_rule(1, 0)
            .expect("rule");
        assert_eq!(duplicate.iTemplate, CF_TEMPLATE_DUPLICATE_VALUES);
        assert_eq!(duplicate.iParam, 0);
        assert_eq!(duplicate.cbFmla1, 0);

        let unique = ConditionalFormatDuplicate::new()
            .invert()
            .begin_rule(1, 0)
            .expect("rule");
        assert_eq!(unique.iTemplate, CF_TEMPLATE_UNIQUE_VALUES);
    }

    #[test]
    fn top_rules_validate_rank_and_percent_ranges() {
        for rule in [
            ConditionalFormatTopRule::Top(1),
            ConditionalFormatTopRule::Top(1000),
            ConditionalFormatTopRule::Bottom(1),
            ConditionalFormatTopRule::Bottom(1000),
            ConditionalFormatTopRule::TopPercent(1),
            ConditionalFormatTopRule::TopPercent(100),
            ConditionalFormatTopRule::BottomPercent(1),
            ConditionalFormatTopRule::BottomPercent(100),
        ] {
            assert!(
                ConditionalFormatTop::new()
                    .set_rule(rule)
                    .validate()
                    .is_ok(),
                "{rule:?}"
            );
        }
        for rule in [
            ConditionalFormatTopRule::Top(0),
            ConditionalFormatTopRule::Bottom(1001),
            ConditionalFormatTopRule::TopPercent(0),
            ConditionalFormatTopRule::BottomPercent(101),
        ] {
            assert!(
                ConditionalFormatTop::new()
                    .set_rule(rule)
                    .validate()
                    .is_err(),
                "{rule:?}"
            );
        }
    }

    #[test]
    fn two_color_scale_uses_default_thresholds_and_colors() {
        let scale = ConditionalFormat2ColorScale::new();
        scale.validate().expect("valid scale");
        let rule = scale.begin_rule(4, u32::MAX).expect("rule");
        assert_eq!(rule.iType, CF_TYPE_GRADIENT);
        assert_eq!(rule.iTemplate, CF_TEMPLATE_GRADIENT);
        assert_eq!(rule.dxfId, u32::MAX);
        assert_eq!(rule.iPri, 4);
        assert_eq!(rule.iParam, 0);
        assert_eq!(rule.flags, 0);
        assert_eq!(rule.cbFmla1, 0);
        assert!(rule.formula1.is_none());

        let mut body = Vec::new();
        scale.write_body(&mut body).expect("body");
        assert_eq!(body.len(), 82);
        let records = records(&body);
        assert_eq!(
            records.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [
                KnownID::BrtBeginColorScale,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtEndColorScale,
            ]
        );

        let (_, min) = BrtCFVO::from_bytes((records[1].1, 0)).expect("minimum");
        assert_eq!(min.iType, CFVO_MIN);
        assert_eq!(f64::from(min.numParam), 0.0);
        let (_, max) = BrtCFVO::from_bytes((records[2].1, 0)).expect("maximum");
        assert_eq!(max.iType, CFVO_MAX);
        let (_, min_color) = BrtColor::from_bytes((records[3].1, 0)).expect("minimum color");
        assert_eq!(min_color, BrtColor::rgb(0xFF, 0xEF, 0x9C));
        let (_, max_color) = BrtColor::from_bytes((records[4].1, 0)).expect("maximum color");
        assert_eq!(max_color, BrtColor::rgb(0x63, 0xBE, 0x7B));
    }

    #[test]
    fn three_color_scale_maps_thresholds_and_colors() {
        let scale = ConditionalFormat3ColorScale::new()
            .set_minimum(ConditionalFormatType::Number, 2)
            .set_midpoint(ConditionalFormatType::Percent, 60)
            .set_maximum(ConditionalFormatType::Percentile, 90)
            .set_minimum_color(Color::Red)
            .set_midpoint_color(Color::White)
            .set_maximum_color(Color::Blue);
        scale.validate().expect("valid scale");

        let mut body = Vec::new();
        scale.write_body(&mut body).expect("body");
        let records = records(&body);
        assert_eq!(
            records.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [
                KnownID::BrtBeginColorScale,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtEndColorScale,
            ]
        );

        let (_, min) = BrtCFVO::from_bytes((records[1].1, 0)).expect("minimum");
        assert_eq!(min.iType, CFVO_NUM);
        assert_eq!(f64::from(min.numParam), 2.0);
        let (_, mid) = BrtCFVO::from_bytes((records[2].1, 0)).expect("midpoint");
        assert_eq!(mid.iType, CFVO_PERCENT);
        assert_eq!(f64::from(mid.numParam), 60.0);
        let (_, max) = BrtCFVO::from_bytes((records[3].1, 0)).expect("maximum");
        assert_eq!(max.iType, CFVO_PERCENTILE);
        assert_eq!(f64::from(max.numParam), 90.0);

        let (_, start_color) = BrtColor::from_bytes((records[4].1, 0)).expect("minimum color");
        assert_eq!(start_color, BrtColor::rgb(0xFF, 0x00, 0x00));
        let (_, middle_color) = BrtColor::from_bytes((records[5].1, 0)).expect("midpoint color");
        assert_eq!(middle_color, BrtColor::rgb(0xFF, 0xFF, 0xFF));
        let (_, end_color) = BrtColor::from_bytes((records[6].1, 0)).expect("maximum color");
        assert_eq!(end_color, BrtColor::rgb(0x00, 0x00, 0xFF));
    }

    #[test]
    fn three_color_scale_defaults_to_percentile_midpoint() {
        let scale = ConditionalFormat3ColorScale::new();
        scale.validate().expect("valid scale");
        let mut body = Vec::new();
        scale.write_body(&mut body).expect("body");
        let records = records(&body);
        let (_, mid) = BrtCFVO::from_bytes((records[2].1, 0)).expect("midpoint");
        assert_eq!(mid.iType, CFVO_PERCENTILE);
        assert_eq!(f64::from(mid.numParam), 50.0);
    }

    #[test]
    fn data_bar_uses_min_max_thresholds_color_and_show_value() {
        let bar = ConditionalFormatDataBar::new()
            .set_minimum(ConditionalFormatType::Number, 3)
            .set_maximum(ConditionalFormatType::Percent, 90)
            .set_fill_color(Color::Green);
        bar.validate().expect("valid data bar");
        let rule = bar.begin_rule(3, 0).expect("rule");
        assert_eq!(rule.iType, CF_TYPE_DATABAR);
        assert_eq!(rule.iTemplate, CF_TEMPLATE_DATABAR);
        assert_eq!(rule.dxfId, u32::MAX);
        assert_eq!(rule.iPri, 3);
        assert_eq!(rule.iParam, 0);
        assert_eq!(rule.flags, 0);
        assert_eq!(rule.cbFmla1, 0);
        assert!(rule.formula1.is_none());

        let mut body = Vec::new();
        bar.write_body(&mut body).expect("body");
        let nested = records(&body);
        assert_eq!(
            nested.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [
                KnownID::BrtBeginDatabar,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtEndDatabar,
            ]
        );
        let (_, begin) = BrtBeginDatabar::from_bytes((nested[0].1, 0)).expect("begin");
        assert_eq!(begin.bLenMin, 10);
        assert_eq!(begin.bLenMax, 90);
        assert_eq!(begin.fShowValue, 1);
        let (_, min) = BrtCFVO::from_bytes((nested[1].1, 0)).expect("minimum");
        assert_eq!(min.iType, CFVO_NUM);
        assert_eq!(f64::from(min.numParam), 3.0);
        let (_, max) = BrtCFVO::from_bytes((nested[2].1, 0)).expect("maximum");
        assert_eq!(max.iType, CFVO_PERCENT);
        assert_eq!(f64::from(max.numParam), 90.0);
        let (_, color) = BrtColor::from_bytes((nested[3].1, 0)).expect("color");
        assert_eq!(color, BrtColor::rgb(0x00, 0x80, 0x00));

        let mut body = Vec::new();
        ConditionalFormatDataBar::new()
            .set_bar_only(true)
            .write_body(&mut body)
            .expect("bar-only body");
        let (_, begin) =
            BrtBeginDatabar::from_bytes((records(&body)[0].1, 0)).expect("bar-only begin");
        assert_eq!(begin.fShowValue, 0);
    }

    #[test]
    fn data_bar_ignores_invalid_fill_color_and_accepts_bounds() {
        let bar = ConditionalFormatDataBar::new().set_fill_color(Color::RGB(0x0100_0000));
        bar.validate().expect("invalid color leaves the default");
        let mut body = Vec::new();
        bar.write_body(&mut body).expect("body");
        let nested = records(&body);
        let (_, color) = BrtColor::from_bytes((nested[3].1, 0)).expect("color");
        assert_eq!(color, BrtColor::rgb(0x63, 0x8E, 0xC6));

        for bar in [
            ConditionalFormatDataBar::new()
                .set_minimum(ConditionalFormatType::Percent, 0)
                .set_maximum(ConditionalFormatType::Percent, 100),
            ConditionalFormatDataBar::new()
                .set_minimum(ConditionalFormatType::Percentile, 0)
                .set_maximum(ConditionalFormatType::Percentile, 100),
            ConditionalFormatDataBar::new()
                .set_minimum(ConditionalFormatType::Number, -1.5)
                .set_maximum(ConditionalFormatType::Number, 1.5),
            ConditionalFormatDataBar::new()
                .set_minimum(ConditionalFormatType::Lowest, 0)
                .set_maximum(ConditionalFormatType::Highest, 0),
        ] {
            bar.validate().expect("valid boundary data bar");
        }
    }

    #[test]
    fn data_bar_rejects_bad_thresholds_and_implicit_color() {
        for bar in [
            ConditionalFormatDataBar::new().set_minimum(ConditionalFormatType::Highest, 1),
            ConditionalFormatDataBar::new().set_maximum(ConditionalFormatType::Lowest, 1),
            ConditionalFormatDataBar::new().set_minimum(ConditionalFormatType::Percent, 101),
            ConditionalFormatDataBar::new().set_maximum(ConditionalFormatType::Percentile, -1),
            ConditionalFormatDataBar::new().set_minimum(ConditionalFormatType::Number, "text"),
            ConditionalFormatDataBar::new().set_fill_color(Color::Default),
        ] {
            assert!(bar.validate().is_err(), "{bar:?}");
        }
    }

    #[test]
    fn text_rules_map_operators_strings_and_formulas() {
        for (rule, parameter, formula) in [
            (
                ConditionalFormatTextRule::Contains("foo".to_owned()),
                CF_TEXTOPER_CONTAINS,
                &[
                    0x17, 3, 0, b'f', 0, b'o', 0, b'o', 0, 0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x42, 2,
                    0x52, 0, 0x41, 3, 0, 0x41, 0x26, 0,
                ][..],
            ),
            (
                ConditionalFormatTextRule::DoesNotContain("foo".to_owned()),
                CF_TEXTOPER_NOT_CONTAINS,
                &[
                    0x17, 3, 0, b'f', 0, b'o', 0, b'o', 0, 0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x42, 2,
                    0x52, 0, 0x41, 3, 0,
                ][..],
            ),
            (
                ConditionalFormatTextRule::BeginsWith("foo".to_owned()),
                CF_TEXTOPER_BEGINS_WITH,
                &[
                    0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x1E, 3, 0, 0x42, 2, 0x73, 0, 0x17, 3, 0, b'f',
                    0, b'o', 0, b'o', 0, 0x0B,
                ][..],
            ),
            (
                ConditionalFormatTextRule::EndsWith("foo".to_owned()),
                CF_TEXTOPER_ENDS_WITH,
                &[
                    0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x1E, 3, 0, 0x42, 2, 0x74, 0, 0x17, 3, 0, b'f',
                    0, b'o', 0, b'o', 0, 0x0B,
                ][..],
            ),
        ] {
            let text = ConditionalFormatText::new()
                .set_rule(rule)
                .set_stop_if_true(true);
            text.validate().expect("valid text rule");
            let record = text.begin_rule(4, 7).expect("rule");
            assert_eq!(record.iType, CF_TYPE_EXPRESSION);
            assert_eq!(record.iTemplate, CF_TEMPLATE_CONTAINS_TEXT);
            assert_eq!(record.dxfId, 7);
            assert_eq!(record.iPri, 4);
            assert_eq!(record.iParam, parameter);
            assert!(record.stop_if_true());
            assert_eq!(record.strParam.to_optional_string().as_deref(), Some("foo"));
            let parsed = record.formula1.as_ref().expect("formula");
            assert_eq!(parsed.cce, u32::try_from(formula.len()).expect("length"));
            assert_eq!(record.cbFmla1, 1);
            assert_eq!(parsed.rgce, formula);
            assert_eq!(parsed.cb, 0);
            assert_eq!(parsed.rgcb.len(), 0);
            assert_eq!(record.cbFmla2, 0);
            assert!(record.formula2.is_none());
        }
    }

    #[test]
    fn text_rule_lengths_are_utf16_units() {
        let text = ConditionalFormatText::new()
            .set_rule(ConditionalFormatTextRule::BeginsWith("é".to_owned()));
        let record = text.begin_rule(1, u32::MAX).expect("rule");
        let formula = record.formula1.as_ref().expect("formula");
        assert_eq!(&formula.rgce[7..10], &[0x1E, 1, 0]);

        let text = ConditionalFormatText::new()
            .set_rule(ConditionalFormatTextRule::EndsWith("😀".to_owned()));
        let record = text.begin_rule(1, u32::MAX).expect("rule");
        let formula = record.formula1.as_ref().expect("formula");
        assert_eq!(&formula.rgce[7..10], &[0x1E, 2, 0]);
    }

    #[test]
    fn text_rule_preserves_non_ascii_utf16_bytes_and_unformatted_dxf() {
        let text = ConditionalFormatText::new()
            .set_rule(ConditionalFormatTextRule::BeginsWith("😀".to_owned()));
        let record = text.begin_rule(1, u32::MAX).expect("rule");
        assert_eq!(record.dxfId, u32::MAX);
        assert!(text.dxf_format().is_none());
        assert_eq!(
            record.formula1.expect("formula").rgce,
            [
                0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x1E, 2, 0, 0x42, 2, 0x73, 0, 0x17, 2, 0, 0x3D, 0xD8,
                0x00, 0xDE, 0x0B,
            ]
        );
    }

    #[test]
    fn text_rules_validate_utf16_length_bounds() {
        assert!(ConditionalFormatText::new().validate().is_err());
        assert!(
            ConditionalFormatText::new()
                .set_rule(ConditionalFormatTextRule::Contains(String::new()))
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormatText::new()
                .set_rule(ConditionalFormatTextRule::Contains("a".repeat(256)))
                .validate()
                .is_err()
        );
        ConditionalFormatText::new()
            .set_rule(ConditionalFormatTextRule::Contains("a".repeat(255)))
            .validate()
            .expect("255 UTF-16 units");
        assert!(
            ConditionalFormatText::new()
                .set_rule(ConditionalFormatTextRule::Contains(String::new()))
                .begin_rule(1, u32::MAX)
                .is_err()
        );
    }

    fn assert_formula_rule(rule: &impl ConditionalFormat, template: u32, formula: &[u8]) {
        rule.validate().expect("valid rule");
        let record = rule.begin_rule(2, 5).expect("rule");
        assert_eq!(record.iType, CF_TYPE_EXPRESSION);
        assert_eq!(record.iTemplate, template);
        assert_eq!(record.dxfId, 5);
        assert_eq!(record.iPri, 2);
        assert_eq!(record.iParam, 0);
        assert!(record.stop_if_true());
        assert!(record.strParam.is_null());
        let parsed = record.formula1.as_ref().expect("formula");
        assert_eq!(record.cbFmla1, 1);
        assert_eq!(parsed.rgce, formula);
        assert_eq!(record.cbFmla2, 0);
        assert!(record.formula2.is_none());
    }

    #[test]
    fn blank_and_error_rules_map_templates_and_formulas() {
        for (rule, template, formula) in [
            (
                ConditionalFormatBlank::new(),
                CF_TEMPLATE_CONTAINS_BLANKS,
                &[
                    0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 0x76, 0, 0x41, 0x20, 0, 0x1E, 0, 0, 0x0B,
                ][..],
            ),
            (
                ConditionalFormatBlank::new().invert(),
                CF_TEMPLATE_CONTAINS_NO_BLANKS,
                &[
                    0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 0x76, 0, 0x41, 0x20, 0, 0x1E, 0, 0, 0x0D,
                ][..],
            ),
        ] {
            assert_formula_rule(&rule.set_stop_if_true(true), template, formula);
        }

        for (rule, template, formula) in [
            (
                ConditionalFormatError::new(),
                CF_TEMPLATE_CONTAINS_ERRORS,
                &[0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 3, 0][..],
            ),
            (
                ConditionalFormatError::new().invert(),
                CF_TEMPLATE_CONTAINS_NO_ERRORS,
                &[0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 3, 0, 0x41, 0x26, 0][..],
            ),
        ] {
            assert_formula_rule(&rule.set_stop_if_true(true), template, formula);
        }
    }

    #[test]
    fn icon_sets_emit_kpi_defaults_and_end_markers() {
        let cases: &[(ConditionalFormatIconType, u32, usize, &[f64])] = &[
            (
                ConditionalFormatIconType::ThreeArrows,
                0,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeArrowsGray,
                1,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeFlags,
                2,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeTrafficLights,
                3,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeTrafficLightsWithRim,
                4,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeSigns,
                5,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeSymbolsCircled,
                6,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::ThreeSymbols,
                7,
                3,
                &[0.0, 33.0, 67.0],
            ),
            (
                ConditionalFormatIconType::FourArrows,
                8,
                4,
                &[0.0, 25.0, 50.0, 75.0],
            ),
            (
                ConditionalFormatIconType::FourArrowsGray,
                9,
                4,
                &[0.0, 25.0, 50.0, 75.0],
            ),
            (
                ConditionalFormatIconType::FourRedToBlack,
                10,
                4,
                &[0.0, 25.0, 50.0, 75.0],
            ),
            (
                ConditionalFormatIconType::FourHistograms,
                11,
                4,
                &[0.0, 25.0, 50.0, 75.0],
            ),
            (
                ConditionalFormatIconType::FourTrafficLights,
                12,
                4,
                &[0.0, 25.0, 50.0, 75.0],
            ),
            (
                ConditionalFormatIconType::FiveArrows,
                13,
                5,
                &[0.0, 20.0, 40.0, 60.0, 80.0],
            ),
            (
                ConditionalFormatIconType::FiveArrowsGray,
                14,
                5,
                &[0.0, 20.0, 40.0, 60.0, 80.0],
            ),
            (
                ConditionalFormatIconType::FiveHistograms,
                15,
                5,
                &[0.0, 20.0, 40.0, 60.0, 80.0],
            ),
            (
                ConditionalFormatIconType::FiveQuadrants,
                16,
                5,
                &[0.0, 20.0, 40.0, 60.0, 80.0],
            ),
        ];
        for &(icon_type, kpi, count, expected) in cases {
            let rule = ConditionalFormatIconSet::new().set_icon_type(icon_type);
            rule.validate().expect("valid icon set");
            let record = rule.begin_rule(3, u32::MAX).expect("rule");
            assert_eq!(record.iType, CF_TYPE_MULTISTATE);
            assert_eq!(record.iTemplate, CF_TEMPLATE_MULTISTATE);
            assert_eq!(record.dxfId, u32::MAX);
            assert_eq!(record.iPri, 3);
            assert_eq!(record.iParam, 0);
            assert_eq!(record.flags, 0);
            assert_eq!(record.cbFmla1, 0);
            assert!(record.formula1.is_none());

            let mut body = Vec::new();
            rule.write_body(&mut body).expect("body");
            let nested = records(&body);
            assert_eq!(nested.len(), count + 2);
            assert_eq!(nested[0].0, KnownID::BrtBeginIconSet);
            let begin = BrtBeginIconSet::from_bytes((nested[0].1, 0))
                .expect("begin")
                .1;
            assert_eq!(begin.iSet, kpi);
            assert_eq!(begin.flags, 0);
            assert_eq!(nested[count + 1].0, KnownID::BrtEndIconSet);

            for (index, threshold) in nested[1..=count].iter().enumerate() {
                assert_eq!(threshold.0, KnownID::BrtCFVO);
                let cfvo = BrtCFVO::from_bytes((threshold.1, 0)).expect("cfvo").1;
                assert_eq!(cfvo.iType, CFVO_PERCENT);
                assert_eq!(f64::from(cfvo.numParam), expected[index]);
                assert_eq!(cfvo.fSaveGTE, 1);
                assert_eq!(cfvo.fGTE, 1);
                assert_eq!(cfvo.cbFmla, 0);
                assert!(cfvo.formula.is_none());
            }
        }
    }

    #[test]
    fn icon_sets_encode_reverse_and_icon_only_flags() {
        for (reverse, icon_only, expected) in [
            (true, false, BrtBeginIconSet::F_REVERSE),
            (false, true, BrtBeginIconSet::F_ICON),
            (
                true,
                true,
                BrtBeginIconSet::F_ICON | BrtBeginIconSet::F_REVERSE,
            ),
        ] {
            let rule = ConditionalFormatIconSet::new()
                .reverse_icons(reverse)
                .show_icons_only(icon_only);
            rule.validate().expect("valid icon set");
            let mut body = Vec::new();
            rule.write_body(&mut body).expect("body");
            let nested = records(&body);
            let begin = BrtBeginIconSet::from_bytes((nested[0].1, 0))
                .expect("begin")
                .1;
            assert_eq!(begin.flags, expected);
        }
    }

    #[test]
    fn icon_sets_accept_custom_thresholds_and_force_first_to_zero() {
        let icons = [
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percent, 99),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percentile, 50),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, 7),
        ];
        let rule = ConditionalFormatIconSet::new().set_icons(&icons);
        rule.validate().expect("valid custom thresholds");
        let mut body = Vec::new();
        rule.write_body(&mut body).expect("body");
        let nested = records(&body);
        let cfvos = nested[1..4]
            .iter()
            .map(|(_, bytes)| BrtCFVO::from_bytes((*bytes, 0)).expect("cfvo").1)
            .collect::<Vec<_>>();
        assert_eq!(
            cfvos.iter().map(|cfvo| cfvo.iType).collect::<Vec<_>>(),
            [CFVO_PERCENT, CFVO_PERCENTILE, CFVO_NUM]
        );
        assert_eq!(
            cfvos
                .iter()
                .map(|cfvo| f64::from(cfvo.numParam))
                .collect::<Vec<_>>(),
            [0.0, 50.0, 7.0]
        );
    }

    #[test]
    fn icon_sets_reject_bad_thresholds_and_counts() {
        for rule in [
            ConditionalFormatIconSet::new().set_icons(&[]),
            ConditionalFormatIconSet::new().set_icons(&[ConditionalFormatCustomIcon::new()]),
            ConditionalFormatIconSet::new().set_icons(&[
                ConditionalFormatCustomIcon::new(),
                ConditionalFormatCustomIcon::new(),
                ConditionalFormatCustomIcon::new(),
                ConditionalFormatCustomIcon::new(),
            ]),
        ] {
            assert!(rule.validate().is_err());
        }

        for icon in [
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percent, 101),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percentile, -1),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, f64::NAN),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Number, "text"),
            ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Lowest, 0),
        ] {
            let rule = ConditionalFormatIconSet::new().set_icons(&[
                ConditionalFormatCustomIcon::new(),
                ConditionalFormatCustomIcon::new(),
                icon,
            ]);
            assert!(rule.write_body(&mut Vec::new()).is_err());
        }
    }

    #[test]
    fn color_scale_validation_rejects_bad_thresholds_and_stop() {
        assert!(
            ConditionalFormat2ColorScale::new()
                .set_minimum(ConditionalFormatType::Highest, 1)
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat2ColorScale::new()
                .set_maximum(ConditionalFormatType::Lowest, 1)
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat2ColorScale::new()
                .set_minimum(ConditionalFormatType::Percent, 101)
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat3ColorScale::new()
                .set_midpoint(ConditionalFormatType::Lowest, 0)
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat3ColorScale::new()
                .set_midpoint(ConditionalFormatType::Percentile, -1)
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat2ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, "text")
                .validate()
                .is_err()
        );
        assert!(
            ConditionalFormat2ColorScale::new()
                .set_stop_if_true(true)
                .validate()
                .is_err()
        );
    }
}
