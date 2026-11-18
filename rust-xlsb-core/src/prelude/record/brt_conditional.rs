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
    /// Packed flags: bit 1 is `fStopTrue`; all other bits are reserved.
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
        if self.iType != 1 || self.iTemplate != 0 {
            return Err("only cellIs expression conditional formats are supported");
        }
        if self.iPri <= 0 {
            return Err("conditional-format priority must be positive");
        }
        if !(1..=8).contains(&self.iParam) {
            return Err("conditional-format operator is outside 1..=8");
        }
        if self.reserved1 != 0 || self.reserved2 != 0 || self.flags & !Self::F_STOP_TRUE != 0 {
            return Err("conditional-format reserved bits must be zero");
        }
        if self.cbFmla3 != 0 || self.formula3.is_some() {
            return Err("conditional-format third formula must be omitted");
        }
        if !self.strParam.is_null() {
            return Err("conditional-format string parameter must be null");
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
