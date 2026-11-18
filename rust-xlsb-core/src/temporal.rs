//! Temporal `chrono` values, stored as Excel serial numbers.
//!
//! Excel keeps a date and a time in one `f64`: whole days since the 1900 epoch
//! plus the fraction of a day. Readers resolve that number to milliseconds, so
//! conversion rounds to the nearest millisecond (half away from zero). Dates
//! are integers and therefore use the compact Rk integer encoding; times and
//! datetimes usually need `BrtCellReal`.
//!
//! The 1900 date system is the only one this workspace can signal: there is no
//! `BrtWbProp`, and Excel's epoch carries the fake leap day 1900-02-29, so
//! serials before 1900-03-01 are shifted by one day.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Utc};

use crate::cell_value::{CellValue, ToCellValue};
use crate::context::WbContext;
use crate::{Error, Result};

const MS_PER_DAY: i32 = 86_400_000;
const MAX_SERIAL: i32 = 2_958_465;
const EPOCH: NaiveDate = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
const MIN_DATE: NaiveDate = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
const MAX_DATE: NaiveDate = NaiveDate::from_ymd_opt(9999, 12, 31).unwrap();
const LEAP_BUG_END: NaiveDate = NaiveDate::from_ymd_opt(1900, 3, 1).unwrap();

/// Excel serial of `date` in the 1900 date system.
fn date_serial(date: NaiveDate) -> Result<i32> {
    if !(MIN_DATE..=MAX_DATE).contains(&date) {
        return Err(Error::ValueNotRepresentable(
            "date outside 1900-01-01..9999-12-31",
        ));
    }
    let days = (date - EPOCH).num_days();
    let serial = if date < LEAP_BUG_END { days - 1 } else { days };
    Ok(i32::try_from(serial).expect("1900..9999 serials fit i32"))
}

/// Milliseconds since midnight; `MS_PER_DAY` means the value rounds to the next day.
fn time_ms(time: NaiveTime) -> Result<i32> {
    if time.nanosecond() >= 1_000_000_000 {
        return Err(Error::ValueNotRepresentable(
            "leap second has no Excel representation",
        ));
    }
    let nanos =
        i64::from(time.num_seconds_from_midnight()) * 1_000_000_000 + i64::from(time.nanosecond());
    let ms = (nanos + 500_000) / 1_000_000;
    Ok(i32::try_from(ms).expect("milliseconds of a day fit i32"))
}

fn serial_value(days: i32, ms: i32) -> f64 {
    f64::from(days) + f64::from(ms) / f64::from(MS_PER_DAY)
}

/// Rounds a duration to whole milliseconds, half away from zero.
fn delta_ms(delta: TimeDelta) -> i64 {
    let nanoseconds =
        i128::from(delta.num_seconds()) * 1_000_000_000 + i128::from(delta.subsec_nanos());
    let milliseconds = if nanoseconds >= 0 {
        (nanoseconds + 500_000) / 1_000_000
    } else {
        (nanoseconds - 500_000) / 1_000_000
    };
    i64::try_from(milliseconds).expect("TimeDelta fits i64 milliseconds")
}

fn delta_days(delta: TimeDelta) -> f64 {
    // TimeDelta spans roughly ±292 years, far inside f64's exact integer range.
    #[expect(
        clippy::cast_precision_loss,
        reason = "TimeDelta milliseconds stay below 2^53, so the cast is exact"
    )]
    let ms = delta_ms(delta) as f64;
    ms / f64::from(MS_PER_DAY)
}

impl ToCellValue for NaiveDate {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Number(
            context.store_f64(f64::from(date_serial(*self)?)),
        ))
    }
}

impl ToCellValue for NaiveTime {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        let ms = time_ms(*self)?;
        if ms >= MS_PER_DAY {
            return Err(Error::ValueNotRepresentable("time rounds past midnight"));
        }
        Ok(CellValue::Number(context.store_f64(serial_value(0, ms))))
    }
}

impl ToCellValue for NaiveDateTime {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        let mut days = date_serial(self.date())?;
        let mut ms = time_ms(self.time())?;
        if ms == MS_PER_DAY {
            days += 1;
            ms = 0;
            // Serial 60 is Excel's fake 1900-02-29; the next real day is 61.
            if days == 60 {
                days = 61;
            }
        }
        if days > MAX_SERIAL {
            return Err(Error::ValueNotRepresentable(
                "datetime rounds past 9999-12-31",
            ));
        }
        Ok(CellValue::Number(context.store_f64(serial_value(days, ms))))
    }
}

impl ToCellValue for DateTime<Utc> {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        self.naive_utc().to_cell_value(context)
    }
}

impl ToCellValue for TimeDelta {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Number(context.store_f64(delta_days(*self))))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, TimeZone};

    use super::*;
    use crate::Workbook;
    use crate::context::NumberEncoding;

    fn stored(value: &dyn ToCellValue, workbook: &Workbook) -> f64 {
        match value.to_cell_value(workbook).expect("converts") {
            CellValue::Number(NumberEncoding::Real(value)) => value,
            CellValue::Number(NumberEncoding::Rk(rk)) => rk.value(),
            other => panic!("expected a number, got {other:?}"),
        }
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn serial_matches_the_1900_epoch() {
        let workbook = Workbook::new().unwrap();
        let cases = [
            (date(1900, 1, 1), 1.0),
            (date(1900, 2, 28), 59.0),
            (date(1900, 3, 1), 61.0),
            (date(2026, 11, 18), 46_344.0),
            (date(9999, 12, 31), 2_958_465.0),
        ];
        for (date, serial) in cases {
            assert_eq!(stored(&date, &workbook), serial);
        }
    }

    #[test]
    fn dates_use_rk_integers() {
        let workbook = Workbook::new().unwrap();
        let value = date(2026, 11, 18).to_cell_value(&workbook).unwrap();
        match value {
            CellValue::Number(NumberEncoding::Rk(rk)) => assert_eq!(rk.value(), 46_344.0),
            other => panic!("expected Rk, got {other:?}"),
        }
    }

    #[test]
    fn times_round_to_milliseconds() {
        let workbook = Workbook::new().unwrap();
        let noon = NaiveTime::from_hms_nano_opt(12, 0, 0, 0).unwrap();
        assert_eq!(stored(&noon, &workbook), 0.5);
        assert!(matches!(
            noon.to_cell_value(&workbook).unwrap(),
            CellValue::Number(NumberEncoding::Rk(_))
        ));

        let almost_midnight = NaiveTime::from_hms_nano_opt(23, 59, 59, 999_400_000).unwrap();
        assert_eq!(
            stored(&almost_midnight, &workbook),
            f64::from(86_399_999) / f64::from(MS_PER_DAY)
        );

        let sub_ms = NaiveTime::from_hms_nano_opt(0, 0, 0, 999_999).unwrap();
        assert_eq!(stored(&sub_ms, &workbook), 1e-3 / 86_400.0);
    }

    #[test]
    fn time_rounding_into_the_next_day_is_rejected() {
        let workbook = Workbook::new().unwrap();
        let time = NaiveTime::from_hms_nano_opt(23, 59, 59, 999_600_000).unwrap();
        assert!(matches!(
            time.to_cell_value(&workbook),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn leap_seconds_are_rejected() {
        let workbook = Workbook::new().unwrap();
        let leap = NaiveTime::from_hms_nano_opt(23, 59, 59, 1_500_000_000).unwrap();
        assert!(matches!(
            leap.to_cell_value(&workbook),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn datetimes_add_the_day_fraction() {
        let workbook = Workbook::new().unwrap();
        let datetime = date(2026, 11, 18)
            .and_hms_milli_opt(12, 34, 56, 789)
            .unwrap();
        let expected = f64::from(46_344) + f64::from(45_296_789) / f64::from(MS_PER_DAY);
        assert_eq!(stored(&datetime, &workbook).to_bits(), expected.to_bits());
    }

    #[test]
    fn datetime_rounding_carries_into_the_next_day() {
        let workbook = Workbook::new().unwrap();
        let datetime = date(2026, 11, 18)
            .and_hms_nano_opt(23, 59, 59, 999_600_000)
            .unwrap();
        assert_eq!(stored(&datetime, &workbook), 46_345.0);

        let last_moment = date(9999, 12, 31)
            .and_hms_nano_opt(23, 59, 59, 999_600_000)
            .unwrap();
        assert!(matches!(
            last_moment.to_cell_value(&workbook),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn datetime_carry_skips_the_fake_1900_leap_day() {
        let workbook = Workbook::new().unwrap();
        let last_moment_of_february = date(1900, 2, 28)
            .and_hms_nano_opt(23, 59, 59, 999_600_000)
            .unwrap();
        assert_eq!(stored(&last_moment_of_february, &workbook), 61.0);

        let first_day = date(1900, 1, 1)
            .and_hms_nano_opt(23, 59, 59, 999_600_000)
            .unwrap();
        assert_eq!(stored(&first_day, &workbook), 2.0);
    }

    #[test]
    fn utc_datetimes_use_the_utc_wall_clock() {
        let workbook = Workbook::new().unwrap();
        let naive = date(2026, 11, 18)
            .and_hms_milli_opt(12, 34, 56, 789)
            .unwrap();
        let utc = Utc.from_utc_datetime(&naive);
        assert_eq!(
            stored(&utc, &workbook).to_bits(),
            stored(&naive, &workbook).to_bits()
        );
        assert_eq!(utc.naive_utc().year(), 2026);
    }

    #[test]
    fn durations_round_to_milliseconds() {
        let workbook = Workbook::new().unwrap();
        assert_eq!(
            stored(&TimeDelta::milliseconds(-1500), &workbook),
            -1500.0 / 86_400_000.0
        );
        assert_eq!(
            stored(&TimeDelta::microseconds(-600), &workbook),
            -1.0 / 86_400_000.0
        );
        assert_eq!(stored(&TimeDelta::microseconds(-400), &workbook), 0.0);
        assert_eq!(stored(&TimeDelta::hours(6), &workbook), 0.25);
    }

    #[test]
    fn out_of_range_dates_are_rejected() {
        let workbook = Workbook::new().unwrap();
        for date in [date(1899, 12, 31), date(10_000, 1, 1)] {
            assert!(matches!(
                date.to_cell_value(&workbook),
                Err(Error::ValueNotRepresentable(_))
            ));
        }
    }
}
