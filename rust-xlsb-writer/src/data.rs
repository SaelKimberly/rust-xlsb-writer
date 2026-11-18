//! Context-free cell data and the public write trait.

use std::borrow::Cow;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
use rust_xlsb_core::prelude::BErr;

use crate::{Error, Format, Result, Worksheet};

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub enum CellData {
    Blank,
    Bool(bool),
    Error(BErr),
    Integer(i64),
    Number(f64),
    String(String),
    Date(NaiveDate),
    Time(NaiveTime),
    DateTime(NaiveDateTime),
    Duration(TimeDelta),
}

/// Converts a Rust value into a worksheet cell.
pub trait IntoExcelData {
    /// Writes this value without formatting.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, a string is too
    /// long, or the value cannot be represented.
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet>;

    /// Writes this value with formatting.
    ///
    /// # Errors
    ///
    /// Returns an error when the row/column is out of range, a string is too
    /// long, or the value cannot be represented.
    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet>;
}

/// Converts a date/time value into context-free cell data.
pub trait IntoExcelDateTime {
    /// Converts this value.
    ///
    /// # Errors
    ///
    /// Returns an error when the value cannot be represented by the core
    /// temporal contract.
    fn into_cell_data(self) -> Result<CellData>;
}

fn write_data<'a>(
    worksheet: &'a mut Worksheet,
    row: u32,
    col: u16,
    data: CellData,
    format: Option<&Format>,
) -> Result<&'a mut Worksheet> {
    worksheet.store(row, col, data, format)
}

macro_rules! impl_integer {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoExcelData for $ty {
                fn write(
                    self,
                    worksheet: &mut Worksheet,
                    row: u32,
                    col: u16,
                ) -> Result<&mut Worksheet> {
                    write_data(worksheet, row, col, CellData::Integer(i64::from(self)), None)
                }

                fn write_with_format<'a>(
                    self,
                    worksheet: &'a mut Worksheet,
                    row: u32,
                    col: u16,
                    format: &Format,
                ) -> Result<&'a mut Worksheet> {
                    write_data(
                        worksheet,
                        row,
                        col,
                        CellData::Integer(i64::from(self)),
                        Some(format),
                    )
                }
            }
        )*
    };
}

impl_integer!(u8, u16, u32, i8, i16, i32, i64);

impl IntoExcelData for u64 {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::Integer(u64_to_i64(self)?),
            None,
        )
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::Integer(u64_to_i64(self)?),
            Some(format),
        )
    }
}

impl IntoExcelData for f32 {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::Number(f64::from(self)), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::Number(f64::from(self)),
            Some(format),
        )
    }
}

impl IntoExcelData for f64 {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::Number(self), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(worksheet, row, col, CellData::Number(self), Some(format))
    }
}

impl IntoExcelData for bool {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::Bool(self), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(worksheet, row, col, CellData::Bool(self), Some(format))
    }
}

impl IntoExcelData for BErr {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::Error(self), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(worksheet, row, col, CellData::Error(self), Some(format))
    }
}

impl IntoExcelData for &str {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::String(self.to_owned()), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::String(self.to_owned()),
            Some(format),
        )
    }
}

impl IntoExcelData for String {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::String(self), None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(worksheet, row, col, CellData::String(self), Some(format))
    }
}

impl IntoExcelData for Cow<'_, str> {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::String(self.into_owned()),
            None,
        )
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(
            worksheet,
            row,
            col,
            CellData::String(self.into_owned()),
            Some(format),
        )
    }
}

impl IntoExcelData for () {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        write_data(worksheet, row, col, CellData::Blank, None)
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        write_data(worksheet, row, col, CellData::Blank, Some(format))
    }
}

impl<T: IntoExcelData> IntoExcelData for Option<T> {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        match self {
            Some(value) => value.write(worksheet, row, col),
            None => write_data(worksheet, row, col, CellData::Blank, None),
        }
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        match self {
            Some(value) => value.write_with_format(worksheet, row, col, format),
            None => write_data(worksheet, row, col, CellData::Blank, Some(format)),
        }
    }
}

impl<T: IntoExcelData, E: IntoExcelData> IntoExcelData for Result<T, E> {
    fn write(self, worksheet: &mut Worksheet, row: u32, col: u16) -> Result<&mut Worksheet> {
        match self {
            Ok(value) => value.write(worksheet, row, col),
            Err(error) => error.write(worksheet, row, col),
        }
    }

    fn write_with_format<'a>(
        self,
        worksheet: &'a mut Worksheet,
        row: u32,
        col: u16,
        format: &Format,
    ) -> Result<&'a mut Worksheet> {
        match self {
            Ok(value) => value.write_with_format(worksheet, row, col, format),
            Err(error) => error.write_with_format(worksheet, row, col, format),
        }
    }
}

macro_rules! impl_datetime {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl IntoExcelDateTime for $ty {
                fn into_cell_data(self) -> Result<CellData> {
                    Ok(CellData::$variant(self))
                }
            }

            impl IntoExcelDateTime for &$ty {
                fn into_cell_data(self) -> Result<CellData> {
                    Ok(CellData::$variant(*self))
                }
            }
        )*
    };
}

impl_datetime!(
    NaiveDate => Date,
    NaiveTime => Time,
    NaiveDateTime => DateTime,
    TimeDelta => Duration,
);

impl IntoExcelDateTime for DateTime<Utc> {
    fn into_cell_data(self) -> Result<CellData> {
        Ok(CellData::DateTime(self.naive_utc()))
    }
}

impl IntoExcelDateTime for &DateTime<Utc> {
    fn into_cell_data(self) -> Result<CellData> {
        Ok(CellData::DateTime(self.naive_utc()))
    }
}

macro_rules! impl_datetime_data {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoExcelData for $ty {
                fn write(
                    self,
                    worksheet: &mut Worksheet,
                    row: u32,
                    col: u16,
                ) -> Result<&mut Worksheet> {
                    let data = self.into_cell_data()?;
                    write_data(worksheet, row, col, data, None)
                }

                fn write_with_format<'a>(
                    self,
                    worksheet: &'a mut Worksheet,
                    row: u32,
                    col: u16,
                    format: &Format,
                ) -> Result<&'a mut Worksheet> {
                    let data = self.into_cell_data()?;
                    write_data(worksheet, row, col, data, Some(format))
                }
            }

            impl IntoExcelData for &$ty {
                fn write(
                    self,
                    worksheet: &mut Worksheet,
                    row: u32,
                    col: u16,
                ) -> Result<&mut Worksheet> {
                    let data = self.into_cell_data()?;
                    write_data(worksheet, row, col, data, None)
                }

                fn write_with_format<'a>(
                    self,
                    worksheet: &'a mut Worksheet,
                    row: u32,
                    col: u16,
                    format: &Format,
                ) -> Result<&'a mut Worksheet> {
                    let data = self.into_cell_data()?;
                    write_data(worksheet, row, col, data, Some(format))
                }
            }
        )*
    };
}

impl_datetime_data!(
    NaiveDate,
    NaiveTime,
    NaiveDateTime,
    TimeDelta,
    DateTime<Utc>,
);

fn u64_to_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| Error::ValueNotRepresentable("u64 exceeds i64 range"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Worksheet;

    fn cell_data(worksheet: &Worksheet, row: u32, col: u16) -> &CellData {
        &worksheet
            .cells()
            .get(&row)
            .expect("row")
            .get(&col)
            .expect("cell")
            .data
    }

    #[test]
    fn writes_integer_and_number_cells() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        42_i32.write(&mut worksheet, 0, 0).expect("writes");
        12.5_f64.write(&mut worksheet, 0, 1).expect("writes");

        assert_eq!(cell_data(&worksheet, 0, 0), &CellData::Integer(42));
        assert_eq!(cell_data(&worksheet, 0, 1), &CellData::Number(12.5));
    }

    #[test]
    fn writes_strings_booleans_and_blank_options() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        "hello".write(&mut worksheet, 0, 0).expect("writes");
        true.write(&mut worksheet, 0, 1).expect("writes");
        Option::<i32>::None
            .write(&mut worksheet, 0, 2)
            .expect("writes");

        assert_eq!(
            cell_data(&worksheet, 0, 0),
            &CellData::String("hello".to_owned())
        );
        assert_eq!(cell_data(&worksheet, 0, 1), &CellData::Bool(true));
        assert_eq!(cell_data(&worksheet, 0, 2), &CellData::Blank);
    }

    #[test]
    fn writes_result_ok_and_err_values() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        Ok::<i32, &str>(7)
            .write(&mut worksheet, 0, 0)
            .expect("writes");
        Err::<i32, &str>("bad")
            .write(&mut worksheet, 0, 1)
            .expect("writes");

        assert_eq!(cell_data(&worksheet, 0, 0), &CellData::Integer(7));
        assert_eq!(
            cell_data(&worksheet, 0, 1),
            &CellData::String("bad".to_owned())
        );
    }

    #[test]
    fn writes_owned_and_borrowed_dates() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("date");
        date.write(&mut worksheet, 0, 0).expect("writes");
        (&date).write(&mut worksheet, 0, 1).expect("writes");

        assert_eq!(cell_data(&worksheet, 0, 0), &CellData::Date(date));
        assert_eq!(cell_data(&worksheet, 0, 1), &CellData::Date(date));
    }

    #[test]
    fn rejects_u64_above_i64() {
        let mut worksheet = Worksheet::new("Sheet1".to_owned());
        assert!(matches!(
            u64::MAX.write(&mut worksheet, 0, 0),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn into_excel_datetime_supports_utc() {
        let datetime = DateTime::<Utc>::from_timestamp(0, 0).expect("timestamp");
        let data = datetime.into_cell_data().expect("converts");
        assert!(matches!(data, CellData::DateTime(_)));
    }
}
