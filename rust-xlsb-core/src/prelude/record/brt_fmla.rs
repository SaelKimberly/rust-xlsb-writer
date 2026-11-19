#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec length field is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{BErr, Cell, XlWideString, Xnum};

/// `2.5.98.4`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellParsedFormula {
    pub cce: u32,
    #[deku(count = "cce")]
    pub rgce: Vec<u8>,
    #[deku(update = "self.rgcb.len() as u32")]
    pub cb: u32,
    #[deku(count = "cb")]
    pub rgcb: Vec<u8>,
}

impl Default for CellParsedFormula {
    fn default() -> Self {
        Self {
            cce: 3,
            rgce: vec![0x1E, 0, 0],
            cb: 0,
            rgcb: Vec::new(),
        }
    }
}

impl Checked for CellParsedFormula {
    fn check(&self) -> Result<(), &'static str> {
        if self.cce == 0 || self.cce >= 16_385 {
            return Err("cell formula size is outside 1..=16384");
        }
        if usize::try_from(self.cce).ok() != Some(self.rgce.len()) {
            return Err("cell formula size does not match its bytes");
        }
        if usize::try_from(self.cb).ok() != Some(self.rgcb.len()) {
            return Err("cell formula extra-data size does not match its bytes");
        }
        Ok(())
    }
}

fn check_cell_formula(cell: &Cell, formula: &CellParsedFormula) -> Result<(), &'static str> {
    cell.check_range()?;
    formula.check()
}

/// `2.4.684`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFmlaBool {
    pub cell: Cell,
    pub bBool: bool,
    pub grbitFlags: u16,
    pub formula: CellParsedFormula,
}

impl Checked for BrtFmlaBool {
    fn check(&self) -> Result<(), &'static str> {
        check_cell_formula(&self.cell, &self.formula)
    }
}

assign_id!(BrtFmlaBool);

/// `2.4.685`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFmlaError {
    pub cell: Cell,
    pub bError: BErr,
    pub grbitFlags: u16,
    pub formula: CellParsedFormula,
}

impl Checked for BrtFmlaError {
    fn check(&self) -> Result<(), &'static str> {
        check_cell_formula(&self.cell, &self.formula)
    }
}

assign_id!(BrtFmlaError);

/// `2.4.686`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFmlaNum {
    pub cell: Cell,
    pub xnum: Xnum,
    pub grbitFlags: u16,
    pub formula: CellParsedFormula,
}

impl Checked for BrtFmlaNum {
    fn check(&self) -> Result<(), &'static str> {
        check_cell_formula(&self.cell, &self.formula)
    }
}

assign_id!(BrtFmlaNum);

/// `2.4.687`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtFmlaString {
    pub cell: Cell,
    pub value: XlWideString,
    pub grbitFlags: u16,
    pub formula: CellParsedFormula,
}

impl Checked for BrtFmlaString {
    fn check(&self) -> Result<(), &'static str> {
        check_cell_formula(&self.cell, &self.formula)?;
        if self.value.len() >= 32_768 {
            return Err("formula result string exceeds 32767 UTF-16 code units");
        }
        Ok(())
    }
}

assign_id!(BrtFmlaString);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::XlStringExt;

    #[test]
    fn formula_lengths_must_match_their_bytes() {
        let formula = CellParsedFormula::default();
        assert!(formula.check().is_ok());

        let formula = CellParsedFormula {
            cce: 2,
            ..CellParsedFormula::default()
        };
        assert!(formula.check().is_err());

        let formula = CellParsedFormula {
            cb: 1,
            ..CellParsedFormula::default()
        };
        assert!(formula.check().is_err());
    }

    #[test]
    fn formula_records_roundtrip_with_payloads() {
        fn roundtrip<T>(record: &T)
        where
            T: for<'a> deku::DekuContainerRead<'a>
                + DekuContainerWrite
                + Checked
                + PartialEq
                + std::fmt::Debug,
        {
            let bytes = record.to_bytes().expect("serializes");
            let (_, parsed) = T::from_bytes((&bytes, 0)).expect("deserializes");
            record.check().expect("source is valid");
            parsed.check().expect("parsed is valid");
            assert_eq!(record, &parsed);
        }

        let formula = CellParsedFormula {
            cce: 7,
            rgce: vec![0x1E, 1, 0, 0x1E, 2, 0, 0x03],
            cb: 0,
            rgcb: Vec::new(),
        };
        roundtrip(&BrtFmlaNum {
            cell: Cell {
                column: 2,
                iStyleRef: 1,
                fPhShow: false,
            },
            xnum: Xnum::from(2.5),
            grbitFlags: 0,
            formula: formula.clone(),
        });
        roundtrip(&BrtFmlaString {
            cell: Cell::default(),
            value: XlWideString::construct(Some("ok")),
            grbitFlags: 0,
            formula: formula.clone(),
        });
        roundtrip(&BrtFmlaBool {
            cell: Cell::default(),
            bBool: true,
            grbitFlags: 0,
            formula: formula.clone(),
        });
        roundtrip(&BrtFmlaError {
            cell: Cell::default(),
            bError: BErr::Div0,
            grbitFlags: 0,
            formula,
        });
    }

    #[test]
    fn formatted_result_stays_within_the_cell_string_limit() {
        let mut record = BrtFmlaString::default();
        assert!(record.check().is_ok());

        record.value = XlWideString::construct(Some("x".repeat(32_768)));
        assert!(record.check().is_err());

        let bytes = record.to_bytes().expect("serializes");
        assert!(bytes.len() > 32_768);
    }
}
