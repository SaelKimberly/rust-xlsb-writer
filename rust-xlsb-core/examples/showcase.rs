//! Writes an XLSB workbook with one row per `ToCellValue` implementation.
//!
//! Run with `cargo run -p rust-xlsb-core --example showcase -- showcase.xlsb`.

#[path = "../src/e2e/harness.rs"]
#[allow(dead_code)]
mod harness;

use chrono::{NaiveDate, NaiveTime, TimeDelta};
use harness::{CellSpec, DATE_STYLE, DATETIME_STYLE, DURATION_STYLE, TIME_STYLE};
use rust_xlsb_core::prelude::BErr;
use rust_xlsb_core::{SharedString, ToCellValue, WbContext, Workbook};

static REFERENCE: i32 = 42;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "showcase.xlsb".to_owned());
    if let Err(error) = write_showcase(&path) {
        eprintln!("failed to write {path}: {error}");
        std::process::exit(1);
    }
    println!("wrote {path}");
}

fn write_showcase(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workbook = Workbook::new()?;
    let shared: SharedString = workbook.shared_string("shared string")?;
    let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("valid date");
    let time = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).expect("valid time");
    let datetime = date.and_time(time);
    let utc = datetime.and_utc();
    let duration = TimeDelta::milliseconds(3_723_500);
    let negative_duration = TimeDelta::milliseconds(-1_500);

    let rows: Vec<(&str, Box<dyn ToCellValue>, u32)> = vec![
        ("what", Box::new("value"), 0),
        ("() blank", Box::new(()), 0),
        ("Option::None", Box::new(Option::<i32>::None), 0),
        ("Option::Some(7)", Box::new(Some(7_i32)), 0),
        ("bool false", Box::new(false), 0),
        ("BErr::Div0", Box::new(BErr::Div0), 0),
        ("&str inline", Box::new("inline text"), 0),
        ("String inline", Box::new(String::from("owned text")), 0),
        ("SharedString", Box::new(shared), 0),
        ("f32 1.5", Box::new(1.5_f32), 0),
        ("f64 Real (PI)", Box::new(std::f64::consts::PI), 0),
        ("f64 Rk int 42", Box::new(42.0_f64), 0),
        ("f64 Rk percent 12.34", Box::new(12.34_f64), 0),
        ("i8 -8", Box::new(-8_i8), 0),
        ("i16 -1600", Box::new(-1_600_i16), 0),
        ("i32 -32000", Box::new(-32_000_i32), 0),
        ("i64 -64000", Box::new(-64_000_i64), 0),
        ("u8 8", Box::new(8_u8), 0),
        ("u16 1600", Box::new(1_600_u16), 0),
        ("u32 32000", Box::new(32_000_u32), 0),
        ("u64 64000", Box::new(64_000_u64), 0),
        ("&T blanket (&i32)", Box::new(&REFERENCE), 0),
        ("NaiveDate", Box::new(date), DATE_STYLE),
        ("NaiveTime", Box::new(time), TIME_STYLE),
        ("NaiveDateTime", Box::new(datetime), DATETIME_STYLE),
        ("DateTime<Utc>", Box::new(utc), DATETIME_STYLE),
        ("TimeDelta", Box::new(duration), DURATION_STYLE),
        // LibreOffice displays negative durations; Excel's 1900 system shows ####.
        (
            "TimeDelta negative",
            Box::new(negative_duration),
            DURATION_STYLE,
        ),
        ("TimeDelta negative raw", Box::new(negative_duration), 0),
    ];

    let mut cells = Vec::with_capacity(rows.len() * 2);
    for (index, (label, value, style)) in rows.iter().enumerate() {
        let row = u32::try_from(index)?;
        cells.push(CellSpec::new(row, 0, label));
        cells.push(CellSpec::new(row, 1, value.as_ref()).style(*style));
    }

    std::fs::write(path, harness::build(&workbook, &cells)?)?;
    Ok(())
}
