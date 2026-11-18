use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::prelude::XlWideString;

/// A formatted text run inside a [`RichStr`]; MS-XLSB `2.5.144`.
#[derive(DekuRead, DekuWrite, Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct StrRun {
    /// Zero-based UTF-16 code-unit offset in the containing string.
    pub ich: u16,
    /// Index of a `BrtFont` record.
    pub ifnt: u16,
}

/// A phonetic text run inside a [`RichStr`]; MS-XLSB `2.5.103`.
#[derive(DekuRead, DekuWrite, Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct PhRun {
    pub ichFirst: u16,
    pub ichMom: u16,
    pub cchMom: u16,
    pub ifnt: u16,
    /// `phType` in bits 0..2, `alcH` in bits 2..4, remaining bits zero.
    pub flags: u16,
}

/// Formatted runs carried by [`RichStr`].
#[derive(DekuRead, DekuWrite, Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct StrRuns {
    pub dwSizeStrRun: u32,
    #[deku(count = "dwSizeStrRun")]
    pub rgsStrRun: Vec<StrRun>,
}

/// Phonetic runs carried by [`RichStr`].
#[derive(DekuRead, DekuWrite, Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct PhoneticRuns {
    pub phoneticStr: XlWideString,
    pub dwPhoneticRun: u32,
    #[deku(count = "dwPhoneticRun")]
    pub rgsPhRun: Vec<PhRun>,
}

/// A rich string with optional formatted and phonetic runs; MS-XLSB `2.5.122`.
#[derive(DekuRead, DekuWrite, Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct RichStr {
    /// Bit 0 `fRichStr`, bit 1 `fExtStr`, bits 2..8 reserved.
    pub flags: u8,
    pub str: XlWideString,
    #[deku(cond = "(flags & 0x01) != 0", default = "None")]
    pub rich: Option<StrRuns>,
    #[deku(cond = "(flags & 0x02) != 0", default = "None")]
    pub phonetic: Option<PhoneticRuns>,
}

impl RichStr {
    /// `fRichStr`: formatted runs are present.
    pub const F_RICH_STR: u8 = 0x01;
    /// `fExtStr`: phonetic runs are present.
    pub const F_EXT_STR: u8 = 0x02;

    /// Returns whether `dwSizeStrRun` and `rgsStrRun` are present.
    #[must_use]
    pub const fn has_rich_runs(&self) -> bool {
        self.flags & Self::F_RICH_STR != 0
    }

    /// Returns whether the phonetic fields are present.
    #[must_use]
    pub const fn has_phonetic_runs(&self) -> bool {
        self.flags & Self::F_EXT_STR != 0
    }
}

impl Checked for RichStr {
    fn check(&self) -> Result<(), &'static str> {
        if self.flags & !(Self::F_RICH_STR | Self::F_EXT_STR) != 0 {
            return Err("rich string flags contain reserved bits");
        }
        if self.str.len() > 0x7FFF {
            return Err("rich string exceeds 32767 UTF-16 code units");
        }

        if self.has_rich_runs() {
            let Some(rich) = &self.rich else {
                return Err("rich string has fRichStr without formatting runs");
            };
            if rich.dwSizeStrRun > 0x7FFF {
                return Err("rich string has too many formatting runs");
            }
            if usize::try_from(rich.dwSizeStrRun).ok() != Some(rich.rgsStrRun.len()) {
                return Err("rich string formatting-run count does not match its array");
            }
            let mut last = None;
            for run in &rich.rgsStrRun {
                if u32::from(run.ich) >= self.str.len() {
                    return Err("rich string formatting-run offset is outside the string");
                }
                if last.is_some_and(|previous| run.ich <= previous) {
                    return Err("rich string formatting-run offsets must increase");
                }
                last = Some(run.ich);
            }
        } else if self.rich.is_some() {
            return Err("rich string has formatting runs without fRichStr");
        }

        if self.has_phonetic_runs() {
            let Some(phonetic) = &self.phonetic else {
                return Err("rich string has fExtStr without phonetic runs");
            };
            if phonetic.dwPhoneticRun > 0x7FFF {
                return Err("rich string has too many phonetic runs");
            }
            if usize::try_from(phonetic.dwPhoneticRun).ok() != Some(phonetic.rgsPhRun.len()) {
                return Err("rich string phonetic-run count does not match its array");
            }
            let mut last_first = None;
            let mut last_mom = None;
            let mut total_cch_mom = 0_u32;
            for run in &phonetic.rgsPhRun {
                if u32::from(run.ichFirst) >= phonetic.phoneticStr.len() {
                    return Err("rich string phonetic-run offset is outside the phonetic text");
                }
                if last_first.is_some_and(|previous| run.ichFirst <= previous) {
                    return Err("rich string phonetic-run text offsets must increase");
                }
                if u32::from(run.ichMom) >= self.str.len() {
                    return Err("rich string phonetic-run parent offset is outside the string");
                }
                if last_mom.is_some_and(|previous| run.ichMom <= previous) {
                    return Err("rich string phonetic-run parent offsets must increase");
                }
                if u32::from(run.cchMom) > self.str.len() {
                    return Err("rich string phonetic-run parent length is outside the string");
                }
                total_cch_mom += u32::from(run.cchMom);
                last_first = Some(run.ichFirst);
                last_mom = Some(run.ichMom);
            }
            if total_cch_mom > self.str.len() {
                return Err("rich string phonetic runs cover more than the string");
            }
        } else if self.phonetic.is_some() {
            return Err("rich string has phonetic data without fExtStr");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use crate::Checked;
    use crate::prelude::XlStringExt;

    use super::{PhRun, PhoneticRuns, RichStr, StrRun, StrRuns, XlWideString};

    #[test]
    fn rich_string_roundtrips_runs_and_phonetic_runs() {
        let record = RichStr {
            flags: RichStr::F_RICH_STR | RichStr::F_EXT_STR,
            str: XlWideString::construct(Some("あい")),
            rich: Some(StrRuns {
                dwSizeStrRun: 2,
                rgsStrRun: vec![StrRun { ich: 0, ifnt: 0 }, StrRun { ich: 1, ifnt: 2 }],
            }),
            phonetic: Some(PhoneticRuns {
                phoneticStr: XlWideString::construct(Some("ai")),
                dwPhoneticRun: 1,
                rgsPhRun: vec![PhRun {
                    ichFirst: 0,
                    ichMom: 0,
                    cchMom: 2,
                    ifnt: 1,
                    flags: 0x0001,
                }],
            }),
        };

        let bytes = record.to_bytes().expect("serialize rich string");
        assert_eq!(
            bytes,
            [
                0x03, 2, 0, 0, 0, 0x42, 0x30, 0x44, 0x30, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0,
                0, 0, 0x61, 0, 0x69, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 1, 0,
            ]
        );
        let (rest, decoded) = RichStr::from_bytes((&bytes, 0)).expect("deserialize rich string");

        assert_eq!(rest.0, []);
        assert_eq!(decoded, record);
        record.check().expect("valid rich string");
        decoded.check().expect("decoded rich string is valid");
    }

    #[test]
    fn rich_string_rejects_unordered_phonetic_runs() {
        let record = RichStr {
            flags: RichStr::F_RICH_STR | RichStr::F_EXT_STR,
            str: XlWideString::construct(Some("abc")),
            rich: Some(StrRuns {
                dwSizeStrRun: 1,
                rgsStrRun: vec![StrRun { ich: 0, ifnt: 0 }],
            }),
            phonetic: Some(PhoneticRuns {
                phoneticStr: XlWideString::construct(Some("ab")),
                dwPhoneticRun: 2,
                rgsPhRun: vec![
                    PhRun {
                        ichFirst: 1,
                        ichMom: 0,
                        cchMom: 1,
                        ifnt: 0,
                        flags: 0,
                    },
                    PhRun {
                        ichFirst: 1,
                        ichMom: 1,
                        cchMom: 1,
                        ifnt: 0,
                        flags: 0,
                    },
                ],
            }),
        };

        assert!(record.check().is_err());

        let overrun = RichStr {
            flags: RichStr::F_RICH_STR | RichStr::F_EXT_STR,
            str: XlWideString::construct(Some("abc")),
            rich: Some(StrRuns {
                dwSizeStrRun: 1,
                rgsStrRun: vec![StrRun { ich: 0, ifnt: 0 }],
            }),
            phonetic: Some(PhoneticRuns {
                phoneticStr: XlWideString::construct(Some("ab")),
                dwPhoneticRun: 2,
                rgsPhRun: vec![
                    PhRun {
                        ichFirst: 0,
                        ichMom: 0,
                        cchMom: 2,
                        ifnt: 0,
                        flags: 0,
                    },
                    PhRun {
                        ichFirst: 1,
                        ichMom: 1,
                        cchMom: 2,
                        ifnt: 0,
                        flags: 0,
                    },
                ],
            }),
        };

        assert!(overrun.check().is_err());
    }

    #[test]
    fn rich_string_rejects_unordered_runs() {
        let record = RichStr {
            flags: RichStr::F_RICH_STR,
            str: XlWideString::construct(Some("abc")),
            rich: Some(StrRuns {
                dwSizeStrRun: 2,
                rgsStrRun: vec![StrRun { ich: 1, ifnt: 0 }, StrRun { ich: 1, ifnt: 1 }],
            }),
            ..RichStr::default()
        };

        assert!(record.check().is_err());
    }
}
