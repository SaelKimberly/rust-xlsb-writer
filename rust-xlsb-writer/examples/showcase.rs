//! Writes a styled XLSB workbook through the public writer API.
//!
//! Run with
//! `cargo run -p rust-xlsb-writer --example showcase -- showcase.xlsb`.

use chrono::{NaiveDate, NaiveTime, TimeDelta};
use rust_xlsb_core::prelude::BErr;
use rust_xlsb_writer::{Color, Format, FormatAlign, FormatBorder, Workbook};

const EDGE: Color = Color::RGB(0x1F_4E_79);
const BAND_A: Color = Color::RGB(0xDD_EB_F7);
const BAND_B: Color = Color::RGB(0xF2_F2_F2);

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

fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(EDGE)
        .set_border(FormatBorder::Medium)
        .set_border_color(EDGE)
        .set_align(FormatAlign::Center)
}

fn label_format(fill: Color) -> Format {
    Format::new()
        .set_italic()
        .set_background_color(fill)
        .set_border(FormatBorder::Thin)
        .set_border_color(EDGE)
        .set_align(FormatAlign::Left)
}

fn value_format(fill: Color) -> Format {
    Format::new()
        .set_font_size(10)
        .set_background_color(fill)
        .set_border(FormatBorder::Thin)
        .set_border_color(EDGE)
        .set_align(FormatAlign::Right)
}

fn write_showcase(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Showcase")?;
    sheet.set_column_width(0, 24)?;
    sheet.set_column_width(1, 30)?;
    sheet.set_row_height(0, 24)?;

    let header = header_format();
    let row_a = Format::new().set_background_color(BAND_A);
    let row_b = Format::new().set_background_color(BAND_B);
    sheet.set_row_format(0, &header)?;
    sheet.set_column_format(0, &label_format(Color::Default))?;
    sheet.set_column_format(1, &value_format(Color::Default))?;

    sheet.write_with_format(0, 0, "Type", &header)?;
    sheet.write_with_format(0, 1, "Value", &header)?;

    let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("valid date");
    let time = NaiveTime::from_hms_milli_opt(12, 34, 56, 789).expect("valid time");
    let datetime = date.and_time(time);
    let duration = TimeDelta::milliseconds(3_723_500);

    let rows = [
        "&str", "String", "i32", "f64", "bool", "error", "date", "time", "datetime", "duration",
        "url", "internal",
    ];

    let fill = |index: usize| {
        if (index + 1) % 2 == 1 { BAND_A } else { BAND_B }
    };

    for (index, label) in rows.iter().enumerate() {
        let row = u32::try_from(index + 1)?;
        let fill = fill(index);
        sheet.set_row_format(row, if row % 2 == 1 { &row_a } else { &row_b })?;
        sheet.write_with_format(row, 0, *label, &label_format(fill))?;
    }

    let row = |index: usize| u32::try_from(index + 1).expect("row fits u32");

    sheet.write_with_format(row(0), 1, "hello", &value_format(fill(0)))?;
    sheet.write_with_format(
        row(1),
        1,
        String::from("owned text"),
        &value_format(fill(1)),
    )?;
    sheet.write_with_format(row(2), 1, 42_i32, &value_format(fill(2)))?;
    sheet.write_with_format(row(3), 1, 12.34_f64, &value_format(fill(3)))?;
    sheet.write_with_format(row(4), 1, true, &value_format(fill(4)))?;
    sheet.write_with_format(row(5), 1, BErr::Div0, &value_format(fill(5)))?;
    sheet.write_date_with_format(
        row(6),
        1,
        date,
        &value_format(fill(6)).set_num_format("yyyy-mm-dd"),
    )?;
    sheet.write_time_with_format(
        row(7),
        1,
        time,
        &value_format(fill(7)).set_num_format("hh:mm:ss.000"),
    )?;
    sheet.write_datetime_with_format(
        row(8),
        1,
        datetime,
        &value_format(fill(8)).set_num_format("yyyy-mm-dd hh:mm:ss.000"),
    )?;
    sheet.write_with_format(
        row(9),
        1,
        duration,
        &value_format(fill(9)).set_num_format("[h]:mm:ss.000"),
    )?;

    sheet.write_url_with_text(row(10), 1, "https://www.rust-lang.org", "Rust language")?;
    sheet.write_url_with_text(row(11), 1, "internal:Showcase!B2", "Jump to B2")?;

    workbook.save(path)?;
    Ok(())
}
