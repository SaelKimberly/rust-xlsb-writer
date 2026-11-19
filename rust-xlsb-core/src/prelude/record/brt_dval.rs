use deku::{DekuRead, DekuWrite};

use crate::prelude::{NameParsedFormula, UncheckedRfX, UncheckedSqRfX, XlNullableWideString};
use crate::{Checked, Unchecked, assign_id};

/// Data-validation properties of a worksheet; MS-XLSB `2.5.36`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct DVals {
    /// LSB-first flags: `fWnClosed` plus 15 reserved bits.
    pub flags: u16,
    pub xLeft: u32,
    pub yTop: u32,
    pub unused3: u32,
    pub idvMac: u32,
}

impl DVals {
    pub const F_WN_CLOSED: u16 = 1 << 0;
}

/// Beginning of a data-validation collection; MS-XLSB `2.4.55`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginDVals {
    pub dVals: DVals,
}

impl Checked for BrtBeginDVals {
    fn check(&self) -> Result<(), &'static str> {
        if self.dVals.flags & !DVals::F_WN_CLOSED != 0 {
            return Err("data-validation flags contain reserved bits");
        }
        if self.dVals.xLeft > 65_535 || self.dVals.yTop > 65_535 {
            return Err("data-validation prompt position exceeds 65535");
        }
        if self.dVals.unused3 != 0 {
            return Err("data-validation reserved field must be zero");
        }
        if self.dVals.idvMac > 65_534 {
            return Err("data-validation count exceeds 65534");
        }
        Ok(())
    }
}

impl BrtBeginDVals {
    #[must_use]
    pub fn new(count: u32) -> Self {
        Self {
            dVals: DVals {
                idvMac: count,
                ..DVals::default()
            },
        }
    }
}

assign_id!(BrtBeginDVals);

/// End of a data-validation collection; MS-XLSB `2.4.411`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndDVals {}

impl Unchecked for BrtEndDVals {}

assign_id!(BrtEndDVals);

/// Data-validation prompt and alert strings; MS-XLSB `2.5.37`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct DValStrings {
    pub strErrorTitle: XlNullableWideString,
    pub strError: XlNullableWideString,
    pub strPromptTitle: XlNullableWideString,
    pub strPrompt: XlNullableWideString,
}

impl Checked for DValStrings {
    fn check(&self) -> Result<(), &'static str> {
        check_length(&self.strErrorTitle, 32, "error title")?;
        check_length(&self.strError, 225, "error message")?;
        check_length(&self.strPromptTitle, 32, "prompt title")?;
        check_length(&self.strPrompt, 255, "prompt message")
    }
}

fn check_length(
    string: &XlNullableWideString,
    max: u32,
    name: &'static str,
) -> Result<(), &'static str> {
    if !string.is_null() && string.len() > max {
        return Err(match name {
            "error title" => "data-validation error title exceeds 32 characters",
            "error message" => "data-validation error message exceeds 225 characters",
            "prompt title" => "data-validation prompt title exceeds 32 characters",
            _ => "data-validation prompt message exceeds 255 characters",
        });
    }
    Ok(())
}

/// One data validation; MS-XLSB `2.4.356`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct BrtDVal {
    /// LSB-first flags: type, error style, allow-blank, dropdown, IME,
    /// input/error visibility, and operator.
    pub flags: u32,
    pub sqrfx: UncheckedSqRfX,
    pub strings: DValStrings,
    pub formula1: NameParsedFormula,
    pub formula2: NameParsedFormula,
}

impl BrtDVal {
    pub const F_ALLOW_BLANK: u32 = 1 << 8;
    pub const F_SUPPRESS_COMBO: u32 = 1 << 9;
    pub const F_SHOW_INPUT_MSG: u32 = 1 << 18;
    pub const F_SHOW_ERROR_MSG: u32 = 1 << 19;

    #[must_use]
    pub const fn val_type(&self) -> u8 {
        (self.flags & 0x0F) as u8
    }

    #[must_use]
    pub const fn err_style(&self) -> u8 {
        ((self.flags >> 4) & 0x07) as u8
    }

    #[must_use]
    pub const fn operator(&self) -> u8 {
        ((self.flags >> 20) & 0x0F) as u8
    }
}

impl Default for BrtDVal {
    fn default() -> Self {
        Self {
            flags: 0,
            sqrfx: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![UncheckedRfX::default()],
            },
            strings: DValStrings::default(),
            formula1: NameParsedFormula::default(),
            formula2: NameParsedFormula::default(),
        }
    }
}

impl Checked for BrtDVal {
    fn check(&self) -> Result<(), &'static str> {
        if self.val_type() > 7 {
            return Err("data-validation type is not supported");
        }
        if self.err_style() > 2 {
            return Err("data-validation error style is not supported");
        }
        if self.flags & (1 << 7) != 0 || self.flags & 0xFF00_0000 != 0 {
            return Err("data-validation reserved bits must be zero");
        }
        if (self.flags >> 10) & 0xFF > 10 {
            return Err("data-validation IME mode is not supported");
        }
        if self.operator() > 7 {
            return Err("data-validation operator is not supported");
        }
        if !(1..8_192).contains(&self.sqrfx.crfx) {
            return Err("data-validation range count must be 1-8191");
        }
        self.sqrfx.check()?;
        self.strings.check()?;
        check_formula(&self.formula1)?;
        check_formula(&self.formula2)?;

        let has_first = self.formula1.cce != 0;
        let has_second = self.formula2.cce != 0;
        if self.val_type() == 0 {
            if has_first || has_second {
                return Err("an Any data validation must not contain formulas");
            }
            return Ok(());
        }

        if self.operator() <= 1 && !matches!(self.val_type(), 3 | 7) {
            if !has_first || !has_second {
                return Err("a Between validation requires two formulas");
            }
        } else if !has_first || has_second {
            return Err("this validation requires exactly one formula");
        }
        Ok(())
    }
}

fn check_formula(formula: &NameParsedFormula) -> Result<(), &'static str> {
    if formula.cce >= 16_385 {
        return Err("data-validation formula exceeds 16384 bytes");
    }
    if usize::try_from(formula.cce).ok() != Some(formula.rgce.len()) {
        return Err("data-validation formula length does not match its bytes");
    }
    if usize::try_from(formula.cb).ok() != Some(formula.rgcb.len()) {
        return Err("data-validation extra-formula length does not match its bytes");
    }
    Ok(())
}

assign_id!(BrtDVal);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    use crate::prelude::{UncheckedRfX, XlStringExt};

    fn formula(bytes: &[u8]) -> NameParsedFormula {
        NameParsedFormula {
            cce: u32::try_from(bytes.len()).expect("small"),
            rgce: bytes.to_vec(),
            ..NameParsedFormula::default()
        }
    }

    #[test]
    fn begin_and_end_markers_encode_the_dvals_body() {
        let begin = BrtBeginDVals::new(2);
        let bytes = begin.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 18);
        assert_eq!(&bytes[0..2], &0_u16.to_le_bytes());
        assert_eq!(&bytes[14..18], &2_u32.to_le_bytes());
        let (rest, decoded) = BrtBeginDVals::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, begin);

        assert_eq!(
            BrtEndDVals::default().to_bytes().expect("bytes"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn dval_flags_pack_lsb_first() {
        let record = BrtDVal {
            flags: 1
                | BrtDVal::F_ALLOW_BLANK
                | BrtDVal::F_SHOW_INPUT_MSG
                | BrtDVal::F_SHOW_ERROR_MSG,
            ..BrtDVal::default()
        };
        let bytes = record.to_bytes().expect("serializes");
        assert_eq!(&bytes[..4], [0x01, 0x01, 0x0C, 0x00]);
        assert_eq!(bytes.len(), 56);

        let (rest, decoded) = BrtDVal::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
    }

    #[test]
    fn dval_roundtrips_formulas_and_strings() {
        let record = BrtDVal {
            flags: 1
                | (2 << 20)
                | BrtDVal::F_ALLOW_BLANK
                | BrtDVal::F_SUPPRESS_COMBO
                | BrtDVal::F_SHOW_INPUT_MSG
                | BrtDVal::F_SHOW_ERROR_MSG,
            strings: DValStrings {
                strErrorTitle: XlNullableWideString::construct(Some("Invalid")),
                strError: XlNullableWideString::construct(Some("Enter a number")),
                strPromptTitle: XlNullableWideString::construct(Some("Input")),
                strPrompt: XlNullableWideString::construct(Some("Whole number")),
            },
            sqrfx: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![UncheckedRfX {
                    rwFirst: 1,
                    rwLast: 1,
                    colFirst: 3,
                    colLast: 3,
                }],
            },
            formula1: formula(&[0x1F, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..BrtDVal::default()
        };

        let bytes = record.to_bytes().expect("serializes");
        let (rest, decoded) = BrtDVal::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
        assert_eq!(
            decoded.strings.strError.to_optional_string().as_deref(),
            Some("Enter a number")
        );
    }

    #[test]
    fn dval_rejects_wrong_formula_shapes() {
        let mut record = BrtDVal {
            flags: 1,
            ..BrtDVal::default()
        };
        assert!(record.check().is_err(), "Between needs two formulas");

        record.flags = 1 | (2 << 20);
        record.formula1 = formula(&[0x1F, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(record.check().is_ok(), "Equal needs one formula");

        record.formula2 = formula(&[0x1F, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(record.check().is_err(), "Equal must not carry formula2");

        record.flags = 0;
        assert!(record.check().is_err(), "Any must not carry formulas");

        let record = BrtDVal {
            flags: 1 | (2 << 20),
            formula1: NameParsedFormula {
                cce: 16_385,
                rgce: vec![0; 16_385],
                ..NameParsedFormula::default()
            },
            ..BrtDVal::default()
        };
        assert!(record.check().is_err(), "formula byte limit is 16384");
    }

    #[test]
    fn dval_rejects_bad_ranges_and_strings() {
        let mut record = BrtDVal::default();
        record.sqrfx.rgRefx.clear();
        assert!(record.check().is_err(), "empty range list");

        let mut record = BrtDVal::default();
        record.sqrfx.crfx = 8_192;
        record.sqrfx.rgRefx = vec![UncheckedRfX::default(); 8_192];
        assert!(record.check().is_err(), "range count must be below 8192");

        let long_title = "x".repeat(33);
        let record = BrtDVal {
            strings: DValStrings {
                strPromptTitle: XlNullableWideString::construct(Some(long_title.as_str())),
                ..DValStrings::default()
            },
            ..BrtDVal::default()
        };
        assert!(record.check().is_err(), "prompt title exceeds 32");
    }
}
