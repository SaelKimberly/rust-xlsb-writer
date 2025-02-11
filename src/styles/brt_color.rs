use std::{
    fmt::{Debug, Write},
    mem::transmute,
};

use deku::{DekuRead, DekuWrite};

#[derive(DekuWrite, DekuRead, Default, Debug, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8", bits = 7)]
#[repr(u8)]
#[allow(clippy::upper_case_acronyms)]
pub(crate) enum ColorType {
    #[default]
    #[deku(id = "0x00", default)]
    ApplicationDetermined = 0x00,

    #[deku(id = "0x01")]
    ColorPaletteIndex = 0x01,

    #[deku(id = "0x02")]
    RGBA = 0x02,

    #[deku(id = "0x03")]
    ThemeColorIndex = 0x03,
}

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Clone, Copy, Default, Debug)]
#[repr(u8)]
pub enum BrtColorTheme {
    #[default]
    dk1 = 0x00,
    lt1 = 0x01,
    dk2 = 0x02,
    lt2 = 0x03,
    accent1 = 0x04,
    accent2 = 0x05,
    accent3 = 0x06,
    accent4 = 0x07,
    accent5 = 0x08,
    accent6 = 0x09,
    hlink = 0x0a,
    folHlink = 0x0b,
}

impl From<u8> for BrtColorTheme {
    fn from(value: u8) -> Self {
        macro_rules! matched {
            ($t: ty, $e: ident $(, $e_other: ident)*) => {
                if value == (<$t>::$e as u8) {
                    return <$t>::$e;
                } $(else if value == (<$t>::$e_other as u8) {
                    return <$t>::$e_other;
                })* else {
                    <$t>::default()
                }
            };
        }
        matched!(
            Self, dk1, lt1, dk2, lt2, accent1, accent2, accent3, accent4, accent5, accent6, hlink,
            folHlink
        )
    }
}

#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(DekuRead, DekuWrite, Clone, Copy, Default, Debug)]
#[deku(id_type = "u8")]
pub(crate) enum Icv {
    #[default]
    #[deku(id = 0x00)]
    icvBlack = 0x00,
    #[deku(id = 0x01)]
    icvWhite = 0x01,
    #[deku(id = 0x02)]
    icvRed = 0x02,
    #[deku(id = 0x03)]
    icvGreen = 0x03,
    #[deku(id = 0x04)]
    icvBlue = 0x04,
    #[deku(id = 0x05)]
    icvYellow = 0x05,
    #[deku(id = 0x06)]
    icvMagenta = 0x06,
    #[deku(id = 0x07)]
    icvCyan = 0x07,
    #[deku(id = 0x08)]
    icvPlt1 = 0x08,
    #[deku(id = 0x09)]
    icvPlt2 = 0x09,
    #[deku(id = 0x0a)]
    icvPlt3 = 0x0a,
    #[deku(id = 0x0b)]
    icvPlt4 = 0x0b,
    #[deku(id = 0x0c)]
    icvPlt5 = 0x0c,
    #[deku(id = 0x0d)]
    icvPlt6 = 0x0d,
    #[deku(id = 0x0e)]
    icvPlt7 = 0x0e,
    #[deku(id = 0x0f)]
    icvPlt8 = 0x0f,
    #[deku(id = 0x10)]
    icvPlt9 = 0x10,
    #[deku(id = 0x11)]
    icvPlt10 = 0x11,
    #[deku(id = 0x12)]
    icvPlt11 = 0x12,
    #[deku(id = 0x13)]
    icvPlt12 = 0x13,
    #[deku(id = 0x14)]
    icvPlt13 = 0x14,
    #[deku(id = 0x15)]
    icvPlt14 = 0x15,
    #[deku(id = 0x16)]
    icvPlt15 = 0x16,
    #[deku(id = 0x17)]
    icvPlt16 = 0x17,
    #[deku(id = 0x18)]
    icvPlt17 = 0x18,
    #[deku(id = 0x19)]
    icvPlt18 = 0x19,
    #[deku(id = 0x1a)]
    icvPlt19 = 0x1a,
    #[deku(id = 0x1b)]
    icvPlt20 = 0x1b,
    #[deku(id = 0x1c)]
    icvPlt21 = 0x1c,
    #[deku(id = 0x1d)]
    icvPlt22 = 0x1d,
    #[deku(id = 0x1e)]
    icvPlt23 = 0x1e,
    #[deku(id = 0x1f)]
    icvPlt24 = 0x1f,
    #[deku(id = 0x20)]
    icvPlt25 = 0x20,
    #[deku(id = 0x21)]
    icvPlt26 = 0x21,
    #[deku(id = 0x22)]
    icvPlt27 = 0x22,
    #[deku(id = 0x23)]
    icvPlt28 = 0x23,
    #[deku(id = 0x24)]
    icvPlt29 = 0x24,
    #[deku(id = 0x25)]
    icvPlt30 = 0x25,
    #[deku(id = 0x26)]
    icvPlt31 = 0x26,
    #[deku(id = 0x27)]
    icvPlt32 = 0x27,
    #[deku(id = 0x28)]
    icvPlt33 = 0x28,
    #[deku(id = 0x29)]
    icvPlt34 = 0x29,
    #[deku(id = 0x2a)]
    icvPlt35 = 0x2a,
    #[deku(id = 0x2b)]
    icvPlt36 = 0x2b,
    #[deku(id = 0x2c)]
    icvPlt37 = 0x2c,
    #[deku(id = 0x2d)]
    icvPlt38 = 0x2d,
    #[deku(id = 0x2e)]
    icvPlt39 = 0x2e,
    #[deku(id = 0x2f)]
    icvPlt40 = 0x2f,
    #[deku(id = 0x30)]
    icvPlt41 = 0x30,
    #[deku(id = 0x31)]
    icvPlt42 = 0x31,
    #[deku(id = 0x32)]
    icvPlt43 = 0x32,
    #[deku(id = 0x33)]
    icvPlt44 = 0x33,
    #[deku(id = 0x34)]
    icvPlt45 = 0x34,
    #[deku(id = 0x35)]
    icvPlt46 = 0x35,
    #[deku(id = 0x36)]
    icvPlt47 = 0x36,
    #[deku(id = 0x37)]
    icvPlt48 = 0x37,
    #[deku(id = 0x38)]
    icvPlt49 = 0x38,
    #[deku(id = 0x39)]
    icvPlt50 = 0x39,
    #[deku(id = 0x3a)]
    icvPlt51 = 0x3a,
    #[deku(id = 0x3b)]
    icvPlt52 = 0x3b,
    #[deku(id = 0x3c)]
    icvPlt53 = 0x3c,
    #[deku(id = 0x3d)]
    icvPlt54 = 0x3d,
    #[deku(id = 0x3e)]
    icvPlt55 = 0x3e,
    #[deku(id = 0x3f)]
    icvPlt56 = 0x3f,
    #[deku(id = 0x40)]
    icvForeground = 0x40,
    #[deku(id = 0x41)]
    icvBackground = 0x41,
    #[deku(id = 0x42)]
    icvFrame = 0x42,
    #[deku(id = 0x43)]
    icv3D = 0x43,
    #[deku(id = 0x44)]
    icv3DText = 0x44,
    #[deku(id = 0x45)]
    icv3DHilite = 0x45,
    #[deku(id = 0x46)]
    icv3DShadow = 0x46,
    #[deku(id = 0x47)]
    icvHilite = 0x47,
    #[deku(id = 0x48)]
    icvCtlText = 0x48,
    #[deku(id = 0x49)]
    icvCtlScrl = 0x49,
    #[deku(id = 0x4a)]
    icvCtlInv = 0x4a,
    #[deku(id = 0x4b)]
    icvCtlBody = 0x4b,
    #[deku(id = 0x4c)]
    icvCtlFrame = 0x4c,
    #[deku(id = 0x4d)]
    icvCtlFore = 0x4d,
    #[deku(id = 0x4e)]
    icvCtlBack = 0x4e,
    #[deku(id = 0x4f)]
    icvCtlNeutral = 0x4f,
    #[deku(id = 0x50)]
    icvInfoBk = 0x50,
    #[deku(id = 0x51)]
    icvInfoText = 0x51,
}

impl From<u8> for Icv {
    fn from(value: u8) -> Self {
        macro_rules! matched {
            ($t: ty, $e: ident $(, $e_other: ident)*) => {
                if value == (<$t>::$e as u8) {
                    return <$t>::$e;
                } $(else if value == (<$t>::$e_other as u8) {
                    return <$t>::$e_other;
                })* else {
                    <$t>::default()
                }
            };
        }

        matched!(
            Self,
            icvBlack,
            icvWhite,
            icvRed,
            icvGreen,
            icvBlue,
            icvYellow,
            icvMagenta,
            icvCyan,
            icvPlt1,
            icvPlt2,
            icvPlt3,
            icvPlt4,
            icvPlt5,
            icvPlt6,
            icvPlt7,
            icvPlt8,
            icvPlt9,
            icvPlt10,
            icvPlt11,
            icvPlt12,
            icvPlt13,
            icvPlt14,
            icvPlt15,
            icvPlt16,
            icvPlt17,
            icvPlt18,
            icvPlt19,
            icvPlt20,
            icvPlt21,
            icvPlt22,
            icvPlt23,
            icvPlt24,
            icvPlt25,
            icvPlt26,
            icvPlt27,
            icvPlt28,
            icvPlt29,
            icvPlt30,
            icvPlt31,
            icvPlt32,
            icvPlt33,
            icvPlt34,
            icvPlt35,
            icvPlt36,
            icvPlt37,
            icvPlt38,
            icvPlt39,
            icvPlt40,
            icvPlt41,
            icvPlt42,
            icvPlt43,
            icvPlt44,
            icvPlt45,
            icvPlt46,
            icvPlt47,
            icvPlt48,
            icvPlt49,
            icvPlt50,
            icvPlt51,
            icvPlt52,
            icvPlt53,
            icvPlt54,
            icvPlt55,
            icvPlt56,
            icvForeground,
            icvBackground,
            icvFrame,
            icv3D,
            icv3DText,
            icv3DHilite,
            icv3DShadow,
            icvHilite,
            icvCtlText,
            icvCtlScrl,
            icvCtlInv,
            icvCtlBody,
            icvCtlFrame,
            icvCtlFore,
            icvCtlBack,
            icvCtlNeutral,
            icvInfoBk,
            icvInfoText
        )
    }
}

#[derive(DekuRead, DekuWrite, Hash, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) struct BrtColor {
    #[deku(bits = 1)]
    fValidRGB: bool,
    xColorType: ColorType,
    index: u8,
    nTintAndShade: i16,
    bRed: u8,
    bGreen: u8,
    bBlue: u8,
    bAlpha: u8,
}

impl Debug for BrtColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fValidRGB {
            f.write_str("Color[RGB:")?;
            f.write_fmt(format_args!(
                "#{:02x}{:02x}{:02x}",
                self.bRed, self.bGreen, self.bBlue
            ))?;
            f.write_str("]")?;
        } else {
            match self.xColorType {
                ColorType::ApplicationDetermined => {
                    f.write_str("Color[Auto]")?;
                }
                ColorType::ColorPaletteIndex => {
                    f.write_str("Color[Icv:")?;
                    f.write_fmt(format_args!(
                        "{:?}",
                        crate::styles::brt_color::Icv::from(self.index)
                    ))?;
                    f.write_str("]")?;
                }
                ColorType::RGBA => {
                    f.write_str("Color[RGBA:")?;
                    f.write_fmt(format_args!(
                        "#{:02x}{:02x}{:02x}{:02x}",
                        self.bRed, self.bGreen, self.bBlue, self.bAlpha
                    ))?;
                    f.write_str("]")?;
                }
                ColorType::ThemeColorIndex => {
                    f.write_str("Color[Theme:")?;
                    f.write_fmt(format_args!(
                        "{:?}",
                        crate::styles::brt_color::BrtColorTheme::from(self.index)
                    ))?;
                    f.write_str("]")?;
                }
            }
        }
        Ok(())
    }
}

impl BrtColor {
    pub(crate) const fn auto() -> Self {
        Self {
            fValidRGB: false,
            xColorType: ColorType::ApplicationDetermined,
            index: 0x00,
            nTintAndShade: 0x0000,
            bRed: 0x00,
            bGreen: 0x00,
            bBlue: 0x00,
            bAlpha: 0x00,
        }
    }
    pub(crate) const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            fValidRGB: true,
            xColorType: ColorType::RGBA,
            index: 0x00,
            nTintAndShade: 0x00,
            bRed: r,
            bGreen: g,
            bBlue: b,
            bAlpha: 0xff,
        }
    }

    pub(crate) const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            fValidRGB: true,
            xColorType: ColorType::RGBA,
            index: 0x00,
            nTintAndShade: 0x00,
            bRed: r,
            bGreen: g,
            bBlue: b,
            bAlpha: a,
        }
    }

    pub(crate) const fn from_icv(icv: Icv) -> Self {
        Self {
            fValidRGB: false,
            xColorType: ColorType::ColorPaletteIndex,
            index: icv as u8,
            nTintAndShade: 0x00,
            bRed: 0x00,
            bGreen: 0x00,
            bBlue: 0x00,
            bAlpha: 0x00,
        }
    }

    pub(crate) const fn from_theme(theme: BrtColorTheme) -> Self {
        Self {
            fValidRGB: false,
            xColorType: ColorType::ThemeColorIndex,
            index: theme as u8,
            nTintAndShade: 0x00,
            bRed: 0,
            bGreen: 0,
            bBlue: 0,
            bAlpha: 0,
        }
    }

    pub(crate) const fn with_tint_and_shade(&mut self, tint_and_shade: i16) -> &mut Self {
        self.nTintAndShade = tint_and_shade;
        self
    }
}

impl Default for BrtColor {
    fn default() -> Self {
        Self {
            fValidRGB: false,
            xColorType: ColorType::ApplicationDetermined,
            index: 1,
            nTintAndShade: 0,
            bRed: 0,
            bGreen: 0,
            bBlue: 0,
            bAlpha: 0xff,
        }
    }
}

impl From<(u8, u8, u8)> for BrtColor {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from_rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for BrtColor {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::from_rgba(r, g, b, a)
    }
}
