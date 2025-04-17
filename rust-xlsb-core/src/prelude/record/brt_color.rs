use deku::{DekuRead, DekuWrite};

use crate::prelude::Checked;

#[derive(Default, DekuRead, DekuWrite, Debug, Hash, PartialEq)]
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

#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq)]
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
                if !self.fValidRGB {
                    Err("Invalid RGB value")
                } else {
                    Ok(())
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
