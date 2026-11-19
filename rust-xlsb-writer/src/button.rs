//! Form-control buttons and their VML shapes.

use std::borrow::Cow;
use std::fmt::Write;

use crate::drawing::{
    EMU_PER_PIXEL, absolute_column_pixels, absolute_row_pixels, advance_columns, advance_rows,
    column_pixels, row_pixels, vml_dpi_size,
};
use crate::url::{escape_xml_attribute, escape_xml_data};
use crate::worksheet::{MAX_COL, MAX_ROW, Worksheet};

/// A form-control button placed on a worksheet.
///
/// Buttons are inserted with
/// [`Worksheet::insert_button`](crate::Worksheet::insert_button) or
/// [`Worksheet::insert_button_with_offset`](crate::Worksheet::insert_button_with_offset).
/// The button uses Excel's default 64x20 pixel size.
#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    caption: String,
    macro_name: String,
    alt_text: String,
    width: f64,
    height: f64,
    x_offset: u32,
    y_offset: u32,
}

impl Default for Button {
    fn default() -> Self {
        Self::new()
    }
}

impl Button {
    /// Creates a button with no caption or macro.
    ///
    /// The emitted VML supplies `Button N` and `[0]!ButtonN_Click` when the
    /// corresponding value is not set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            caption: String::new(),
            macro_name: String::new(),
            alt_text: String::new(),
            width: 64.0,
            height: 20.0,
            x_offset: 0,
            y_offset: 0,
        }
    }

    /// Sets the caption. Captions longer than 255 UTF-16 code units are ignored.
    #[must_use]
    pub fn set_caption(mut self, caption: impl Into<String>) -> Self {
        let caption = caption.into();
        if caption.encode_utf16().count() <= 255 {
            self.caption = caption;
        }
        self
    }

    /// Sets the macro run by the button. The emitted value is prefixed with
    /// `[0]!`. Names longer than 255 UTF-16 code units are ignored.
    #[must_use]
    pub fn set_macro(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if name.encode_utf16().count() <= 255 {
            self.macro_name = name;
        }
        self
    }

    /// Sets the alternative text shown by accessibility tools.
    ///
    /// Text longer than 255 UTF-16 code units is ignored.
    #[must_use]
    pub fn set_alt_text(mut self, alt_text: impl Into<String>) -> Self {
        let alt_text = alt_text.into();
        if alt_text.encode_utf16().count() <= 255 {
            self.alt_text = alt_text;
        }
        self
    }

    /// Sets the button width in pixels. Zero is ignored.
    #[must_use]
    pub fn set_width(mut self, width: u32) -> Self {
        if width != 0 {
            self.width = f64::from(width);
        }
        self
    }

    /// Sets the button height in pixels. Zero is ignored.
    #[must_use]
    pub fn set_height(mut self, height: u32) -> Self {
        if height != 0 {
            self.height = f64::from(height);
        }
        self
    }

    pub(crate) const fn set_offset(mut self, x_offset: u32, y_offset: u32) -> Self {
        self.x_offset = x_offset;
        self.y_offset = y_offset;
        self
    }

    pub(crate) fn caption(&self) -> &str {
        &self.caption
    }

    pub(crate) fn macro_name(&self) -> &str {
        &self.macro_name
    }

    pub(crate) fn alt_text(&self) -> &str {
        &self.alt_text
    }

    pub(crate) const fn width(&self) -> f64 {
        self.width
    }

    pub(crate) const fn height(&self) -> f64 {
        self.height
    }

    pub(crate) const fn x_offset(&self) -> u32 {
        self.x_offset
    }

    pub(crate) const fn y_offset(&self) -> u32 {
        self.y_offset
    }
}

/// Writes the button shapetype and all button shapes for a worksheet.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the note VML writer"
)]
pub(crate) fn write_button_shapes(
    out: &mut String,
    sheet: &Worksheet,
    shape_id: &mut u32,
    z_index: &mut u32,
) {
    if sheet.buttons().is_empty() {
        return;
    }
    out.push_str(concat!(
        r#"<v:shapetype id="_x0000_t201" coordsize="21600,21600" o:spt="201" path="m,l,21600r21600,l21600,xe">"#,
        r#"<v:stroke joinstyle="miter" /><v:path shadowok="f" o:extrusionok="f" strokeok="f" fillok="f" o:connecttype="rect" /><o:lock v:ext="edit" shapetype="t" /></v:shapetype>"#,
    ));
    for (index, (&(row, col, _, _), button)) in sheet.buttons().iter().enumerate() {
        *shape_id += 1;
        *z_index += 1;
        write_button_shape(out, sheet, row, col, button, index + 1, *shape_id, *z_index);
    }
}

fn write_button_shape(
    out: &mut String,
    sheet: &Worksheet,
    row: u32,
    col: u16,
    button: &Button,
    number: usize,
    shape_id: u32,
    z_index: u32,
) {
    let anchor = button_anchor(sheet, row, col, button);
    let left = vml_dpi_size(
        absolute_column_pixels(sheet, anchor.start_col) + f64::from(anchor.start_col_off),
    );
    let top = vml_dpi_size(
        absolute_row_pixels(sheet, anchor.start_row) + f64::from(anchor.start_row_off),
    );
    let width = vml_dpi_size(button.width());
    let height = vml_dpi_size(button.height());
    write!(out, r#"<v:shape id="_x0000_s{shape_id}" "#).expect("writing to String cannot fail");
    if !button.alt_text().is_empty() {
        write!(out, r#"alt="{}" "#, escape_xml_attribute(button.alt_text()))
            .expect("writing to String cannot fail");
    }
    write!(
        out,
        r##"type="#_x0000_t201" style="position:absolute;margin-left:{left}pt;margin-top:{top}pt;width:{width}pt;height:{height}pt;z-index:{z_index};mso-wrap-style:tight" o:button="t" fillcolor="buttonFace [67]" strokecolor="windowText [64]" o:insetmode="auto"><v:fill color2="buttonFace [67]" o:detectmouseclick="t" /><o:lock v:ext="edit" rotation="t" /><v:textbox style="mso-direction-alt:auto" o:singleclick="f"><div style="text-align:center"><font face="Calibri" size="220" color="#000000">{caption}</font></div></v:textbox><x:ClientData ObjectType="Button"><x:Anchor>{},{},{},{},{},{},{},{}</x:Anchor><x:PrintObject>False</x:PrintObject><x:AutoFill>False</x:AutoFill><x:FmlaMacro>{macro_name}</x:FmlaMacro><x:TextHAlign>Center</x:TextHAlign><x:TextVAlign>Center</x:TextVAlign></x:ClientData></v:shape>"##,
        anchor.start_col,
        anchor.start_col_off,
        anchor.start_row,
        anchor.start_row_off,
        anchor.end_col,
        anchor.end_col_off,
        anchor.end_row,
        anchor.end_row_off,
        caption = caption(button, number),
        macro_name = macro_text(button, number),
    )
    .expect("writing to String cannot fail");
}

fn caption(button: &Button, number: usize) -> Cow<'_, str> {
    if button.caption().is_empty() {
        Cow::Owned(format!("Button {number}"))
    } else {
        escape_xml_data(button.caption())
    }
}

fn macro_text(button: &Button, number: usize) -> Cow<'_, str> {
    if button.macro_name().is_empty() {
        Cow::Owned(format!("[0]!Button{number}_Click"))
    } else {
        Cow::Owned(format!("[0]!{}", escape_xml_data(button.macro_name())))
    }
}

struct ButtonAnchor {
    start_col: u16,
    start_col_off: u32,
    start_row: u32,
    start_row_off: u32,
    end_col: u16,
    end_col_off: u64,
    end_row: u32,
    end_row_off: u64,
}

fn button_anchor(sheet: &Worksheet, row: u32, col: u16, button: &Button) -> ButtonAnchor {
    let (mut start_col, mut start_col_off) = (col, button.x_offset());
    let (mut start_row, mut start_row_off) = (row, button.y_offset());
    while start_col < MAX_COL && f64::from(start_col_off) >= column_pixels(sheet, start_col) {
        start_col_off = start_col_off.saturating_sub(pixel_u32(column_pixels(sheet, start_col)));
        start_col += 1;
    }
    while start_row < MAX_ROW && f64::from(start_row_off) >= row_pixels(sheet, start_row) {
        start_row_off = start_row_off.saturating_sub(pixel_u32(row_pixels(sheet, start_row)));
        start_row += 1;
    }
    let (mut end_col, end_col_emu) =
        advance_columns(sheet, start_col, start_col_off, button.width());
    let (mut end_row, end_row_emu) = advance_rows(sheet, start_row, start_row_off, button.height());
    let mut end_col_off = end_col_emu / EMU_PER_PIXEL;
    let mut end_row_off = end_row_emu / EMU_PER_PIXEL;
    if end_col < MAX_COL && end_col_off >= u64::from(pixel_u32(column_pixels(sheet, end_col))) {
        end_col += 1;
        end_col_off = 0;
    }
    if end_row < MAX_ROW && end_row_off >= u64::from(pixel_u32(row_pixels(sheet, end_row))) {
        end_row += 1;
        end_row_off = 0;
    }
    ButtonAnchor {
        start_col,
        start_col_off,
        start_row,
        start_row_off,
        end_col,
        end_col_off,
        end_row,
        end_row_off,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell pixel dimensions are non-negative"
)]
const fn pixel_u32(value: f64) -> u32 {
    value.round() as u32
}

#[cfg(test)]
mod tests {
    use super::Button;
    use crate::Worksheet;

    #[test]
    fn button_vml_emits_defaults_and_escaped_values() {
        let mut sheet = Worksheet::new();
        sheet
            .insert_button(2, 1, &Button::new().set_caption("Press \"Me\" & <Go>"))
            .expect("button");
        let vml = crate::note::vml_xml(&sheet, 1);
        assert!(vml.contains(r#"<v:shapetype id="_x0000_t201""#), "{vml}");
        assert!(vml.contains(r#"id="_x0000_s1025""#), "{vml}");
        assert!(vml.contains("Press \"Me\" &amp; &lt;Go&gt;"), "{vml}");
        assert!(
            vml.contains("<x:FmlaMacro>[0]!Button1_Click</x:FmlaMacro>"),
            "{vml}"
        );
        assert!(
            vml.contains("<x:Anchor>1,0,2,0,2,0,3,0</x:Anchor>"),
            "{vml}"
        );
    }

    #[test]
    fn button_offsets_and_alt_text_change_the_anchor() {
        let mut sheet = Worksheet::new();
        let button = Button::new()
            .set_caption("Go")
            .set_macro("run_it")
            .set_alt_text("a \"quote\"")
            .set_width(100)
            .set_height(30);
        sheet
            .insert_button_with_offset(1, 1, &button, 10, 5)
            .expect("button");
        let vml = crate::note::vml_xml(&sheet, 1);
        assert!(vml.contains(r#"alt="a &quot;quote&quot;""#), "{vml}");
        assert!(
            vml.contains("<x:FmlaMacro>[0]!run_it</x:FmlaMacro>"),
            "{vml}"
        );
        assert!(vml.contains("width:75pt;height:22.5pt"), "{vml}");
        assert!(vml.contains("<x:Anchor>1,10,1,5,"), "{vml}");
    }

    #[test]
    fn button_limits_keep_previous_values() {
        let mut sheet = Worksheet::new();
        let button = Button::new()
            .set_caption("keep")
            .set_caption("x".repeat(256))
            .set_macro("keep")
            .set_macro("x".repeat(256))
            .set_alt_text("keep")
            .set_alt_text("x".repeat(256))
            .set_width(0)
            .set_height(0);
        sheet.insert_button(0, 0, &button).expect("button");
        let vml = crate::note::vml_xml(&sheet, 1);
        assert!(vml.contains(">keep</font>"), "{vml}");
        assert!(vml.contains("<x:FmlaMacro>[0]!keep</x:FmlaMacro>"), "{vml}");
        assert!(vml.contains(r#"alt="keep""#), "{vml}");
        assert!(vml.contains("width:48pt;height:15pt"), "{vml}");
    }

    #[test]
    fn worksheet_rejects_out_of_range_buttons() {
        let mut sheet = Worksheet::new();
        assert!(matches!(
            sheet.insert_button(crate::worksheet::MAX_ROW + 1, 0, &Button::new()),
            Err(crate::Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_button(0, crate::worksheet::MAX_COL + 1, &Button::new()),
            Err(crate::Error::RowColumnLimit)
        ));
    }
}
