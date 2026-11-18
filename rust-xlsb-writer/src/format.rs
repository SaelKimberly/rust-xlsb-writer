//! Public cell formatting builder and its mapping to core style records.

use rust_xlsb_core::prelude::{
    Bold, BorderType, BrtBorder, BrtColor, BrtFill, BrtFont, BrtXF, HAlign, VAlign, XlStringExt,
    XlWideString,
};
use rust_xlsb_core::{CellStyle, StyleRegistry};

use crate::{Error, Result};

/// A color accepted by [`Format`] setters.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum Color {
    /// A user-defined `0xRRGGBB` color.
    RGB(u32),
    /// A theme color: `(color index 0..=9, shade 0..=5)`.
    Theme(u8, u8),
    /// The default color for the property.
    #[default]
    Default,
    /// The automatic color.
    Automatic,
    /// Black, `0x000000`.
    Black,
    /// Blue, `0x0000FF`.
    Blue,
    /// Brown, `0x800000`.
    Brown,
    /// Cyan, `0x00FFFF`.
    Cyan,
    /// Gray, `0x808080`.
    Gray,
    /// Green, `0x008000`.
    Green,
    /// Lime, `0x00FF00`.
    Lime,
    /// Magenta, `0xFF00FF`.
    Magenta,
    /// Navy, `0x000080`.
    Navy,
    /// Orange, `0xFF6600`.
    Orange,
    /// Pink, `0xFFC0CB`.
    Pink,
    /// Purple, `0x800080`.
    Purple,
    /// Red, `0xFF0000`.
    Red,
    /// Silver, `0xC0C0C0`.
    Silver,
    /// White, `0xFFFFFF`.
    White,
    /// Yellow, `0xFFFF00`.
    Yellow,
}

impl Color {
    const fn named_rgb(self) -> Option<u32> {
        match self {
            Self::Black => Some(0x0000_0000),
            Self::Blue => Some(0x0000_00FF),
            Self::Brown => Some(0x0080_0000),
            Self::Cyan => Some(0x0000_FFFF),
            Self::Gray => Some(0x0080_8080),
            Self::Green => Some(0x0000_8000),
            Self::Lime => Some(0x0000_FF00),
            Self::Magenta => Some(0x00FF_00FF),
            Self::Navy => Some(0x0000_0080),
            Self::Orange => Some(0x00FF_6600),
            Self::Pink => Some(0x00FF_C0CB),
            Self::Purple => Some(0x0080_0080),
            Self::Red => Some(0x00FF_0000),
            Self::Silver => Some(0x00C0_C0C0),
            Self::White => Some(0x00FF_FFFF),
            Self::Yellow => Some(0x00FF_FF00),
            Self::RGB(_) | Self::Theme(_, _) | Self::Default | Self::Automatic => None,
        }
    }

    fn to_brt_color(self) -> Result<BrtColor> {
        let value = match self {
            Self::Default | Self::Automatic => return Ok(BrtColor::default()),
            Self::RGB(value) => value,
            Self::Theme(color, shade) => {
                if color > 9 || shade > 5 {
                    return Err(Error::ValueNotRepresentable("theme color out of range"));
                }
                return Ok(BrtColor::theme(color, theme_tint(color, shade)));
            }
            named => named.named_rgb().expect("only named colors reach this arm"),
        };

        if value > 0x00FF_FFFF {
            return Err(Error::ValueNotRepresentable("RGB color exceeds 0xFFFFFF"));
        }
        let red = u8::try_from((value >> 16) & 0xFF).expect("masked to u8");
        let green = u8::try_from((value >> 8) & 0xFF).expect("masked to u8");
        let blue = u8::try_from(value & 0xFF).expect("masked to u8");
        Ok(BrtColor::rgb(red, green, blue))
    }
}

impl From<u32> for Color {
    fn from(value: u32) -> Self {
        Self::RGB(value)
    }
}

impl From<&str> for Color {
    fn from(value: &str) -> Self {
        let hex = value
            .strip_prefix('#')
            .or_else(|| value.strip_prefix("0x"))
            .or_else(|| value.strip_prefix("0X"))
            .unwrap_or(value);
        u32::from_str_radix(hex, 16).map_or(Self::Default, Self::RGB)
    }
}

impl From<String> for Color {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Horizontal and vertical alignment values accepted by [`Format::set_align`].
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatAlign {
    /// General/default alignment.
    #[default]
    General,
    /// Align left.
    Left,
    /// Center horizontally.
    Center,
    /// Align right.
    Right,
    /// Fill the cell with repeated text.
    Fill,
    /// Justify horizontally.
    Justify,
    /// Center across the selection.
    CenterAcross,
    /// Distribute horizontally.
    Distributed,
    /// Align top.
    Top,
    /// Align bottom.
    Bottom,
    /// Center vertically.
    VerticalCenter,
    /// Justify vertically.
    VerticalJustify,
    /// Distribute vertically.
    VerticalDistributed,
}

/// Border styles accepted by [`Format`] border setters.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatBorder {
    /// No border.
    #[default]
    None,
    /// Thin border.
    Thin,
    /// Medium border.
    Medium,
    /// Dashed border.
    Dashed,
    /// Dotted border.
    Dotted,
    /// Thick border.
    Thick,
    /// Double border.
    Double,
    /// Hair border.
    Hair,
    /// Medium dashed border.
    MediumDashed,
    /// Dash-dot border.
    DashDot,
    /// Medium dash-dot border.
    MediumDashDot,
    /// Dash-dot-dot border.
    DashDotDot,
    /// Medium dash-dot-dot border.
    MediumDashDotDot,
    /// Slant dash-dot border.
    SlantDashDot,
}

/// Fill patterns accepted by [`Format::set_pattern`].
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatPattern {
    /// No pattern.
    #[default]
    None,
    /// Solid fill.
    Solid,
    /// Medium gray.
    MediumGray,
    /// Dark gray.
    DarkGray,
    /// Light gray.
    LightGray,
    /// Dark horizontal lines.
    DarkHorizontal,
    /// Dark vertical lines.
    DarkVertical,
    /// Dark diagonal stripes, down.
    DarkDown,
    /// Dark diagonal stripes, up.
    DarkUp,
    /// Dark grid.
    DarkGrid,
    /// Dark trellis.
    DarkTrellis,
    /// Light horizontal lines.
    LightHorizontal,
    /// Light vertical lines.
    LightVertical,
    /// Light diagonal stripes, down.
    LightDown,
    /// Light diagonal stripes, up.
    LightUp,
    /// Light grid.
    LightGrid,
    /// Light trellis.
    LightTrellis,
    /// 12.5% gray.
    Gray125,
    /// 6.25% gray.
    Gray0625,
}

/// A complete cell format.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Format {
    font: BrtFont,
    fill: BrtFill,
    border: BrtBorder,
    xf: BrtXF,
    number_format_spec: Option<NumFormat>,
    pattern_set: bool,
    font_color: Option<Color>,
    background_color: Option<Color>,
    foreground_color: Option<Color>,
    border_top_color: Option<Color>,
    border_bottom_color: Option<Color>,
    border_left_color: Option<Color>,
    border_right_color: Option<Color>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum NumFormat {
    Code(String),
    Index(u16),
}

impl Default for Format {
    fn default() -> Self {
        Self::new()
    }
}

impl Format {
    /// Creates a format using the workbook default font, fill, border and XF.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font: BrtFont::default(),
            fill: BrtFill::default(),
            border: BrtBorder::default(),
            xf: BrtXF::default(),
            number_format_spec: None,
            pattern_set: false,
            font_color: None,
            background_color: None,
            foreground_color: None,
            border_top_color: None,
            border_bottom_color: None,
            border_left_color: None,
            border_right_color: None,
        }
    }

    /// Sets bold text.
    #[must_use]
    pub const fn set_bold(mut self) -> Self {
        self.font.bls = Bold::BLSBOLD;
        self
    }

    /// Sets italic text.
    #[must_use]
    pub const fn set_italic(mut self) -> Self {
        self.font.grbit.fItalic = true;
        self
    }

    /// Sets the font color.
    #[must_use]
    pub fn set_font_color(mut self, color: impl Into<Color>) -> Self {
        self.font_color = Some(color.into());
        self
    }

    /// Sets the font name.
    #[must_use]
    pub fn set_font_name(mut self, name: impl Into<String>) -> Self {
        self.font.name = XlWideString::construct(Some(name.into()));
        self
    }

    /// Sets the font size in points.
    #[must_use]
    pub fn set_font_size(mut self, size: impl Into<f64>) -> Self {
        self.font.dyHeight = points_to_twips(size.into());
        self
    }

    /// Sets a custom number-format code.
    #[must_use]
    pub fn set_num_format(mut self, code: impl Into<String>) -> Self {
        self.number_format_spec = Some(NumFormat::Code(code.into()));
        self
    }

    /// Sets a built-in number-format index.
    #[must_use]
    pub fn set_num_format_index(mut self, index: u8) -> Self {
        self.number_format_spec = Some(NumFormat::Index(u16::from(index)));
        self
    }

    /// Sets horizontal or vertical alignment.
    #[must_use]
    pub const fn set_align(mut self, align: FormatAlign) -> Self {
        match align {
            FormatAlign::General => {
                self.xf.alc = HAlign::General;
                self.xf.alcv = VAlign::Top;
            }
            FormatAlign::Left => self.xf.alc = HAlign::Left,
            FormatAlign::Center => self.xf.alc = HAlign::Center,
            FormatAlign::Right => self.xf.alc = HAlign::Right,
            FormatAlign::Fill => self.xf.alc = HAlign::Fill,
            FormatAlign::Justify => self.xf.alc = HAlign::Justify,
            FormatAlign::CenterAcross => self.xf.alc = HAlign::CenterAcrossSelection,
            FormatAlign::Distributed => self.xf.alc = HAlign::Distributed,
            FormatAlign::Top => self.xf.alcv = VAlign::Top,
            FormatAlign::Bottom => self.xf.alcv = VAlign::Bottom,
            FormatAlign::VerticalCenter => self.xf.alcv = VAlign::Center,
            FormatAlign::VerticalJustify => self.xf.alcv = VAlign::Justify,
            FormatAlign::VerticalDistributed => self.xf.alcv = VAlign::Distributed,
        }
        self
    }

    /// Enables text wrapping.
    #[must_use]
    pub const fn set_text_wrap(mut self) -> Self {
        self.xf.fWrap = true;
        self
    }

    /// Sets the fill pattern.
    #[must_use]
    pub const fn set_pattern(mut self, pattern: FormatPattern) -> Self {
        self.fill.fls = pattern_id(pattern);
        self.pattern_set = true;
        self
    }

    /// Sets the background fill color and defaults the pattern to solid.
    #[must_use]
    pub fn set_background_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        self.background_color = Some(color);
        if !self.pattern_set && !matches!(color, Color::Default | Color::Automatic) {
            self.fill.fls = pattern_id(FormatPattern::Solid);
            self.pattern_set = true;
        }
        self
    }

    /// Sets the foreground/pattern fill color.
    #[must_use]
    pub fn set_foreground_color(mut self, color: impl Into<Color>) -> Self {
        self.foreground_color = Some(color.into());
        self
    }

    /// Sets all four border styles.
    #[must_use]
    pub const fn set_border(mut self, border: FormatBorder) -> Self {
        self.border.blxfTop.border_type = border_type(border);
        self.border.blxfBottom.border_type = border_type(border);
        self.border.blxfLeft.border_type = border_type(border);
        self.border.blxfRight.border_type = border_type(border);
        self
    }

    /// Sets all four border colors.
    #[must_use]
    pub fn set_border_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        self.border_top_color = Some(color);
        self.border_bottom_color = Some(color);
        self.border_left_color = Some(color);
        self.border_right_color = Some(color);
        self
    }

    /// Sets the top border style.
    #[must_use]
    pub const fn set_border_top(mut self, border: FormatBorder) -> Self {
        self.border.blxfTop.border_type = border_type(border);
        self
    }

    /// Sets the top border color.
    #[must_use]
    pub fn set_border_top_color(mut self, color: impl Into<Color>) -> Self {
        self.border_top_color = Some(color.into());
        self
    }

    /// Sets the bottom border style.
    #[must_use]
    pub const fn set_border_bottom(mut self, border: FormatBorder) -> Self {
        self.border.blxfBottom.border_type = border_type(border);
        self
    }

    /// Sets the bottom border color.
    #[must_use]
    pub fn set_border_bottom_color(mut self, color: impl Into<Color>) -> Self {
        self.border_bottom_color = Some(color.into());
        self
    }

    /// Sets the left border style.
    #[must_use]
    pub const fn set_border_left(mut self, border: FormatBorder) -> Self {
        self.border.blxfLeft.border_type = border_type(border);
        self
    }

    /// Sets the left border color.
    #[must_use]
    pub fn set_border_left_color(mut self, color: impl Into<Color>) -> Self {
        self.border_left_color = Some(color.into());
        self
    }

    /// Sets the right border style.
    #[must_use]
    pub const fn set_border_right(mut self, border: FormatBorder) -> Self {
        self.border.blxfRight.border_type = border_type(border);
        self
    }

    /// Sets the right border color.
    #[must_use]
    pub fn set_border_right_color(mut self, color: impl Into<Color>) -> Self {
        self.border_right_color = Some(color.into());
        self
    }

    /// Locks the cell when the sheet is protected.
    #[must_use]
    pub const fn set_locked(mut self) -> Self {
        self.xf.fLocked = true;
        self
    }

    /// Hides formulas in the cell when the sheet is protected.
    #[must_use]
    pub const fn set_hidden(mut self) -> Self {
        self.xf.fHidden = true;
        self
    }

    /// Clears the hidden formula flag.
    #[must_use]
    pub const fn unset_hidden(mut self) -> Self {
        self.xf.fHidden = false;
        self
    }

    pub(crate) fn to_cell_style(&self, registry: &mut StyleRegistry) -> Result<CellStyle> {
        let mut font = self.font.clone();
        if let Some(color) = self.font_color {
            font.brtColor = color.to_brt_color()?;
        }
        if font.name.len() > 31 {
            return Err(Error::ValueNotRepresentable(
                "font name exceeds 31 UTF-16 code units",
            ));
        }
        if !(20..=8_180).contains(&font.dyHeight) {
            return Err(Error::ValueNotRepresentable(
                "font size is outside 1..=409 points",
            ));
        }

        let mut fill = self.fill.clone();
        if let Some(color) = self.background_color {
            fill.brtColorBack = color.to_brt_color()?;
        }
        if let Some(color) = self.foreground_color {
            fill.brtColorFore = color.to_brt_color()?;
        }
        if fill.fls == pattern_id(FormatPattern::Solid)
            && self.foreground_color.is_none()
            && self.background_color.is_some()
        {
            fill.brtColorFore = fill.brtColorBack.clone();
        }

        let mut border = self.border.clone();
        if let Some(color) = self.border_top_color {
            border.blxfTop.color = color.to_brt_color()?;
        }
        if let Some(color) = self.border_bottom_color {
            border.blxfBottom.color = color.to_brt_color()?;
        }
        if let Some(color) = self.border_left_color {
            border.blxfLeft.color = color.to_brt_color()?;
        }
        if let Some(color) = self.border_right_color {
            border.blxfRight.color = color.to_brt_color()?;
        }

        let mut xf = self.xf.clone();
        xf.iFmt = match &self.number_format_spec {
            Some(NumFormat::Code(code)) => registry.add_number_format(code)?,
            Some(NumFormat::Index(index)) => *index,
            None => xf.iFmt,
        };

        Ok(CellStyle {
            font,
            fill,
            border,
            xf,
        })
    }
}

fn points_to_twips(points: f64) -> u16 {
    let twips = (points * 20.0).round();
    if twips.is_nan() || twips <= 0.0 {
        0
    } else if twips >= f64::from(u16::MAX) {
        u16::MAX
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "value is finite and clamped to 0..=u16::MAX"
        )]
        {
            twips as u16
        }
    }
}

fn theme_tint(color: u8, shade: u8) -> i16 {
    let tint: [f64; 6] = match color {
        0 => [
            0.0,
            -0.049_989_318_521_683_4,
            -0.149_998_474_074_526_2,
            -0.249_977_111_117_893,
            -0.349_986_266_670_735_8,
            -0.499_984_740_745_262,
        ],
        1 => [
            0.0,
            0.499_984_740_745_262,
            0.349_986_266_670_735_8,
            0.249_977_111_117_893,
            0.149_998_474_074_526_2,
            0.049_989_318_521_683_4,
        ],
        2 => [
            0.0,
            -0.099_978_637_043_366_8,
            -0.249_977_111_117_893,
            -0.499_984_740_745_262,
            -0.749_992_370_372_631,
            -0.899_990_844_447_157_2,
        ],
        _ => [
            0.0,
            0.799_981_688_894_314_4,
            0.599_993_896_298_104_9,
            0.399_975_585_192_419_2,
            -0.249_977_111_117_893,
            -0.499_984_740_745_262,
        ],
    };
    let index = usize::from(shade);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "tint is in -1..=1 and the scaled value fits i16"
    )]
    {
        (tint[index] * 32_767.0).round() as i16
    }
}

const fn pattern_id(pattern: FormatPattern) -> u32 {
    match pattern {
        FormatPattern::None => 0,
        FormatPattern::Solid => 1,
        FormatPattern::MediumGray => 2,
        FormatPattern::DarkGray => 3,
        FormatPattern::LightGray => 4,
        FormatPattern::DarkHorizontal => 5,
        FormatPattern::DarkVertical => 6,
        FormatPattern::DarkDown => 7,
        FormatPattern::DarkUp => 8,
        FormatPattern::DarkGrid => 9,
        FormatPattern::DarkTrellis => 10,
        FormatPattern::LightHorizontal => 11,
        FormatPattern::LightVertical => 12,
        FormatPattern::LightDown => 13,
        FormatPattern::LightUp => 14,
        FormatPattern::LightGrid => 15,
        FormatPattern::LightTrellis => 16,
        FormatPattern::Gray125 => 17,
        FormatPattern::Gray0625 => 18,
    }
}

const fn border_type(border: FormatBorder) -> BorderType {
    match border {
        FormatBorder::None => BorderType::None,
        FormatBorder::Thin => BorderType::Thin,
        FormatBorder::Medium => BorderType::Medium,
        FormatBorder::Dashed => BorderType::Dashed,
        FormatBorder::Dotted => BorderType::Dotted,
        FormatBorder::Thick => BorderType::Thick,
        FormatBorder::Double => BorderType::Double,
        FormatBorder::Hair => BorderType::Hairline,
        FormatBorder::MediumDashed => BorderType::MediumDashed,
        FormatBorder::DashDot => BorderType::DashDot,
        FormatBorder::MediumDashDot => BorderType::MediumDashDot,
        FormatBorder::DashDotDot => BorderType::DashDotDot,
        FormatBorder::MediumDashDotDot => BorderType::MediumDashDotDot,
        FormatBorder::SlantDashDot => BorderType::SlantDashDot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsb_core::prelude::{BorderType, BrtColor};

    fn cell_style(format: &Format) -> CellStyle {
        let mut registry = StyleRegistry::new();
        format.to_cell_style(&mut registry).expect("valid format")
    }

    #[test]
    fn new_format_deduplicates_to_default_xf() {
        let mut registry = StyleRegistry::new();
        let style = Format::new()
            .to_cell_style(&mut registry)
            .expect("valid format");

        assert_eq!(registry.add(style).expect("adds"), 0);
    }

    #[test]
    fn maps_font_properties() {
        let style = cell_style(
            &Format::new()
                .set_bold()
                .set_italic()
                .set_font_size(10.5)
                .set_font_name("Test")
                .set_font_color(Color::Red),
        );

        assert_eq!(style.font.bls, Bold::BLSBOLD);
        assert!(style.font.grbit.fItalic);
        assert_eq!(style.font.dyHeight, 210);
        assert_eq!(
            style.font.name.to_optional_string().as_deref(),
            Some("Test")
        );
        assert_eq!(style.font.brtColor, BrtColor::rgb(0xFF, 0x00, 0x00));
    }

    #[test]
    fn registers_custom_number_format_once() {
        let mut registry = StyleRegistry::new();
        let first = Format::new()
            .set_num_format("0.00")
            .to_cell_style(&mut registry)
            .expect("valid format");
        let second = Format::new()
            .set_num_format("0.00")
            .to_cell_style(&mut registry)
            .expect("valid format");

        assert_eq!(first.xf.iFmt, 164);
        assert_eq!(second.xf.iFmt, 164);
    }

    #[test]
    fn maps_builtin_number_format_index() {
        let style = cell_style(&Format::new().set_num_format_index(14));
        assert_eq!(style.xf.iFmt, 14);
    }

    #[test]
    fn maps_borders_and_colors() {
        let style = cell_style(
            &Format::new()
                .set_border(FormatBorder::Thin)
                .set_border_top(FormatBorder::Medium)
                .set_border_color(Color::Blue)
                .set_border_top_color(Color::Red),
        );

        assert_eq!(style.border.blxfTop.border_type, BorderType::Medium);
        assert_eq!(style.border.blxfBottom.border_type, BorderType::Thin);
        assert_eq!(style.border.blxfTop.color, BrtColor::rgb(0xFF, 0x00, 0x00));
        assert_eq!(
            style.border.blxfBottom.color,
            BrtColor::rgb(0x00, 0x00, 0xFF)
        );
    }

    #[test]
    fn background_color_defaults_to_solid_fill() {
        let style = cell_style(&Format::new().set_background_color(Color::Red));

        assert_eq!(style.fill.fls, 1);
        assert_eq!(style.fill.brtColorFore, BrtColor::rgb(0xFF, 0x00, 0x00));
    }

    #[test]
    fn explicit_none_pattern_is_not_overridden() {
        let style = cell_style(
            &Format::new()
                .set_pattern(FormatPattern::None)
                .set_background_color(Color::Red),
        );

        assert_eq!(style.fill.fls, 0);
    }

    #[test]
    fn maps_alignment() {
        let style = cell_style(
            &Format::new()
                .set_align(FormatAlign::Center)
                .set_align(FormatAlign::VerticalCenter),
        );

        assert_eq!(style.xf.alc, HAlign::Center);
        assert_eq!(style.xf.alcv, VAlign::Center);
    }

    #[test]
    fn maps_protection_flags() {
        let style = cell_style(&Format::new().set_locked().set_hidden());
        assert!(style.xf.fLocked);
        assert!(style.xf.fHidden);

        let style = cell_style(&Format::new().set_hidden().unset_hidden());
        assert!(!style.xf.fHidden);
    }

    #[test]
    fn rejects_invalid_font_values() {
        let mut registry = StyleRegistry::new();
        assert!(matches!(
            Format::new()
                .set_font_size(0.0)
                .to_cell_style(&mut registry),
            Err(Error::ValueNotRepresentable(_))
        ));

        let long_name = "x".repeat(32);
        assert!(matches!(
            Format::new()
                .set_font_name(long_name)
                .to_cell_style(&mut registry),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn rejects_invalid_theme_color() {
        let mut registry = StyleRegistry::new();
        assert!(matches!(
            Format::new()
                .set_font_color(Color::Theme(10, 0))
                .to_cell_style(&mut registry),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn parses_hex_colors() {
        assert_eq!(Color::from("#FF0000"), Color::RGB(0x00FF_0000));
        assert_eq!(Color::from("00FF00"), Color::RGB(0x0000_FF00));
        assert_eq!(Color::from("0x0000FF"), Color::RGB(0x0000_00FF));
        assert_eq!(Color::from("0XFF00FF"), Color::RGB(0x00FF_00FF));
        assert_eq!(Color::from("not-hex"), Color::Default);
    }
}
