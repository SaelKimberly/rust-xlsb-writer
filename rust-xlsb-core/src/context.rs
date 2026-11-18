//! Workbook context: the shared-string table plus the numeric conversion policy.

use crate::prelude::RkNumber;
use crate::{Error, Result, SSTHolder};

/// A reference to one shared string.
///
/// The value carries no workbook handle: the same object may be written into
/// any workbook whose shared-string table contains [`SharedString::id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedString {
    id: u32,
}

impl SharedString {
    /// Zero-based index of the string in the workbook shared-string table.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }
}

/// The record-level encoding chosen for a numeric cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberEncoding {
    /// `BrtCellReal`, eight bytes.
    Real(f64),
    /// `BrtCellRk`, four bytes.
    Rk(RkNumber),
}

/// Which compact number encodings a workbook accepts.
///
/// Every enabled encoding is lossless; the policy only trades bytes for a
/// cheaper or more verbose record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberPolicy {
    /// Store integral values in the signed 30-bit range as Rk integers.
    pub integers: bool,
    /// Store values whose hundredfold is a 30-bit integer as Rk percentages.
    pub percents: bool,
    /// Store doubles whose low 34 bits are zero as Rk floats.
    pub rk_floats: bool,
}

impl Default for NumberPolicy {
    fn default() -> Self {
        Self {
            integers: true,
            percents: true,
            rk_floats: true,
        }
    }
}

impl NumberPolicy {
    /// Encodes a double without losing a bit.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both casts are guarded by a signed 30-bit range check"
    )]
    pub fn encode_f64(&self, value: f64) -> NumberEncoding {
        if !value.is_finite() || (value == 0.0 && value.is_sign_negative()) {
            return NumberEncoding::Real(value);
        }

        if self.integers
            && value.fract() == 0.0
            && (-536_870_912.0..=536_870_911.0).contains(&value)
        {
            let int = value as i32;
            let rk = RkNumber::int(int).expect("range checked above");
            return NumberEncoding::Rk(rk);
        }

        if self.percents {
            let scaled = (value * 100.0).round();
            if (-536_870_912.0..=536_870_911.0).contains(&scaled)
                && (scaled / 100.0).to_bits() == value.to_bits()
            {
                let int = scaled as i32;
                let rk = RkNumber::percent(int).expect("range checked above");
                return NumberEncoding::Rk(rk);
            }
        }

        if self.rk_floats
            && let Some(rk) = RkNumber::float(value)
        {
            return NumberEncoding::Rk(rk);
        }

        NumberEncoding::Real(value)
    }
}

/// Everything a value converter needs from the workbook it is written into.
pub trait WbContext {
    /// The workbook's shared-string table.
    fn shared_strings(&self) -> &SSTHolder;

    /// Encoding policy for numbers. Defaults to [`NumberPolicy::default`].
    fn number_policy(&self) -> NumberPolicy {
        NumberPolicy::default()
    }

    /// Interns `value` and returns its shared-string handle.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sst`] if the shared-string table rejects the insert.
    fn shared_string(&self, value: &str) -> Result<SharedString> {
        let id = self.shared_strings().push(value)?;
        Ok(SharedString { id })
    }

    /// Infallible: every `f64` is representable as `Real`.
    fn store_f64(&self, value: f64) -> NumberEncoding {
        self.number_policy().encode_f64(value)
    }

    /// `f32` widens to `f64` exactly; kept separate for a future f32 policy.
    fn store_f32(&self, value: f32) -> NumberEncoding {
        self.store_f64(f64::from(value))
    }

    /// Encodes an integer without rounding.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValueNotRepresentable`] when the value is outside the
    /// range where `f64` represents every integer exactly (`±2^53`).
    #[expect(
        clippy::cast_precision_loss,
        reason = "the cast is guarded by the exact-integer range of f64"
    )]
    fn store_integer(&self, value: i64) -> Result<NumberEncoding> {
        const RK_MIN: i64 = -536_870_912; // -2^29
        const RK_MAX: i64 = 536_870_911; // 2^29 - 1
        const F64_EXACT: i64 = 9_007_199_254_740_992; // 2^53

        if self.number_policy().integers && (RK_MIN..=RK_MAX).contains(&value) {
            let value = i32::try_from(value).expect("range checked above");
            let rk = RkNumber::int(value).expect("range checked above");
            return Ok(NumberEncoding::Rk(rk));
        }
        if (-F64_EXACT..=F64_EXACT).contains(&value) {
            return Ok(NumberEncoding::Real(value as f64));
        }
        Err(Error::ValueNotRepresentable(
            "integer exceeds f64 precision",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::TestContext;

    fn rk_int(value: i32) -> NumberEncoding {
        NumberEncoding::Rk(RkNumber::int(value).expect("fits"))
    }

    fn rk_percent(value: i32) -> NumberEncoding {
        NumberEncoding::Rk(RkNumber::percent(value).expect("fits"))
    }

    #[test]
    fn encodes_integral_values_as_rk() {
        assert_eq!(NumberPolicy::default().encode_f64(42.0), rk_int(42));
        assert_eq!(NumberPolicy::default().encode_f64(42.5), rk_percent(4250));
    }

    #[test]
    fn keeps_negative_zero_real() {
        match NumberPolicy::default().encode_f64(-0.0) {
            NumberEncoding::Real(value) => {
                assert_eq!(value, 0.0);
                assert!(value.is_sign_negative());
            }
            other @ NumberEncoding::Rk(_) => panic!("expected Real, got {other:?}"),
        }
    }

    #[test]
    fn keeps_non_finite_values_real() {
        assert!(matches!(
            NumberPolicy::default().encode_f64(f64::INFINITY),
            NumberEncoding::Real(value) if value.is_infinite()
        ));
        assert!(matches!(
            NumberPolicy::default().encode_f64(f64::NAN),
            NumberEncoding::Real(value) if value.is_nan()
        ));
    }

    #[test]
    fn rejects_inexact_compact_encodings() {
        assert_eq!(
            NumberPolicy::default().encode_f64(1e-9),
            NumberEncoding::Real(1e-9)
        );
        assert_eq!(
            NumberPolicy::default().encode_f64(std::f64::consts::PI),
            NumberEncoding::Real(std::f64::consts::PI)
        );
    }

    #[test]
    fn policy_switches_disable_compact_encodings() {
        // 42.0 is also an exact hundredth, so the percent rule still applies.
        let no_ints = NumberPolicy {
            integers: false,
            ..NumberPolicy::default()
        };
        assert_eq!(no_ints.encode_f64(42.0), rk_percent(4200));

        let no_percents = NumberPolicy {
            percents: false,
            ..NumberPolicy::default()
        };
        assert_eq!(no_percents.encode_f64(0.1), NumberEncoding::Real(0.1));

        let no_rk_floats = NumberPolicy {
            rk_floats: false,
            ..NumberPolicy::default()
        };
        let value = 2f64.powi(29);
        assert_eq!(no_rk_floats.encode_f64(value), NumberEncoding::Real(value));

        let all_off = NumberPolicy {
            integers: false,
            percents: false,
            rk_floats: false,
        };
        assert_eq!(all_off.encode_f64(42.0), NumberEncoding::Real(42.0));
    }

    #[test]
    fn every_rk_round_trips_bit_exactly() {
        let policy = NumberPolicy::default();
        for value in [
            -2f64.powi(29),
            2f64.powi(29) - 1.0,
            2f64.powi(29),
            0.1,
            1.5,
            42.5,
            -7.25,
            2f64.powi(53),
            0.0,
            -1.0,
            1e-9,
            2f64.powi(-1074),
        ] {
            if let NumberEncoding::Rk(rk) = policy.encode_f64(value) {
                assert_eq!(rk.value().to_bits(), value.to_bits(), "value {value:?}");
            }
        }
    }

    #[test]
    fn store_integer_rejects_values_f64_cannot_hold() {
        let workbook = TestContext::new();

        assert_eq!(workbook.store_integer(42).expect("fits"), rk_int(42));
        assert_eq!(
            workbook.store_integer(2_i64.pow(29)).expect("real"),
            NumberEncoding::Real(536_870_912.0)
        );
        assert_eq!(
            workbook.store_integer(2_i64.pow(53)).expect("real"),
            NumberEncoding::Real(9_007_199_254_740_992.0)
        );
        assert!(matches!(
            workbook.store_integer(i64::MAX),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn context_is_object_safe() {
        fn accepts(_: &dyn WbContext) {}
        let workbook = TestContext::new();
        accepts(&workbook);
    }
}
