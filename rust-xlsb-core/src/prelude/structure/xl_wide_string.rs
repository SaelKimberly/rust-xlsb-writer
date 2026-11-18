#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec length field is 32-bit; deku expands the cast into a generated impl"
)]

use std::{
    fmt::{Debug, Display},
    ops::Deref,
    str::FromStr,
};

use deku::{DekuRead, DekuWrite};
use zerocopy::IntoBytes;

use crate::Error;

#[derive(Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlString {
    #[deku(update = "self.str.len() as u32 / 2")]
    len: u32,
    #[deku(count = "if *len == 0xffff_ffff { 0 } else { len * 2 }")]
    str: Box<[u8]>,
}

impl FromStr for XlString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let o: Box<[u16]> = s.encode_utf16().collect::<Box<[_]>>();
        Ok(Self {
            len: o.len() as u32,
            str: Box::from(o.as_bytes()),
        })
    }
}

impl XlString {
    pub const fn is_null(&self) -> bool {
        self.len == 0xffff_ffff
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.len, 0 | 0xffff_ffff)
    }

    pub fn new_null() -> Self {
        Self {
            len: 0xffff_ffff,
            str: Box::new([]),
        }
    }
    pub fn new_empty() -> Self {
        Self {
            len: 0,
            str: Box::new([]),
        }
    }

    pub const fn len(&self) -> u32 {
        self.len
    }

    /// Returns the UTF-16LE bytes without the length prefix.
    #[must_use]
    pub fn utf16_bytes(&self) -> &[u8] {
        &self.str
    }

    pub fn from_string<T: Deref<Target = str>>(s: T) -> Self {
        let s = &*s;
        let o = s.encode_utf16().collect::<Box<[_]>>();
        Self {
            len: o.len() as u32,
            str: Box::from(o.as_bytes()),
        }
    }

    pub fn to_optional_string(&self) -> Option<String> {
        match self.len {
            0xffff_ffff => None,
            0 => Some(String::new()),
            _ => Some(String::from_utf16le(&self.str).unwrap()),
        }
    }

    pub fn try_to_string(&self) -> Result<String, Error> {
        self.to_optional_string()
            .ok_or_else(|| Error::ValidationError("Null XlWideString is not allowed".to_owned()))
    }
}

impl Debug for XlString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XlString[{:?}]", self.to_optional_string())
    }
}

impl Display for XlString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_optional_string())
    }
}

pub trait XlStringExt: Deref<Target = XlString> + Sized {
    fn new(xls: XlString) -> Self;

    fn construct<T: Deref<Target = str>>(s: Option<T>) -> Self {
        let new_inner = s.map_or_else(XlString::new_null, XlString::from_string);
        Self::new(new_inner)
    }
}

macro_rules! declare_xl_string {
    ($name: ident) => {
        impl Deref for $name {
            type Target = XlString;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl XlStringExt for $name {
            fn new(xls: XlString) -> Self {
                Self { inner: xls }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    inner: XlString::new_empty(),
                }
            }
        }

        impl crate::Unchecked for $name {}
    };

    ($name: ident, $nullable: literal) => {
        impl Deref for $name {
            type Target = XlString;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl XlStringExt for $name {
            fn new(xls: XlString) -> Self {
                Self { inner: xls }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    inner: XlString::new_null(),
                }
            }
        }

        impl crate::Unchecked for $name {}
    };
}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlNullableWideString {
    inner: XlString,
}

declare_xl_string!(XlNullableWideString, true);

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlWideString {
    #[deku(assert = "!inner.is_null()")]
    inner: XlString,
}

declare_xl_string!(XlWideString);

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct CodeName {
    #[deku(assert = "matches!(inner.len(), 0..=31)")]
    inner: XlString,
}

declare_xl_string!(CodeName);

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct HeaderFooterString {
    #[deku(assert = "inner.is_null() || inner.len() <= 255")]
    inner: XlString,
}

declare_xl_string!(HeaderFooterString, true);

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct SimpleRichStr {
    #[deku(pad_bytes_before = "1")]
    #[deku(assert = "!inner.is_null()")]
    inner: XlString,
}

declare_xl_string!(SimpleRichStr);

#[cfg(test)]
mod tests {

    use deku::{DekuContainerRead, DekuContainerWrite};

    use crate::prelude::structure::xl_wide_string::{XlString, XlStringExt};

    use super::{HeaderFooterString, XlWideString};

    #[test]
    fn test_xl_wide_string() {
        let xlws = XlWideString::construct(Some("abc"));
        assert_eq!(xlws.len(), 3);

        println!("{xlws:?}");

        let data = xlws.to_bytes().expect("Should be ok");
        println!("Data: {data:?}");

        let (_, parsed) = XlWideString::from_bytes((&data, 0)).expect("Should be ok");

        assert_eq!(xlws, parsed);

        let str = parsed.try_to_string().expect("Should be ok");
        assert_eq!(&str, "abc");

        let non_ascii = XlWideString::construct(Some("привет"));
        assert_eq!(non_ascii.len(), 6);
        let data = non_ascii.to_bytes().expect("Should be ok");
        assert_eq!(data.len(), 4 + 6 * 2);
        let (_, parsed) = XlWideString::from_bytes((&data, 0)).expect("Should be ok");
        assert_eq!(parsed.try_to_string().expect("Should be ok"), "привет");

        let r = XlWideString::from_bytes((&[0xff, 0xff, 0xff, 0xff], 0));
        assert!(r.is_err());

        let x = XlWideString::new(XlString {
            len: 0xffff_ffff,
            str: Box::new([]),
        });
        println!("{x:?}");

        assert!(x.is_null());

        let r = x.to_bytes();
        assert!(r.is_err());
    }

    #[test]
    fn header_footer_string_limits_utf16_units() {
        let max = HeaderFooterString::construct(Some("x".repeat(255)));
        assert_eq!(max.try_to_string().expect("valid").len(), 255);
        assert!(max.to_bytes().is_ok());

        let too_long = HeaderFooterString::construct(Some("x".repeat(256)));
        assert!(too_long.to_bytes().is_err());

        let null = HeaderFooterString::default();
        assert!(null.is_null());
        assert!(null.to_bytes().is_ok());

        let non_ascii = HeaderFooterString::construct(Some("Привет 😀"));
        let bytes = non_ascii.to_bytes().expect("serializes");
        let (_, parsed) = HeaderFooterString::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(parsed.try_to_string().expect("valid"), "Привет 😀");
    }

    #[test]
    fn header_footer_string_counts_surrogate_pairs_and_rejects_long_decodes() {
        let maximum = HeaderFooterString::construct(Some("😀".repeat(127).as_str()));
        assert_eq!(maximum.len(), 254);
        assert!(maximum.to_bytes().is_ok());

        let too_long = HeaderFooterString::construct(Some("😀".repeat(128).as_str()));
        assert_eq!(too_long.len(), 256);
        assert!(too_long.to_bytes().is_err());

        let mut bytes = vec![0_u8; 4 + 256 * 2];
        bytes[..4].copy_from_slice(&256_u32.to_le_bytes());
        assert!(HeaderFooterString::from_bytes((&bytes, 0)).is_err());
    }
}
