//! Writes an XLSB workbook with one row per `ToCellValue` implementation.
//! The header row and the label/value columns carry distinct row and column
//! styles.
//!
//! Run with `cargo run -p rust-xlsb-core --example showcase -- showcase.xlsb`.

#[path = "../src/e2e/harness.rs"]
#[allow(dead_code)]
mod harness;

use chrono::{NaiveDate, NaiveTime, TimeDelta};
use harness::{CellSpec, DATE_STYLE, DATETIME_STYLE, DURATION_STYLE, TIME_STYLE};
use rust_xlsb_core::prelude::{
    BErr, Blxf, Bold, BorderType, BrtBorder, BrtColor, BrtFill, BrtFont, BrtXF, FontFlags, HAlign,
};
use rust_xlsb_core::{CellStyle, SharedString, ToCellValue, WbContext};

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

fn styled(font: BrtFont, fill: BrtFill, alignment: HAlign) -> CellStyle {
    CellStyle {
        font,
        fill,
        xf: BrtXF {
            alc: alignment,
            ..BrtXF::default()
        },
        ..CellStyle::default()
    }
}

fn table_border(row: usize, last_row: usize, col: usize) -> BrtBorder {
    const HORIZONTAL: [BorderType; 13] = [
        BorderType::Thin,
        BorderType::Dashed,
        BorderType::Dotted,
        BorderType::Double,
        BorderType::Medium,
        BorderType::Thick,
        BorderType::Hairline,
        BorderType::DashDot,
        BorderType::MediumDashed,
        BorderType::DashDotDot,
        BorderType::MediumDashDot,
        BorderType::MediumDashDotDot,
        BorderType::SlantDashDot,
    ];

    let edge = BrtColor::rgb(0x1f, 0x4e, 0x79);
    let horizontal_color = match row % 3 {
        0 => edge.clone(),
        1 => BrtColor::rgb(0x80, 0x80, 0x80),
        _ => BrtColor::rgb(0x40, 0x40, 0x40),
    };

    let mut border = BrtBorder::default();
    if row == 0 {
        border.blxfTop = Blxf {
            border_type: BorderType::Medium,
            color: edge.clone(),
        };
    }
    border.blxfBottom = Blxf {
        border_type: if row == last_row {
            BorderType::Medium
        } else {
            HORIZONTAL[row % HORIZONTAL.len()].clone()
        },
        color: if row == last_row {
            edge.clone()
        } else {
            horizontal_color
        },
    };
    border.blxfLeft = Blxf {
        border_type: if col == 0 {
            BorderType::Medium
        } else {
            BorderType::Thin
        },
        color: edge.clone(),
    };
    border.blxfRight = Blxf {
        border_type: if col == 1 {
            BorderType::Medium
        } else {
            BorderType::Thin
        },
        color: edge,
    };
    border
}

fn write_showcase(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = harness::temporal_workbook()?;
    let header_font = BrtFont {
        bls: Bold::BLSBOLD,
        brtColor: BrtColor::rgb(0xff, 0xff, 0xff),
        ..BrtFont::default()
    };
    let label_font = BrtFont {
        grbit: FontFlags {
            fItalic: true,
            ..FontFlags::default()
        },
        ..BrtFont::default()
    };
    let value_font = BrtFont {
        dyHeight: 200,
        ..BrtFont::default()
    };

    let band_a = BrtFill::solid(BrtColor::rgb(0xdd, 0xeb, 0xf7));
    let band_b = BrtFill::solid(BrtColor::rgb(0xf2, 0xf2, 0xf2));
    let header_fill = BrtFill::solid(BrtColor::rgb(0x1f, 0x4e, 0x79));
    let header_style = workbook.add_style(styled(
        header_font.clone(),
        header_fill.clone(),
        HAlign::Center,
    ))?;
    let label_style =
        workbook.add_style(styled(label_font.clone(), BrtFill::default(), HAlign::Left))?;
    let value_style = workbook.add_style(styled(
        value_font.clone(),
        BrtFill::default(),
        HAlign::Right,
    ))?;
    let row_a = workbook.add_style(styled(BrtFont::default(), band_a.clone(), HAlign::Left))?;
    let row_b = workbook.add_style(styled(BrtFont::default(), band_b.clone(), HAlign::Center))?;

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

    // Row and column defaults do not reach cells that carry an explicit style
    // in LibreOffice, so mirror the same styles per cell for a visible demo.
    let date_fmt = workbook.add_number_format("yyyy-mm-dd")?;
    let time_fmt = workbook.add_number_format("hh:mm:ss.000")?;
    let datetime_fmt = workbook.add_number_format("yyyy-mm-dd hh:mm:ss.000")?;
    let duration_fmt = workbook.add_number_format("[h]:mm:ss.000")?;
    let last_row = rows.len().saturating_sub(1);
    let mut cell_styles = Vec::with_capacity(rows.len());
    for (row, (_, _, cell_style)) in rows.iter().enumerate() {
        let fill = if row == 0 {
            header_fill.clone()
        } else if row % 2 == 1 {
            band_a.clone()
        } else {
            band_b.clone()
        };
        let alignment = if row == 0 {
            HAlign::Center
        } else if row % 2 == 1 {
            HAlign::Left
        } else {
            HAlign::Center
        };
        let mut styles = [0_u32; 2];
        for (col, slot) in styles.iter_mut().enumerate() {
            let font = if row == 0 {
                header_font.clone()
            } else if col == 0 {
                label_font.clone()
            } else {
                value_font.clone()
            };
            let mut xf = BrtXF {
                alc: alignment.clone(),
                ..BrtXF::default()
            };
            if col == 1 {
                xf.iFmt = match *cell_style {
                    DATE_STYLE => date_fmt,
                    TIME_STYLE => time_fmt,
                    DATETIME_STYLE => datetime_fmt,
                    DURATION_STYLE => duration_fmt,
                    _ => 0,
                };
            }
            *slot = workbook.add_style(CellStyle {
                font,
                fill: fill.clone(),
                border: table_border(row, last_row, col),
                xf,
            })?;
        }
        cell_styles.push(styles);
    }

    let mut sheet = workbook.worksheet();
    sheet.set_column_style(0, 0, label_style)?;
    sheet.set_column_style(1, 1, value_style)?;
    sheet.set_row_style(0, header_style)?;
    for (index, _) in rows.iter().enumerate() {
        let row = u32::try_from(index)?;
        if row > 0 {
            sheet.set_row_style(row, if row % 2 == 1 { row_a } else { row_b })?;
        }
    }

    let mut cells = Vec::with_capacity(rows.len() * 2);
    for (index, (label, value, _)) in rows.iter().enumerate() {
        let row = u32::try_from(index)?;
        cells.push(CellSpec::new(row, 0, label).style(cell_styles[index][0]));
        cells.push(CellSpec::new(row, 1, value.as_ref()).style(cell_styles[index][1]));
    }

    std::fs::write(path, harness::build_with_sheet(&workbook, &sheet, &cells)?)?;
    Ok(())
}
