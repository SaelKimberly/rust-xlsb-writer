use std::{fmt::Debug, hash::Hash};

use super::{BrtColor, Xnum};
use deku::{DekuRead, DekuWrite};
#[derive(DekuRead, DekuWrite, Debug, Default, Hash, Clone)]
pub(crate) struct GradientStop {
    brtColor: BrtColor,
    xnumPosition: Xnum,
}

#[derive(DekuRead, DekuWrite, Debug, Clone, Copy, Default, Hash)]
#[deku(id_type = "u32")]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub(crate) enum FillStyle {
    #[default]
    #[deku(id = 0x00000000, default)]
    None = 0x00000000,
    #[deku(id = 0x00000001)]
    Solid = 0x00000001,
    #[deku(id = 0x00000002)]
    MediumGray = 0x00000002,
    #[deku(id = 0x00000003)]
    DarkGray = 0x00000003,
    #[deku(id = 0x00000004)]
    LightGray = 0x00000004,
    #[deku(id = 0x00000005)]
    HorzStripes = 0x00000005,
    #[deku(id = 0x00000006)]
    VertStripes = 0x00000006,
    #[deku(id = 0x00000007)]
    DownDiagStripes = 0x00000007,
    #[deku(id = 0x00000008)]
    UpDiagStripes = 0x00000008,
    #[deku(id = 0x00000009)]
    Grid = 0x00000009,
    #[deku(id = 0x0000000a)]
    Trellis = 0x0000000a,
    #[deku(id = 0x0000000b)]
    LightHorzStripes = 0x0000000b,
    #[deku(id = 0x0000000c)]
    LightVertStripes = 0x0000000c,
    #[deku(id = 0x0000000d)]
    LightDownDiagStripes = 0x0000000d,
    #[deku(id = 0x0000000e)]
    LightUpDiagStripes = 0x0000000e,
    #[deku(id = 0x0000000f)]
    LightGrid = 0x0000000f,
    #[deku(id = 0x00000010)]
    LightTrellis = 0x00000010,
    #[deku(id = 0x00000011)]
    Grayscale_1_8 = 0x00000011,
    #[deku(id = 0x00000012)]
    Grayscale_1_16 = 0x00000012,
    #[deku(id = 0x00000028)]
    GradientFill = 0x00000028,
}

#[derive(DekuRead, DekuWrite, Default, Hash)]
/// **`2.4.674`**
pub(crate) struct BrtFill {
    fls: FillStyle,
    brtColorFore: BrtColor,
    brtColorBack: BrtColor,
    iGradientType: u32,
    xnumDegree: Xnum,
    xnumFillToLeft: Xnum,
    xnumFillToRight: Xnum,
    xnumFillToTop: Xnum,
    xnumFillToBottom: Xnum,
    #[deku(update = "self.xfillGradientStop.len()")]
    cNumStop: u32,
    #[deku(count = "cNumStop")]
    xfillGradientStop: Vec<GradientStop>,
}

#[derive(Default, Debug)]
pub(crate) enum Fill {
    #[default]
    None,
    Solid(BrtColor),
    Gradient {
        rectangular: bool,
        degree: f64,
        fill_to_left: f64,
        fill_to_right: f64,
        fill_to_top: f64,
        fill_to_bottom: f64,
        gradient_stops: Vec<GradientStop>,
    },
    BuiltIn(FillStyle, BrtColor, BrtColor),
}

impl Debug for BrtFill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Fill::from(self).fmt(f)
    }
}

impl From<&BrtFill> for Fill {
    fn from(value: &BrtFill) -> Self {
        match &value.fls {
            FillStyle::None => Self::None,
            FillStyle::Solid => Self::Solid(value.brtColorFore.clone()),
            FillStyle::GradientFill => Self::Gradient {
                rectangular: value.iGradientType == 0x0000_0001,
                degree: value.xnumDegree.into(),
                fill_to_left: value.xnumFillToLeft.into(),
                fill_to_right: value.xnumFillToRight.into(),
                fill_to_top: value.xnumFillToTop.into(),
                fill_to_bottom: value.xnumFillToBottom.into(),
                gradient_stops: value.xfillGradientStop.to_vec(),
            },
            X => Self::BuiltIn(*X, value.brtColorFore.clone(), value.brtColorBack.clone()),
        }
    }
}

impl From<BrtColor> for BrtFill {
    fn from(value: BrtColor) -> Self {
        Self {
            fls: FillStyle::Solid,
            brtColorFore: value,
            ..Default::default()
        }
    }
}

impl From<Fill> for BrtFill {
    fn from(value: Fill) -> Self {
        match value {
            Fill::None => BrtFill::default(),
            Fill::Solid(fore_color) => BrtFill {
                fls: FillStyle::Solid,
                brtColorFore: fore_color,
                ..Default::default()
            },
            Fill::BuiltIn(fls, brtColorFore, brtColorBack) => BrtFill {
                fls,
                brtColorFore,
                brtColorBack,
                ..Default::default()
            },
            Fill::Gradient {
                rectangular,
                degree,
                fill_to_left,
                fill_to_right,
                fill_to_top,
                fill_to_bottom,
                gradient_stops,
            } => BrtFill {
                fls: FillStyle::GradientFill,
                iGradientType: if rectangular {
                    0x0000_0001
                } else {
                    0x0000_0000
                },
                xnumDegree: degree.into(),
                xnumFillToLeft: fill_to_left.into(),
                xnumFillToRight: fill_to_right.into(),
                xnumFillToTop: fill_to_top.into(),
                xnumFillToBottom: fill_to_bottom.into(),
                cNumStop: gradient_stops.len() as u32,
                xfillGradientStop: gradient_stops,
                ..Default::default()
            },
        }
    }
}
