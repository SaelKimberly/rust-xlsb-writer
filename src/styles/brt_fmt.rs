use std::{fmt::Display, mem::transmute};

use super::{utf16_read, utf16_write};
use deku::{DekuRead, DekuWrite};
use zerocopy::transmute;

#[derive(Clone, Debug)]
#[repr(u16)]
pub(crate) enum BuiltinFmt {
    /// "General"
    General = 0x0000,
    /// "0"
    Integer = 0x0001,
    /// "0.00"
    Decimal = 0x0002,
    /// "#,##0"
    PrettyInteger = 0x0003,
    /// "#,##0.00"
    PrettyDecimal = 0x0004,
    /// "0%"
    PercentInteger = 0x0009,
    /// "0.00%"
    PercentDecimal = 0x000a,
    /// "0.00E+00"
    Scientific = 0x000b,
    /// "# ?/?"
    SingleFraction = 0x000c,
    /// "# ??/??"
    DoubleFraction = 0x000d,
    /// "mm-dd-yy"
    Date = 0x000e,
    /// "d-mmm-yy"
    DateMonthVerbose = 0x000f,
    /// "d-mmm"
    DateMonthVerboseNoYear = 0x0010,
    /// "mmm-yy"
    DateMonthVerboseNoDay = 0x0011,
    /// "h:mm AM/PM"
    TimeMinutesWithAmPm = 0x0012,
    /// "h:mm:ss AM/PM"
    TimeSecondsWithAmPm = 0x0013,
    /// "h:mm"
    TimeMinutes = 0x0014,
    /// "h:mm:ss"
    TimeSeconds = 0x0015,
    /// "m/d/yy h:mm"
    DateTimeMinutes = 0x0016,
    /// "#,##0 ;(#,##0)"
    NegativePrettyInteger = 0x0025,
    /// "#,##0 ;[Red](#,##0)"
    NegativeRedPrettyInteger = 0x0026,
    /// "#,##0.00 ;(#,##0.00)"
    NegativePrettyDecimal = 0x0027,
    /// "#,##0.00 ;[Red](#,##0.00)"
    NegativeRedPrettyDecimal = 0x0028,
    /// "mm:ss"
    DurationSeconds = 0x002d,
    /// "[h]:mm:ss"
    DurationOptionalHour = 0x002e,
    /// "mmss.0"
    DurationMilliseconds = 0x002f,
    /// "##0.0E+0"
    PrettyScientific = 0x0030,
    /// "@"
    Text = 0x0031,
    /// Custom format string, declared by user
    Custom = 0xffff,
}

impl BuiltinFmt {
    pub(crate) const fn fmt_code(&self) -> &'static str {
        match self {
            BuiltinFmt::General => "General",
            BuiltinFmt::Integer => "0",
            BuiltinFmt::Decimal => "0.00",
            BuiltinFmt::PrettyInteger => "#,##0",
            BuiltinFmt::PrettyDecimal => "#,##0.00",
            BuiltinFmt::PercentInteger => "0%",
            BuiltinFmt::PercentDecimal => "0.00%",
            BuiltinFmt::Scientific => "0.00E+00",
            BuiltinFmt::SingleFraction => "# ?/?",
            BuiltinFmt::DoubleFraction => "# ??/??",
            BuiltinFmt::Date => "mm-dd-yy",
            BuiltinFmt::DateMonthVerbose => "d-mmm-yy",
            BuiltinFmt::DateMonthVerboseNoYear => "d-mmm",
            BuiltinFmt::DateMonthVerboseNoDay => "mmm-yy",
            BuiltinFmt::TimeMinutesWithAmPm => "h:mm AM/PM",
            BuiltinFmt::TimeSecondsWithAmPm => "h:mm:ss AM/PM",
            BuiltinFmt::TimeMinutes => "h:mm",
            // TODO
            BuiltinFmt::TimeSeconds => "h:mm:ss",
            BuiltinFmt::DateTimeMinutes => "m/d/yy h:mm",
            BuiltinFmt::NegativePrettyInteger => "#,##0 ;(#,##0)",
            BuiltinFmt::NegativeRedPrettyInteger => "#,##0 ;[Red](#,##0)",
            BuiltinFmt::NegativePrettyDecimal => "#,##0.00;(#,##0.00)",
            BuiltinFmt::NegativeRedPrettyDecimal => "#,##0.00;[Red](#,##0.00)",
            BuiltinFmt::DurationSeconds => "mm:ss",
            BuiltinFmt::DurationOptionalHour => "[h]:mm:ss",
            BuiltinFmt::DurationMilliseconds => "mmss.0",
            BuiltinFmt::PrettyScientific => "##0.0E+0",
            BuiltinFmt::Text => "@",
            BuiltinFmt::Custom => "Custom",
        }
    }
}

impl Display for BuiltinFmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_code())
    }
}

#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) struct BrtFmt {
    ifmt: u16,
    #[deku(
        reader = "utf16_read(deku::reader)",
        writer = "utf16_write(stFmtCode, deku::writer)"
    )]
    stFmtCode: String,
}

impl From<BuiltinFmt> for BrtFmt {
    fn from(fmt: BuiltinFmt) -> Self {
        Self {
            ifmt: fmt.clone() as u16,
            stFmtCode: fmt.fmt_code().to_string(),
        }
    }
}
impl Default for BrtFmt {
    fn default() -> Self {
        Self::from(BuiltinFmt::General)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brt_fmt() {
        let fmt = BrtFmt::from(BuiltinFmt::General);
        assert_eq!(fmt.ifmt, 0);
        assert_eq!(fmt.stFmtCode, "General");
        println!("{:?}", fmt);
    }
}
