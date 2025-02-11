use std::fmt::{Debug, Formatter, Write};

use super::{BrtColor, utf16_read, utf16_write};
use crate::util::{raw_bit_read, raw_bit_write};
use bitflags::{Flags, bitflags};
use deku::{DekuRead, DekuWrite, ctx::BitSize};

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u16")]
#[repr(u16)]
pub(crate) enum ScriptStyle {
    #[deku(id = 0x0000, default)]
    #[default]
    None = 0x0000,
    #[deku(id = 0x0001)]
    Sub = 0x0001,
    #[deku(id = 0x0002)]
    Sup = 0x0002,
}

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8")]
#[repr(u8)]
pub(crate) enum Underline {
    #[deku(id = 0x00, default)]
    #[default]
    None = 0x00,
    #[deku(id = 0x01)]
    Single = 0x01,
    #[deku(id = 0x02)]
    Double = 0x02,
    #[deku(id = 0x21)]
    SingleAccounting = 0x21,
    #[deku(id = 0x22)]
    DoubleAccounting = 0x22,
}

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8")]
#[repr(u8)]
pub(crate) enum FontFamily {
    #[deku(id = 0x00, default)]
    #[default]
    NotApplicable = 0x00,
    #[deku(id = 0x01)]
    Roman = 0x01,
    #[deku(id = 0x02)]
    Swiss = 0x02,
    #[deku(id = 0x03)]
    Modern = 0x03,
    #[deku(id = 0x04)]
    Script = 0x04,
    #[deku(id = 0x05)]
    Decorative = 0x05,
}

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8")]
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
pub(crate) enum CharSet {
    #[deku(id = 0x00)]
    ANSI = 0x00,
    #[deku(id = 0x01, default)]
    #[default]
    Default = 0x01,
    #[deku(id = 0x02)]
    Symbol = 0x02,
    #[deku(id = 0x4d)]
    MAC = 0x4d,
    #[deku(id = 0x80)]
    ShiftJIS = 0x80,
    #[deku(id = 0x81)]
    Hangul = 0x81,
    #[deku(id = 0x82)]
    Johab = 0x82,
    #[deku(id = 0x86)]
    GB2312 = 0x86,
    #[deku(id = 0x88)]
    ChineseBig5 = 0x88,
    #[deku(id = 0xa1)]
    Greek = 0xa1,
    #[deku(id = 0xa2)]
    Turkish = 0xa2,
    #[deku(id = 0xa3)]
    Vietnamese = 0xa3,
    #[deku(id = 0xb1)]
    Hebrew = 0xb1,
    #[deku(id = 0xb2)]
    Arabic = 0xb2,
    #[deku(id = 0xba)]
    Baltic = 0xba,
    #[deku(id = 0xcc)]
    Russian = 0xcc,
    #[deku(id = 0xde)]
    Thai = 0xde,
    #[deku(id = 0xee)]
    EastEurope = 0xee,
    #[deku(id = 0xff)]
    OEM = 0xff,
}

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8")]
#[repr(u8)]
pub(crate) enum FontScheme {
    #[deku(id = 0, default)]
    #[default]
    None = 0x00,
    #[deku(id = 1)]
    Major = 0x01,
    #[deku(id = 2)]
    Minor = 0x02,
}

#[derive(DekuRead, DekuWrite, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u16")]
#[repr(u16)]
pub(crate) enum Boldness {
    #[deku(id = 0x0190, default)]
    #[default]
    Normal = 0x0190,
    #[deku(id = 0x02BC)]
    Bold = 0x02BC,
}

bitflags! {
    #[derive(Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
    pub(crate) struct FontFlags: u16 {
        // padding (le) ----> vvvv-vvvv
        const fItalic    = 0b_0000_0000_0000_0010;
        const fStrikeOut = 0b_0000_0000_0000_1000;
        const fOutline   = 0b_0000_0000_0001_0000;
        const fShadow    = 0b_0000_0000_0010_0000;
        const fCondense  = 0b_0000_0000_0100_0000;
        const fExtend    = 0b_0000_0000_1000_0000;
    }
}

#[derive(DekuRead, DekuWrite, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) struct BrtFont {
    dyHeight: u16,
    #[deku(
        reader = "raw_bit_read(deku::reader, None)",
        writer = "raw_bit_write(grbit, deku::writer, None)"
    )]
    grbit: FontFlags,
    bls: Boldness,
    sss: ScriptStyle,
    uls: Underline,
    bFamily: FontFamily,
    bCharSet: CharSet,
    #[deku(pad_bytes_before = "1")]
    brtColor: BrtColor,
    bFontScheme: FontScheme,

    #[deku(
        reader = "utf16_read(deku::reader)",
        writer = "utf16_write(name, deku::writer)"
    )]
    name: String,
}

impl Debug for BrtFont {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut i = 0;
        f.write_str("Font[")?;
        if self.dyHeight != 220 {
            f.write_fmt(format_args!("Height:{}", self.dyHeight))?;
            i += 1;
        }
        if self.grbit != FontFlags::empty() {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Flags:{:?}", self.grbit))?;
            i += 1;
        }
        if self.bls != Boldness::Normal {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_str("bold")?;
            i += 1;
        }
        if self.sss != ScriptStyle::None {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Script:{:?}", self.sss))?;
            i += 1;
        }
        if self.uls != Underline::None {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Underline:{:?}", self.uls))?;
            i += 1;
        }
        if self.bFamily != FontFamily::Swiss {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Family:{:?}", self.bFamily))?;
            i += 1;
        }
        if self.bCharSet != CharSet::Russian {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("CharSet:{:?}", self.bCharSet))?;
            i += 1;
        }
        if self.brtColor != BrtColor::default() {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Color:{:?}", self.brtColor))?;
            i += 1;
        }
        if self.bFontScheme != FontScheme::Minor {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Scheme:{:?}", self.bFontScheme))?;
            i += 1;
        }
        if !self.name.is_empty() {
            if i > 0 {
                f.write_char(',')?;
            }
            f.write_fmt(format_args!("Name:{}", self.name))?;
        }
        f.write_str("]")?;
        Ok(())
    }
}

impl Default for BrtFont {
    fn default() -> Self {
        Self {
            dyHeight: 220,
            grbit: FontFlags::empty(),
            bls: Boldness::Normal,
            sss: ScriptStyle::None,
            uls: Underline::None,
            bFamily: FontFamily::Swiss,
            bCharSet: CharSet::Russian,
            brtColor: BrtColor::default(),
            bFontScheme: FontScheme::Minor,
            name: "Calibri".to_owned(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct FontBuilder {
    inner: BrtFont,
}

impl From<BrtFont> for FontBuilder {
    fn from(value: BrtFont) -> Self {
        FontBuilder { inner: value }
    }
}

impl From<FontBuilder> for BrtFont {
    fn from(value: FontBuilder) -> Self {
        value.inner
    }
}

impl FontBuilder {
    pub fn set_italic(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fItalic, v);
        self
    }

    pub fn set_strike_out(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fStrikeOut, v);
        self
    }

    pub fn set_outline(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fOutline, v);
        self
    }

    pub fn set_shadow(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fShadow, v);
        self
    }

    pub fn set_condense(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fCondense, v);
        self
    }

    pub fn set_extend(&mut self, v: bool) -> &mut Self {
        self.inner.grbit.set(FontFlags::fExtend, v);
        self
    }

    pub fn set_bold(&mut self, v: bool) -> &mut Self {
        self.inner.bls = if v { Boldness::Bold } else { Boldness::Normal };
        self
    }

    pub fn set_subscript(&mut self, v: bool) -> &mut Self {
        self.inner.sss = if v {
            ScriptStyle::Sub
        } else if matches!(self.inner.sss, ScriptStyle::Sub) {
            ScriptStyle::None
        } else {
            return self;
        };
        self
    }

    pub fn set_supscript(&mut self, v: bool) -> &mut Self {
        self.inner.sss = if v {
            ScriptStyle::Sup
        } else if matches!(self.inner.sss, ScriptStyle::Sup) {
            ScriptStyle::None
        } else {
            return self;
        };
        self
    }

    pub fn set_underline(&mut self, u: Underline) -> &mut Self {
        self.inner.uls = u;
        self
    }

    pub fn set_font_family(&mut self, ff: FontFamily) -> &mut Self {
        self.inner.bFamily = ff;
        self
    }

    pub fn set_charset(&mut self, c: CharSet) -> &mut Self {
        self.inner.bCharSet = c;
        self
    }

    pub fn set_font_scheme(&mut self, fs: FontScheme) -> &mut Self {
        self.inner.bFontScheme = fs;
        self
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.inner.name = name.to_owned();
        self
    }

    pub fn set_color(&mut self, color: BrtColor) -> &mut Self {
        self.inner.brtColor = color;
        self
    }

    pub fn set_height(&mut self, twips: u16) -> &mut Self {
        self.inner.dyHeight = twips;
        self
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    #[test]
    fn test_enum() {
        println!("{:?}", BrtFont::default().to_bytes());
    }

    #[test]
    fn test_from_static() {
        let data: [u8; 39] = [
            0xDC, 0x00, 0x00, 0x00, 0x90, 0x01, 0x00, 0x00, 0x00, 0x02, 0xCC, 0x00, 0x07, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x02, 0x07, 0x00, 0x00, 0x00, 0x43, 0x00, 0x61,
            0x00, 0x6C, 0x00, 0x69, 0x00, 0x62, 0x00, 0x72, 0x00, 0x69, 0x00,
        ];

        let font = BrtFont::from_bytes((data.as_ref(), 0))
            .expect("Malformed BrtFont")
            .1;
        println!("{:?}", font);
    }
}
