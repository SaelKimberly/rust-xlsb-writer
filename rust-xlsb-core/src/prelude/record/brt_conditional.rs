#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec length field is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::prelude::{UncheckedRfX, UncheckedSqRfX, XlNullableWideString};
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
/// `CF_TYPE_FILTER`.
pub const CF_TYPE_FILTER: u32 = 5;

/// `CF_TEMPLATE_EXPR`.
pub const CF_TEMPLATE_EXPR: u32 = 0;
/// `CF_TEMPLATE_FILTER`.
pub const CF_TEMPLATE_FILTER: u32 = 5;
/// `CF_TEMPLATE_UNIQUEVALUES`.
pub const CF_TEMPLATE_UNIQUE_VALUES: u32 = 7;
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
            || !self.strParam.is_null()
        {
            return Err("conditional-format reserved fields must be zero");
        }

        match (self.iType, self.iTemplate) {
            (CF_TYPE_CELL_IS, CF_TEMPLATE_EXPR) => self.check_cell_rule(),
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
        if self.cbFmla1 != formula1.cce {
            return Err("conditional-format first formula size is inconsistent");
        }
        match (self.iParam, &self.formula2) {
            (1 | 2, Some(formula2)) => {
                formula2.check()?;
                if self.cbFmla2 != formula2.cce {
                    return Err("conditional-format second formula size is inconsistent");
                }
            }
            (1 | 2, None) => return Err("between rules require a second formula"),
            (_, Some(_)) => return Err("only between rules may have a second formula"),
            (_, None) if self.cbFmla2 != 0 => {
                return Err("conditional-format second formula size must be zero");
            }
            (_, None) => {}
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
        if self.sqrfx.crfx != 1 {
            return Err("conditional-format block must contain one range");
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
    BrtBeginConditionalFormatting {
        ccf: 1,
        fPivot: 0,
        sqrfx: UncheckedSqRfX {
            crfx: 1,
            rgRefx: vec![range],
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
