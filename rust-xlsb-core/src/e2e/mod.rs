//! End-to-end coverage: records are written by `rust-xlsb-core` and read back
//! by an independent reader (`calamine`), so the package layout, the BIFF
//! framing, and every cell record are exercised together.

mod harness;

use calamine::{CellErrorType, Data};

use crate::prelude::BErr;

use harness::{CellValue, roundtrip};

#[test]
fn reads_shared_string() {
    let range = roundtrip(&[(0, 0, CellValue::SharedString("hello"))]);
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::String("hello".to_owned()))
    );
}

#[test]
fn reads_inline_string() {
    let range = roundtrip(&[(0, 0, CellValue::InlineString("hello"))]);
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::String("hello".to_owned()))
    );
}

#[test]
fn reads_real_number() {
    let range = roundtrip(&[(0, 0, CellValue::Real(3.75))]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Float(3.75)));
}

#[test]
fn reads_rk_integer() {
    let range = roundtrip(&[(0, 0, CellValue::RkInt(-42))]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Int(-42)));
}

#[test]
fn reads_rk_float() {
    let range = roundtrip(&[(0, 0, CellValue::RkFloat(1.5))]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Float(1.5)));
}

#[test]
fn reads_rk_percentage() {
    let range = roundtrip(&[(0, 0, CellValue::RkPercent(1234))]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Float(12.34)));
}

#[test]
fn reads_boolean() {
    let range = roundtrip(&[(0, 0, CellValue::Bool(true))]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Bool(true)));
}

#[test]
fn reads_error() {
    let range = roundtrip(&[(0, 0, CellValue::Error(BErr::Div0))]);
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::Error(CellErrorType::Div0))
    );
}

#[test]
fn blank_cell_keeps_columns_aligned() {
    let range = roundtrip(&[
        (0, 0, CellValue::Real(1.0)),
        (0, 1, CellValue::Blank),
        (0, 2, CellValue::SharedString("next")),
    ]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Float(1.0)));
    assert_eq!(
        range.get_value((0, 2)),
        Some(&Data::String("next".to_owned()))
    );
}

#[test]
fn date_format_makes_a_datetime() {
    let range = roundtrip(&[(0, 0, CellValue::Date(45_000.0))]);
    match range.get_value((0, 0)) {
        Some(Data::DateTime(value)) => assert_eq!(value.as_f64(), 45_000.0),
        other => panic!("expected a datetime, got {other:?}"),
    }
}

#[test]
fn cells_land_on_their_own_positions() {
    let range = roundtrip(&[
        (0, 0, CellValue::Real(1.5)),
        (0, 2, CellValue::Bool(true)),
        (2, 1, CellValue::SharedString("x")),
    ]);
    assert_eq!(range.get_value((0, 0)), Some(&Data::Float(1.5)));
    assert_eq!(range.get_value((0, 2)), Some(&Data::Bool(true)));
    assert_eq!(range.get_value((1, 0)), Some(&Data::Empty));
    assert_eq!(range.get_value((2, 1)), Some(&Data::String("x".to_owned())));
}
