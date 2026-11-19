#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec length field is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::prelude::{UncheckedRfX, UncheckedSqRfX, XlNullableWideString, Xnum};
use crate::{Checked, Unchecked, assign_id};

/// `2.5.98.6`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CFParsedFormula {
    pub cce: u32,
    #[deku(count = "cce")]
    pub rgce: Vec<u8>,
    #[deku(update = "self.rgcb.len() as u32")]
    pub cb: u32,
    #[deku(count = "cb")]
    pub rgcb: Vec<u8>,
}

impl Default for CFParsedFormula {
    fn default() -> Self {
        Self {
            cce: 9,
            rgce: vec![0x1F, 0, 0, 0, 0, 0, 0, 0, 0],
            cb: 0,
            rgcb: Vec::new(),
        }
    }
}

impl Checked for CFParsedFormula {
    fn check(&self) -> Result<(), &'static str> {
        if self.cce == 0 || self.cce >= 16_385 {
            return Err("conditional-format formula size is outside 1..=16384");
        }
        if usize::try_from(self.cce).ok() != Some(self.rgce.len()) {
            return Err("conditional-format formula size does not match its bytes");
        }
        if usize::try_from(self.cb).ok() != Some(self.rgcb.len()) {
            return Err("conditional-format extra data size does not match its bytes");
        }
        Ok(())
    }
}

/// `CF_TYPE_CELLIS`.
pub const CF_TYPE_CELL_IS: u32 = 1;
/// `CF_TYPE_EXPRIS`.
pub const CF_TYPE_EXPRESSION: u32 = 2;
/// `CF_TYPE_GRADIENT`.
pub const CF_TYPE_GRADIENT: u32 = 3;
/// `CF_TYPE_DATABAR`.
pub const CF_TYPE_DATABAR: u32 = 4;
/// `CF_TYPE_FILTER`.
pub const CF_TYPE_FILTER: u32 = 5;
/// `CF_TYPE_MULTISTATE`.
pub const CF_TYPE_MULTISTATE: u32 = 6;

/// `CF_TEMPLATE_EXPR`.
pub const CF_TEMPLATE_EXPR: u32 = 0;
/// `CF_TEMPLATE_FMLA`.
pub const CF_TEMPLATE_FMLA: u32 = 1;
/// `CF_TEMPLATE_GRADIENT`.
pub const CF_TEMPLATE_GRADIENT: u32 = 2;
/// `CF_TEMPLATE_DATABAR`.
pub const CF_TEMPLATE_DATABAR: u32 = 3;
/// `CF_TEMPLATE_MULTISTATE`.
pub const CF_TEMPLATE_MULTISTATE: u32 = 4;
/// `CF_TEMPLATE_FILTER`.
pub const CF_TEMPLATE_FILTER: u32 = 5;
/// `CF_TEMPLATE_UNIQUEVALUES`.
pub const CF_TEMPLATE_UNIQUE_VALUES: u32 = 7;
/// `CF_TEMPLATE_CONTAINSTEXT`.
pub const CF_TEMPLATE_CONTAINS_TEXT: u32 = 0x08;
/// `CF_TEMPLATE_CONTAINSBLANKS`.
pub const CF_TEMPLATE_CONTAINS_BLANKS: u32 = 0x09;
/// `CF_TEMPLATE_CONTAINSNOBLANKS`.
pub const CF_TEMPLATE_CONTAINS_NO_BLANKS: u32 = 0x0A;
/// `CF_TEMPLATE_CONTAINSERRORS`.
pub const CF_TEMPLATE_CONTAINS_ERRORS: u32 = 0x0B;
/// `CF_TEMPLATE_CONTAINSNOERRORS`.
pub const CF_TEMPLATE_CONTAINS_NO_ERRORS: u32 = 0x0C;
/// `CF_TEMPLATE_TIMEPERIODTODAY`.
pub const CF_TEMPLATE_TIME_PERIOD_TODAY: u32 = 0x0F;
/// `CF_TEMPLATE_TIMEPERIODTOMORROW`.
pub const CF_TEMPLATE_TIME_PERIOD_TOMORROW: u32 = 0x10;
/// `CF_TEMPLATE_TIMEPERIODYESTERDAY`.
pub const CF_TEMPLATE_TIME_PERIOD_YESTERDAY: u32 = 0x11;
/// `CF_TEMPLATE_TIMEPERIODLAST7DAYS`.
pub const CF_TEMPLATE_TIME_PERIOD_LAST7DAYS: u32 = 0x12;
/// `CF_TEMPLATE_TIMEPERIODLASTMONTH`.
pub const CF_TEMPLATE_TIME_PERIOD_LAST_MONTH: u32 = 0x13;
/// `CF_TEMPLATE_TIMEPERIODNEXTMONTH`.
pub const CF_TEMPLATE_TIME_PERIOD_NEXT_MONTH: u32 = 0x14;
/// `CF_TEMPLATE_TIMEPERIODTHISWEEK`.
pub const CF_TEMPLATE_TIME_PERIOD_THIS_WEEK: u32 = 0x15;
/// `CF_TEMPLATE_TIMEPERIODNEXTWEEK`.
pub const CF_TEMPLATE_TIME_PERIOD_NEXT_WEEK: u32 = 0x16;
/// `CF_TEMPLATE_TIMEPERIODLASTWEEK`.
pub const CF_TEMPLATE_TIME_PERIOD_LAST_WEEK: u32 = 0x17;
/// `CF_TEMPLATE_TIMEPERIODTHISMONTH`.
pub const CF_TEMPLATE_TIME_PERIOD_THIS_MONTH: u32 = 0x18;
/// `CF_TEMPLATE_ABOVEAVERAGE`.
pub const CF_TEMPLATE_ABOVE_AVERAGE: u32 = 0x19;
/// `CF_TEMPLATE_BELOWAVERAGE`.
pub const CF_TEMPLATE_BELOW_AVERAGE: u32 = 0x1A;
/// `CF_TEMPLATE_DUPLICATEVALUES`.
pub const CF_TEMPLATE_DUPLICATE_VALUES: u32 = 0x1B;
/// `CF_TEMPLATE_EQUALABOVEAVERAGE`.
pub const CF_TEMPLATE_EQUAL_ABOVE_AVERAGE: u32 = 0x1D;
/// `CF_TEMPLATE_EQUALBELOWAVERAGE`.
pub const CF_TEMPLATE_EQUAL_BELOW_AVERAGE: u32 = 0x1E;

/// `CF_TEXTOPER_CONTAINS`.
pub const CF_TEXTOPER_CONTAINS: u32 = 0;
/// `CF_TEXTOPER_NOTCONTAINS`.
pub const CF_TEXTOPER_NOT_CONTAINS: u32 = 1;
/// `CF_TEXTOPER_BEGINSWITH`.
pub const CF_TEXTOPER_BEGINS_WITH: u32 = 2;
/// `CF_TEXTOPER_ENDSWITH`.
pub const CF_TEXTOPER_ENDS_WITH: u32 = 3;

/// `CFVONUM`.
pub const CFVO_NUM: u32 = 1;
/// `CFVOMIN`.
pub const CFVO_MIN: u32 = 2;
/// `CFVOMAX`.
pub const CFVO_MAX: u32 = 3;
/// `CFVOPERCENT`.
pub const CFVO_PERCENT: u32 = 4;
/// `CFVOPERCENTILE`.
pub const CFVO_PERCENTILE: u32 = 5;

/// `2.4.23`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginCFRule {
    pub iType: u32,
    pub iTemplate: u32,
    pub dxfId: u32,
    pub iPri: i32,
    pub iParam: u32,
    pub reserved1: u32,
    pub reserved2: u32,
    /// Packed flags: bit 1 is `fStopTrue`, bit 2 `fAbove`, bit 3 `fBottom`,
    /// bit 4 `fPercent`; all other bits are reserved.
    pub flags: u16,
    pub cbFmla1: u32,
    pub cbFmla2: u32,
    pub cbFmla3: u32,
    pub strParam: XlNullableWideString,
    #[deku(cond = "*cbFmla1 != 0", default = "None")]
    pub formula1: Option<CFParsedFormula>,
    #[deku(cond = "*cbFmla2 != 0", default = "None")]
    pub formula2: Option<CFParsedFormula>,
    #[deku(cond = "*cbFmla3 != 0", default = "None")]
    pub formula3: Option<CFParsedFormula>,
}

impl BrtBeginCFRule {
    /// `fStopTrue` flag.
    pub const F_STOP_TRUE: u16 = 1 << 1;
    /// `fAbove` flag.
    pub const F_ABOVE: u16 = 1 << 2;
    /// `fBottom` flag.
    pub const F_BOTTOM: u16 = 1 << 3;
    /// `fPercent` flag.
    pub const F_PERCENT: u16 = 1 << 4;
    const FLAG_MASK: u16 = Self::F_STOP_TRUE | Self::F_ABOVE | Self::F_BOTTOM | Self::F_PERCENT;

    #[must_use]
    pub const fn stop_if_true(&self) -> bool {
        self.flags & Self::F_STOP_TRUE != 0
    }

    pub const fn set_stop_if_true(&mut self, enable: bool) {
        if enable {
            self.flags |= Self::F_STOP_TRUE;
        } else {
            self.flags &= !Self::F_STOP_TRUE;
        }
    }

    const fn check_no_formulas(&self) -> Result<(), &'static str> {
        if self.cbFmla1 != 0
            || self.cbFmla2 != 0
            || self.cbFmla3 != 0
            || self.formula1.is_some()
            || self.formula2.is_some()
            || self.formula3.is_some()
        {
            return Err("this conditional-format rule must not contain formulas");
        }
        Ok(())
    }
}

impl Default for BrtBeginCFRule {
    fn default() -> Self {
        Self {
            iType: 1,
            iTemplate: 0,
            dxfId: u32::MAX,
            iPri: 1,
            iParam: 3,
            reserved1: 0,
            reserved2: 0,
            flags: 0,
            cbFmla1: 9,
            cbFmla2: 0,
            cbFmla3: 0,
            strParam: XlNullableWideString::default(),
            formula1: Some(CFParsedFormula::default()),
            formula2: None,
            formula3: None,
        }
    }
}

impl Checked for BrtBeginCFRule {
    fn check(&self) -> Result<(), &'static str> {
        if self.iPri <= 0 {
            return Err("conditional-format priority must be positive");
        }
        if self.reserved1 != 0
            || self.reserved2 != 0
            || self.flags & !Self::FLAG_MASK != 0
            || self.cbFmla3 != 0
            || self.formula3.is_some()
            || (!self.strParam.is_null() && self.iTemplate != CF_TEMPLATE_CONTAINS_TEXT)
        {
            return Err("conditional-format reserved fields must be zero");
        }

        match (self.iType, self.iTemplate) {
            (CF_TYPE_CELL_IS, CF_TEMPLATE_EXPR) => self.check_cell_rule(),
            (CF_TYPE_EXPRESSION, CF_TEMPLATE_FMLA) => self.check_formula_rule(),
            (CF_TYPE_EXPRESSION, CF_TEMPLATE_CONTAINS_TEXT) => self.check_text_rule(),
            (
                CF_TYPE_EXPRESSION,
                CF_TEMPLATE_CONTAINS_BLANKS
                | CF_TEMPLATE_CONTAINS_NO_BLANKS
                | CF_TEMPLATE_CONTAINS_ERRORS
                | CF_TEMPLATE_CONTAINS_NO_ERRORS,
            ) => self.check_fixed_formula_rule(),
            (
                CF_TYPE_EXPRESSION,
                CF_TEMPLATE_TIME_PERIOD_TODAY
                | CF_TEMPLATE_TIME_PERIOD_TOMORROW
                | CF_TEMPLATE_TIME_PERIOD_YESTERDAY
                | CF_TEMPLATE_TIME_PERIOD_LAST7DAYS
                | CF_TEMPLATE_TIME_PERIOD_LAST_MONTH
                | CF_TEMPLATE_TIME_PERIOD_NEXT_MONTH
                | CF_TEMPLATE_TIME_PERIOD_THIS_WEEK
                | CF_TEMPLATE_TIME_PERIOD_NEXT_WEEK
                | CF_TEMPLATE_TIME_PERIOD_LAST_WEEK
                | CF_TEMPLATE_TIME_PERIOD_THIS_MONTH,
            ) => self.check_date_rule(),
            (CF_TYPE_FILTER, CF_TEMPLATE_FILTER) => self.check_filter_rule(),
            (
                CF_TYPE_EXPRESSION,
                CF_TEMPLATE_ABOVE_AVERAGE
                | CF_TEMPLATE_BELOW_AVERAGE
                | CF_TEMPLATE_EQUAL_ABOVE_AVERAGE
                | CF_TEMPLATE_EQUAL_BELOW_AVERAGE,
            ) => self.check_average_rule(),
            (CF_TYPE_EXPRESSION, CF_TEMPLATE_UNIQUE_VALUES | CF_TEMPLATE_DUPLICATE_VALUES) => {
                if self.iParam != 0 {
                    return Err("duplicate conditional formats must have a zero parameter");
                }
                if self.flags & (Self::F_ABOVE | Self::F_BOTTOM | Self::F_PERCENT) != 0 {
                    return Err("duplicate conditional formats use only the stop flag");
                }
                self.check_no_formulas()
            }
            (CF_TYPE_GRADIENT, CF_TEMPLATE_GRADIENT)
            | (CF_TYPE_DATABAR, CF_TEMPLATE_DATABAR)
            | (CF_TYPE_MULTISTATE, CF_TEMPLATE_MULTISTATE) => self.check_no_dxf_rule(),
            _ => Err("unsupported conditional-format type and template"),
        }
    }
}

impl BrtBeginCFRule {
    fn check_cell_rule(&self) -> Result<(), &'static str> {
        if self.flags & (Self::F_ABOVE | Self::F_BOTTOM | Self::F_PERCENT) != 0 {
            return Err("cell-value conditional formats use only the stop flag");
        }
        if !(1..=8).contains(&self.iParam) {
            return Err("conditional-format operator is outside 1..=8");
        }
        let Some(formula1) = &self.formula1 else {
            return Err("conditional-format first formula is required");
        };
        formula1.check()?;
        if self.cbFmla1 == 0 {
            return Err("conditional-format first formula marker must be nonzero");
        }
        match (self.iParam, &self.formula2) {
            (1 | 2, Some(formula2)) => {
                formula2.check()?;
                if self.cbFmla2 == 0 {
                    return Err("conditional-format second formula marker must be nonzero");
                }
            }
            (1 | 2, None) => return Err("between rules require a second formula"),
            (_, Some(_)) => return Err("only between rules may have a second formula"),
            (_, None) if self.cbFmla2 != 0 => {
                return Err("conditional-format second formula marker must be zero");
            }
            (_, None) => {}
        }
        Ok(())
    }

    fn check_formula_rule(&self) -> Result<(), &'static str> {
        if self.iParam != 0 {
            return Err("formula conditional formats must have a zero parameter");
        }
        self.check_single_formula_rule()
    }

    fn check_text_rule(&self) -> Result<(), &'static str> {
        if self.iParam > CF_TEXTOPER_ENDS_WITH {
            return Err("text conditional-format operator is outside 0..=3");
        }
        if self.strParam.is_null() {
            return Err("text conditional formats require a search string");
        }
        if !(1..=255).contains(&self.strParam.len()) {
            return Err("text conditional-format string must contain 1..=255 units");
        }
        self.check_single_formula_rule()
    }

    fn check_fixed_formula_rule(&self) -> Result<(), &'static str> {
        if self.iParam != 0 {
            return Err("blank and error conditional formats must have a zero parameter");
        }
        if !self.strParam.is_null() {
            return Err("blank and error conditional formats must not have a string parameter");
        }
        self.check_single_formula_rule()
    }

    fn check_date_rule(&self) -> Result<(), &'static str> {
        let expected = match self.iTemplate {
            CF_TEMPLATE_TIME_PERIOD_TODAY => 0,
            CF_TEMPLATE_TIME_PERIOD_YESTERDAY => 1,
            CF_TEMPLATE_TIME_PERIOD_LAST7DAYS => 2,
            CF_TEMPLATE_TIME_PERIOD_THIS_WEEK => 3,
            CF_TEMPLATE_TIME_PERIOD_LAST_WEEK => 4,
            CF_TEMPLATE_TIME_PERIOD_LAST_MONTH => 5,
            CF_TEMPLATE_TIME_PERIOD_TOMORROW => 6,
            CF_TEMPLATE_TIME_PERIOD_NEXT_WEEK => 7,
            CF_TEMPLATE_TIME_PERIOD_NEXT_MONTH => 8,
            CF_TEMPLATE_TIME_PERIOD_THIS_MONTH => 9,
            _ => return Err("unsupported date conditional-format template"),
        };
        if self.iParam != expected {
            return Err("date conditional-format parameter does not match its template");
        }
        if !self.strParam.is_null() {
            return Err("date conditional formats must not have a string parameter");
        }
        self.check_single_formula_rule()
    }

    fn check_single_formula_rule(&self) -> Result<(), &'static str> {
        if self.flags & (Self::F_ABOVE | Self::F_BOTTOM | Self::F_PERCENT) != 0 {
            return Err("this conditional format uses only the stop flag");
        }
        let Some(formula1) = &self.formula1 else {
            return Err("conditional-format first formula is required");
        };
        formula1.check()?;
        if self.cbFmla1 == 0 {
            return Err("conditional-format first formula marker must be nonzero");
        }
        if self.cbFmla2 != 0 || self.formula2.is_some() {
            return Err("this conditional format must not contain a second formula");
        }
        Ok(())
    }

    fn check_filter_rule(&self) -> Result<(), &'static str> {
        if self.flags & Self::F_ABOVE != 0 {
            return Err("top/bottom conditional formats must not set fAbove");
        }
        let maximum = if self.flags & Self::F_PERCENT != 0 {
            100
        } else {
            1000
        };
        if !(1..=maximum).contains(&self.iParam) {
            return Err("top/bottom rank is outside its allowed range");
        }
        self.check_no_formulas()
    }

    const fn check_no_dxf_rule(&self) -> Result<(), &'static str> {
        if self.dxfId != u32::MAX {
            return Err("this conditional format must not reference a differential format");
        }
        if self.iParam != 0 || self.flags != 0 {
            return Err("this conditional format uses only zero parameters and flags");
        }
        self.check_no_formulas()
    }

    const fn check_average_rule(&self) -> Result<(), &'static str> {
        if self.flags & (Self::F_BOTTOM | Self::F_PERCENT) != 0 {
            return Err("average conditional formats must not set fBottom or fPercent");
        }
        let above = matches!(
            self.iTemplate,
            CF_TEMPLATE_ABOVE_AVERAGE | CF_TEMPLATE_EQUAL_ABOVE_AVERAGE
        );
        if (self.flags & Self::F_ABOVE != 0) != above {
            return Err("average conditional-format fAbove does not match its template");
        }
        if self.iParam > 3 {
            return Err("average standard deviation is outside 0..=3");
        }
        if matches!(
            self.iTemplate,
            CF_TEMPLATE_EQUAL_ABOVE_AVERAGE | CF_TEMPLATE_EQUAL_BELOW_AVERAGE
        ) && self.iParam != 0
        {
            return Err("equal-average conditional formats must use a zero parameter");
        }
        self.check_no_formulas()
    }
}

assign_id!(BrtBeginCFRule);

/// `2.4.380`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndCFRule {}

impl Unchecked for BrtEndCFRule {}

assign_id!(BrtEndCFRule);

/// `2.4.28`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtBeginColorScale {}

impl Unchecked for BrtBeginColorScale {}

assign_id!(BrtBeginColorScale);

/// `2.4.385`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndColorScale {}

impl Unchecked for BrtEndColorScale {}

assign_id!(BrtEndColorScale);

/// `2.4.43`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginDatabar {
    pub bLenMin: u8,
    pub bLenMax: u8,
    pub fShowValue: u8,
}

impl Default for BrtBeginDatabar {
    fn default() -> Self {
        Self {
            bLenMin: 10,
            bLenMax: 90,
            fShowValue: 1,
        }
    }
}

impl Checked for BrtBeginDatabar {
    fn check(&self) -> Result<(), &'static str> {
        if self.bLenMin > self.bLenMax || self.bLenMax > 100 {
            return Err("data-bar lengths must satisfy 0 <= minimum <= maximum <= 100");
        }
        if self.fShowValue > 1 {
            return Err("data-bar show-value flag must be 0 or 1");
        }
        Ok(())
    }
}

assign_id!(BrtBeginDatabar);

/// `2.4.399`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndDatabar {}

impl Unchecked for BrtEndDatabar {}

assign_id!(BrtEndDatabar);

/// `2.4.91`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginIconSet {
    pub iSet: u32,
    /// Packed flags: bit 1 is `fIcon`, bit 2 is `fReverse`; all other bits are
    /// reserved.
    pub flags: u16,
}

impl BrtBeginIconSet {
    /// `fIcon`: show only the icon.
    pub const F_ICON: u16 = 1 << 1;
    /// `fReverse`: reverse the icon order.
    pub const F_REVERSE: u16 = 1 << 2;
}

impl Default for BrtBeginIconSet {
    fn default() -> Self {
        Self { iSet: 3, flags: 0 }
    }
}

impl Checked for BrtBeginIconSet {
    fn check(&self) -> Result<(), &'static str> {
        if self.iSet > 0x10 {
            return Err("icon set is outside the KPISets range");
        }
        if self.flags & !(Self::F_ICON | Self::F_REVERSE) != 0 {
            return Err("icon-set flags contain reserved bits");
        }
        Ok(())
    }
}

assign_id!(BrtBeginIconSet);

/// `2.4.445`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndIconSet {}

impl Unchecked for BrtEndIconSet {}

assign_id!(BrtEndIconSet);

/// `2.4.334`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtCFVO {
    pub iType: u32,
    pub numParam: Xnum,
    pub fSaveGTE: u32,
    pub fGTE: u32,
    pub cbFmla: u32,
    #[deku(cond = "*cbFmla != 0", default = "None")]
    pub formula: Option<CFParsedFormula>,
}

impl Default for BrtCFVO {
    fn default() -> Self {
        Self {
            iType: CFVO_MIN,
            numParam: Xnum::default(),
            fSaveGTE: 0,
            fGTE: 0,
            cbFmla: 0,
            formula: None,
        }
    }
}

impl Checked for BrtCFVO {
    fn check(&self) -> Result<(), &'static str> {
        if self.fSaveGTE > 1 || self.fGTE > 1 {
            return Err("conditional-format value flags must be 0 or 1");
        }
        if self.cbFmla != 0 || self.formula.is_some() {
            return Err("conditional-format value formulas are not supported");
        }
        if !self.numParam.is_finite() {
            return Err("conditional-format value must be finite");
        }
        match self.iType {
            CFVO_MIN | CFVO_MAX | CFVO_NUM => Ok(()),
            CFVO_PERCENT | CFVO_PERCENTILE => {
                if (0.0..=100.0).contains(&f64::from(self.numParam)) {
                    Ok(())
                } else {
                    Err("conditional-format percentage must be in 0..=100")
                }
            }
            _ => Err("unsupported conditional-format value type"),
        }
    }
}

assign_id!(BrtCFVO);

/// `2.4.34`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrtBeginConditionalFormatting {
    pub ccf: u32,
    pub fPivot: u32,
    pub sqrfx: UncheckedSqRfX,
}

impl Default for BrtBeginConditionalFormatting {
    fn default() -> Self {
        conditional_format_block(UncheckedRfX::default())
    }
}

impl Checked for BrtBeginConditionalFormatting {
    fn check(&self) -> Result<(), &'static str> {
        if self.ccf != 1 {
            return Err("conditional-format block must contain one rule");
        }
        if self.fPivot > 1 {
            return Err("conditional-format pivot flag must be 0 or 1");
        }
        if !(1..=8_192).contains(&self.sqrfx.crfx) {
            return Err("conditional-format block must contain 1-8192 ranges");
        }
        self.sqrfx.check()
    }
}

assign_id!(BrtBeginConditionalFormatting);

/// `2.4.391`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtEndConditionalFormatting {}

impl Unchecked for BrtEndConditionalFormatting {}

assign_id!(BrtEndConditionalFormatting);

/// Builds the range block used by a conditional-formatting rule.
#[must_use]
pub fn conditional_format_block(range: UncheckedRfX) -> BrtBeginConditionalFormatting {
    conditional_format_block_ranges(std::slice::from_ref(&range))
}

/// Builds a conditional-formatting block for one or more ranges.
#[must_use]
pub fn conditional_format_block_ranges(ranges: &[UncheckedRfX]) -> BrtBeginConditionalFormatting {
    BrtBeginConditionalFormatting {
        ccf: 1,
        fPivot: 0,
        sqrfx: UncheckedSqRfX {
            crfx: u32::try_from(ranges.len()).unwrap_or(u32::MAX),
            rgRefx: ranges.to_vec(),
        },
    }
}

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::XlStringExt;

    fn number(value: f64) -> CFParsedFormula {
        let mut rgce = Vec::with_capacity(9);
        rgce.push(0x1F);
        rgce.extend_from_slice(&value.to_le_bytes());
        CFParsedFormula {
            cce: 9,
            rgce,
            cb: 0,
            rgcb: Vec::new(),
        }
    }

    #[test]
    fn encodes_a_between_rule_with_two_number_formulas() {
        let rule = BrtBeginCFRule {
            dxfId: 0,
            iPri: 1,
            iParam: 1,
            cbFmla1: 9,
            cbFmla2: 9,
            formula1: Some(number(1.0)),
            formula2: Some(number(2.0)),
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("valid rule");
        let bytes = rule.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 80);
        assert_eq!(&bytes[0..4], &1_u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &0_u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &1_i32.to_le_bytes());
        assert_eq!(&bytes[16..20], &1_u32.to_le_bytes());
        assert_eq!(&bytes[28..30], &[0, 0]);
        assert_eq!(&bytes[30..34], &9_u32.to_le_bytes());
        assert_eq!(&bytes[34..38], &9_u32.to_le_bytes());
        assert_eq!(&bytes[38..42], &0_u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[46..50], &9_u32.to_le_bytes());
        assert_eq!(bytes[50], 0x1F);
        assert_eq!(&bytes[51..59], &1.0_f64.to_le_bytes());
        assert_eq!(&bytes[59..63], &0_u32.to_le_bytes());
        assert_eq!(&bytes[63..67], &9_u32.to_le_bytes());
        assert_eq!(bytes[67], 0x1F);
        assert_eq!(&bytes[68..76], &2.0_f64.to_le_bytes());
        assert_eq!(&bytes[76..80], &0_u32.to_le_bytes());
    }

    #[test]
    fn formula_rules_require_template_one_and_one_formula() {
        let mut rule = BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: CF_TEMPLATE_FMLA,
            iParam: 0,
            formula1: Some(number(1.0)),
            cbFmla1: 9,
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("formula rule");

        rule.iParam = 1;
        assert!(rule.check().is_err());
        rule.iParam = 0;
        rule.formula2 = Some(number(1.0));
        rule.cbFmla2 = 9;
        assert!(rule.check().is_err());
        rule.formula2 = None;
        rule.cbFmla2 = 0;
        rule.iTemplate = CF_TEMPLATE_EXPR;
        assert!(rule.check().is_err());
    }

    #[test]
    fn stop_if_true_uses_the_second_flag_bit() {
        let mut rule = BrtBeginCFRule {
            formula1: Some(number(1.0)),
            cbFmla1: 9,
            ..BrtBeginCFRule::default()
        };
        rule.set_stop_if_true(true);
        assert!(rule.stop_if_true());
        rule.check().expect("valid rule");
        let bytes = rule.to_bytes().expect("serializes");
        assert_eq!(&bytes[28..30], &[2, 0]);
    }

    #[test]
    fn filter_rule_uses_bottom_and_percent_flags() {
        let rule = BrtBeginCFRule {
            iType: CF_TYPE_FILTER,
            iTemplate: CF_TEMPLATE_FILTER,
            iParam: 20,
            flags: BrtBeginCFRule::F_STOP_TRUE
                | BrtBeginCFRule::F_BOTTOM
                | BrtBeginCFRule::F_PERCENT,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("valid filter rule");
        let bytes = rule.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 46);
        assert_eq!(&bytes[0..4], &CF_TYPE_FILTER.to_le_bytes());
        assert_eq!(&bytes[4..8], &CF_TEMPLATE_FILTER.to_le_bytes());
        assert_eq!(&bytes[16..20], &20_u32.to_le_bytes());
        assert_eq!(&bytes[28..30], &26_u16.to_le_bytes());
        assert_eq!(&bytes[30..34], &0_u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &u32::MAX.to_le_bytes());
    }

    #[test]
    fn average_and_duplicate_rules_omit_formulas() {
        for (template, flag, parameter) in [
            (CF_TEMPLATE_ABOVE_AVERAGE, BrtBeginCFRule::F_ABOVE, 0),
            (CF_TEMPLATE_BELOW_AVERAGE, 0, 3),
            (CF_TEMPLATE_EQUAL_ABOVE_AVERAGE, BrtBeginCFRule::F_ABOVE, 0),
            (CF_TEMPLATE_EQUAL_BELOW_AVERAGE, 0, 0),
            (CF_TEMPLATE_UNIQUE_VALUES, 0, 0),
            (CF_TEMPLATE_DUPLICATE_VALUES, 0, 0),
        ] {
            let rule = BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: template,
                iParam: parameter,
                flags: flag,
                cbFmla1: 0,
                formula1: None,
                ..BrtBeginCFRule::default()
            };
            rule.check().expect("valid expression rule");
            let bytes = rule.to_bytes().expect("serializes");
            assert_eq!(bytes.len(), 46);
            assert_eq!(&bytes[16..20], &parameter.to_le_bytes());
            assert_eq!(&bytes[28..30], &flag.to_le_bytes());
        }
    }

    #[test]
    fn rejects_filter_and_average_flag_mismatches() {
        let filter = BrtBeginCFRule {
            iType: CF_TYPE_FILTER,
            iTemplate: CF_TEMPLATE_FILTER,
            iParam: 101,
            flags: BrtBeginCFRule::F_PERCENT,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        assert!(filter.check().is_err());

        let average = BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: CF_TEMPLATE_ABOVE_AVERAGE,
            iParam: 0,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        assert!(average.check().is_err());

        let duplicate = BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: CF_TEMPLATE_DUPLICATE_VALUES,
            iParam: 1,
            flags: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        assert!(duplicate.check().is_err());
    }

    #[test]
    fn gradient_rule_uses_no_dxf_flags_or_formulas() {
        let rule = BrtBeginCFRule {
            iType: CF_TYPE_GRADIENT,
            iTemplate: CF_TEMPLATE_GRADIENT,
            dxfId: u32::MAX,
            iPri: 1,
            iParam: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("valid color scale");
        let bytes = rule.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 46);
        assert_eq!(&bytes[0..4], &CF_TYPE_GRADIENT.to_le_bytes());
        assert_eq!(&bytes[4..8], &CF_TEMPLATE_GRADIENT.to_le_bytes());
        assert_eq!(&bytes[8..12], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
        assert_eq!(&bytes[28..30], &[0, 0]);
        assert_eq!(&bytes[30..34], &0_u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &u32::MAX.to_le_bytes());
    }

    #[test]
    fn gradient_rule_rejects_format_flags_and_formulas() {
        let mut rule = BrtBeginCFRule {
            iType: CF_TYPE_GRADIENT,
            iTemplate: CF_TEMPLATE_GRADIENT,
            dxfId: u32::MAX,
            iPri: 1,
            iParam: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        rule.dxfId = 0;
        assert!(rule.check().is_err());
        rule.dxfId = u32::MAX;
        rule.flags = BrtBeginCFRule::F_STOP_TRUE;
        assert!(rule.check().is_err());
        rule.flags = 0;
        rule.cbFmla1 = 9;
        rule.formula1 = Some(number(1.0));
        assert!(rule.check().is_err());
    }

    #[test]
    fn databar_records_encode_lengths_and_show_value() {
        let bar = BrtBeginDatabar::default();
        bar.check().expect("valid data bar");
        let bytes = bar.to_bytes().expect("serializes");
        assert_eq!(bytes, [10, 90, 1]);

        let bar = BrtBeginDatabar {
            fShowValue: 0,
            ..BrtBeginDatabar::default()
        };
        bar.check().expect("valid bar-only data bar");
        assert_eq!(bar.to_bytes().expect("serializes"), [10, 90, 0]);

        for bar in [
            BrtBeginDatabar {
                bLenMin: 0,
                bLenMax: 0,
                ..BrtBeginDatabar::default()
            },
            BrtBeginDatabar {
                bLenMin: 100,
                bLenMax: 100,
                ..BrtBeginDatabar::default()
            },
        ] {
            bar.check().expect("valid boundary data bar");
        }

        for bar in [
            BrtBeginDatabar {
                bLenMin: 91,
                ..BrtBeginDatabar::default()
            },
            BrtBeginDatabar {
                bLenMax: 101,
                ..BrtBeginDatabar::default()
            },
            BrtBeginDatabar {
                fShowValue: 2,
                ..BrtBeginDatabar::default()
            },
        ] {
            assert!(bar.check().is_err());
        }

        assert_eq!(
            BrtEndDatabar::default().to_bytes().expect("serializes"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn databar_rule_uses_no_dxf_flags_or_formulas() {
        let rule = BrtBeginCFRule {
            iType: CF_TYPE_DATABAR,
            iTemplate: CF_TEMPLATE_DATABAR,
            dxfId: u32::MAX,
            iPri: 1,
            iParam: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("valid data bar");
        let bytes = rule.to_bytes().expect("serializes");
        assert_eq!(
            bytes,
            [
                4, 0, 0, 0, 3, 0, 0, 0, 255, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255,
            ]
        );
        assert_eq!(&bytes[0..4], &CF_TYPE_DATABAR.to_le_bytes());
        assert_eq!(&bytes[4..8], &CF_TEMPLATE_DATABAR.to_le_bytes());
        assert_eq!(&bytes[8..12], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[28..30], &[0, 0]);
        assert_eq!(&bytes[30..34], &0_u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &u32::MAX.to_le_bytes());

        for invalid in [
            BrtBeginCFRule {
                dxfId: 0,
                ..rule.clone()
            },
            BrtBeginCFRule {
                iParam: 1,
                ..rule.clone()
            },
            BrtBeginCFRule {
                flags: BrtBeginCFRule::F_STOP_TRUE,
                ..rule.clone()
            },
            BrtBeginCFRule {
                cbFmla1: 9,
                formula1: Some(number(1.0)),
                ..rule
            },
        ] {
            assert!(invalid.check().is_err());
        }
    }

    #[test]
    fn iconset_records_encode_flags_and_reject_reserved_bits() {
        let set = BrtBeginIconSet::default();
        set.check().expect("valid icon set");
        assert_eq!(set.to_bytes().expect("serializes"), [3, 0, 0, 0, 0, 0]);

        for (flags, expected) in [
            (BrtBeginIconSet::F_ICON, [3, 0, 0, 0, 2, 0]),
            (BrtBeginIconSet::F_REVERSE, [3, 0, 0, 0, 4, 0]),
            (
                BrtBeginIconSet::F_ICON | BrtBeginIconSet::F_REVERSE,
                [3, 0, 0, 0, 6, 0],
            ),
        ] {
            let set = BrtBeginIconSet {
                flags,
                ..set.clone()
            };
            set.check().expect("valid icon-set flag");
            assert_eq!(set.to_bytes().expect("serializes"), expected);
        }

        for invalid in [
            BrtBeginIconSet {
                iSet: 0x11,
                ..set.clone()
            },
            BrtBeginIconSet {
                flags: 1,
                ..set.clone()
            },
            BrtBeginIconSet {
                flags: 1 << 3,
                ..set
            },
        ] {
            assert!(invalid.check().is_err());
        }

        assert_eq!(
            BrtEndIconSet::default().to_bytes().expect("serializes"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn multistate_rule_uses_no_dxf_flags_or_formulas() {
        let rule = BrtBeginCFRule {
            iType: CF_TYPE_MULTISTATE,
            iTemplate: CF_TEMPLATE_MULTISTATE,
            dxfId: u32::MAX,
            iPri: 2,
            iParam: 0,
            cbFmla1: 0,
            formula1: None,
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("valid icon-set rule");
        let bytes = rule.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 46);
        assert_eq!(&bytes[0..4], &CF_TYPE_MULTISTATE.to_le_bytes());
        assert_eq!(&bytes[4..8], &CF_TEMPLATE_MULTISTATE.to_le_bytes());
        assert_eq!(&bytes[8..12], &u32::MAX.to_le_bytes());
        assert_eq!(&bytes[12..16], &2_i32.to_le_bytes());
        assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
        assert_eq!(&bytes[28..30], &[0, 0]);
        assert_eq!(&bytes[30..34], &0_u32.to_le_bytes());
        assert_eq!(&bytes[42..46], &u32::MAX.to_le_bytes());

        for invalid in [
            BrtBeginCFRule {
                iParam: 1,
                ..rule.clone()
            },
            BrtBeginCFRule {
                flags: BrtBeginCFRule::F_STOP_TRUE,
                ..rule.clone()
            },
            BrtBeginCFRule {
                cbFmla1: 9,
                formula1: Some(number(1.0)),
                ..rule
            },
        ] {
            assert!(invalid.check().is_err());
        }
    }

    #[test]
    fn cfvo_encodes_thresholds_and_rejects_bad_values() {
        let cfvo = BrtCFVO::default();
        cfvo.check().expect("valid minimum");
        let bytes = cfvo.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 24);
        assert_eq!(&bytes[0..4], &CFVO_MIN.to_le_bytes());
        assert_eq!(&bytes[4..12], &0.0_f64.to_le_bytes());
        assert_eq!(&bytes[12..16], &0_u32.to_le_bytes());
        assert_eq!(&bytes[16..20], &0_u32.to_le_bytes());
        assert_eq!(&bytes[20..24], &0_u32.to_le_bytes());

        for (i_type, value) in [
            (CFVO_NUM, 5.5),
            (CFVO_PERCENT, 0.0),
            (CFVO_PERCENT, 100.0),
            (CFVO_PERCENTILE, 50.0),
        ] {
            let cfvo = BrtCFVO {
                iType: i_type,
                numParam: value.into(),
                ..BrtCFVO::default()
            };
            cfvo.check().expect("valid threshold");
        }

        for (i_type, value) in [(CFVO_PERCENT, 100.1), (CFVO_PERCENTILE, -1.0)] {
            let cfvo = BrtCFVO {
                iType: i_type,
                numParam: value.into(),
                ..BrtCFVO::default()
            };
            assert!(cfvo.check().is_err());
        }

        let cfvo = BrtCFVO {
            iType: CFVO_NUM,
            numParam: f64::NAN.into(),
            ..BrtCFVO::default()
        };
        assert!(cfvo.check().is_err());
    }

    #[test]
    fn cfvo_rejects_formulas_and_reserved_flags() {
        let cfvo = BrtCFVO {
            cbFmla: 1,
            ..BrtCFVO::default()
        };
        assert!(cfvo.check().is_err());

        let cfvo = BrtCFVO {
            fSaveGTE: 2,
            ..BrtCFVO::default()
        };
        assert!(cfvo.check().is_err());
    }

    #[test]
    fn encodes_the_conditional_formatting_block() {
        let block = conditional_format_block(UncheckedRfX {
            rwFirst: 1,
            rwLast: 3,
            colFirst: 2,
            colLast: 4,
        });
        block.check().expect("valid block");
        let bytes = block.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 28);
        assert_eq!(&bytes[0..4], &1_u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &1_u32.to_le_bytes());
        assert_eq!(
            &bytes[12..28],
            &[1, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0]
        );
    }

    #[test]
    fn encodes_a_multi_range_block_and_checks_its_count() {
        let ranges = [
            UncheckedRfX {
                rwFirst: 2,
                rwLast: 5,
                colFirst: 1,
                colLast: 3,
            },
            UncheckedRfX {
                rwFirst: 8,
                rwLast: 11,
                colFirst: 8,
                colLast: 10,
            },
        ];
        let block = conditional_format_block_ranges(&ranges);
        block.check().expect("valid block");
        let bytes = block.to_bytes().expect("serializes");

        assert_eq!(bytes.len(), 44);
        assert_eq!(&bytes[0..4], &1_u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &0_u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &2_u32.to_le_bytes());
        assert_eq!(
            &bytes[12..28],
            &[2, 0, 0, 0, 5, 0, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0]
        );
        assert_eq!(
            &bytes[28..44],
            &[8, 0, 0, 0, 11, 0, 0, 0, 8, 0, 0, 0, 10, 0, 0, 0]
        );

        let empty = conditional_format_block_ranges(&[]);
        assert!(empty.check().is_err());

        let at_limit = conditional_format_block_ranges(&vec![UncheckedRfX::default(); 8_192]);
        assert!(at_limit.check().is_ok());

        let too_many = conditional_format_block_ranges(&vec![UncheckedRfX::default(); 8_193]);
        assert!(too_many.check().is_err());
    }

    #[test]
    fn formula_markers_are_presence_flags_not_sizes() {
        let rule = BrtBeginCFRule {
            dxfId: 0,
            iPri: 1,
            iParam: 1,
            cbFmla1: 1,
            cbFmla2: 1,
            formula1: Some(number(1.0)),
            formula2: Some(number(2.0)),
            ..BrtBeginCFRule::default()
        };
        rule.check().expect("markers are ignored sizes");

        let invalid = BrtBeginCFRule {
            cbFmla1: 0,
            formula1: Some(number(1.0)),
            ..BrtBeginCFRule::default()
        };
        assert!(invalid.check().is_err());
    }

    #[test]
    fn date_rule_templates_require_matching_parameters() {
        let pairs = [
            (CF_TEMPLATE_TIME_PERIOD_TODAY, 0),
            (CF_TEMPLATE_TIME_PERIOD_YESTERDAY, 1),
            (CF_TEMPLATE_TIME_PERIOD_LAST7DAYS, 2),
            (CF_TEMPLATE_TIME_PERIOD_THIS_WEEK, 3),
            (CF_TEMPLATE_TIME_PERIOD_LAST_WEEK, 4),
            (CF_TEMPLATE_TIME_PERIOD_LAST_MONTH, 5),
            (CF_TEMPLATE_TIME_PERIOD_TOMORROW, 6),
            (CF_TEMPLATE_TIME_PERIOD_NEXT_WEEK, 7),
            (CF_TEMPLATE_TIME_PERIOD_NEXT_MONTH, 8),
            (CF_TEMPLATE_TIME_PERIOD_THIS_MONTH, 9),
        ];
        for (template, parameter) in pairs {
            let rule = BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: template,
                dxfId: 0,
                iPri: 1,
                iParam: parameter,
                cbFmla1: 1,
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            };
            rule.check().expect("valid date rule");
        }

        let today = BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: CF_TEMPLATE_TIME_PERIOD_TODAY,
            dxfId: 0,
            iPri: 1,
            iParam: 0,
            cbFmla1: 1,
            formula1: Some(number(1.0)),
            ..BrtBeginCFRule::default()
        };
        for invalid in [
            BrtBeginCFRule {
                iParam: 1,
                ..today.clone()
            },
            BrtBeginCFRule {
                strParam: XlNullableWideString::construct(Some("today")),
                ..today.clone()
            },
            BrtBeginCFRule {
                flags: BrtBeginCFRule::F_ABOVE,
                ..today.clone()
            },
            BrtBeginCFRule {
                formula2: Some(number(2.0)),
                ..today.clone()
            },
            BrtBeginCFRule {
                formula1: None,
                ..today.clone()
            },
            BrtBeginCFRule {
                cbFmla1: 0,
                ..today
            },
        ] {
            assert!(invalid.check().is_err());
        }
    }

    #[test]
    fn text_blank_and_error_rules_validate_template_parameters() {
        for (template, parameter) in [
            (CF_TEMPLATE_CONTAINS_TEXT, CF_TEXTOPER_CONTAINS),
            (CF_TEMPLATE_CONTAINS_TEXT, CF_TEXTOPER_NOT_CONTAINS),
            (CF_TEMPLATE_CONTAINS_TEXT, CF_TEXTOPER_BEGINS_WITH),
            (CF_TEMPLATE_CONTAINS_TEXT, CF_TEXTOPER_ENDS_WITH),
        ] {
            let rule = BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: template,
                dxfId: 0,
                iPri: 1,
                iParam: parameter,
                cbFmla1: 1,
                strParam: XlNullableWideString::construct(Some("rust")),
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            };
            rule.check()
                .expect("valid text rule with an ignored size marker");
        }

        for template in [
            CF_TEMPLATE_CONTAINS_BLANKS,
            CF_TEMPLATE_CONTAINS_NO_BLANKS,
            CF_TEMPLATE_CONTAINS_ERRORS,
            CF_TEMPLATE_CONTAINS_NO_ERRORS,
        ] {
            let rule = BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: template,
                dxfId: 0,
                iPri: 1,
                iParam: 0,
                cbFmla1: 9,
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            };
            rule.check().expect("valid blank or error rule");
        }

        let text = BrtBeginCFRule {
            iType: CF_TYPE_EXPRESSION,
            iTemplate: CF_TEMPLATE_CONTAINS_TEXT,
            dxfId: 0,
            iPri: 1,
            iParam: CF_TEXTOPER_CONTAINS,
            cbFmla1: 9,
            strParam: XlNullableWideString::construct(Some("a".repeat(255))),
            formula1: Some(number(1.0)),
            ..BrtBeginCFRule::default()
        };
        text.check().expect("255 UTF-16 units are allowed");

        for invalid in [
            BrtBeginCFRule {
                strParam: XlNullableWideString::construct(Some("")),
                ..text.clone()
            },
            BrtBeginCFRule {
                strParam: XlNullableWideString::construct(Some("a".repeat(256))),
                ..text.clone()
            },
            BrtBeginCFRule {
                iParam: 4,
                ..text.clone()
            },
            BrtBeginCFRule {
                cbFmla1: 0,
                ..text.clone()
            },
            BrtBeginCFRule {
                cbFmla2: 9,
                formula2: Some(number(2.0)),
                ..text.clone()
            },
            BrtBeginCFRule {
                flags: BrtBeginCFRule::F_ABOVE,
                ..text
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_TEXT,
                iPri: 1,
                iParam: CF_TEXTOPER_CONTAINS,
                strParam: XlNullableWideString::construct(Some("rust")),
                formula1: None,
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_BLANKS,
                iPri: 1,
                iParam: 1,
                cbFmla1: 9,
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_ERRORS,
                iPri: 1,
                iParam: 0,
                cbFmla1: 9,
                strParam: XlNullableWideString::construct(Some("unexpected")),
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_TEXT,
                iPri: 1,
                iParam: CF_TEXTOPER_CONTAINS,
                cbFmla1: 9,
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_CELL_IS,
                iTemplate: CF_TEMPLATE_CONTAINS_TEXT,
                iPri: 1,
                iParam: CF_TEXTOPER_CONTAINS,
                cbFmla1: 9,
                strParam: XlNullableWideString::construct(Some("rust")),
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_BLANKS,
                iPri: 1,
                iParam: 0,
                cbFmla1: 9,
                flags: BrtBeginCFRule::F_PERCENT,
                formula1: Some(number(1.0)),
                ..BrtBeginCFRule::default()
            },
            BrtBeginCFRule {
                iType: CF_TYPE_EXPRESSION,
                iTemplate: CF_TEMPLATE_CONTAINS_ERRORS,
                iPri: 1,
                iParam: 0,
                formula1: None,
                ..BrtBeginCFRule::default()
            },
        ] {
            assert!(invalid.check().is_err());
        }
    }

    #[test]
    fn rejects_missing_and_unexpected_operands() {
        let mut rule = BrtBeginCFRule {
            formula1: Some(number(1.0)),
            cbFmla1: 9,
            ..BrtBeginCFRule::default()
        };
        rule.formula1 = None;
        assert!(rule.check().is_err());

        rule.formula1 = Some(number(1.0));
        rule.iParam = 1;
        assert!(rule.check().is_err());

        rule.formula2 = Some(number(2.0));
        rule.cbFmla2 = 9;
        rule.iParam = 3;
        assert!(rule.check().is_err());

        rule.formula2 = None;
        rule.cbFmla2 = 0;
        rule.iParam = 3;
        rule.strParam = XlNullableWideString::construct(Some("unexpected"));
        assert!(rule.check().is_err());
    }
}
