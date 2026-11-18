//! End-to-end coverage: records are written by `rust-xlsb-core` and read back
//! by an independent reader (`calamine`), so the package layout, the BIFF
//! framing, the number policy, and every cell record are exercised together.

mod harness;

use calamine::{CellErrorType, Data};
use chrono::{NaiveDate, NaiveTime, TimeDelta};

use crate::prelude::{BErr, Bold, BrtColInfo, BrtFont, BrtRowHdr};
use crate::{CellStyle, Error, KnownID, ToCellValue, WbContext, Workbook};

use harness::{
    CellSpec, DATE_STYLE, DATETIME_STYLE, DURATION_STYLE, TIME_STYLE, cell_record_ids,
    includes_shared_strings_part, roundtrip,
};

fn workbook() -> Workbook {
    harness::temporal_workbook().expect("in-memory workbook")
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
fn reads_dates_around_the_1900_leap_bug() {
    let workbook = workbook();
    let dates = [
        NaiveDate::from_ymd_opt(1900, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(1900, 2, 28).unwrap(),
        NaiveDate::from_ymd_opt(1900, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 11, 18).unwrap(),
        NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
    ];
    let cells: Vec<CellSpec<'_>> = dates
        .iter()
        .enumerate()
        .map(|(row, date)| {
            let row = u32::try_from(row).expect("five dates fit u32");
            CellSpec::new(row, 0, date).style(DATE_STYLE)
        })
        .collect();
    let range = roundtrip(&workbook, &cells);
    for (row, date) in dates.iter().enumerate() {
        let row = u32::try_from(row).expect("five dates fit u32");
        match range.get_value((row, 0)) {
            Some(Data::DateTime(value)) => assert_eq!(
                value.as_datetime().expect("within the Excel range").date(),
                *date
            ),
            other => panic!("expected a datetime in row {row}, got {other:?}"),
        }
    }
}

#[test]
fn reads_datetime_milliseconds() {
    let workbook = workbook();
    let value = NaiveDate::from_ymd_opt(2026, 11, 18)
        .unwrap()
        .and_hms_milli_opt(12, 34, 56, 789)
        .unwrap();
    let range = roundtrip(&workbook, &[CellSpec::new(0, 0, &value).style(DATE_STYLE)]);
    match range.get_value((0, 0)) {
        Some(Data::DateTime(datetime)) => assert_eq!(datetime.as_datetime(), Some(value)),
        other => panic!("expected a datetime, got {other:?}"),
    }
}

#[test]
fn reads_time_and_duration_fractions() {
    let workbook = workbook();
    let time = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).unwrap();
    let duration = TimeDelta::milliseconds(-1500);
    let range = roundtrip(
        &workbook,
        &[CellSpec::new(0, 0, &time), CellSpec::new(1, 0, &duration)],
    );
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::Float(45_296_789.0 / 86_400_000.0))
    );
    assert_eq!(
        range.get_value((1, 0)),
        Some(&Data::Float(-1500.0 / 86_400_000.0))
    );
}

#[test]
fn temporal_styles_display_time_and_duration() {
    let workbook = workbook();
    let time = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).unwrap();
    let instant = NaiveDate::from_ymd_opt(2026, 11, 18)
        .unwrap()
        .and_time(time);
    let duration = TimeDelta::milliseconds(3_723_500);
    let range = roundtrip(
        &workbook,
        &[
            CellSpec::new(0, 0, &time).style(TIME_STYLE),
            CellSpec::new(1, 0, &instant).style(DATETIME_STYLE),
            CellSpec::new(2, 0, &duration).style(DURATION_STYLE),
        ],
    );

    let expected: [f64; 3] = [
        45_296_789.0 / 86_400_000.0,
        46_344.0 + 45_296_789.0 / 86_400_000.0,
        3_723_500.0 / 86_400_000.0,
    ];
    for (row, expected) in expected.into_iter().enumerate() {
        let row = u32::try_from(row).expect("three styles fit u32");
        let actual = match range.get_value((row, 0)) {
            Some(Data::DateTime(value)) => value.as_f64(),
            Some(Data::Float(value)) => *value,
            other => panic!("expected a numeric temporal value in row {row}, got {other:?}"),
        };
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
}

#[test]
fn temporal_cells_pick_compact_or_real_records() {
    let workbook = workbook();
    let date = NaiveDate::from_ymd_opt(2026, 11, 18).unwrap();
    let date_ids = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &date)]);
    assert!(date_ids.contains(&KnownID::BrtCellRk));

    let instant = date.and_hms_milli_opt(12, 34, 56, 789).unwrap();
    let instant_ids = cell_record_ids(&workbook, &[CellSpec::new(0, 0, &instant)]);
    assert!(instant_ids.contains(&KnownID::BrtCellReal));
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

fn bold_style(workbook: &mut Workbook) -> u32 {
    workbook
        .add_style(CellStyle {
            font: BrtFont {
                bls: Bold::BLSBOLD,
                ..BrtFont::default()
            },
            ..CellStyle::default()
        })
        .expect("adds a bold style")
}

#[test]
fn row_and_column_styles_reach_their_records() {
    let mut workbook = workbook();
    let style = bold_style(&mut workbook);
    let mut sheet = workbook.worksheet();
    sheet.set_row_style(0, style).expect("valid row style");
    sheet
        .set_column_style(1, 2, style)
        .expect("valid column style");

    let cells = [CellSpec::new(0, 1, &1_i32)];
    let bytes = harness::worksheet_bytes(&sheet, &cells).expect("worksheet serializes");

    let columns: Vec<BrtColInfo> = harness::typed_records(&bytes);
    assert_eq!(columns.len(), 1);
    assert_eq!(columns[0].colFirst, 1);
    assert_eq!(columns[0].colLast, 2);
    assert_eq!(columns[0].ixfe, style);

    let rows: Vec<BrtRowHdr> = harness::typed_records(&bytes);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rw, 0);
    assert_eq!(rows[0].ixfe, style);
    assert!(rows[0].fGhostDirty);

    let range = harness::roundtrip_with_sheet(&workbook, &sheet, &cells);
    assert_eq!(range.get((0, 0)), Some(&Data::Int(1)));
}

#[test]
fn style_only_rows_emit_a_row_header() {
    let mut workbook = workbook();
    let style = bold_style(&mut workbook);
    let mut sheet = workbook.worksheet();
    sheet.set_row_style(7, style).expect("valid row style");

    let bytes = harness::worksheet_bytes(&sheet, &[]).expect("worksheet serializes");
    let rows: Vec<BrtRowHdr> = harness::typed_records(&bytes);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rw, 7);
    assert_eq!(rows[0].ixfe, style);
    assert!(rows[0].fGhostDirty);
}
