//! Public cell formatting builder and its mapping to core style records.

use rust_xlsb_core::prelude::{
    Bold, BorderType, BrtBorder, BrtColor, BrtFill, BrtFont, BrtXF, HAlign, ReadingOrder, VAlign,
    XlStringExt, XlWideString,
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

    pub(crate) const fn is_valid(self) -> bool {
        match self {
            Self::RGB(value) => value <= 0x00FF_FFFF,
            Self::Theme(color, shade) => color <= 9 && shade <= 5,
            _ => true,
        }
    }

    pub(crate) fn to_brt_color(self) -> Result<BrtColor> {
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

/// Font underline styles.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatUnderline {
    /// No underline.
    #[default]
    None,
    /// Single underline.
    Single,
    /// Double underline.
    Double,
    /// Single accounting underline.
    SingleAccounting,
    /// Double accounting underline.
    DoubleAccounting,
}

/// Font superscript/subscript styles.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatScript {
    /// Normal text.
    #[default]
    None,
    /// Superscript.
    Superscript,
    /// Subscript.
    Subscript,
}

/// Font scheme classification.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FontScheme {
    /// Minor/body theme font.
    #[default]
    Body,
    /// Major/heading theme font.
    Headings,
    /// Custom font outside the theme.
    None,
}

/// Diagonal border direction.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub enum FormatDiagonalBorder {
    /// No diagonal border.
    #[default]
    None,
    /// Bottom-left to top-right.
    BorderUp,
    /// Top-left to bottom-right.
    BorderDown,
    /// Both diagonals.
    BorderUpDown,
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
    border_diagonal_color: Option<Color>,
    vertical_align_set: bool,
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the worksheet implementation"
)]
#[derive(Clone, Copy)]
pub(crate) enum BorderSide {
    Top,
    Bottom,
    Left,
    Right,
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
            xf: BrtXF {
                fLocked: true,
                ..BrtXF::default()
            },
            number_format_spec: None,
            pattern_set: false,
            font_color: None,
            background_color: None,
            foreground_color: None,
            border_top_color: None,
            border_bottom_color: None,
            border_left_color: None,
            border_right_color: None,
            border_diagonal_color: None,
            vertical_align_set: false,
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

    /// Applies Excel's blue, single-underlined hyperlink font style.
    #[must_use]
    pub const fn set_hyperlink(mut self) -> Self {
        self.font_color = Some(Color::RGB(0x0005_63C1));
        self.font.uls = 0x01;
        self.font.bFontScheme = 0;
        self
    }

    /// Removes Excel's hyperlink font style.
    #[must_use]
    pub const fn unset_hyperlink_style(mut self) -> Self {
        self.font_color = None;
        self.font.uls = 0;
        self.font.bFontScheme = 2;
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
                self.vertical_align_set = false;
            }
            FormatAlign::Left => self.xf.alc = HAlign::Left,
            FormatAlign::Center => self.xf.alc = HAlign::Center,
            FormatAlign::Right => self.xf.alc = HAlign::Right,
            FormatAlign::Fill => self.xf.alc = HAlign::Fill,
            FormatAlign::Justify => self.xf.alc = HAlign::Justify,
            FormatAlign::CenterAcross => self.xf.alc = HAlign::CenterAcrossSelection,
            FormatAlign::Distributed => self.xf.alc = HAlign::Distributed,
            FormatAlign::Top => {
                self.xf.alcv = VAlign::Top;
                self.vertical_align_set = true;
            }
            FormatAlign::Bottom => {
                self.xf.alcv = VAlign::Bottom;
                self.vertical_align_set = true;
            }
            FormatAlign::VerticalCenter => {
                self.xf.alcv = VAlign::Center;
                self.vertical_align_set = true;
            }
            FormatAlign::VerticalJustify => {
                self.xf.alcv = VAlign::Justify;
                self.vertical_align_set = true;
            }
            FormatAlign::VerticalDistributed => {
                self.xf.alcv = VAlign::Distributed;
                self.vertical_align_set = true;
            }
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

    /// Copies one border side, including its color, from another format.
    pub(crate) fn copy_border_side(&mut self, other: &Self, side: BorderSide) {
        match side {
            BorderSide::Top => {
                self.border.blxfTop.border_type = other.border.blxfTop.border_type.clone();
                self.border_top_color = other.border_top_color;
            }
            BorderSide::Bottom => {
                self.border.blxfBottom.border_type = other.border.blxfBottom.border_type.clone();
                self.border_bottom_color = other.border_bottom_color;
            }
            BorderSide::Left => {
                self.border.blxfLeft.border_type = other.border.blxfLeft.border_type.clone();
                self.border_left_color = other.border_left_color;
            }
            BorderSide::Right => {
                self.border.blxfRight.border_type = other.border.blxfRight.border_type.clone();
                self.border_right_color = other.border_right_color;
            }
        }
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

    /// Unlocks the cell when the sheet is protected.
    #[must_use]
    pub const fn set_unlocked(mut self) -> Self {
        self.xf.fLocked = false;
        self
    }

    /// Clears bold text.
    #[must_use]
    pub const fn unset_bold(mut self) -> Self {
        self.font.bls = Bold::BLSNORMAL;
        self
    }

    /// Clears italic text.
    #[must_use]
    pub const fn unset_italic(mut self) -> Self {
        self.font.grbit.fItalic = false;
        self
    }

    /// Disables text wrapping.
    #[must_use]
    pub const fn unset_text_wrap(mut self) -> Self {
        self.xf.fWrap = false;
        self
    }

    /// Sets font strikethrough.
    #[must_use]
    pub const fn set_font_strikethrough(mut self) -> Self {
        self.font.grbit.fStrikeOut = true;
        self
    }

    /// Clears font strikethrough.
    #[must_use]
    pub const fn unset_font_strikethrough(mut self) -> Self {
        self.font.grbit.fStrikeOut = false;
        self
    }

    /// Sets the font underline style.
    #[must_use]
    pub const fn set_underline(mut self, underline: FormatUnderline) -> Self {
        self.font.uls = match underline {
            FormatUnderline::None => 0x00,
            FormatUnderline::Single => 0x01,
            FormatUnderline::Double => 0x02,
            FormatUnderline::SingleAccounting => 0x21,
            FormatUnderline::DoubleAccounting => 0x22,
        };
        self
    }

    /// Sets font superscript/subscript.
    #[must_use]
    pub const fn set_font_script(mut self, script: FormatScript) -> Self {
        self.font.sss = match script {
            FormatScript::None => 0,
            FormatScript::Superscript => 1,
            FormatScript::Subscript => 2,
        };
        self
    }

    /// Sets the indentation level, clamped to the XLSB range `0..=250`.
    #[must_use]
    pub const fn set_indent(mut self, indent: u8) -> Self {
        if indent <= 250 {
            self.xf.indent = indent;
        }
        self
    }

    /// Enables shrink-to-fit.
    #[must_use]
    pub const fn set_shrink(mut self) -> Self {
        self.xf.fShrinkToFit = true;
        self
    }

    /// Disables shrink-to-fit.
    #[must_use]
    pub const fn unset_shrink(mut self) -> Self {
        self.xf.fShrinkToFit = false;
        self
    }

    /// Sets text rotation in degrees.
    ///
    /// Valid inputs are `-90..=90` and `270` for vertical text. Other values
    /// are ignored.
    #[must_use]
    pub fn set_rotation(mut self, rotation: i16) -> Self {
        match rotation {
            270 => self.xf.trot = 255,
            -90..=-1 => {
                self.xf.trot = u8::try_from(90 - rotation).expect("rotation range checked");
            }
            0..=90 => {
                self.xf.trot = u8::try_from(rotation).expect("rotation range checked");
            }
            _ => {}
        }
        self
    }

    /// Sets reading direction: `0` context, `1` left-to-right, `2` right-to-left.
    #[must_use]
    pub const fn set_reading_direction(mut self, reading_direction: u8) -> Self {
        self.xf.iReadingOrder = match reading_direction {
            0 => ReadingOrder::ContextDependent,
            1 => ReadingOrder::LeftToRight,
            2 => ReadingOrder::RightToLeft,
            _ => return self,
        };
        self
    }

    /// Sets the font scheme.
    #[must_use]
    pub const fn set_font_scheme(mut self, scheme: FontScheme) -> Self {
        self.font.bFontScheme = match scheme {
            FontScheme::None => 0,
            FontScheme::Headings => 1,
            FontScheme::Body => 2,
        };
        self
    }

    /// Sets the font family.
    #[must_use]
    pub const fn set_font_family(mut self, font_family: u8) -> Self {
        self.font.bFamily = font_family;
        self
    }

    /// Sets the font character set.
    #[must_use]
    pub const fn set_font_charset(mut self, font_charset: u8) -> Self {
        self.font.bCharSet = font_charset;
        self
    }

    /// Sets the diagonal border style.
    #[must_use]
    pub const fn set_border_diagonal(mut self, border: FormatBorder) -> Self {
        self.border.blxfDiag.border_type = border_type(border);
        self
    }

    /// Sets the diagonal border color.
    #[must_use]
    pub fn set_border_diagonal_color(mut self, color: impl Into<Color>) -> Self {
        self.border_diagonal_color = Some(color.into());
        self
    }

    /// Sets the diagonal border direction.
    #[must_use]
    pub const fn set_border_diagonal_type(mut self, border_type: FormatDiagonalBorder) -> Self {
        match border_type {
            FormatDiagonalBorder::None => {
                self.border.fBdrDiagDown = false;
                self.border.fBdrDiagUp = false;
            }
            FormatDiagonalBorder::BorderUp => {
                self.border.fBdrDiagDown = false;
                self.border.fBdrDiagUp = true;
            }
            FormatDiagonalBorder::BorderDown => {
                self.border.fBdrDiagDown = true;
                self.border.fBdrDiagUp = false;
            }
            FormatDiagonalBorder::BorderUpDown => {
                self.border.fBdrDiagDown = true;
                self.border.fBdrDiagUp = true;
            }
        }
        self
    }

    /// Marks the cell text as quote-prefixed.
    #[must_use]
    pub const fn set_quote_prefix(mut self) -> Self {
        self.xf.f123Prefix = true;
        self
    }

    /// Clears the quote-prefix marker.
    #[must_use]
    pub const fn unset_quote_prefix(mut self) -> Self {
        self.xf.f123Prefix = false;
        self
    }

    /// Merges another format into this one and returns a new format.
    ///
    /// Properties already set on `self` take precedence over unset properties
    /// on `other`. Boolean attributes are unioned, except `locked`, which is
    /// intersected because it defaults to true.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.merge_with(other);
        merged
    }

    fn merge_with(&mut self, other: &Self) {
        self.xf.fLocked &= other.xf.fLocked;
        self.xf.fHidden |= other.xf.fHidden;
        self.xf.fWrap |= other.xf.fWrap;
        self.xf.fShrinkToFit |= other.xf.fShrinkToFit;
        self.xf.f123Prefix |= other.xf.f123Prefix;

        if self.number_format_spec.is_none() {
            self.number_format_spec
                .clone_from(&other.number_format_spec);
        }

        if self.font_color.is_none() {
            self.font_color = other.font_color;
        }
        if self.background_color.is_none() {
            self.background_color = other.background_color;
        }
        if self.foreground_color.is_none() {
            self.foreground_color = other.foreground_color;
        }
        if self.border_top_color.is_none() {
            self.border_top_color = other.border_top_color;
        }
        if self.border_bottom_color.is_none() {
            self.border_bottom_color = other.border_bottom_color;
        }
        if self.border_left_color.is_none() {
            self.border_left_color = other.border_left_color;
        }
        if self.border_right_color.is_none() {
            self.border_right_color = other.border_right_color;
        }
        if self.border_diagonal_color.is_none() {
            self.border_diagonal_color = other.border_diagonal_color;
        }

        if !self.pattern_set && self.fill.fls == 0 {
            self.fill.fls = other.fill.fls;
        }
        if self.fill.brtColorFore == BrtColor::default() {
            self.fill.brtColorFore = other.fill.brtColorFore.clone();
        }
        if self.fill.brtColorBack == BrtColor::default() {
            self.fill.brtColorBack = other.fill.brtColorBack.clone();
        }
        self.pattern_set |= other.pattern_set;

        merge_border_side(&mut self.border.blxfTop, &other.border.blxfTop);
        merge_border_side(&mut self.border.blxfBottom, &other.border.blxfBottom);
        merge_border_side(&mut self.border.blxfLeft, &other.border.blxfLeft);
        merge_border_side(&mut self.border.blxfRight, &other.border.blxfRight);
        merge_border_side(&mut self.border.blxfDiag, &other.border.blxfDiag);
        if !self.border.fBdrDiagUp && !self.border.fBdrDiagDown {
            self.border.fBdrDiagUp = other.border.fBdrDiagUp;
            self.border.fBdrDiagDown = other.border.fBdrDiagDown;
        }

        if self.xf.alc == HAlign::General {
            self.xf.alc = other.xf.alc.clone();
        }
        if !self.vertical_align_set {
            self.xf.alcv = other.xf.alcv.clone();
            self.vertical_align_set = other.vertical_align_set;
        }
        if self.xf.indent == 0 {
            self.xf.indent = other.xf.indent;
        }
        if self.xf.trot == 0 {
            self.xf.trot = other.xf.trot;
        }
        if self.xf.iReadingOrder == ReadingOrder::ContextDependent {
            self.xf.iReadingOrder = other.xf.iReadingOrder.clone();
        }

        self.font.bls = if self.font.bls == Bold::BLSBOLD || other.font.bls == Bold::BLSBOLD {
            Bold::BLSBOLD
        } else {
            Bold::BLSNORMAL
        };
        self.font.grbit.fItalic |= other.font.grbit.fItalic;
        self.font.grbit.fStrikeOut |= other.font.grbit.fStrikeOut;
        self.font.grbit.fOutline |= other.font.grbit.fOutline;
        self.font.grbit.fShadow |= other.font.grbit.fShadow;
        self.font.grbit.fCondense |= other.font.grbit.fCondense;
        self.font.grbit.fExtend |= other.font.grbit.fExtend;

        let default_font = BrtFont::default();
        if self.font.dyHeight == default_font.dyHeight {
            self.font.dyHeight = other.font.dyHeight;
        }
        if self.font.name == default_font.name {
            self.font.name.clone_from(&other.font.name);
        }
        if self.font.bFamily == default_font.bFamily {
            self.font.bFamily = other.font.bFamily;
        }
        if self.font.bCharSet == default_font.bCharSet {
            self.font.bCharSet = other.font.bCharSet;
        }
        if self.font.uls == 0 {
            self.font.uls = other.font.uls;
        }
        if self.font.sss == 0 {
            self.font.sss = other.font.sss;
        }
        if self.font.bFontScheme == default_font.bFontScheme {
            self.font.bFontScheme = other.font.bFontScheme;
        }
        if self.font.brtColor == default_font.brtColor {
            self.font.brtColor = other.font.brtColor.clone();
        }
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
        if let Some(color) = self.border_diagonal_color {
            border.blxfDiag.color = color.to_brt_color()?;
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

fn merge_border_side(
    target: &mut rust_xlsb_core::prelude::Blxf,
    other: &rust_xlsb_core::prelude::Blxf,
) {
    if target.border_type == BorderType::None {
        target.border_type = other.border_type.clone();
    }
    if target.color == BrtColor::default() {
        target.color = other.color.clone();
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
    fn merge_fills_unset_properties() {
        let base = Format::new().set_bold();
        let other = Format::new()
            .set_italic()
            .set_font_name("Arial")
            .set_font_size(14)
            .set_font_color(Color::Blue)
            .set_num_format("0.00")
            .set_background_color(Color::Yellow)
            .set_foreground_color(Color::Red)
            .set_border(FormatBorder::Thin)
            .set_border_top(FormatBorder::Double)
            .set_border_top_color(Color::Green)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap()
            .set_unlocked();
        let merged = base.merge(&other);
        let style = cell_style(&merged);

        assert_eq!(style.font.bls, Bold::BLSBOLD);
        assert!(style.font.grbit.fItalic);
        assert_eq!(
            style.font.name.to_optional_string().as_deref(),
            Some("Arial")
        );
        assert_eq!(style.font.dyHeight, 280);
        assert_eq!(style.font.brtColor, BrtColor::rgb(0x00, 0x00, 0xFF));
        assert_eq!(style.xf.iFmt, 164);
        assert_eq!(style.fill.fls, 1);
        assert_eq!(style.fill.brtColorFore, BrtColor::rgb(0xFF, 0x00, 0x00));
        assert_eq!(style.fill.brtColorBack, BrtColor::rgb(0xFF, 0xFF, 0x00));
        assert_eq!(style.border.blxfTop.border_type, BorderType::Double);
        assert_eq!(style.border.blxfBottom.border_type, BorderType::Thin);
        assert_eq!(style.border.blxfTop.color, BrtColor::rgb(0x00, 0x80, 0x00));
        assert_eq!(style.xf.alc, HAlign::Center);
        assert_eq!(style.xf.alcv, VAlign::Center);
        assert!(style.xf.fWrap);
        assert!(!style.xf.fLocked);

        let base_style = cell_style(&base);
        assert_eq!(base_style.font.bls, Bold::BLSBOLD);
        assert!(!base_style.font.grbit.fItalic);
    }

    #[test]
    fn merge_preserves_explicit_top_alignment() {
        let merged = Format::new()
            .set_align(FormatAlign::Top)
            .merge(&Format::new().set_align(FormatAlign::VerticalCenter));
        let style = cell_style(&merged);

        assert_eq!(style.xf.alcv, VAlign::Top);
    }

    #[test]
    fn merge_preserves_explicit_diagonal_direction() {
        let merged = Format::new()
            .set_border_diagonal_type(FormatDiagonalBorder::BorderDown)
            .merge(&Format::new().set_border_diagonal_type(FormatDiagonalBorder::BorderUp));
        let style = cell_style(&merged);

        assert!(style.border.fBdrDiagDown);
        assert!(!style.border.fBdrDiagUp);
    }

    #[test]
    fn merge_is_identity_for_each_format() {
        let complete = Format::new()
            .set_bold()
            .set_italic()
            .set_font_strikethrough()
            .set_underline(FormatUnderline::Double)
            .set_font_script(FormatScript::Superscript)
            .set_font_name("Arial")
            .set_font_size(12.0)
            .set_font_scheme(FontScheme::Headings)
            .set_font_family(3)
            .set_font_charset(2)
            .set_font_color(Color::Blue)
            .set_num_format("0.00")
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_text_wrap()
            .set_shrink()
            .set_indent(2)
            .set_rotation(45)
            .set_reading_direction(2)
            .set_pattern(FormatPattern::Solid)
            .set_background_color(Color::Red)
            .set_foreground_color(Color::Yellow)
            .set_border(FormatBorder::Thin)
            .set_border_top(FormatBorder::Double)
            .set_border_color(Color::Blue)
            .set_border_top_color(Color::Green)
            .set_border_diagonal(FormatBorder::Thick)
            .set_border_diagonal_color(Color::Orange)
            .set_border_diagonal_type(FormatDiagonalBorder::BorderUp)
            .set_unlocked()
            .set_hidden()
            .set_quote_prefix();

        assert_eq!(Format::new().merge(&complete), complete);
        assert_eq!(complete.merge(&Format::new()), complete);
    }

    #[test]
    fn merge_keeps_explicit_none_pattern() {
        let merged = Format::new()
            .set_pattern(FormatPattern::None)
            .merge(&Format::new().set_background_color(Color::Red));
        let style = cell_style(&merged);

        assert_eq!(style.fill.fls, 0);
    }

    #[test]
    fn merge_keeps_first_format_values_and_unions_flags() {
        let base = Format::new()
            .set_italic()
            .set_font_name("Arial")
            .set_font_size(10.0)
            .set_num_format("0")
            .set_border_top(FormatBorder::Thin)
            .set_border_top_color(Color::Red)
            .set_align(FormatAlign::Left)
            .set_hidden();
        let other = Format::new()
            .set_font_name("Calibri")
            .set_font_size(14.0)
            .set_num_format("0.00")
            .set_border_top(FormatBorder::Thick)
            .set_border_top_color(Color::Blue)
            .set_align(FormatAlign::Right)
            .set_text_wrap()
            .set_quote_prefix();
        let style = cell_style(&base.merge(&other));

        assert_eq!(
            style.font.name.to_optional_string().as_deref(),
            Some("Arial")
        );
        assert_eq!(style.font.dyHeight, 200);
        assert_eq!(style.xf.iFmt, 164);
        assert_eq!(style.border.blxfTop.border_type, BorderType::Thin);
        assert_eq!(style.border.blxfTop.color, BrtColor::rgb(0xFF, 0x00, 0x00));
        assert_eq!(style.xf.alc, HAlign::Left);
        assert!(style.xf.fHidden);
        assert!(style.xf.fWrap);
        assert!(style.xf.f123Prefix);
    }

    #[test]
    fn unset_hyperlink_style_restores_default_format() {
        assert_eq!(
            Format::new().set_hyperlink().unset_hyperlink_style(),
            Format::new()
        );
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
    fn new_format_is_locked_and_unlocked_format_gets_its_own_xf() {
        let mut registry = StyleRegistry::new();
        let default = Format::new()
            .to_cell_style(&mut registry)
            .expect("valid format");
        assert!(default.xf.fLocked);
        assert_eq!(registry.add(default).expect("adds"), 0);

        let unlocked = Format::new()
            .set_unlocked()
            .to_cell_style(&mut registry)
            .expect("valid format");
        assert!(!unlocked.xf.fLocked);
        assert_ne!(registry.add(unlocked).expect("adds"), 0);
    }

    #[test]
    fn maps_font_decoration_and_metadata() {
        let style = cell_style(
            &Format::new()
                .set_underline(FormatUnderline::DoubleAccounting)
                .set_font_script(FormatScript::Superscript)
                .set_font_strikethrough()
                .set_font_family(3)
                .set_font_charset(1)
                .set_font_scheme(FontScheme::Headings),
        );

        assert_eq!(style.font.uls, 0x22);
        assert_eq!(style.font.sss, 1);
        assert!(style.font.grbit.fStrikeOut);
        assert_eq!(style.font.bFamily, 3);
        assert_eq!(style.font.bCharSet, 1);
        assert_eq!(style.font.bFontScheme, 1);
    }

    #[test]
    fn maps_alignment_extras_and_protection_resets() {
        let style = cell_style(
            &Format::new()
                .set_unlocked()
                .set_indent(2)
                .set_shrink()
                .set_rotation(-30)
                .set_reading_direction(2)
                .set_quote_prefix(),
        );

        assert!(!style.xf.fLocked);
        assert_eq!(style.xf.indent, 2);
        assert!(style.xf.fShrinkToFit);
        assert_eq!(style.xf.trot, 120);
        assert_eq!(style.xf.iReadingOrder, ReadingOrder::RightToLeft);
        assert!(style.xf.f123Prefix);

        let style = cell_style(
            &Format::new()
                .set_reading_direction(1)
                .set_reading_direction(9),
        );
        assert_eq!(style.xf.iReadingOrder, ReadingOrder::LeftToRight);

        let style = cell_style(
            &Format::new()
                .unset_bold()
                .unset_italic()
                .unset_text_wrap()
                .unset_shrink()
                .unset_font_strikethrough()
                .unset_quote_prefix(),
        );
        assert_eq!(style.font.bls, Bold::BLSNORMAL);
        assert!(!style.font.grbit.fItalic);
        assert!(!style.xf.fWrap);
        assert!(!style.xf.fShrinkToFit);
        assert!(!style.font.grbit.fStrikeOut);
        assert!(!style.xf.f123Prefix);
    }

    #[test]
    fn maps_diagonal_border() {
        let style = cell_style(
            &Format::new()
                .set_border_diagonal(FormatBorder::Thin)
                .set_border_diagonal_color(Color::Red)
                .set_border_diagonal_type(FormatDiagonalBorder::BorderUpDown),
        );

        assert_eq!(style.border.blxfDiag.border_type, BorderType::Thin);
        assert_eq!(style.border.blxfDiag.color, BrtColor::rgb(0xFF, 0x00, 0x00));
        assert!(style.border.fBdrDiagDown);
        assert!(style.border.fBdrDiagUp);
    }

    #[test]
    fn rotation_boundaries() {
        assert_eq!(cell_style(&Format::new().set_rotation(270)).xf.trot, 255);
        assert_eq!(cell_style(&Format::new().set_rotation(90)).xf.trot, 90);
        assert_eq!(cell_style(&Format::new().set_rotation(-90)).xf.trot, 180);
        assert_eq!(cell_style(&Format::new().set_rotation(100)).xf.trot, 0);
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
