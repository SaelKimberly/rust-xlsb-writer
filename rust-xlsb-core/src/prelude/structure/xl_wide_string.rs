use std::{
    fmt::{Debug, Display},
    ops::Deref,
    str::FromStr,
};

use deku::{DekuRead, DekuWrite};
use zerocopy::IntoBytes;

use crate::Error;

#[derive(Clone, PartialEq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlString {
    #[deku(update = "self.str.len() as u32 / 2")]
    len: u32,
    #[deku(count = "if *len != 0xffff_ffff { len * 2 } else { 0 }")]
    str: Box<[u8]>,
}

impl FromStr for XlString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let o: Box<[u16]> = s.encode_utf16().collect::<Box<[_]>>();
        Ok(Self {
            len: s.len() as u32,
            str: Box::from(o.as_bytes()),
        })
    }
}

impl XlString {
    pub const fn is_null(&self) -> bool {
        self.len == 0xffffffff
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.len, 0 | 0xffffffff)
    }

    pub fn new_null() -> Self {
        Self {
            len: 0xffffffff,
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

    pub fn from_string<T: Deref<Target = str>>(s: T) -> Self {
        let s = s.deref();
        let o = s.encode_utf16().collect::<Box<[_]>>();
        Self {
            len: s.len() as u32,
            str: Box::from(o.as_bytes()),
        }
    }

    pub fn to_optional_string(&self) -> Option<String> {
        match self.len {
            0xffff_ffff => None,
            0 => Some("".to_owned()),
            _ => Some(String::from_utf16le(&self.str).unwrap()),
        }
    }

    pub fn try_to_string(&self) -> Result<String, Error> {
        match self.to_optional_string() {
            Some(s) => Ok(s),
            None => Err(Error::ValidationError(
                "Null XlWideString is not allowed".to_owned(),
            )),
        }
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
        let new_inner = match s {
            Some(s) => XlString::from_string(s),
            None => XlString::new_null(),
        };
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

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlNullableWideString {
    inner: XlString,
}

declare_xl_string!(XlNullableWideString, true);

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct XlWideString {
    #[deku(assert = "!inner.is_null()")]
    inner: XlString,
}

declare_xl_string!(XlWideString);

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite, std::hash::Hash)]
pub struct CodeName {
    #[deku(assert = "matches!(inner.len(), 0..=31)")]
    inner: XlString,
}

declare_xl_string!(CodeName);

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite, std::hash::Hash)]
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

    use super::XlWideString;

    #[test]
    fn test_xl_wide_string() {
        let xlws = XlWideString::construct(Some("abc"));
        assert_eq!(xlws.len(), 3);

        println!("{:?}", xlws);

        let data = xlws.to_bytes().expect("Should be ok");
        println!("Data: {:?}", data);

        let (_, parsed) = XlWideString::from_bytes((&data, 0)).expect("Should be ok");

        assert_eq!(xlws, parsed);

        let str = parsed.try_to_string().expect("Should be ok");
        assert_eq!(&str, "abc");

        let r = XlWideString::from_bytes((&[0xff, 0xff, 0xff, 0xff], 0));
        assert!(r.is_err());

        let x = XlWideString::new(XlString {
            len: 0xffff_ffff,
            str: Box::new([]),
        });
        println!("{:?}", x);

        assert!(x.is_null());

        let r = x.to_bytes();
        assert!(r.is_err());
    }
}
