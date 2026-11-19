//! Scalar formula parsing and formula value type.

use rust_xlsb_core::prelude::{BErr, CellParsedFormula};

use crate::data::{FormulaResult, IntoExcelData};
use crate::{Error, Format, Result, Worksheet};

const PTG_ADD: u8 = 0x03;
const PTG_SUB: u8 = 0x04;
const PTG_MUL: u8 = 0x05;
const PTG_DIV: u8 = 0x06;
const PTG_POWER: u8 = 0x07;
const PTG_CONCAT: u8 = 0x08;
const PTG_LT: u8 = 0x09;
const PTG_LE: u8 = 0x0A;
const PTG_EQ: u8 = 0x0B;
const PTG_GE: u8 = 0x0C;
const PTG_GT: u8 = 0x0D;
const PTG_NE: u8 = 0x0E;
const PTG_UPLUS: u8 = 0x12;
const PTG_UMINUS: u8 = 0x13;
const PTG_PERCENT: u8 = 0x14;
const PTG_PAREN: u8 = 0x15;
const PTG_MISS_ARG: u8 = 0x16;
const PTG_STR: u8 = 0x17;
const PTG_ERR: u8 = 0x1C;
const PTG_BOOL: u8 = 0x1D;
const PTG_INT: u8 = 0x1E;
const PTG_NUM: u8 = 0x1F;
const PTG_FUNC: u8 = 0x41;
const PTG_FUNC_VAR: u8 = 0x42;
const PTG_REF: u8 = 0x44;
const PTG_AREA: u8 = 0x45;

const MAX_FORMULA_TOKENS: usize = 16_384;
const MAX_FORMULA_DEPTH: u32 = 64;
const MAX_UNARY_DEPTH: u32 = 64;
const MAX_STRING_UTF16: usize = 32_767;

/// A worksheet formula with an optional cached result.
///
/// The formula is parsed when it is written. Strings use US-English Excel
/// syntax and a comma argument separator. A leading `=` and surrounding array
/// braces are optional. Supported references are same-sheet A1 cells and
/// ranges; cross-sheet references, defined names, array formulas, and dynamic
/// array functions are rejected.
#[derive(Clone, PartialEq, Eq)]
pub struct Formula {
    formula: Box<str>,
    result: Option<Box<str>>,
}

impl Formula {
    /// Creates a formula.
    #[must_use]
    pub fn new(formula: impl AsRef<str>) -> Self {
        let mut formula = formula.as_ref();
        if let Some(rest) = formula.strip_prefix('{') {
            formula = rest;
        }
        if let Some(rest) = formula.strip_prefix('=') {
            formula = rest;
        }
        if let Some(rest) = formula.strip_suffix('}') {
            formula = rest;
        }
        Self {
            formula: formula.into(),
            result: None,
        }
    }

    /// Sets the cached result for this formula.
    ///
    /// Spreadsheet applications that do not calculate formulas display this
    /// value. Numeric, `TRUE`/`FALSE`, and Excel error strings are stored as
    /// their matching XLSB result types; other values are stored as strings.
    #[must_use]
    pub fn set_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into().into_boxed_str());
        self
    }

    pub(crate) fn into_parts(self) -> (Box<str>, Option<Box<str>>) {
        (self.formula, self.result)
    }
}

impl From<&str> for Formula {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&Self> for Formula {
    fn from(value: &Self) -> Self {
        value.clone()
    }
}

impl IntoExcelData for Formula {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        worksheet.store_formula(row, col, self, None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        worksheet.store_formula(row, col, self, Some(format))
    }
}

impl IntoExcelData for &Formula {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        worksheet.store_formula(row, col, self.clone(), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        worksheet.store_formula(row, col, self.clone(), Some(format))
    }
}

pub fn compile(formula: &str) -> Result<CellParsedFormula> {
    let formula = formula.strip_prefix('=').unwrap_or(formula);
    let mut parser = Parser {
        input: formula,
        bytes: formula.as_bytes(),
        pos: 0,
        depth: 0,
        unary_depth: 0,
        out: Vec::with_capacity(formula.len().min(MAX_FORMULA_TOKENS)),
    };
    parser.skip_whitespace();
    if parser.input.is_empty() {
        return Err(formula_error("formula is empty"));
    }
    parser.parse_expression()?;
    parser.skip_whitespace();
    if parser.pos != parser.bytes.len() {
        return Err(formula_error("unexpected trailing formula text"));
    }
    if parser.out.len() > MAX_FORMULA_TOKENS {
        return Err(formula_error("formula token stream exceeds 16384 bytes"));
    }
    let cce = u32::try_from(parser.out.len())
        .map_err(|_| formula_error("formula token stream is too long"))?;
    Ok(CellParsedFormula {
        cce,
        rgce: parser.out,
        cb: 0,
        rgcb: Vec::new(),
    })
}

pub fn cached_result(value: &str) -> FormulaResult {
    if let Ok(number) = value.parse::<f64>()
        && number.is_finite()
    {
        return FormulaResult::Number(number);
    }
    match value {
        "TRUE" => FormulaResult::Bool(true),
        "FALSE" => FormulaResult::Bool(false),
        "#NULL!" => FormulaResult::Error(BErr::Null),
        "#DIV/0!" => FormulaResult::Error(BErr::Div0),
        "#VALUE!" => FormulaResult::Error(BErr::Value),
        "#REF!" => FormulaResult::Error(BErr::Ref),
        "#NAME?" => FormulaResult::Error(BErr::Name),
        "#NUM!" => FormulaResult::Error(BErr::Num),
        "#N/A" => FormulaResult::Error(BErr::NA),
        "#GETTING_DATA" => FormulaResult::Error(BErr::GettingData),
        _ => FormulaResult::String(value.into()),
    }
}

pub fn result_is_too_long(result: &FormulaResult) -> bool {
    matches!(result, FormulaResult::String(text) if text.encode_utf16().count() > MAX_STRING_UTF16)
}

fn formula_error(message: &str) -> Error {
    Error::FormulaError(message.to_owned())
}

#[derive(Clone, Copy)]
struct CellRef {
    row: u32,
    col: u16,
    row_relative: bool,
    col_relative: bool,
    end: usize,
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    depth: u32,
    unary_depth: u32,
    out: Vec<u8>,
}

impl Parser<'_> {
    fn parse_expression(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_FORMULA_DEPTH {
            return Err(formula_error("formula nesting exceeds 64 levels"));
        }
        let result = self.parse_comparison();
        self.depth -= 1;
        result
    }

    fn parse_comparison(&mut self) -> Result<()> {
        self.parse_concat()?;
        loop {
            self.skip_whitespace();
            let operator = match (self.peek(), self.peek_n(1)) {
                (Some(b'<'), Some(b'=')) => PTG_LE,
                (Some(b'>'), Some(b'=')) => PTG_GE,
                (Some(b'<'), Some(b'>')) => PTG_NE,
                (Some(b'<'), _) => PTG_LT,
                (Some(b'>'), _) => PTG_GT,
                (Some(b'='), _) => PTG_EQ,
                _ => return Ok(()),
            };
            self.pos += if matches!(operator, PTG_LE | PTG_GE | PTG_NE) {
                2
            } else {
                1
            };
            self.parse_concat()?;
            self.out.push(operator);
        }
    }

    fn parse_concat(&mut self) -> Result<()> {
        self.parse_additive()?;
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'&') {
                return Ok(());
            }
            self.pos += 1;
            self.parse_additive()?;
            self.out.push(PTG_CONCAT);
        }
    }

    fn parse_additive(&mut self) -> Result<()> {
        self.parse_multiplicative()?;
        loop {
            self.skip_whitespace();
            let operator = match self.peek() {
                Some(b'+') => PTG_ADD,
                Some(b'-') => PTG_SUB,
                _ => return Ok(()),
            };
            self.pos += 1;
            self.parse_multiplicative()?;
            self.out.push(operator);
        }
    }

    fn parse_multiplicative(&mut self) -> Result<()> {
        self.parse_power()?;
        loop {
            self.skip_whitespace();
            let operator = match self.peek() {
                Some(b'*') => PTG_MUL,
                Some(b'/') => PTG_DIV,
                _ => return Ok(()),
            };
            self.pos += 1;
            self.parse_power()?;
            self.out.push(operator);
        }
    }

    fn parse_power(&mut self) -> Result<()> {
        self.parse_unary()?;
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'^') {
                return Ok(());
            }
            self.pos += 1;
            self.parse_unary()?;
            self.out.push(PTG_POWER);
        }
    }

    fn parse_unary(&mut self) -> Result<()> {
        self.unary_depth += 1;
        if self.unary_depth > MAX_UNARY_DEPTH {
            return Err(formula_error("formula unary nesting exceeds 64 levels"));
        }
        let result = self.parse_unary_inner();
        self.unary_depth -= 1;
        result
    }

    fn parse_unary_inner(&mut self) -> Result<()> {
        self.skip_whitespace();
        let operator = match self.peek() {
            Some(b'+') => PTG_UPLUS,
            Some(b'-') => PTG_UMINUS,
            _ => return self.parse_postfix(),
        };
        self.pos += 1;
        self.parse_unary()?;
        self.out.push(operator);
        Ok(())
    }

    fn parse_postfix(&mut self) -> Result<()> {
        self.parse_primary()?;
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'%') {
                return Ok(());
            }
            self.pos += 1;
            self.out.push(PTG_PERCENT);
        }
    }

    fn parse_primary(&mut self) -> Result<()> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                self.parse_expression()?;
                self.skip_whitespace();
                self.expect(b')')?;
                self.out.push(PTG_PAREN);
                Ok(())
            }
            Some(b'"') => self.parse_string(),
            Some(b'#') => self.parse_error(),
            Some(b'0'..=b'9' | b'.') => self.parse_number(),
            Some(b'$' | b'A'..=b'Z' | b'a'..=b'z') => self.parse_name_or_ref(),
            _ => Err(formula_error("expected a formula value")),
        }
    }

    fn parse_name_or_ref(&mut self) -> Result<()> {
        let start = self.pos;
        if let Some(reference) = self.cell_ref_at(self.pos) {
            let mut after = reference.end;
            while self
                .bytes
                .get(after)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            {
                after += 1;
            }
            if self.bytes.get(after) == Some(&b'(') {
                self.pos = reference.end;
                return self.parse_function(&self.input[start..reference.end]);
            }
            self.pos = reference.end;
            self.skip_whitespace();
            if self.peek() == Some(b':') {
                self.pos += 1;
                self.skip_whitespace();
                let Some(last) = self.cell_ref_at(self.pos) else {
                    return Err(formula_error("expected a cell reference after ':'"));
                };
                self.pos = last.end;
                self.push_area(&reference, &last);
            } else if self.peek() == Some(b'!') {
                return Err(formula_error("cross-sheet references are not supported"));
            } else {
                self.push_ref(&reference);
            }
            return Ok(());
        }

        let start = self.pos;
        self.pos += 1;
        while let Some(byte) = self.peek() {
            if is_identifier_byte(byte) {
                self.pos += 1;
            } else {
                break;
            }
        }
        let name = &self.input[start..self.pos];
        self.skip_whitespace();
        if self.peek() == Some(b'!') {
            return Err(formula_error("cross-sheet references are not supported"));
        }
        if self.peek() == Some(b'(') {
            return self.parse_function(name);
        }
        if name.eq_ignore_ascii_case("TRUE") {
            self.out.extend_from_slice(&[PTG_BOOL, 1]);
            return Ok(());
        }
        if name.eq_ignore_ascii_case("FALSE") {
            self.out.extend_from_slice(&[PTG_BOOL, 0]);
            return Ok(());
        }
        Err(formula_error("named ranges are not supported"))
    }

    fn parse_function(&mut self, name: &str) -> Result<()> {
        let Some(function) = lookup_function(name) else {
            return Err(formula_error("unknown formula function"));
        };
        self.expect(b'(')?;
        let mut arguments = 0_u8;
        self.skip_whitespace();
        if self.peek() == Some(b')') {
            self.pos += 1;
        } else {
            loop {
                self.skip_whitespace();
                if matches!(self.peek(), Some(b',' | b')')) {
                    self.out.push(PTG_MISS_ARG);
                } else {
                    self.parse_expression()?;
                }
                arguments = arguments
                    .checked_add(1)
                    .ok_or_else(|| formula_error("too many function arguments"))?;
                self.skip_whitespace();
                match self.peek() {
                    Some(b',') => self.pos += 1,
                    Some(b')') => {
                        self.pos += 1;
                        break;
                    }
                    _ => return Err(formula_error("expected ',' or ')' in function call")),
                }
            }
        }
        function.check_arity(arguments)?;
        function.push_call(arguments, &mut self.out);
        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is checked to be a non-negative integer below 65536"
    )]
    fn parse_number(&mut self) -> Result<()> {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_digit() || byte == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let exponent = self.pos;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let digits = self.pos;
            while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
                self.pos += 1;
            }
            if self.pos == digits {
                self.pos = exponent;
            }
        }
        let value = self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|_| formula_error("invalid number"))?;
        if !value.is_finite() {
            return Err(formula_error("formula number is not finite"));
        }
        if value >= 0.0 && value <= f64::from(u16::MAX) && value.fract() == 0.0 {
            self.out.push(PTG_INT);
            self.out.extend_from_slice(&(value as u16).to_le_bytes());
        } else {
            self.out.push(PTG_NUM);
            self.out.extend_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }

    fn parse_string(&mut self) -> Result<()> {
        self.pos += 1;
        self.out.push(PTG_STR);
        let length_at = self.out.len();
        self.out.extend_from_slice(&[0, 0]);
        let mut units = 0_u16;
        loop {
            let start = self.pos;
            while self.peek().is_some_and(|byte| byte != b'"') {
                self.pos += 1;
            }
            for unit in self.input[start..self.pos].encode_utf16() {
                if units == 255 {
                    return Err(formula_error("formula string exceeds 255 characters"));
                }
                self.out.extend_from_slice(&unit.to_le_bytes());
                units += 1;
            }
            if self.peek() != Some(b'"') {
                return Err(formula_error("unterminated formula string"));
            }
            self.pos += 1;
            if self.peek() == Some(b'"') {
                if units == 255 {
                    return Err(formula_error("formula string exceeds 255 characters"));
                }
                self.out.extend_from_slice(&(u16::from(b'"')).to_le_bytes());
                units += 1;
                self.pos += 1;
            } else {
                break;
            }
        }
        self.out[length_at..length_at + 2].copy_from_slice(&units.to_le_bytes());
        Ok(())
    }

    fn parse_error(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += 1;
        while self.peek().is_some_and(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'!' | b'_' | b'?')
        }) {
            self.pos += 1;
        }
        let error = match &self.input[start..self.pos] {
            "#NULL!" => BErr::Null,
            "#DIV/0!" => BErr::Div0,
            "#VALUE!" => BErr::Value,
            "#REF!" => BErr::Ref,
            "#NAME?" => BErr::Name,
            "#NUM!" => BErr::Num,
            "#N/A" => BErr::NA,
            "#GETTING_DATA" => BErr::GettingData,
            _ => return Err(formula_error("unknown formula error literal")),
        };
        self.out.push(PTG_ERR);
        self.out.push(error_byte(error));
        Ok(())
    }

    fn push_ref(&mut self, reference: &CellRef) {
        self.out.push(PTG_REF);
        self.out.extend_from_slice(&reference.row.to_le_bytes());
        self.out
            .extend_from_slice(&column_word(reference).to_le_bytes());
    }

    fn push_area(&mut self, first: &CellRef, last: &CellRef) {
        self.out.push(PTG_AREA);
        self.out.extend_from_slice(&first.row.to_le_bytes());
        self.out.extend_from_slice(&last.row.to_le_bytes());
        self.out
            .extend_from_slice(&column_word(first).to_le_bytes());
        self.out.extend_from_slice(&column_word(last).to_le_bytes());
    }

    fn cell_ref_at(&self, start: usize) -> Option<CellRef> {
        let mut pos = start;
        let col_relative = self.bytes.get(pos) != Some(&b'$');
        if !col_relative {
            pos += 1;
        }
        let col_start = pos;
        while self.bytes.get(pos).is_some_and(u8::is_ascii_alphabetic) {
            pos += 1;
        }
        if pos == col_start || pos - col_start > 3 {
            return None;
        }
        let mut col = 0_u32;
        for byte in &self.bytes[col_start..pos] {
            col = col * 26 + u32::from(byte.to_ascii_uppercase() - b'A') + 1;
        }
        if col == 0 || col > 16_384 {
            return None;
        }
        let row_relative = self.bytes.get(pos) != Some(&b'$');
        if !row_relative {
            pos += 1;
        }
        let row_start = pos;
        while self.bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
        if pos == row_start || pos - row_start > 7 {
            return None;
        }
        let row = self.input[row_start..pos].parse::<u32>().ok()?;
        if row == 0 || row > 1_048_576 {
            return None;
        }
        Some(CellRef {
            row: row - 1,
            col: u16::try_from(col - 1).ok()?,
            row_relative,
            col_relative,
            end: pos,
        })
    }

    fn expect(&mut self, expected: u8) -> Result<()> {
        self.skip_whitespace();
        if self.peek() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(formula_error("unexpected formula character"))
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_n(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }
}

const fn column_word(reference: &CellRef) -> u16 {
    reference.col
        | if reference.col_relative { 0x4000 } else { 0 }
        | if reference.row_relative { 0x8000 } else { 0 }
}

const fn error_byte(error: BErr) -> u8 {
    match error {
        BErr::Null => 0x00,
        BErr::Div0 => 0x07,
        BErr::Value => 0x0F,
        BErr::Ref => 0x17,
        BErr::Name => 0x1D,
        BErr::Num => 0x24,
        BErr::NA => 0x2A,
        BErr::GettingData => 0x2B,
    }
}

const fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'\\')
}

#[derive(Clone, Copy)]
enum Arity {
    Fixed(u8),
    Range(u8, u8),
}

#[derive(Clone, Copy)]
struct Function {
    name: &'static str,
    id: u16,
    arity: Arity,
}

impl Function {
    fn check_arity(self, arguments: u8) -> Result<()> {
        let valid = match self.arity {
            Arity::Fixed(expected) => arguments == expected,
            Arity::Range(min, max) => (min..=max).contains(&arguments),
        };
        if valid {
            Ok(())
        } else {
            Err(formula_error("wrong number of function arguments"))
        }
    }

    fn push_call(self, arguments: u8, out: &mut Vec<u8>) {
        match self.arity {
            Arity::Fixed(_) => {
                out.push(PTG_FUNC);
                out.extend_from_slice(&self.id.to_le_bytes());
            }
            Arity::Range(_, _) => {
                out.push(PTG_FUNC_VAR);
                out.push(arguments);
                out.extend_from_slice(&self.id.to_le_bytes());
            }
        }
    }
}

const fn function(name: &'static str, id: u16, arity: Arity) -> Function {
    Function { name, id, arity }
}

const FUNCTIONS: &[Function] = &[
    function("COUNT", 0x0000, Arity::Range(1, 255)),
    function("IF", 0x0001, Arity::Range(2, 3)),
    function("ISNA", 0x0002, Arity::Fixed(1)),
    function("ISERROR", 0x0003, Arity::Fixed(1)),
    function("SUM", 0x0004, Arity::Range(1, 255)),
    function("AVERAGE", 0x0005, Arity::Range(1, 255)),
    function("MIN", 0x0006, Arity::Range(1, 255)),
    function("MAX", 0x0007, Arity::Range(1, 255)),
    function("ROW", 0x0008, Arity::Range(0, 1)),
    function("COLUMN", 0x0009, Arity::Range(0, 1)),
    function("NA", 0x000A, Arity::Fixed(0)),
    function("STDEV", 0x000C, Arity::Range(1, 255)),
    function("SIN", 0x000F, Arity::Fixed(1)),
    function("COS", 0x0010, Arity::Fixed(1)),
    function("TAN", 0x0011, Arity::Fixed(1)),
    function("ATAN", 0x0012, Arity::Fixed(1)),
    function("PI", 0x0013, Arity::Fixed(0)),
    function("SQRT", 0x0014, Arity::Fixed(1)),
    function("EXP", 0x0015, Arity::Fixed(1)),
    function("LN", 0x0016, Arity::Fixed(1)),
    function("LOG10", 0x0017, Arity::Fixed(1)),
    function("ABS", 0x0018, Arity::Fixed(1)),
    function("INT", 0x0019, Arity::Fixed(1)),
    function("SIGN", 0x001A, Arity::Fixed(1)),
    function("ROUND", 0x001B, Arity::Fixed(2)),
    function("LOOKUP", 0x001C, Arity::Range(2, 3)),
    function("INDEX", 0x001D, Arity::Range(2, 4)),
    function("REPT", 0x001E, Arity::Fixed(2)),
    function("MID", 0x001F, Arity::Fixed(3)),
    function("LEN", 0x0020, Arity::Fixed(1)),
    function("VALUE", 0x0021, Arity::Fixed(1)),
    function("TRUE", 0x0022, Arity::Fixed(0)),
    function("FALSE", 0x0023, Arity::Fixed(0)),
    function("AND", 0x0024, Arity::Range(1, 255)),
    function("OR", 0x0025, Arity::Range(1, 255)),
    function("NOT", 0x0026, Arity::Fixed(1)),
    function("MOD", 0x0027, Arity::Fixed(2)),
    function("VAR", 0x002E, Arity::Range(1, 255)),
    function("TEXT", 0x0030, Arity::Fixed(2)),
    function("RAND", 0x003F, Arity::Fixed(0)),
    function("MATCH", 0x0040, Arity::Range(2, 3)),
    function("DATE", 0x0041, Arity::Fixed(3)),
    function("TIME", 0x0042, Arity::Fixed(3)),
    function("DAY", 0x0043, Arity::Fixed(1)),
    function("MONTH", 0x0044, Arity::Fixed(1)),
    function("YEAR", 0x0045, Arity::Fixed(1)),
    function("WEEKDAY", 0x0046, Arity::Range(1, 2)),
    function("HOUR", 0x0047, Arity::Fixed(1)),
    function("MINUTE", 0x0048, Arity::Fixed(1)),
    function("SECOND", 0x0049, Arity::Fixed(1)),
    function("NOW", 0x004A, Arity::Fixed(0)),
    function("AREAS", 0x004B, Arity::Fixed(1)),
    function("ROWS", 0x004C, Arity::Fixed(1)),
    function("COLUMNS", 0x004D, Arity::Fixed(1)),
    function("SEARCH", 0x0052, Arity::Range(2, 3)),
    function("TRANSPOSE", 0x0053, Arity::Fixed(1)),
    function("TYPE", 0x0056, Arity::Fixed(1)),
    function("ATAN2", 0x0061, Arity::Fixed(2)),
    function("ASIN", 0x0062, Arity::Fixed(1)),
    function("ACOS", 0x0063, Arity::Fixed(1)),
    function("CHOOSE", 0x0064, Arity::Range(2, 255)),
    function("HLOOKUP", 0x0065, Arity::Range(3, 4)),
    function("VLOOKUP", 0x0066, Arity::Range(3, 4)),
    function("ISREF", 0x0069, Arity::Fixed(1)),
    function("LOG", 0x006D, Arity::Range(1, 2)),
    function("CHAR", 0x006F, Arity::Fixed(1)),
    function("LOWER", 0x0070, Arity::Fixed(1)),
    function("UPPER", 0x0071, Arity::Fixed(1)),
    function("PROPER", 0x0072, Arity::Fixed(1)),
    function("LEFT", 0x0073, Arity::Range(1, 2)),
    function("RIGHT", 0x0074, Arity::Range(1, 2)),
    function("EXACT", 0x0075, Arity::Fixed(2)),
    function("TRIM", 0x0076, Arity::Fixed(1)),
    function("REPLACE", 0x0077, Arity::Fixed(4)),
    function("SUBSTITUTE", 0x0078, Arity::Range(3, 4)),
    function("CODE", 0x0079, Arity::Fixed(1)),
    function("FIND", 0x007C, Arity::Range(2, 3)),
    function("ISERR", 0x007E, Arity::Fixed(1)),
    function("ISTEXT", 0x007F, Arity::Fixed(1)),
    function("ISNUMBER", 0x0080, Arity::Fixed(1)),
    function("ISBLANK", 0x0081, Arity::Fixed(1)),
    function("T", 0x0082, Arity::Fixed(1)),
    function("N", 0x0083, Arity::Fixed(1)),
    function("DATEVALUE", 0x008C, Arity::Fixed(1)),
    function("TIMEVALUE", 0x008D, Arity::Fixed(1)),
    function("INDIRECT", 0x0094, Arity::Range(1, 2)),
    function("CLEAN", 0x00A2, Arity::Fixed(1)),
    function("COUNTA", 0x00A9, Arity::Range(1, 255)),
    function("PRODUCT", 0x00B7, Arity::Range(1, 255)),
    function("FACT", 0x00B8, Arity::Fixed(1)),
    function("STDEVP", 0x00C1, Arity::Range(1, 255)),
    function("VARP", 0x00C2, Arity::Range(1, 255)),
    function("TRUNC", 0x00C5, Arity::Range(1, 2)),
    function("ISLOGICAL", 0x00C6, Arity::Fixed(1)),
    function("ROUNDUP", 0x00D4, Arity::Fixed(2)),
    function("ROUNDDOWN", 0x00D5, Arity::Fixed(2)),
    function("RANK", 0x00D8, Arity::Range(2, 3)),
    function("TODAY", 0x00DD, Arity::Fixed(0)),
    function("MEDIAN", 0x00E3, Arity::Range(1, 255)),
    function("SUMPRODUCT", 0x00E4, Arity::Range(1, 255)),
    function("SINH", 0x00E5, Arity::Fixed(1)),
    function("COSH", 0x00E6, Arity::Fixed(1)),
    function("TANH", 0x00E7, Arity::Fixed(1)),
    function("ASINH", 0x00E8, Arity::Fixed(1)),
    function("ACOSH", 0x00E9, Arity::Fixed(1)),
    function("ATANH", 0x00EA, Arity::Fixed(1)),
    function("CONCATENATE", 0x0150, Arity::Range(1, 255)),
    function("POWER", 0x0151, Arity::Fixed(2)),
    function("RADIANS", 0x0156, Arity::Fixed(1)),
    function("DEGREES", 0x0157, Arity::Fixed(1)),
    function("SUMIF", 0x0159, Arity::Range(2, 3)),
    function("COUNTIF", 0x015A, Arity::Fixed(2)),
    function("COUNTBLANK", 0x015B, Arity::Fixed(1)),
    function("ISPMT", 0x015E, Arity::Fixed(4)),
    function("DATEDIF", 0x015F, Arity::Fixed(3)),
    function("ROMAN", 0x0162, Arity::Range(1, 2)),
];

fn lookup_function(name: &str) -> Option<Function> {
    FUNCTIONS
        .iter()
        .copied()
        .find(|function| function.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(formula: &str) -> Vec<u8> {
        compile(formula).expect("formula compiles").rgce
    }

    #[test]
    fn compiles_arithmetic_and_references() {
        assert_eq!(
            tokens("=A1+1"),
            [PTG_REF, 0, 0, 0, 0, 0, 0xC0, PTG_INT, 1, 0, PTG_ADD]
        );
        assert_eq!(
            tokens("=1+2*3"),
            [
                PTG_INT, 1, 0, PTG_INT, 2, 0, PTG_INT, 3, 0, PTG_MUL, PTG_ADD
            ]
        );
        assert_eq!(tokens("=$B$2"), [PTG_REF, 1, 0, 0, 0, 1, 0]);
    }

    #[test]
    fn compiles_ranges_and_functions() {
        assert_eq!(
            tokens("=SUM(A1:B2)"),
            [
                PTG_AREA,
                0,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                0xC0,
                1,
                0xC0,
                PTG_FUNC_VAR,
                1,
                4,
                0
            ]
        );
        assert_eq!(
            tokens("=IF(A1>0,\"yes\",\"no\")"),
            [
                PTG_REF,
                0,
                0,
                0,
                0,
                0,
                0xC0,
                PTG_INT,
                0,
                0,
                PTG_GT,
                PTG_STR,
                3,
                0,
                b'y',
                0,
                b'e',
                0,
                b's',
                0,
                PTG_STR,
                2,
                0,
                b'n',
                0,
                b'o',
                0,
                PTG_FUNC_VAR,
                3,
                1,
                0
            ]
        );
        assert_eq!(
            tokens("=ABS(-1)"),
            [PTG_INT, 1, 0, PTG_UMINUS, PTG_FUNC, 0x18, 0]
        );
        assert_eq!(tokens("=LOG10(100)"), [PTG_INT, 100, 0, PTG_FUNC, 0x17, 0]);
    }

    #[test]
    fn compiles_parentheses_and_error_literals() {
        assert_eq!(
            tokens("=(1+2)*3"),
            [
                PTG_INT, 1, 0, PTG_INT, 2, 0, PTG_ADD, PTG_PAREN, PTG_INT, 3, 0, PTG_MUL
            ]
        );
        assert_eq!(tokens("=#N/A"), [PTG_ERR, 0x2A]);
        assert_eq!(tokens("=#GETTING_DATA"), [PTG_ERR, 0x2B]);
    }

    #[test]
    fn compiles_strings_unary_operators_and_missing_arguments() {
        assert_eq!(
            tokens(r#"=-"a""b"%"#),
            [
                PTG_STR,
                3,
                0,
                b'a',
                0,
                b'"',
                0,
                b'b',
                0,
                PTG_PERCENT,
                PTG_UMINUS
            ]
        );
        assert_eq!(tokens("=IF(,1,2)")[..4], [PTG_MISS_ARG, PTG_INT, 1, 0]);
    }

    #[test]
    fn rejects_unsupported_or_malformed_formulas() {
        for formula in [
            "",
            "=Sheet2!A1",
            "=named_range",
            "=UNKNOWN(1)",
            "=SUM(",
            "=A0",
            "=XFE1",
            "=1+",
            "=\"unterminated",
            "=A1:B2:C3",
            "=#BOGUS!",
            "=ABS()",
            "=ABS(1,2)",
            "=IF(1)",
            "=PI(1)",
            "=SUM()",
            "=COUNT()",
        ] {
            assert!(compile(formula).is_err(), "{formula}");
        }

        let nested = format!("{}1{}", "(".repeat(65), ")".repeat(65));
        assert!(compile(&nested).is_err());
        let unary = format!("={}1", "-".repeat(65));
        assert!(compile(&unary).is_err());

        let string_255 = format!("=\"{}\"", "x".repeat(255));
        let string_tokens = tokens(&string_255);
        assert_eq!(&string_tokens[..3], [PTG_STR, 255, 0]);
        assert_eq!(string_tokens.len(), 3 + 255 * 2);
        let string_256 = format!("=\"{}\"", "x".repeat(256));
        assert!(compile(&string_256).is_err());

        let long_formula = format!("={}", "1+".repeat(5_000) + "1");
        assert!(compile(&long_formula).is_err());
    }

    #[test]
    fn infers_cached_results() {
        assert_eq!(cached_result("0"), FormulaResult::Number(0.0));
        assert_eq!(cached_result("TRUE"), FormulaResult::Bool(true));
        assert_eq!(cached_result("FALSE"), FormulaResult::Bool(false));
        assert_eq!(cached_result("#DIV/0!"), FormulaResult::Error(BErr::Div0));
        assert_eq!(cached_result("done"), FormulaResult::String("done".into()));
        assert_eq!(
            cached_result("TRUE "),
            FormulaResult::String("TRUE ".into())
        );
    }
}
