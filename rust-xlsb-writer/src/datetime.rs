//! Excel-native date and time values.

use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::{Error, Result};

const MAX_SERIAL: f64 = 2_958_466.0;
const MIN_TIMESTAMP: i64 = -2_209_075_200;
const MAX_TIMESTAMP: i64 = 253_402_300_800;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// A date, time, or datetime expressed with Excel's 1900 date system.
///
/// Unlike `chrono` and `jiff`, this type can represent Excel's fake
/// 1900-02-29 leap day and times with an hour component of 24 or more. Values
/// are written as Excel serial numbers, so a number format is required to
/// display them as dates or times.
///
/// ```
/// use rust_xlsb_writer::ExcelDateTime;
///
/// let datetime = ExcelDateTime::from_ymd(2026, 11, 18)?.and_hms(12, 30, 0)?;
/// assert_eq!(datetime.to_excel(), 46344.520833333336);
/// # Ok::<(), rust_xlsb_writer::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ExcelDateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u16,
    min: u8,
    sec: f64,
    serial_datetime: Option<f64>,
    datetime_type: ExcelDateTimeType,
}

impl Default for ExcelDateTime {
    fn default() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            min: 0,
            sec: 0.0,
            serial_datetime: None,
            datetime_type: ExcelDateTimeType::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExcelDateTimeType {
    Default,
    DateOnly,
    TimeOnly,
    DateAndTime,
}

impl ExcelDateTime {
    /// Creates a date from year, month, and day.
    ///
    /// Excel accepts 1899-12-31 through 9999-12-31, including its fake
    /// 1900-02-29 leap day.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] for a year, month, or day outside
    /// Excel's range.
    pub fn from_ymd(year: u16, month: u8, day: u8) -> Result<Self> {
        Self::validate_ymd(year, month, day)?;
        Ok(Self {
            year,
            month,
            day,
            datetime_type: ExcelDateTimeType::DateOnly,
            ..Self::default()
        })
    }

    /// Creates a time from hours, minutes, and seconds.
    ///
    /// Hours are not restricted, so values beyond 24 represent durations.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] when minutes exceed 60 or seconds
    /// exceed 59.999.
    pub fn from_hms(hour: u16, min: u8, sec: impl Into<f64>) -> Result<Self> {
        Self::default().and_hms(hour, min, sec)
    }

    /// Creates a time from hours, minutes, seconds, and milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] when a component is outside
    /// Excel's range.
    pub fn from_hms_milli(hour: u16, min: u8, sec: u8, milli: u16) -> Result<Self> {
        Self::default().and_hms_milli(hour, min, sec, milli)
    }

    /// Adds a time to this date, or replaces the time of a datetime.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] when minutes exceed 60 or seconds
    /// exceed 59.999.
    pub fn and_hms(mut self, hour: u16, min: u8, sec: impl Into<f64>) -> Result<Self> {
        let sec = sec.into();
        Self::validate_hms(min, sec)?;
        self.datetime_type = if self.datetime_type == ExcelDateTimeType::DateOnly {
            ExcelDateTimeType::DateAndTime
        } else {
            ExcelDateTimeType::TimeOnly
        };
        self.hour = hour;
        self.min = min;
        self.sec = sec;
        Ok(self)
    }

    /// Adds a time with milliseconds to this date, or replaces the time of a
    /// datetime.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] when a component is outside
    /// Excel's range.
    pub fn and_hms_milli(mut self, hour: u16, min: u8, sec: u8, milli: u16) -> Result<Self> {
        Self::validate_hms_milli(min, sec, milli)?;
        self.datetime_type = if self.datetime_type == ExcelDateTimeType::DateOnly {
            ExcelDateTimeType::DateAndTime
        } else {
            ExcelDateTimeType::TimeOnly
        };
        self.hour = hour;
        self.min = min;
        self.sec = f64::from(sec) + f64::from(milli) / 1000.0;
        Ok(self)
    }

    /// Creates a value from an existing Excel serial number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] outside Excel's serial range
    /// `0.0..2_958_466.0`.
    pub fn from_serial_datetime(number: impl Into<f64>) -> Result<Self> {
        let number = number.into();
        if !(0.0..MAX_SERIAL).contains(&number) {
            return Err(range_error(format!(
                "serial datetime '{number}' outside Excel's 1900-9999 range"
            )));
        }
        Ok(Self {
            serial_datetime: Some(number),
            ..Self::default()
        })
    }

    /// Creates a value from a Unix timestamp in seconds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeRangeError`] outside the range that maps to
    /// 1899-12-31 through 9999-12-31.
    pub fn from_timestamp(timestamp: i64) -> Result<Self> {
        if !(MIN_TIMESTAMP..MAX_TIMESTAMP).contains(&timestamp) {
            return Err(range_error(format!(
                "Unix timestamp '{timestamp}' outside Excel's 1900-9999 range"
            )));
        }
        let datetime = DateTime::<Utc>::from_timestamp(timestamp, 0)
            .ok_or_else(|| range_error(format!("Unix timestamp '{timestamp}' is invalid")))?
            .naive_utc();
        let year = u16::try_from(datetime.year())
            .map_err(|_| range_error("timestamp year outside the Excel range"))?;
        let month = u8::try_from(datetime.month())
            .map_err(|_| range_error("timestamp month outside the Excel range"))?;
        let day = u8::try_from(datetime.day())
            .map_err(|_| range_error("timestamp day outside the Excel range"))?;
        let hour = u16::try_from(datetime.hour())
            .map_err(|_| range_error("timestamp hour outside the Excel range"))?;
        let min = u8::try_from(datetime.minute())
            .map_err(|_| range_error("timestamp minute outside the Excel range"))?;
        let sec = f64::from(datetime.second());
        Self::validate_ymd(year, month, day)?;
        Self::validate_hms(min, sec)?;
        Ok(Self {
            year,
            month,
            day,
            hour,
            min,
            sec,
            serial_datetime: None,
            datetime_type: ExcelDateTimeType::DateAndTime,
        })
    }

    /// Parses a date, time, or datetime in approximate ISO 8601 form.
    ///
    /// Accepted forms are `yyyy-mm-dd`, `hh:mm[:ss[.fff]]`, and the same date
    /// with a `T` or space separator and optional time. A trailing `Z` and
    /// surrounding whitespace are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DateTimeParseError`] when the string has neither a date
    /// nor a time part, or [`Error::DateTimeRangeError`] when a parsed value is
    /// outside Excel's range.
    pub fn parse_from_str(datetime: &str) -> Result<Self> {
        let datetime = datetime.trim();
        let mut parts = datetime.split(['-', 'T', 'Z', ' ', ':']);

        if datetime.contains('-') {
            let year = parse_u16(parts.next());
            let month = parse_u8(parts.next());
            let day = parse_u8(parts.next());
            let hour = parse_u16(parts.next());
            let min = parse_u8(parts.next());
            let sec = parse_f64(parts.next());
            Self::from_ymd(year, month, day)?.and_hms(hour, min, sec)
        } else if datetime.contains(':') {
            let hour = parse_u16(parts.next());
            let min = parse_u8(parts.next());
            let sec = parse_f64(parts.next());
            Self::from_hms(hour, min, sec)
        } else {
            Err(Error::DateTimeParseError(datetime.to_owned()))
        }
    }

    /// Returns the Excel serial number for this value.
    #[must_use]
    pub fn to_excel(&self) -> f64 {
        self.serial_datetime
            .unwrap_or_else(|| self.to_excel_from_ymd_hms())
    }

    fn validate_ymd(year: u16, month: u8, day: u8) -> Result<()> {
        let mut months = [31_u8, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        if year > 9999 || (year < 1900 && (year != 1899 || month != 12 || day != 31)) {
            return Err(range_error(format!(
                "year '{year}' outside Excel's 1900-9999 range"
            )));
        }
        if !(1..=12).contains(&month) {
            return Err(range_error(format!(
                "month '{month}' outside Excel's 1-12 range"
            )));
        }
        if Self::is_leap_year(u64::from(year)) || year == 1900 {
            months[1] = 29;
        }
        if day < 1 || day > months[usize::from(month - 1)] {
            return Err(range_error(format!(
                "day '{day}' is invalid for {year}-{month:02}"
            )));
        }
        Ok(())
    }

    fn validate_hms(min: u8, sec: f64) -> Result<()> {
        if min > 60 {
            return Err(range_error(format!(
                "minutes '{min}' outside Excel's 0-60 range"
            )));
        }
        if sec > 59.999 {
            return Err(range_error(format!(
                "seconds '{sec}' outside Excel's 0-59.999 range"
            )));
        }
        Ok(())
    }

    fn validate_hms_milli(min: u8, sec: u8, milli: u16) -> Result<()> {
        if min > 60 {
            return Err(range_error(format!(
                "minutes '{min}' outside Excel's 0-60 range"
            )));
        }
        if sec > 60 {
            return Err(range_error(format!(
                "seconds '{sec}' outside Excel's 0-60 range"
            )));
        }
        if milli > 999 {
            return Err(range_error(format!(
                "milliseconds '{milli}' outside Excel's 0-999 range"
            )));
        }
        Ok(())
    }

    // Excel cannot represent the fake leap day or hours beyond 24 with chrono,
    // so the native type mirrors the reference serial formula.
    #[expect(
        clippy::suboptimal_flops,
        reason = "plain operations match the reference serial byte-for-byte"
    )]
    fn to_excel_from_ymd_hms(&self) -> f64 {
        let mut year = u32::from(self.year);
        let mut month = u32::from(self.month);
        let mut day = u32::from(self.day);
        let mut months = [31_u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        if year == 0 {
            year = 1899;
            month = 12;
            day = 31;
        }

        let seconds = (f64::from(self.hour) * 3600.0 + f64::from(self.min) * 60.0 + self.sec)
            / SECONDS_PER_DAY;

        if year == 1899 && month == 12 && day == 31 {
            return seconds;
        }
        if year == 1900 && month == 2 && day == 29 {
            return 60.0 + seconds;
        }

        let mut days = 0;
        let leap_day = if Self::is_leap_year(u64::from(year)) {
            months[1] = 29;
            1
        } else {
            0
        };
        for days_in_month in months
            .iter()
            .take(usize::try_from(month - 1).expect("validated month is 1-12"))
        {
            days += days_in_month;
        }
        days += day;

        let range = year - 1900;
        days += range * 365;
        days += range / 4;
        days -= range / 100;
        days += (range + 300) / 400;
        days -= leap_day;
        if days > 59 {
            days += 1;
        }

        f64::from(days) + seconds
    }

    const fn is_leap_year(year: u64) -> bool {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    }
}

fn range_error(message: impl Into<String>) -> Error {
    Error::DateTimeRangeError(message.into())
}

fn parse_u16(part: Option<&str>) -> u16 {
    part.and_then(|part| part.parse().ok()).unwrap_or_default()
}

fn parse_u8(part: Option<&str>) -> u8 {
    part.and_then(|part| part.parse().ok()).unwrap_or_default()
}

fn parse_f64(part: Option<&str>) -> f64 {
    part.and_then(|part| part.parse().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_reference_ranges() {
        assert!(matches!(
            ExcelDateTime::from_ymd(1899, 12, 30),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_ymd(10_000, 1, 1),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_ymd(2000, 0, 1),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_ymd(2000, 4, 31),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_hms(0, 61, 0),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_hms(0, 0, 59.9999),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_hms_milli(0, 0, 0, 1000),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_serial_datetime(-1),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_serial_datetime(2_958_466),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_timestamp(-2_209_075_201),
            Err(Error::DateTimeRangeError(_))
        ));
        assert!(matches!(
            ExcelDateTime::from_timestamp(253_402_300_800),
            Err(Error::DateTimeRangeError(_))
        ));
    }

    #[test]
    fn serializes_excel_boundaries() {
        let cases = [
            ("1899-12-31", 0.0),
            ("1900-01-01", 1.0),
            ("1900-02-28", 59.0),
            ("1900-02-29", 60.0),
            ("1900-03-01", 61.0),
            ("2026-11-18", 46_344.0),
            ("9999-12-31", 2_958_465.0),
        ];
        for (text, expected) in cases {
            assert_eq!(
                ExcelDateTime::parse_from_str(text)
                    .expect("valid date")
                    .to_excel(),
                expected,
                "{text}"
            );
        }
    }

    #[test]
    fn serializes_times_and_datetimes() {
        assert_eq!(ExcelDateTime::from_hms(12, 0, 0).unwrap().to_excel(), 0.5);
        assert_eq!(
            ExcelDateTime::from_hms(36, 0, 0).unwrap().to_excel(),
            1.5,
            "hours beyond 24 are durations"
        );
        assert_eq!(
            ExcelDateTime::from_hms_milli(12, 30, 45, 500)
                .unwrap()
                .to_excel(),
            0.521_359_953_703_703_7
        );
        assert_eq!(
            ExcelDateTime::from_ymd(2026, 11, 18)
                .unwrap()
                .and_hms(12, 30, 0)
                .unwrap()
                .to_excel(),
            46_344.520_833_333_336
        );
        assert_eq!(
            ExcelDateTime::parse_from_str("1982-08-25T00:15:20.213")
                .unwrap()
                .to_excel(),
            30_188.010_650_613_425
        );
        assert_eq!(
            ExcelDateTime::parse_from_str("9999-12-31T23:59:59.000")
                .unwrap()
                .to_excel(),
            2_958_465.999_988_426
        );
        assert_eq!(
            ExcelDateTime::parse_from_str("1900-02-29T12:00")
                .unwrap()
                .to_excel(),
            60.5
        );
    }

    #[test]
    fn parses_dates_times_and_datetimes() {
        assert_eq!(
            ExcelDateTime::parse_from_str(" 12:00 ").unwrap().to_excel(),
            0.5
        );
        assert_eq!(
            ExcelDateTime::parse_from_str("2023-01-31T12:30:45Z")
                .unwrap()
                .to_excel(),
            44_957.521_354_166_67
        );
        assert!(matches!(
            ExcelDateTime::parse_from_str("not a date"),
            Err(Error::DateTimeParseError(_))
        ));
    }

    #[test]
    fn serial_and_timestamp_round_trip() {
        assert_eq!(
            ExcelDateTime::from_serial_datetime(1.5)
                .unwrap()
                .to_excel()
                .to_bits(),
            1.5_f64.to_bits()
        );
        assert_eq!(
            ExcelDateTime::from_timestamp(0).unwrap().to_excel(),
            25_569.0
        );
        assert_eq!(
            ExcelDateTime::from_timestamp(-2_209_075_200)
                .unwrap()
                .to_excel(),
            0.0
        );
    }
}
