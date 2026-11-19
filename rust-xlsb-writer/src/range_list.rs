//! Parsing and storage for one or more same-sheet A1 ranges.

use rust_xlsb_core::prelude::UncheckedRfX;

use crate::worksheet::{MAX_COL, MAX_ROW};
use crate::{Error, Result};

/// The effective ranges for a validation or conditional-format rule.
#[derive(Clone, Debug)]
pub enum RangeList {
    Single(UncheckedRfX),
    Multiple(Box<[UncheckedRfX]>),
}

impl RangeList {
    pub(crate) fn from_primary(
        primary: UncheckedRfX,
        multi_range: Option<&str>,
        max: usize,
    ) -> Result<Self> {
        match multi_range {
            Some(text) if !text.trim().is_empty() => Self::parse(text, max),
            _ => Ok(Self::Single(primary)),
        }
    }

    pub(crate) fn parse(input: &str, max: usize) -> Result<Self> {
        let mut first = None;
        let mut rest = Vec::new();
        let mut count = 0_usize;

        for token in input.split(|ch: char| ch.is_whitespace() || ch == ',') {
            if token.is_empty() {
                continue;
            }
            count += 1;
            if count > max {
                return Err(Error::Parameter(format!(
                    "multi-range contains more than {max} areas"
                )));
            }
            let range = parse_area(token).map_err(|reason| {
                Error::Parameter(format!("invalid multi-range '{input}': {reason}"))
            })?;
            if first.is_none() {
                first = Some(range);
            } else {
                rest.push(range);
            }
        }

        let Some(first) = first else {
            return Err(Error::Parameter(
                "multi-range must contain at least one area".to_owned(),
            ));
        };
        if rest.is_empty() {
            return Ok(Self::Single(first));
        }
        rest.insert(0, first);
        Ok(Self::Multiple(rest.into_boxed_slice()))
    }

    pub(crate) fn as_slice(&self) -> &[UncheckedRfX] {
        match self {
            Self::Single(range) => std::slice::from_ref(range),
            Self::Multiple(ranges) => ranges,
        }
    }

    pub(crate) fn bounding_first_cell(&self) -> (u32, u32) {
        let mut row = u32::MAX;
        let mut col = u32::MAX;
        for range in self.as_slice() {
            row = row.min(range.rwFirst);
            col = col.min(range.colFirst);
        }
        (row, col)
    }
}

fn parse_area(token: &str) -> std::result::Result<UncheckedRfX, &'static str> {
    let (first, last) = if let Some((first, last)) = token.split_once(':') {
        if last.contains(':') {
            return Err("an area must contain at most one ':'");
        }
        (parse_cell(first)?, parse_cell(last)?)
    } else {
        let cell = parse_cell(token)?;
        (cell, cell)
    };

    if first.0 > last.0 || first.1 > last.1 {
        return Err("the first cell must not follow the last cell");
    }
    Ok(UncheckedRfX {
        rwFirst: first.0,
        rwLast: last.0,
        colFirst: u32::from(first.1),
        colLast: u32::from(last.1),
    })
}

fn parse_cell(token: &str) -> std::result::Result<(u32, u16), &'static str> {
    let bytes = token.as_bytes();
    let mut index = 0;
    if bytes.first() == Some(&b'$') {
        index += 1;
    }

    let col_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        index += 1;
    }
    if !(1..=3).contains(&(index - col_start)) {
        return Err("a cell must start with one to three column letters");
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
        return Err("a cell must end with a row number");
    }

    let mut column = 0_u32;
    for byte in &bytes[col_start..col_end] {
        column = column * 26 + u32::from(byte.to_ascii_uppercase() - b'A' + 1);
    }
    let column = column.checked_sub(1).ok_or("column is out of range")?;
    let row = token[row_start..]
        .parse::<u32>()
        .map_err(|_| "row is out of range")?
        .checked_sub(1)
        .ok_or("row is out of range")?;

    if column > u32::from(MAX_COL) || row > MAX_ROW {
        return Err("cell is outside the worksheet limits");
    }
    Ok((
        row,
        u16::try_from(column).map_err(|_| "column is out of range")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(row_first: u32, row_last: u32, col_first: u16, col_last: u16) -> UncheckedRfX {
        UncheckedRfX {
            rwFirst: row_first,
            rwLast: row_last,
            colFirst: u32::from(col_first),
            colLast: u32::from(col_last),
        }
    }

    #[test]
    fn parses_cells_anchors_and_both_separators() {
        let ranges = RangeList::parse("$B$3:$D$6,$I$3:$K$6 B9:D12\tI9:K12", 8_192).unwrap();
        assert_eq!(
            ranges.as_slice(),
            &[
                area(2, 5, 1, 3),
                area(2, 5, 8, 10),
                area(8, 11, 1, 3),
                area(8, 11, 8, 10),
            ]
        );
    }

    #[test]
    fn single_cell_is_a_single_range_without_a_boxed_list() {
        let ranges = RangeList::parse("B3", 8_192).unwrap();
        assert!(matches!(ranges, RangeList::Single(_)));
        assert_eq!(ranges.as_slice(), &[area(2, 2, 1, 1)]);
    }

    #[test]
    fn formula_bases_use_the_last_area_and_the_bounding_top_left() {
        let ranges = RangeList::parse("$I$3:$K$6,$B$9:$D$12", 8_192).unwrap();
        assert_eq!(ranges.as_slice()[1], area(8, 11, 1, 3));
        assert_eq!(ranges.bounding_first_cell(), (2, 1));
    }

    #[test]
    fn from_primary_uses_the_fallback_for_empty_overrides() {
        let primary = area(1, 2, 3, 4);
        for override_text in [None, Some(""), Some("   ")] {
            let ranges = RangeList::from_primary(primary, override_text, 8_191).unwrap();
            assert_eq!(ranges.as_slice(), &[primary]);
        }
    }

    #[test]
    fn accepts_the_limit_and_rejects_one_more() {
        let at_limit = vec!["A1"; 8_192].join(" ");
        let parsed = RangeList::parse(&at_limit, 8_192).expect("limit");
        assert_eq!(parsed.as_slice().len(), 8_192);

        let too_many = vec!["A1"; 8_193].join(" ");
        assert!(RangeList::parse(&too_many, 8_192).is_err());
    }

    #[test]
    fn rejects_malformed_areas_and_limits() {
        for text in [
            "",
            " ",
            "A0",
            "A1:",
            ":A1",
            "A1:B2:C3",
            "B2:A1",
            "Sheet1!A1",
            "[Book]Sheet1!A1",
            "XFE1",
            "A1048577",
        ] {
            assert!(RangeList::parse(text, 8_192).is_err(), "{text}");
        }
        assert!(RangeList::parse("A1 B1", 1).is_err());
    }
}
