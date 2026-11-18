use deku::{DekuRead, DekuWrite};

use crate::prelude::Checked;

#[derive(Clone, Default, DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
#[deku(
    bit_order = "lsb",
    ctx = "order: deku::ctx::Order",
    ctx_default = "deku::ctx::Order::Lsb0"
)]
#[deku(id_type = "u8", bits = 7)]
pub enum ColorType {
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

#[derive(DekuRead, DekuWrite, Clone, Debug, Hash, PartialEq, Eq)]
#[deku(
    bit_order = "lsb",
    ctx = "order: deku::ctx::Order",
    ctx_default = "deku::ctx::Order::Lsb0"
)]
pub struct BrtColor {
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

impl BrtColor {
    /// Creates an opaque RGB color.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 0xff)
    }

    /// Creates a straight-alpha RGBA color.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            fValidRGB: true,
            xColorType: ColorType::RGBA,
            index: 0,
            nTintAndShade: 0,
            bRed: red,
            bGreen: green,
            bBlue: blue,
            bAlpha: alpha,
        }
    }

    const fn check_internal(&self) -> Result<(), &'static str> {
        match self.xColorType {
            ColorType::ApplicationDetermined => Ok(()),
            ColorType::ColorPaletteIndex => {
                if matches!(self.index, 0x00..=0x51) {
                    Ok(())
                } else {
                    Err("Invalid color palette index")
                }
            }
            ColorType::RGBA => {
                if self.fValidRGB {
                    Ok(())
                } else {
                    Err("Invalid RGB value")
                }
            }
            ColorType::ThemeColorIndex => {
                if matches!(self.index, 0x00..=0x0b) {
                    Ok(())
                } else {
                    Err("Invalid theme color index")
                }
            }
        }
    }
}

impl Checked for BrtColor {
    fn check(&self) -> Result<(), &'static str> {
        self.check_internal()
    }
}

#[test]
fn test_brt_color() {
    let mut color = BrtColor::default();
    assert!(color.check().is_ok());
    color.index = 0x52;
    color.xColorType = ColorType::ColorPaletteIndex;
    assert!(color.check().is_err());
}

#[test]
fn test_brt_color_uses_lsb_bit_order() {
    use deku::DekuContainerWrite;

    let color = BrtColor {
        fValidRGB: true,
        xColorType: ColorType::ThemeColorIndex,
        ..BrtColor::default()
    };

    let bytes = color.to_bytes().expect("serializes");
    assert_eq!(bytes[0], 0x07);
}

#[test]
fn rgb_color_serializes_as_rgba() {
    use deku::DekuContainerWrite;

    let color = BrtColor::rgb(0x12, 0x34, 0x56);
    assert!(color.check().is_ok());

    let bytes = color.to_bytes().expect("serializes");
    assert_eq!(bytes[0], 0x05);
    assert_eq!(&bytes[4..8], &[0x12, 0x34, 0x56, 0xff]);
}
