#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec count fields are 32-bit; deku expands the cast into a generated impl"
)]

//! Future-record header and payload structures used by sparklines.

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::prelude::UncheckedSqRfX;

/// `2.5.62`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct FrtProductVersion {
    pub version: u16,
    #[deku(bits = 15, bit_order = "lsb")]
    pub product: u16,
    #[deku(bits = 1, bit_order = "lsb")]
    pub reserved: bool,
}

impl Default for FrtProductVersion {
    fn default() -> Self {
        Self {
            version: 0x0E02,
            product: 0,
            reserved: false,
        }
    }
}

impl Checked for FrtProductVersion {
    fn check(&self) -> Result<(), &'static str> {
        if self.reserved {
            return Err("FRT product version reserved bit must be zero");
        }
        Ok(())
    }
}

/// `2.5.61`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the FRT flags are independent wire-format bits"
)]
pub struct FrtHeader {
    #[deku(bits = 1, bit_order = "lsb")]
    pub fRef: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fSqref: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fFormulas: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fRelID: bool,
    #[deku(bits = 28, bit_order = "lsb")]
    pub reserved: u32,
    #[deku(cond = "*fSqref", default = "None")]
    pub rgSqrefs: Option<FrtSqrefs>,
    #[deku(cond = "*fFormulas", default = "None")]
    pub rgFormulas: Option<FrtFormulas>,
}

impl Checked for FrtHeader {
    fn check(&self) -> Result<(), &'static str> {
        if self.fRef || self.fRelID {
            return Err("sparkline FRT headers do not support reference or rel-id payloads");
        }
        if self.reserved != 0 {
            return Err("FRT header reserved bits must be zero");
        }
        if self.fSqref != self.rgSqrefs.is_some() {
            return Err("FRT header sqref flag does not match its payload");
        }
        if self.fFormulas != self.rgFormulas.is_some() {
            return Err("FRT header formula flag does not match its payload");
        }
        if let Some(sqrefs) = &self.rgSqrefs {
            sqrefs.check()?;
        }
        if let Some(formulas) = &self.rgFormulas {
            formulas.check()?;
        }
        Ok(())
    }
}

/// `2.5.66`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the FRT flags are independent wire-format bits"
)]
pub struct FrtSqref {
    #[deku(bits = 1, bit_order = "lsb")]
    pub fAdjDelete: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fDoAdjust: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fAdjChange: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fEdit: bool,
    #[deku(bits = 12, bit_order = "lsb")]
    pub reserved1: u16,
    #[deku(bits = 1, bit_order = "lsb")]
    pub unused: bool,
    #[deku(bits = 15, bit_order = "lsb")]
    pub reserved2: u16,
    pub sqrfx: UncheckedSqRfX,
}

impl Default for FrtSqref {
    fn default() -> Self {
        Self {
            fAdjDelete: false,
            fDoAdjust: true,
            fAdjChange: false,
            fEdit: false,
            reserved1: 0,
            unused: false,
            reserved2: 0,
            sqrfx: UncheckedSqRfX::default(),
        }
    }
}

impl Checked for FrtSqref {
    fn check(&self) -> Result<(), &'static str> {
        if !self.fDoAdjust {
            return Err("FRT sqref fDoAdjust must be set");
        }
        if self.reserved1 != 0 || self.reserved2 != 0 {
            return Err("FRT sqref reserved bits must be zero");
        }
        self.sqrfx.check()
    }
}

/// `2.5.67`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct FrtSqrefs {
    #[deku(update = "self.array.len() as u32")]
    pub csqref: u32,
    #[deku(count = "csqref")]
    pub array: Vec<FrtSqref>,
}

impl Checked for FrtSqrefs {
    fn check(&self) -> Result<(), &'static str> {
        if usize::try_from(self.csqref).ok() != Some(self.array.len()) {
            return Err("FRT sqref count does not match the array");
        }
        for sqref in &self.array {
            sqref.check()?;
        }
        Ok(())
    }
}

/// `2.5.59`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct FrtFormula {
    #[deku(bits = 1, bit_order = "lsb")]
    pub reserved1: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub reserved2: bool,
    #[deku(bits = 30, bit_order = "lsb")]
    pub reserved3: u32,
    pub formula: FrtParsedFormula,
}

impl Default for FrtFormula {
    fn default() -> Self {
        Self {
            reserved1: false,
            reserved2: true,
            reserved3: 0,
            formula: FrtParsedFormula::default(),
        }
    }
}

impl Checked for FrtFormula {
    fn check(&self) -> Result<(), &'static str> {
        if self.reserved1 || !self.reserved2 || self.reserved3 != 0 {
            return Err("FRT formula reserved bits are invalid");
        }
        self.formula.check()
    }
}

/// `2.5.60`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct FrtFormulas {
    #[deku(update = "self.array.len() as u32")]
    pub cformula: u32,
    #[deku(count = "cformula")]
    pub array: Vec<FrtFormula>,
}

impl Checked for FrtFormulas {
    fn check(&self) -> Result<(), &'static str> {
        if usize::try_from(self.cformula).ok() != Some(self.array.len()) {
            return Err("FRT formula count does not match the array");
        }
        for formula in &self.array {
            formula.check()?;
        }
        Ok(())
    }
}

/// `2.5.98.9`
///
/// Unlike [`CellParsedFormula`](crate::prelude::CellParsedFormula), the `cb`
/// field precedes `rgce` in this structure.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct FrtParsedFormula {
    #[deku(update = "self.rgce.len() as u32")]
    pub cce: u32,
    #[deku(update = "self.rgcb.len() as u32")]
    pub cb: u32,
    #[deku(count = "cce")]
    pub rgce: Vec<u8>,
    #[deku(count = "cb")]
    pub rgcb: Vec<u8>,
}

impl Default for FrtParsedFormula {
    fn default() -> Self {
        Self {
            cce: 3,
            cb: 0,
            rgce: vec![0x1E, 0, 0],
            rgcb: Vec::new(),
        }
    }
}

impl Checked for FrtParsedFormula {
    fn check(&self) -> Result<(), &'static str> {
        if self.cce == 0 || self.cce >= 16_385 {
            return Err("FRT formula size is outside 1..=16384");
        }
        if usize::try_from(self.cce).ok() != Some(self.rgce.len()) {
            return Err("FRT formula size does not match its bytes");
        }
        if usize::try_from(self.cb).ok() != Some(self.rgcb.len()) {
            return Err("FRT formula extra-data size does not match its bytes");
        }
        Ok(())
    }
}
