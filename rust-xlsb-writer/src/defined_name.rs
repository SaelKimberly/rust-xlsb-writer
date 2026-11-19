//! User-defined names and their `NameParsedFormula` token encoding.

use std::borrow::Cow;

use crate::worksheet::{MAX_COL, MAX_ROW, Worksheet};
use crate::{Error, Result};

/// A workbook or worksheet-scoped defined name.
#[derive(Debug, Clone)]
pub struct DefinedName {
    pub(crate) name: String,
    pub(crate) formula: String,
    pub(crate) scope: Option<String>,
    pub(crate) sort_name: String,
}

impl DefinedName {
    pub(crate) fn new(name: impl Into<String>, formula: &str) -> Result<Self> {
        let name = name.into();
        if name.starts_with('!') {
            return Err(name_error(&name, "name cannot start with '!'"));
        }

        let (scope, local_name) = match split_local_name(&name)? {
            Some((scope, local_name)) => (Some(scope.into_owned()), local_name),
            None => (None, name.as_str()),
        };
        check_name(local_name)?;

        let formula = formula.strip_prefix('=').unwrap_or(formula);
        if formula.trim().is_empty() {
            return Err(Error::Parameter(
                "defined-name formula cannot be blank".to_owned(),
            ));
        }

        Ok(Self {
            name: local_name.to_owned(),
            formula: formula.to_owned(),
            sort_name: local_name.to_lowercase(),
            scope,
        })
    }

    pub(crate) fn encode(
        &self,
        sheets: &[Worksheet],
        xti: &mut XtiCollector,
        names: &NameTable<'_>,
    ) -> Result<Vec<u8>> {
        encode_formula_with_names(
            &self.formula,
            self.scope.as_deref(),
            sheets,
            xti,
            Some(names),
        )
    }
}

/// Lazily maps worksheet indexes onto `BrtExternSheet` Xti indexes.
pub struct XtiCollector {
    ranges: Vec<(usize, usize)>,
    indexes: Vec<Option<u16>>,
}

impl XtiCollector {
    pub(crate) fn new(sheet_count: usize) -> Self {
        Self {
            ranges: Vec::new(),
            indexes: vec![None; sheet_count],
        }
    }

    pub(crate) fn index(&mut self, sheet: usize) -> Result<u16> {
        if let Some(index) = self.indexes[sheet] {
            return Ok(index);
        }
        let index = self.push(sheet, sheet)?;
        self.indexes[sheet] = Some(index);
        Ok(index)
    }

    pub(crate) fn range(&mut self, first: usize, last: usize) -> Result<u16> {
        if first > last {
            return Err(Error::Parameter(
                "worksheet ranges must be in workbook order".to_owned(),
            ));
        }
        if first == last {
            return self.index(first);
        }
        if let Some(index) = self.ranges.iter().position(|range| *range == (first, last)) {
            return u16::try_from(index)
                .map_err(|_| Error::ValueNotRepresentable("too many defined-name ranges"));
        }
        self.push(first, last)
    }

    fn push(&mut self, first: usize, last: usize) -> Result<u16> {
        let index = u16::try_from(self.ranges.len())
            .map_err(|_| Error::ValueNotRepresentable("too many defined-name ranges"))?;
        self.ranges.push((first, last));
        Ok(index)
    }

    pub const fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub(crate) fn ranges(&self) -> &[(usize, usize)] {
        &self.ranges
    }
}

/// One user-defined name in `BrtName` collection order.
#[derive(Clone, Copy)]
pub struct NameEntry<'a> {
    pub(crate) defined: &'a DefinedName,
    pub(crate) scope: Option<usize>,
    pub(crate) index: u32,
}

/// The ordered user-name table used by `PtgName` and `BrtName`.
pub struct NameTable<'a> {
    entries: Vec<NameEntry<'a>>,
}

impl<'a> NameTable<'a> {
    pub(crate) fn new(
        sheets: &[Worksheet],
        defined_names: &'a [DefinedName],
        builtin_count: usize,
    ) -> Result<Self> {
        let mut entries = Vec::with_capacity(defined_names.len());
        for name in defined_names {
            let scope = name
                .scope
                .as_deref()
                .map(|scope| sheet_index(sheets, scope))
                .transpose()?;
            entries.push(NameEntry {
                defined: name,
                scope,
                index: 0,
            });
        }
        entries.sort_by(|left, right| left.defined.sort_name.cmp(&right.defined.sort_name));

        for (offset, entry) in entries.iter_mut().enumerate() {
            entry.index = u32::try_from(builtin_count + offset + 1)
                .map_err(|_| Error::ValueNotRepresentable("too many defined names"))?;
        }
        Ok(Self { entries })
    }

    pub(crate) fn entries(&self) -> &[NameEntry<'a>] {
        &self.entries
    }

    /// Resolves a name against the requested sheet, then the global scope.
    pub(crate) fn lookup(&self, name: &str, scope: Option<usize>) -> Result<u32> {
        let folded = (!name.is_ascii()).then(|| name.to_lowercase());
        let mut global = None;
        let mut has_local = false;
        for entry in &self.entries {
            let matches = entry.defined.name.eq_ignore_ascii_case(name)
                || folded
                    .as_deref()
                    .is_some_and(|folded| entry.defined.sort_name == folded);
            if !matches {
                continue;
            }
            if entry.scope.is_none() {
                global = Some(entry.index);
            } else if entry.scope == scope {
                return Ok(entry.index);
            } else {
                has_local = true;
            }
        }
        if let Some(index) = global {
            return Ok(index);
        }
        if scope.is_none() && has_local {
            return Err(Error::FormulaError(
                "worksheet-scoped defined names must be sheet-qualified".to_owned(),
            ));
        }
        Err(Error::FormulaError(format!("unknown defined name: {name}")))
    }
}

#[cfg(test)]
fn encode_formula(
    formula: &str,
    scope: Option<&str>,
    sheets: &[Worksheet],
    xti: &mut XtiCollector,
) -> Result<Vec<u8>> {
    encode_formula_with_names(formula, scope, sheets, xti, None)
}

fn encode_formula_with_names(
    formula: &str,
    scope: Option<&str>,
    sheets: &[Worksheet],
    xti: &mut XtiCollector,
    names: Option<&NameTable<'_>>,
) -> Result<Vec<u8>> {
    let formula = formula.trim();
    let formula = formula.strip_prefix('=').unwrap_or(formula).trim();
    if formula.is_empty() {
        return Err(Error::Parameter(
            "defined-name formula cannot be blank".to_owned(),
        ));
    }

    if formula.starts_with('"')
        && let Ok(rgce) = encode_string(formula)
    {
        return Ok(rgce);
    }
    if formula.eq_ignore_ascii_case("TRUE") {
        return Ok(vec![0x1D, 1]);
    }
    if formula.eq_ignore_ascii_case("FALSE") {
        return Ok(vec![0x1D, 0]);
    }
    if let Some(error) = error_token(formula) {
        return Ok(vec![0x1C, error]);
    }
    if let Ok(value) = formula.parse::<f64>() {
        if !value.is_finite() {
            return Err(unsupported(formula));
        }
        let mut rgce = Vec::with_capacity(9);
        rgce.push(0x1F);
        rgce.extend_from_slice(&value.to_le_bytes());
        return Ok(rgce);
    }

    if let Ok(Some((sheet_name, reference))) = split_sheet_prefix(formula)
        && is_simple_sheet_name(sheet_name.as_ref())
        && let Ok(sheet) = sheet_index(sheets, sheet_name.as_ref())
        && (absolute_a1(reference).is_some()
            || reference.split_once(':').is_some_and(|(first, last)| {
                absolute_a1(first).is_some() && absolute_a1(last).is_some()
            }))
    {
        let ixti = xti.index(sheet)?;
        return encode_reference(reference, ixti);
    }

    if let Some((row, col)) = absolute_a1(formula) {
        let Some(scope) = scope else {
            return Err(Error::Parameter(
                "global defined-name references must include a sheet name".to_owned(),
            ));
        };
        let ixti = xti.index(sheet_index(sheets, scope)?)?;
        let mut rgce = Vec::with_capacity(9);
        rgce.push(0x3A);
        rgce.extend_from_slice(&ixti.to_le_bytes());
        rgce.extend_from_slice(&row.to_le_bytes());
        rgce.extend_from_slice(&col.to_le_bytes());
        return Ok(rgce);
    }

    if formula
        .split_once(':')
        .is_some_and(|(first, last)| absolute_a1(first).is_some() && absolute_a1(last).is_some())
        && scope.is_none()
    {
        return Err(Error::Parameter(
            "global defined-name references must include a sheet name".to_owned(),
        ));
    }

    let default_sheet = scope.map(|scope| sheet_index(sheets, scope)).transpose()?;
    let compiled =
        crate::formula::compile_name_formula(formula, sheets, xti, default_sheet, names)?;
    debug_assert_eq!(compiled.cb, 0);
    Ok(compiled.rgce)
}

fn encode_reference(reference: &str, ixti: u16) -> Result<Vec<u8>> {
    if let Some((first, last)) = reference.split_once(':') {
        let (first_row, first_col) = absolute_a1(first).ok_or_else(|| unsupported(reference))?;
        let (last_row, last_col) = absolute_a1(last).ok_or_else(|| unsupported(reference))?;
        if first_row > last_row || first_col > last_col {
            return Err(Error::RowColumnOrder);
        }
        return Ok(area3d(ixti, first_row, first_col, last_row, last_col).to_vec());
    }

    let (row, col) = absolute_a1(reference).ok_or_else(|| unsupported(reference))?;
    let mut rgce = Vec::with_capacity(9);
    rgce.push(0x3A);
    rgce.extend_from_slice(&ixti.to_le_bytes());
    rgce.extend_from_slice(&row.to_le_bytes());
    rgce.extend_from_slice(&col.to_le_bytes());
    Ok(rgce)
}

fn encode_string(formula: &str) -> Result<Vec<u8>> {
    let mut rgce = Vec::with_capacity(3 + formula.len() * 2);
    rgce.push(0x17);
    rgce.extend_from_slice(&[0, 0]);
    let mut count = 0_u16;
    let mut chars = formula[1..].chars().peekable();
    let mut closed = false;
    let mut units = [0_u16; 2];

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if chars.peek() == Some(&'"') {
                chars.next();
            } else if chars.peek().is_none() {
                closed = true;
                break;
            } else {
                return Err(unsupported(formula));
            }
        }
        for unit in ch.encode_utf16(&mut units) {
            count = count.checked_add(1).ok_or_else(|| unsupported(formula))?;
            rgce.extend_from_slice(&unit.to_le_bytes());
        }
        if count > 255 {
            return Err(Error::Parameter(
                "defined-name string literal exceeds 255 characters".to_owned(),
            ));
        }
    }

    if !closed {
        return Err(unsupported(formula));
    }
    rgce[1..3].copy_from_slice(&count.to_le_bytes());
    Ok(rgce)
}

fn error_token(formula: &str) -> Option<u8> {
    const ERRORS: [(&str, u8); 8] = [
        ("#NULL!", 0x00),
        ("#DIV/0!", 0x07),
        ("#VALUE!", 0x0F),
        ("#REF!", 0x17),
        ("#NAME?", 0x1D),
        ("#NUM!", 0x24),
        ("#N/A", 0x2A),
        ("#GETTING_DATA", 0x2B),
    ];
    ERRORS
        .iter()
        .find(|(name, _)| formula.eq_ignore_ascii_case(name))
        .map(|(_, code)| *code)
}

pub fn area3d(ixti: u16, first_row: u32, first_col: u16, last_row: u32, last_col: u16) -> [u8; 15] {
    let mut rgce = [0u8; 15];
    rgce[0] = 0x3B;
    rgce[1..3].copy_from_slice(&ixti.to_le_bytes());
    rgce[3..7].copy_from_slice(&first_row.to_le_bytes());
    rgce[7..11].copy_from_slice(&last_row.to_le_bytes());
    rgce[11..13].copy_from_slice(&first_col.to_le_bytes());
    rgce[13..15].copy_from_slice(&last_col.to_le_bytes());
    rgce
}

fn split_local_name(name: &str) -> Result<Option<(Cow<'_, str>, &str)>> {
    if name.starts_with('\'') {
        let (sheet, rest) =
            quoted_sheet(name).ok_or_else(|| name_error(name, "unterminated quoted sheet name"))?;
        let local_name = rest
            .strip_prefix('!')
            .ok_or_else(|| name_error(name, "missing '!' after a quoted sheet name"))?;
        if local_name.is_empty() {
            return Err(name_error(name, "local name is blank"));
        }
        return Ok(Some((sheet, local_name)));
    }

    match name.split_once('!') {
        Some((_, "")) => Err(name_error(name, "local name is blank")),
        Some(("", _)) => Err(name_error(name, "sheet scope is blank")),
        Some((sheet, local_name)) => Ok(Some((Cow::Borrowed(sheet), local_name))),
        None => Ok(None),
    }
}

fn split_sheet_prefix(formula: &str) -> Result<Option<(Cow<'_, str>, &str)>> {
    if formula.starts_with('\'') {
        let (sheet, rest) = quoted_sheet(formula).ok_or_else(|| unsupported(formula))?;
        let Some(reference) = rest.strip_prefix('!') else {
            return Ok(None);
        };
        if sheet.is_empty() {
            return Err(unsupported(formula));
        }
        return Ok(Some((sheet, reference)));
    }

    match formula.split_once('!') {
        Some((_, "") | ("", _)) => Err(unsupported(formula)),
        Some((sheet, reference)) => Ok(Some((Cow::Borrowed(sheet), reference))),
        None => Ok(None),
    }
}

fn quoted_sheet(input: &str) -> Option<(Cow<'_, str>, &str)> {
    let bytes = input.as_bytes();
    let mut index = 1;
    while index < bytes.len() {
        if bytes[index] != b'\'' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'\'') {
            index += 2;
            continue;
        }
        let sheet = &input[1..index];
        let sheet = if sheet.contains("''") {
            Cow::Owned(sheet.replace("''", "'"))
        } else {
            Cow::Borrowed(sheet)
        };
        return Some((sheet, &input[index + 1..]));
    }
    None
}

pub fn sheet_index(sheets: &[Worksheet], name: &str) -> Result<usize> {
    let folded = name.to_lowercase();
    sheets
        .iter()
        .position(|sheet| sheet.name_ref().to_lowercase() == folded)
        .ok_or_else(|| Error::UnknownWorksheetNameOrIndex(name.to_owned()))
}

fn is_simple_sheet_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains([
            '(', ')', '+', '-', '*', '/', '^', '&', '%', '=', '<', '>', ',',
        ])
}

fn absolute_a1(reference: &str) -> Option<(u32, u16)> {
    let bytes = reference.as_bytes();
    let mut index = 0;
    if bytes.get(index) != Some(&b'$') || bytes.len() < 4 {
        return None;
    }
    index += 1;
    let col_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        index += 1;
    }
    if !(1..=3).contains(&(index - col_start)) || bytes.get(index) != Some(&b'$') {
        return None;
    }
    let col_end = index;
    index += 1;
    let row_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if index != bytes.len() || row_start == index {
        return None;
    }

    let mut column = 0_u32;
    for byte in &bytes[col_start..col_end] {
        column = column * 26 + u32::from(byte.to_ascii_uppercase() - b'A' + 1);
    }
    let column = column.checked_sub(1)?;
    let row = reference[row_start..].parse::<u32>().ok()?.checked_sub(1)?;
    if column > u32::from(MAX_COL) || row > MAX_ROW {
        return None;
    }
    Some((row, u16::try_from(column).ok()?))
}

const INVALID_NAME_CHARS: [char; 29] = [
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '/', ':', ';', '<', '=',
    '>', '@', '[', ']', '^', '`', '{', '|', '}', '~',
];

pub fn is_name_char(ch: char) -> bool {
    !INVALID_NAME_CHARS.contains(&ch) && !ch.is_control()
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the table name validator"
)]
pub(crate) fn check_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(name_error(name, "name cannot be blank"));
    }
    if name.encode_utf16().count() > 255 {
        return Err(name_error(name, "name exceeds Excel's 255-character limit"));
    }

    if let Some(ch) = name.chars().find(|ch| !is_name_char(*ch)) {
        return Err(name_error(
            name,
            &format!("name contains a character that Excel disallows: {ch:?}"),
        ));
    }

    let first = name.chars().next().expect("name is not empty");
    if first.is_ascii_digit() || matches!(first, '.' | '?') {
        return Err(name_error(
            name,
            "name starts with a character Excel disallows",
        ));
    }
    if name.chars().count() == 2
        && name.starts_with('\\')
        && name.ends_with(|ch: char| ch.is_ascii_alphanumeric())
    {
        return Err(name_error(
            name,
            "name cannot be a backslash followed by one character",
        ));
    }
    if name.eq_ignore_ascii_case("TRUE") || name.eq_ignore_ascii_case("FALSE") {
        return Err(name_error(name, "name cannot be TRUE or FALSE"));
    }
    if is_cell_reference_name(name) {
        return Err(name_error(
            name,
            "name cannot look like an Excel cell reference",
        ));
    }
    if name.eq_ignore_ascii_case("_xlnm.Print_Area")
        || name.eq_ignore_ascii_case("_xlnm._FilterDatabase")
        || name.eq_ignore_ascii_case("_xlnm.Print_Titles")
    {
        return Err(name_error(name, "name is reserved by Excel"));
    }

    Ok(())
}

fn is_cell_reference_name(name: &str) -> bool {
    if absolute_or_relative_a1(name) {
        return true;
    }

    let uppercase = name.to_ascii_uppercase();
    let Some(position) = uppercase.find(|ch: char| ch.is_ascii_digit()) else {
        return matches!(uppercase.as_str(), "R" | "C" | "RC");
    };
    let (letters, number) = uppercase.split_at(position);
    let digits = number
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0_u32, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(u32::from(digit - b'0'))
        });

    match letters {
        "R" => (1..=MAX_ROW + 1).contains(&digits),
        "C" | "RC" => (1..=u32::from(MAX_COL) + 1).contains(&digits),
        _ => false,
    }
}

fn absolute_or_relative_a1(reference: &str) -> bool {
    let bytes = reference.as_bytes();
    let mut index = 0;
    if bytes.get(index) == Some(&b'$') {
        index += 1;
    }
    let col_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        index += 1;
    }
    if !(1..=3).contains(&(index - col_start)) {
        return false;
    }
    let col_end = index;
    if bytes.get(index) == Some(&b'$') {
        index += 1;
    }
    let row_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if index != bytes.len() || row_start == index {
        return false;
    }

    let mut column = 0_u32;
    for byte in &bytes[col_start..col_end] {
        column = column * 26 + u32::from(byte.to_ascii_uppercase() - b'A' + 1);
    }
    let Some(column) = column.checked_sub(1) else {
        return false;
    };
    let Ok(row) = reference[row_start..].parse::<u32>() else {
        return false;
    };
    let Some(row) = row.checked_sub(1) else {
        return false;
    };
    column <= u32::from(MAX_COL) && row <= MAX_ROW
}

fn name_error(name: &str, reason: &str) -> Error {
    Error::Parameter(format!("invalid defined name {name:?}: {reason}"))
}

fn unsupported(formula: &str) -> Error {
    Error::Parameter(format!(
        "unsupported defined-name formula {formula:?}: use a constant, an absolute sheet cell/range, or a supported expression"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoder() -> (Vec<Worksheet>, XtiCollector) {
        let mut first = Worksheet::new_named("Data Sheet".to_owned());
        first.set_name("Sheet1").expect("valid sheet name");
        let second = Worksheet::new_named("Data Sheet".to_owned());
        (vec![first, second], XtiCollector::new(2))
    }

    #[test]
    fn constants_encode_as_value_tokens() {
        let (sheets, mut xti) = encoder();
        let mut expected = vec![0x1F];
        expected.extend_from_slice(&0.96_f64.to_le_bytes());
        assert_eq!(
            encode_formula("=0.96", None, &sheets, &mut xti).expect("number"),
            expected
        );
        assert_eq!(
            encode_formula("\"Yes\"", None, &sheets, &mut xti).expect("string"),
            [0x17, 3, 0, b'Y', 0, b'e', 0, b's', 0]
        );
        assert_eq!(
            encode_formula("TRUE", None, &sheets, &mut xti).expect("bool"),
            [0x1D, 1]
        );
        assert_eq!(
            encode_formula("#N/A", None, &sheets, &mut xti).expect("error"),
            [0x1C, 0x2A]
        );
        assert!(xti.is_empty());
    }

    #[test]
    fn local_references_use_the_name_scope() {
        let (sheets, mut xti) = encoder();
        let rgce = encode_formula("=$A$1", Some("Data Sheet"), &sheets, &mut xti)
            .expect("local reference");
        assert_eq!(rgce.len(), 9);
        assert_eq!(rgce[0], 0x3A);
        assert_eq!(&rgce[1..3], &[0, 0]);
        assert_eq!(xti.ranges(), &[(1, 1)]);
    }

    #[test]
    fn qualified_ranges_allocate_an_xti_per_sheet() {
        let (sheets, mut xti) = encoder();
        let rgce = encode_formula("='Data Sheet'!$A$1:$B$2", None, &sheets, &mut xti)
            .expect("quoted range");
        assert_eq!(rgce, area3d(0, 0, 0, 1, 1));
        assert_eq!(xti.ranges(), &[(1, 1)]);
    }

    #[test]
    fn repeated_ranges_reuse_the_same_xti() {
        let mut xti = XtiCollector::new(3);
        assert_eq!(xti.range(0, 2).expect("range"), 0);
        assert_eq!(xti.range(0, 2).expect("repeated range"), 0);
        assert_eq!(xti.index(1).expect("single sheet"), 1);
        assert_eq!(xti.ranges(), &[(0, 2), (1, 1)]);
    }

    #[test]
    fn worksheet_ranges_span_one_xti() {
        let sheets = vec![
            Worksheet::new_named("Sheet1".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
            Worksheet::new_named("Sheet3".to_owned()),
        ];
        let mut xti = XtiCollector::new(3);
        let mut expected = area3d(0, 0, 0, 2, 0).to_vec();
        expected.extend_from_slice(&[0x42, 1, 0x04, 0]);
        assert_eq!(
            encode_formula("=SUM(Sheet1:Sheet3!$A$1:$A$3)", None, &sheets, &mut xti)
                .expect("worksheet range"),
            expected
        );
        assert_eq!(xti.ranges(), &[(0, 2)]);
    }

    #[test]
    fn bare_worksheet_ranges_encode_a_reference() {
        let sheets = vec![
            Worksheet::new_named("Sheet1".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
            Worksheet::new_named("Sheet3".to_owned()),
        ];
        let mut xti = XtiCollector::new(3);
        assert_eq!(
            encode_formula("=Sheet1:Sheet3!$A$1", None, &sheets, &mut xti)
                .expect("bare worksheet range"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(xti.ranges(), &[(0, 2)]);
    }

    #[test]
    fn quoted_worksheet_ranges_span_one_xti() {
        let sheets = vec![
            Worksheet::new_named("Data Sheet".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
            Worksheet::new_named("Sheet3".to_owned()),
        ];
        let mut xti = XtiCollector::new(3);
        assert_eq!(
            encode_formula("=SUM('Data Sheet':'Sheet3'!$A$1)", None, &sheets, &mut xti)
                .expect("quoted worksheet range"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x42, 1, 0x04, 0]
        );
        assert_eq!(xti.ranges(), &[(0, 2)]);
    }

    #[test]
    fn escaped_quotes_resolve_in_worksheet_ranges() {
        let sheets = vec![
            Worksheet::new_named("a'b".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
        ];
        let mut xti = XtiCollector::new(2);
        assert_eq!(
            encode_formula("=SUM('a''b':'Sheet2'!$A$1)", None, &sheets, &mut xti)
                .expect("escaped range"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x42, 1, 0x04, 0]
        );
        assert_eq!(xti.ranges(), &[(0, 1)]);
    }

    #[test]
    fn reversed_worksheet_ranges_are_rejected() {
        let sheets = vec![
            Worksheet::new_named("Sheet1".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
            Worksheet::new_named("Sheet3".to_owned()),
        ];
        let mut xti = XtiCollector::new(3);
        let error = encode_formula("=SUM(Sheet3:Sheet1!$A$1)", None, &sheets, &mut xti)
            .expect_err("reversed range");
        assert!(error.to_string().contains("workbook order"));
    }

    #[test]
    fn global_references_require_a_sheet() {
        let (sheets, mut xti) = encoder();
        let error = encode_formula("=$A$1", None, &sheets, &mut xti).expect_err("rejected");
        assert!(error.to_string().contains("must include a sheet name"));
    }

    #[test]
    fn formula_expressions_encode_three_dimensional_references() {
        let (sheets, mut xti) = encoder();
        let mut expected = area3d(0, 0, 0, 2, 0).to_vec();
        expected.extend_from_slice(&[0x42, 1, 0x04, 0x00]);
        assert_eq!(
            encode_formula("=SUM(Sheet1!$A$1:$A$3)", None, &sheets, &mut xti)
                .expect("function over a range"),
            expected
        );

        assert_eq!(
            encode_formula("=Sheet1!$A$1*2", None, &sheets, &mut xti).expect("cell expression"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x1E, 2, 0, 0x05]
        );

        assert_eq!(
            encode_formula("='Data Sheet'!$A$1+1", None, &sheets, &mut xti)
                .expect("quoted sheet expression"),
            [0x3A, 1, 0, 0, 0, 0, 0, 0, 0, 0x1E, 1, 0, 0x03]
        );
        assert_eq!(xti.ranges(), &[(0, 0), (1, 1)]);
    }

    #[test]
    fn local_expression_references_use_the_name_scope() {
        let (sheets, mut xti) = encoder();
        assert_eq!(
            encode_formula("=$A$1+1", Some("Data Sheet"), &sheets, &mut xti)
                .expect("local expression"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x1E, 1, 0, 0x03]
        );
        assert_eq!(xti.ranges(), &[(1, 1)]);
        assert!(encode_formula("=A1+1", Some("Data Sheet"), &sheets, &mut xti).is_err());
    }

    #[test]
    fn string_leading_expressions_reach_the_parser() {
        let (sheets, mut xti) = encoder();
        let mut expected = vec![0x17, 7, 0];
        for unit in "Total: ".encode_utf16() {
            expected.extend_from_slice(&unit.to_le_bytes());
        }
        expected.extend_from_slice(&[0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x08]);
        assert_eq!(
            encode_formula("=\"Total: \"&Sheet1!$A$1", None, &sheets, &mut xti)
                .expect("string and reference"),
            expected
        );
        assert_eq!(
            encode_formula("=\"a\"=\"b\"", None, &sheets, &mut xti).expect("string comparison"),
            [0x17, 1, 0, b'a', 0, 0x17, 1, 0, b'b', 0, 0x0B]
        );
    }

    #[test]
    fn reversed_three_dimensional_ranges_are_rejected() {
        let (sheets, mut xti) = encoder();
        assert!(matches!(
            encode_formula("=$B$2:$A$1", Some("Data Sheet"), &sheets, &mut xti),
            Err(Error::RowColumnOrder)
        ));
        let quoted = vec![Worksheet::new_named("Q1-2024".to_owned())];
        assert!(matches!(
            encode_formula(
                "='Q1-2024'!$B$2:$A$1",
                None,
                &quoted,
                &mut XtiCollector::new(1)
            ),
            Err(Error::RowColumnOrder)
        ));
    }

    #[test]
    fn global_expressions_require_a_sheet_and_external_workbooks_are_rejected() {
        let (sheets, mut xti) = encoder();
        assert!(encode_formula("=$A$1+1", None, &sheets, &mut xti).is_err());
        assert!(encode_formula("=[Book]Sheet1!$A$1", None, &sheets, &mut xti).is_err());
    }

    #[test]
    fn function_names_win_over_parenthesized_sheet_names() {
        let sheets = vec![
            Worksheet::new_named("SUM(Sheet1".to_owned()),
            Worksheet::new_named("Sheet1".to_owned()),
        ];
        let mut xti = XtiCollector::new(2);
        assert_eq!(
            encode_formula("=SUM(Sheet1!$A$1)", None, &sheets, &mut xti).expect("function"),
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x42, 1, 0x04, 0]
        );
        assert_eq!(xti.ranges(), &[(1, 1)]);
    }

    #[test]
    fn name_references_encode_the_target_name_index() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("Base", "=Sheet1!$A$1").expect("base"),
            DefinedName::new("Double", "=Base*2").expect("double"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let mut xti = XtiCollector::new(1);
        assert_eq!(
            definitions[1]
                .encode(&sheets, &mut xti, &names)
                .expect("name reference"),
            [0x23, 1, 0, 0, 0, 0x1E, 2, 0, 0x05]
        );
    }

    #[test]
    fn forward_name_references_use_the_complete_table() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("Aaa", "=Zzz*2").expect("forward"),
            DefinedName::new("Zzz", "=1").expect("target"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let mut xti = XtiCollector::new(1);
        assert_eq!(
            definitions[0]
                .encode(&sheets, &mut xti, &names)
                .expect("forward reference"),
            [0x23, 2, 0, 0, 0, 0x1E, 2, 0, 0x05]
        );
    }

    #[test]
    fn local_names_shadow_global_names() {
        let sheets = vec![
            Worksheet::new_named("Sheet1".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
        ];
        let definitions = [
            DefinedName::new("Rate", "=0.1").expect("global rate"),
            DefinedName::new("Sheet1!Rate", "=0.2").expect("local rate"),
            DefinedName::new("UseGlobal", "=Rate").expect("global use"),
            DefinedName::new("UseLocal", "=Sheet1!Rate").expect("qualified use"),
            DefinedName::new("Sheet1!UseLocal", "=Rate").expect("local use"),
            DefinedName::new("Sheet2!OnlyLocal", "=0.3").expect("other local"),
            DefinedName::new("BadLocal", "=OnlyLocal").expect("bad global use"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let global = names.lookup("Rate", None).expect("global rate");
        let local = names.lookup("Rate", Some(0)).expect("local rate");
        assert_ne!(global, local);
        assert_eq!(
            names.lookup("Rate", Some(1)).expect("global fallback"),
            global
        );

        let mut xti = XtiCollector::new(2);
        for (index, expected) in [(2, global), (3, local), (4, local)] {
            let mut token = vec![0x23];
            token.extend_from_slice(&expected.to_le_bytes());
            assert_eq!(
                definitions[index]
                    .encode(&sheets, &mut xti, &names)
                    .expect("name reference"),
                token
            );
        }
        assert!(definitions[6].encode(&sheets, &mut xti, &names).is_err());
    }

    #[test]
    fn unicode_names_resolve_in_expressions() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("Résumé", "=Sheet1!$A$1").expect("unicode name"),
            DefinedName::new("Use", "=RÉSUMÉ*2").expect("unicode reference"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let mut xti = XtiCollector::new(1);
        assert_eq!(
            definitions[1]
                .encode(&sheets, &mut xti, &names)
                .expect("unicode reference"),
            [0x23, 1, 0, 0, 0, 0x1E, 2, 0, 0x05]
        );
    }

    #[test]
    fn underscore_and_backslash_names_resolve() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("_Rate", "=1").expect("underscore name"),
            DefinedName::new("\\Rate", "=2").expect("backslash name"),
            DefinedName::new("Use", "=_Rate+\\Rate").expect("references"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let mut xti = XtiCollector::new(1);
        let underscore = names.lookup("_Rate", None).expect("underscore index");
        let backslash = names.lookup("\\Rate", None).expect("backslash index");
        let mut expected = vec![0x23];
        expected.extend_from_slice(&underscore.to_le_bytes());
        expected.extend_from_slice(&[0x23]);
        expected.extend_from_slice(&backslash.to_le_bytes());
        expected.push(0x03);
        assert_eq!(
            definitions[2]
                .encode(&sheets, &mut xti, &names)
                .expect("name references"),
            expected
        );
    }

    #[test]
    fn cell_lookalike_names_win_over_cell_prefixes() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("Q1Sales", "=1").expect("lookalike"),
            DefinedName::new("Use", "=Q1Sales*2").expect("plain reference"),
            DefinedName::new("Qualified", "=Sheet1!Q1Sales*2").expect("qualified"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let index = names.lookup("Q1Sales", None).expect("lookalike index");
        let mut expected = vec![0x23];
        expected.extend_from_slice(&index.to_le_bytes());
        expected.extend_from_slice(&[0x1E, 2, 0, 0x05]);
        let mut xti = XtiCollector::new(1);
        for definition in &definitions[1..] {
            assert_eq!(
                definition
                    .encode(&sheets, &mut xti, &names)
                    .expect("lookalike reference"),
                expected
            );
        }
    }

    #[test]
    fn builtin_names_shift_user_name_indexes() {
        let sheets = vec![Worksheet::new_named("Sheet1".to_owned())];
        let definitions = [
            DefinedName::new("Base", "=Sheet1!$A$1").expect("base"),
            DefinedName::new("Double", "=Base*2").expect("double"),
        ];
        let names = NameTable::new(&sheets, &definitions, 1).expect("name table");
        let mut xti = XtiCollector::new(1);
        assert_eq!(
            definitions[1]
                .encode(&sheets, &mut xti, &names)
                .expect("name reference"),
            [0x23, 2, 0, 0, 0, 0x1E, 2, 0, 0x05]
        );
    }

    #[test]
    fn qualified_names_without_a_table_are_errors() {
        let (sheets, mut xti) = encoder();
        assert!(encode_formula("='Data Sheet'!Rate", None, &sheets, &mut xti).is_err());
    }

    #[test]
    fn sheet_ranges_cannot_qualify_defined_names() {
        let sheets = vec![
            Worksheet::new_named("Sheet1".to_owned()),
            Worksheet::new_named("Sheet2".to_owned()),
        ];
        let definitions = [
            DefinedName::new("Target", "=1").expect("target"),
            DefinedName::new("Bad", "=SUM(Sheet1:Sheet2!Target)").expect("bad"),
        ];
        let names = NameTable::new(&sheets, &definitions, 0).expect("name table");
        let mut xti = XtiCollector::new(2);
        let error = definitions[1]
            .encode(&sheets, &mut xti, &names)
            .expect_err("sheet range name");
        assert!(error.to_string().contains("sheet ranges"));
    }

    #[test]
    fn name_validation_covers_excel_reserved_shapes() {
        assert!(check_name("").is_err());
        assert!(check_name("A1").is_err());
        assert!(check_name("R1C1").is_err());
        assert!(check_name("R1Data").is_err());
        assert!(check_name("RC1").is_err());
        assert!(check_name("R").is_err());
        assert!(check_name("TRUE").is_err());
        assert!(check_name("Has Space").is_err());
        assert!(check_name("1Name").is_err());
        assert!(check_name("_xlnm.Print_Area").is_err());
        assert!(check_name("Sales").is_ok());
        assert!(check_name("_Rate").is_ok());
    }

    #[test]
    fn local_name_split_unquotes_the_sheet() {
        let name = DefinedName::new("'Data Sheet'!Rate", "=0.5").expect("name");
        assert_eq!(name.scope.as_deref(), Some("Data Sheet"));
        assert_eq!(name.name, "Rate");
        assert_eq!(name.formula, "0.5");
    }

    #[test]
    fn escaped_quotes_resolve_to_one_sheet_name() {
        let sheets = vec![
            Worksheet::new_named("a'b".to_owned()),
            Worksheet::new_named("a''b".to_owned()),
        ];
        let mut xti = XtiCollector::new(2);
        let rgce = encode_formula("='a''b'!$A$1", None, &sheets, &mut xti).expect("quoted");
        assert_eq!(rgce[0], 0x3A);
        assert_eq!(&rgce[1..3], &[0, 0]);
        assert_eq!(xti.ranges(), &[(0, 0)]);

        let name = DefinedName::new("'a''b'!Rate", "=1").expect("name");
        assert_eq!(name.scope.as_deref(), Some("a'b"));
    }
}
