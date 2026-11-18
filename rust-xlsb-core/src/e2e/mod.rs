//! End-to-end coverage: records are written by `rust-xlsb-core` and read back
//! by an independent reader (`calamine`), so the package layout, the BIFF
//! framing, the number policy, and every cell record are exercised together.

mod harness;

use calamine::{CellErrorType, Data};

use crate::prelude::BErr;
use crate::{Error, KnownID, ToCellValue, WbContext, Workbook};

use harness::{CellSpec, DATE_STYLE, cell_record_ids, includes_shared_strings_part, roundtrip};

fn workbook() -> Workbook {
    Workbook::new().expect("in-memory workbook")
}

#[test]
fn reads_shared_string() {
    let workbook = workbook();
    let hello = workbook.shared_string("hello").expect("interns");
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &hello)]);
    assert_eq!(range.get((0, 0)), Some(&Data::String("hello".to_owned())));
}

#[test]
fn reads_inline_string() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &"hello")]);
    assert_eq!(range.get((0, 0)), Some(&Data::String("hello".to_owned())));
}

#[test]
fn reads_real_number() {
    let workbook = workbook();
    let value = std::f64::consts::PI;
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &value)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Float(value)));
}

#[test]
fn reads_fractional_number() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &3.75_f64)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Float(3.75)));
}

#[test]
fn reads_integer() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &-42_i32)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Int(-42)));
}

#[test]
fn reads_percentage() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &12.34_f64)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Float(12.34)));
}

#[test]
fn reads_boolean() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &true)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Bool(true)));
}

#[test]
fn reads_error() {
    let workbook = workbook();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &BErr::Div0)]);
    assert_eq!(range.get((0, 0)), Some(&Data::Error(CellErrorType::Div0)));
}

#[test]
fn blank_cell_keeps_columns_aligned() {
    let workbook = workbook();
    let range = roundtrip(
        &workbook,
        &[
            CellSpec::new(0, 0, &1.0_f64),
            CellSpec::new(0, 1, &()),
            CellSpec::new(0, 2, &"next"),
        ],
    );
    assert_eq!(range.get((0, 0)), Some(&Data::Int(1)));
    assert_eq!(range.get((0, 2)), Some(&Data::String("next".to_owned())));
}

#[test]
fn date_format_makes_a_datetime() {
    let workbook = workbook();
    let range = roundtrip(
        &workbook,
        &[CellSpec::new(0, 0, &45_000.0_f64).style(DATE_STYLE)],
    );
    match range.get((0, 0)) {
        Some(Data::DateTime(value)) => assert_eq!(value.as_f64(), 45_000.0),
        other => panic!("expected a datetime, got {other:?}"),
    }
}

#[test]
fn cells_land_on_their_own_positions() {
    let workbook = workbook();
    let range = roundtrip(
        &workbook,
        &[
            CellSpec::new(0, 0, &1.5_f64),
            CellSpec::new(0, 2, &true),
            CellSpec::new(2, 1, &"x"),
        ],
    );
    assert_eq!(range.get((0, 0)), Some(&Data::Float(1.5)));
    assert_eq!(range.get((0, 2)), Some(&Data::Bool(true)));
    assert_eq!(range.get((1, 0)), Some(&Data::Empty));
    assert_eq!(range.get((2, 1)), Some(&Data::String("x".to_owned())));
}

#[test]
fn compact_encodings_pick_the_expected_records() {
    let workbook = workbook();

    let integer = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &42_i32)]);
    assert!(integer.contains(&KnownID::BrtCellRk));

    let percentage = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &12.34_f64)]);
    assert!(percentage.contains(&KnownID::BrtCellRk));

    let pi = std::f64::consts::PI;
    let real = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &pi)]);
    assert!(real.contains(&KnownID::BrtCellReal));
}

#[test]
fn string_records_follow_the_conversion() {
    let workbook = workbook();

    let inline = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &"hello")]);
    assert!(inline.contains(&KnownID::BrtCellSt));
    assert!(!inline.contains(&KnownID::BrtCellIsst));

    let hello = workbook.shared_string("hello").expect("interns");
    let shared = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &hello)]);
    assert!(shared.contains(&KnownID::BrtCellIsst));
}

#[test]
fn shared_strings_part_follows_the_table() {
    let workbook = workbook();

    assert!(!includes_shared_strings_part(
        &workbook,
        &[CellSpec::new(0, 0, &"inline only")]
    ));

    let hello = workbook.shared_string("hello").expect("interns");
    assert!(includes_shared_strings_part(
        &workbook,
        &[CellSpec::new(0, 0, &hello)]
    ));
}

#[test]
fn foreign_shared_string_fails_before_writing() {
    let source = workbook();
    let target = workbook();
    let hello = source.shared_string("hello").expect("interns");

    assert!(matches!(
        hello.to_cell_value(&target),
        Err(Error::SharedStringOutOfRange { id: 0, len: 0 })
    ));
}

#[test]
fn inexact_integer_fails_before_writing() {
    let workbook = workbook();

    assert!(matches!(
        u64::MAX.to_cell_value(&workbook),
        Err(Error::ValueNotRepresentable(_))
    ));
    assert!(matches!(
        i64::MAX.to_cell_value(&workbook),
        Err(Error::ValueNotRepresentable(_))
    ));
}
