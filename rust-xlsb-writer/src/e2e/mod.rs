//! Package-level tests through the public writer API and `calamine`.

use std::io::BufReader;

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use chrono::{NaiveDate, NaiveTime, TimeDelta};
use deku::DekuContainerRead;
use rust_xlsb_core::prelude::{BErr, BrtBeginCellXFs, BrtBeginFonts, BrtFont};
use rust_xlsb_core::{BiffHead, CheckedBiff};

use crate::{Format, Workbook};

fn records<T>(part: &[u8]) -> Vec<T>
where
    T: for<'a> DekuContainerRead<'a> + Default + CheckedBiff,
{
    let target = T::default().id();
    let mut reader = BufReader::new(part);
    let mut records = Vec::new();
    while let Ok((head, body)) = BiffHead::read_from_with_data(&mut reader) {
        if head.known_id() == Some(target)
            && let Some(body) = body
        {
            let (_, record) = T::from_bytes((&body, 0)).expect("record decodes");
            records.push(record);
        }
    }
    records
}

fn range(workbook: &mut Workbook) -> calamine::Range<Data> {
    let bytes = workbook.save_to_buffer().expect("saves");
    let mut opened = open_workbook_auto_from_rs(std::io::Cursor::new(bytes)).expect("opens");
    opened.worksheet_range("Sheet1").expect("reads")
}

#[test]
fn values_roundtrip_through_the_public_api() {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.write(0, 0, "hello").expect("string");
    sheet.write(1, 0, 42_i32).expect("integer");
    sheet.write(2, 0, 12.5_f64).expect("number");
    sheet.write(3, 0, true).expect("bool");
    sheet.write(4, 0, BErr::Div0).expect("error");
    sheet.write(5, 0, ()).expect("blank");
    sheet.write(5, 1, "end").expect("end");

    let range = range(&mut workbook);

    assert_eq!(range.get((0, 0)), Some(&Data::String("hello".to_owned())));
    assert_eq!(range.get((1, 0)), Some(&Data::Int(42)));
    assert_eq!(range.get((2, 0)), Some(&Data::Float(12.5)));
    assert_eq!(range.get((3, 0)), Some(&Data::Bool(true)));
    assert!(matches!(range.get((4, 0)), Some(Data::Error(_))));
    assert_eq!(range.get((5, 0)), Some(&Data::Empty));
}

#[test]
fn dates_use_number_formats_and_roundtrip() {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let date_format = Format::new().set_num_format("yyyy-mm-dd");
    let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("date");
    let datetime = date.and_hms_milli_opt(12, 34, 56, 789).expect("datetime");
    let time = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).expect("time");
    let duration = TimeDelta::milliseconds(-1_500);

    sheet
        .write_date_with_format(0, 0, date, &date_format)
        .expect("date");
    sheet
        .write_datetime_with_format(1, 0, datetime, &date_format)
        .expect("datetime");
    sheet.write_time(2, 0, time).expect("time");
    sheet.write(3, 0, duration).expect("duration");

    let range = range(&mut workbook);

    match range.get((0, 0)) {
        Some(Data::DateTime(value)) => {
            assert_eq!(value.as_datetime().expect("date").date(), date);
        }
        other => panic!("expected a date, got {other:?}"),
    }
    match range.get((1, 0)) {
        Some(Data::DateTime(value)) => {
            assert_eq!(value.as_datetime(), Some(datetime));
        }
        other => panic!("expected a datetime, got {other:?}"),
    }
    assert_eq!(
        range.get((2, 0)),
        Some(&Data::Float(45_296_789.0 / 86_400_000.0))
    );
    assert_eq!(
        range.get((3, 0)),
        Some(&Data::Float(-1_500.0 / 86_400_000.0))
    );
}

#[test]
fn multiple_sheets_keep_their_values() {
    let mut workbook = Workbook::new();
    workbook
        .add_worksheet()
        .write(0, 0, "first")
        .expect("write");
    workbook
        .add_worksheet()
        .set_name("Data")
        .expect("name")
        .write(0, 0, "second")
        .expect("write");

    let bytes = workbook.save_to_buffer().expect("saves");
    let mut opened = open_workbook_auto_from_rs(std::io::Cursor::new(bytes)).expect("opens");

    assert_eq!(
        opened.worksheet_range("Sheet1").expect("reads").get((0, 0)),
        Some(&Data::String("first".to_owned()))
    );
    assert_eq!(
        opened.worksheet_range("Data").expect("reads").get((0, 0)),
        Some(&Data::String("second".to_owned()))
    );
}

#[test]
fn styles_deduplicate_and_default_font_is_calibri() {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let bold = Format::new().set_bold();
    sheet.write_with_format(0, 0, 1_i32, &bold).expect("first");
    sheet.write_with_format(1, 0, 2_i32, &bold).expect("second");
    sheet.write(2, 0, 3_i32).expect("default");

    let bytes = workbook.save_to_buffer().expect("saves");
    let styles = crate::package::part_bytes(&bytes, "xl/styles.bin").expect("styles part");
    let xfs = records::<BrtBeginCellXFs>(&styles);
    assert_eq!(xfs[0].count, 2);
    let fonts = records::<BrtBeginFonts>(&styles);
    assert_eq!(fonts[0].count, 2);
    let font_records = records::<BrtFont>(&styles);
    assert_eq!(
        font_records[0].name.to_optional_string().as_deref(),
        Some("Calibri")
    );
    assert_eq!(font_records[0].dyHeight, 220);
}
