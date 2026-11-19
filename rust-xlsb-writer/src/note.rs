//! Cell notes and their VML/comments payloads.

/// A non-threaded Excel cell note.
///
/// Notes are inserted with [`Worksheet::insert_note`](crate::Worksheet::insert_note).
/// Text is stored as one plain rich-text run; author-prefix formatting and
/// threaded comments are not part of this slice.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    text: String,
    author: Option<String>,
    visible: Option<bool>,
    width: f64,
    height: f64,
    fill_color: u32,
    alt_text: String,
}

impl Note {
    /// Creates a note with the default 128x74 pixel box.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            author: None,
            visible: None,
            width: 128.0,
            height: 74.0,
            fill_color: 0x00FF_FFE1,
            alt_text: String::new(),
        }
    }

    /// Sets the note author. Names longer than 52 UTF-16 code units are ignored.
    #[must_use]
    pub fn set_author(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if !name.is_empty() && name.encode_utf16().count() <= 52 {
            self.author = Some(name);
        }
        self
    }

    /// Makes the note visible when the file opens.
    #[must_use]
    pub const fn set_visible(mut self, enable: bool) -> Self {
        self.visible = Some(enable);
        self
    }

    /// Sets the note background color.
    ///
    /// Theme, default, and automatic colors are ignored because Excel notes
    /// accept only RGB colors.
    #[must_use]
    pub fn set_background_color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        let rgb = match color {
            Color::RGB(value) if value <= 0x00FF_FFFF => Some(value),
            Color::Default | Color::Automatic | Color::Theme(_, _) | Color::RGB(_) => None,
            named => named.named_rgb(),
        };
        if let Some(rgb) = rgb {
            self.fill_color = rgb;
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

    /// Replaces the note text so one note can be reused.
    pub fn reset_text(&mut self, text: impl Into<String>) -> &mut Self {
        let text = text.into();
        self.text.clear();
        self.text.push_str(&text);
        self
    }

    /// Sets the note box width in pixels. Zero is ignored.
    #[must_use]
    pub fn set_width(mut self, width: u32) -> Self {
        if width != 0 {
            self.width = f64::from(width);
        }
        self
    }

    /// Sets the note box height in pixels. Zero is ignored.
    #[must_use]
    pub fn set_height(mut self, height: u32) -> Self {
        if height != 0 {
            self.height = f64::from(height);
        }
        self
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub(crate) fn is_visible(&self, all_visible: bool) -> bool {
        self.visible.unwrap_or(all_visible)
    }

    pub(crate) const fn width(&self) -> f64 {
        self.width
    }

    pub(crate) const fn height(&self) -> f64 {
        self.height
    }

    pub(crate) const fn fill_color(&self) -> u32 {
        self.fill_color
    }

    pub(crate) fn alt_text(&self) -> &str {
        &self.alt_text
    }
}

use std::fmt::Write;

use rust_xlsb_core::prelude::{
    BrtBeginComment, BrtBeginCommentAuthors, BrtBeginCommentList, BrtBeginComments,
    BrtCommentAuthor, BrtCommentText, BrtEndComment, BrtEndCommentAuthors, BrtEndCommentList,
    BrtEndComments, RichStr, StrRun, StrRuns, UncheckedRfX, XlStringExt, XlWideString,
};
use rust_xlsb_core::push_record;

use crate::Result;
use crate::drawing::{
    absolute_column_pixels, absolute_row_pixels, advance_columns, advance_rows, column_pixels,
    row_pixels,
};
use crate::format::Color;
use crate::worksheet::{MAX_COL, Worksheet};

const EMU_PER_PIXEL: u64 = 9_525;

/// Builds the binary `commentsN.bin` part for one worksheet.
///
/// # Errors
///
/// Returns an error when a comment record cannot be serialized.
pub fn comments_bin(sheet: &Worksheet) -> Result<Vec<u8>> {
    let default_author = sheet.default_note_author();
    let mut authors: Vec<&str> = Vec::new();
    for note in sheet.notes().values() {
        let author = note_author(note, default_author);
        if !authors.contains(&author) {
            authors.push(author);
        }
    }

    let mut out = Vec::with_capacity(64 + authors.len() * 16 + sheet.notes().len() * 80);
    push_record(&mut out, &BrtBeginComments::default())?;
    push_record(&mut out, &BrtBeginCommentAuthors::default())?;
    for author in &authors {
        push_record(
            &mut out,
            &BrtCommentAuthor {
                author: XlWideString::construct(Some(*author)),
            },
        )?;
    }
    push_record(&mut out, &BrtEndCommentAuthors::default())?;
    push_record(&mut out, &BrtBeginCommentList::default())?;
    for (&(row, col), note) in sheet.notes() {
        let author = note_author(note, default_author);
        let iauthor = u32::try_from(
            authors
                .iter()
                .position(|known| *known == author)
                .expect("author"),
        )
        .map_err(|_| crate::Error::ValueNotRepresentable("too many authors"))?;
        push_record(
            &mut out,
            &BrtBeginComment {
                iauthor,
                rfx: UncheckedRfX {
                    rwFirst: row,
                    rwLast: row,
                    colFirst: u32::from(col),
                    colLast: u32::from(col),
                },
                guid: [0; 16],
            },
        )?;
        let text = note.text();
        let runs = if text.is_empty() {
            Vec::new()
        } else {
            vec![StrRun { ich: 0, ifnt: 0 }]
        };
        push_record(
            &mut out,
            &BrtCommentText {
                text: RichStr {
                    flags: RichStr::F_RICH_STR,
                    str: XlWideString::construct(Some(text)),
                    rich: Some(StrRuns {
                        dwSizeStrRun: u32::try_from(runs.len())
                            .map_err(|_| crate::Error::ValueNotRepresentable("too many runs"))?,
                        rgsStrRun: runs,
                    }),
                    phonetic: None,
                },
            },
        )?;
        push_record(&mut out, &BrtEndComment::default())?;
    }
    push_record(&mut out, &BrtEndCommentList::default())?;
    push_record(&mut out, &BrtEndComments::default())?;
    Ok(out)
}

/// Number of 1024-id VML blocks needed by a worksheet's notes.
pub fn note_vml_blocks(sheet: &Worksheet) -> u32 {
    u32::try_from(sheet.notes().len().div_ceil(1024).max(1)).expect("note blocks fit u32")
}

/// Builds the note VML drawing for one worksheet.
pub fn vml_notes_xml(sheet: &Worksheet, data_id: u32) -> String {
    let visible_all = sheet.all_notes_visible();
    let blocks = note_vml_blocks(sheet);
    let mut data_ids = String::with_capacity(usize::try_from(blocks).expect("fits usize") * 5);
    for block in 0..blocks {
        if block != 0 {
            data_ids.push(',');
        }
        write!(data_ids, "{}", data_id + block).expect("writing to String cannot fail");
    }
    let mut out = String::with_capacity(640 + sheet.notes().len() * 620);
    out.push_str(r#"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:x="urn:schemas-microsoft-com:office:excel">"#);
    write!(
        out,
        r#"<o:shapelayout v:ext="edit"><o:idmap v:ext="edit" data="{data_ids}" /></o:shapelayout>"#
    )
    .expect("writing to String cannot fail");
    out.push_str(concat!(
        r#"<v:shapetype id="_x0000_t202" coordsize="21600,21600" o:spt="202" path="m,l,21600r21600,l21600,xe">"#,
        r#"<v:stroke joinstyle="miter" /><v:path gradientshapeok="t" o:connecttype="rect" /></v:shapetype>"#,
    ));
    for (index, (&(row, col), note)) in sheet.notes().iter().enumerate() {
        let anchor = note_anchor(sheet, row, col, note.width(), note.height());
        let left = vml_dpi_size(
            absolute_column_pixels(sheet, anchor.start_col) + f64::from(anchor.start_col_off),
        );
        let top = vml_dpi_size(
            absolute_row_pixels(sheet, anchor.start_row) + f64::from(anchor.start_row_off),
        );
        let width = vml_dpi_size(note.width());
        let height = vml_dpi_size(note.height());
        let visible = note.is_visible(visible_all);
        let visibility = if visible { "visible" } else { "hidden" };
        let fill_color = note.fill_color();
        let shape_id = 1025 + (data_id - 1) * 1024 + u32::try_from(index).expect("fits u32");
        write!(out, r#"<v:shape id="_x0000_s{shape_id}" "#).expect("writing to String cannot fail");
        if !note.alt_text().is_empty() {
            write!(
                out,
                r#"alt="{}" "#,
                crate::url::escape_xml_attribute(note.alt_text())
            )
            .expect("writing to String cannot fail");
        }
        write!(
            out,
            r##"type="#_x0000_t202" style="position:absolute;margin-left:{left}pt;margin-top:{top}pt;width:{width}pt;height:{height}pt;z-index:{index};visibility:{visibility};" fillcolor="#{fill_color:06X}" o:insetmode="auto"><v:fill color2="#{fill_color:06X}" /><v:shadow on="t" color="black" obscured="t" /><v:path o:connecttype="none" /><v:textbox style="mso-direction-alt:auto"><div style="text-align:Left" /></v:textbox><x:ClientData ObjectType="Note"><x:MoveWithCells /><x:SizeWithCells /><x:Anchor>{},{},{},{},{},{},{},{}</x:Anchor><x:AutoFill>False</x:AutoFill><x:Row>{row}</x:Row><x:Column>{col}</x:Column>"##,
            anchor.start_col,
            anchor.start_col_off,
            anchor.start_row,
            anchor.start_row_off,
            anchor.end_col,
            anchor.end_col_off,
            anchor.end_row,
            anchor.end_row_off,
        )
        .expect("writing to String cannot fail");
        if visible {
            out.push_str("<x:Visible />");
        }
        out.push_str("</x:ClientData></v:shape>");
    }
    out.push_str("</xml>");
    out
}

fn note_author<'a>(note: &'a Note, default: Option<&'a str>) -> &'a str {
    note.author().unwrap_or_else(|| default.unwrap_or("Author"))
}

struct NoteAnchor {
    start_col: u16,
    start_col_off: u32,
    start_row: u32,
    start_row_off: u32,
    end_col: u16,
    end_col_off: u64,
    end_row: u32,
    end_row_off: u64,
}

fn note_anchor(sheet: &Worksheet, row: u32, col: u16, width: f64, height: f64) -> NoteAnchor {
    let (mut start_col, mut start_col_off) = note_start_col(col);
    let (mut start_row, mut start_row_off) = note_start_row(row);
    while start_col < MAX_COL && f64::from(start_col_off) >= column_pixels(sheet, start_col) {
        start_col_off = start_col_off.saturating_sub(pixel_u32(column_pixels(sheet, start_col)));
        start_col += 1;
    }
    while start_row < crate::worksheet::MAX_ROW
        && f64::from(start_row_off) >= row_pixels(sheet, start_row)
    {
        start_row_off = start_row_off.saturating_sub(pixel_u32(row_pixels(sheet, start_row)));
        start_row += 1;
    }
    let (end_col, end_col_emu) = advance_columns(sheet, start_col, start_col_off, width);
    let (end_row, end_row_emu) = advance_rows(sheet, start_row, start_row_off, height);
    NoteAnchor {
        start_col,
        start_col_off,
        start_row,
        start_row_off,
        end_col,
        end_col_off: end_col_emu / EMU_PER_PIXEL,
        end_row,
        end_row_off: end_row_emu / EMU_PER_PIXEL,
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

const fn note_start_col(col: u16) -> (u16, u32) {
    if col == MAX_COL - 2 {
        (MAX_COL - 5, 49)
    } else if col == MAX_COL - 1 {
        (MAX_COL - 4, 49)
    } else if col == MAX_COL {
        (MAX_COL - 3, 49)
    } else {
        (col + 1, 15)
    }
}

const fn note_start_row(row: u32) -> (u32, u32) {
    let max = crate::worksheet::MAX_ROW;
    if row == 0 {
        (0, 2)
    } else if row == max - 2 {
        (max - 6, 16)
    } else if row == max - 1 {
        (max - 5, 16)
    } else if row == max {
        (max - 4, 14)
    } else {
        (row - 1, 10)
    }
}

fn vml_dpi_size(dimension: f64) -> f64 {
    (dimension + 0.25).floor() * 72.0 / 96.0
}

#[cfg(test)]
mod tests {
    use super::{comments_bin, vml_notes_xml};
    use crate::format::Color;
    use crate::{Note, Worksheet};

    #[test]
    fn notes_emit_authors_visibility_and_cell_reference() {
        let mut sheet = Worksheet::new();
        sheet.set_default_note_author("Author");
        sheet.insert_note(0, 0, &Note::new("First")).expect("note");
        sheet
            .insert_note(
                2,
                2,
                &Note::new("Other").set_author("Bob").set_visible(true),
            )
            .expect("note");
        sheet.show_all_notes(true);

        let comments = comments_bin(&sheet).expect("comments");
        assert_eq!(&comments[..5], &[0xF4, 0x04, 0, 0xF6, 0x04]);

        let vml = vml_notes_xml(&sheet, 1);
        assert!(
            vml.contains(r#"<o:idmap v:ext="edit" data="1" />"#),
            "{vml}"
        );
        assert!(vml.contains("visibility:visible;"));
        assert!(vml.contains("<x:Row>0</x:Row><x:Column>0</x:Column>"));
        assert!(vml.contains("<x:Row>2</x:Row><x:Column>2</x:Column>"));
        assert!(vml.contains("<x:Visible />"));
    }

    #[test]
    fn note_validation_rejects_bad_cells_and_text() {
        let mut sheet = Worksheet::new();
        assert!(matches!(
            sheet.insert_note(crate::worksheet::MAX_ROW + 1, 0, &Note::new("x")),
            Err(crate::Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_note(0, crate::worksheet::MAX_COL + 1, &Note::new("x")),
            Err(crate::Error::RowColumnLimit)
        ));
        let long = "x".repeat(32_768);
        assert!(matches!(
            sheet.insert_note(0, 0, &Note::new(long)),
            Err(crate::Error::Parameter(_))
        ));
        let note = Note::new("x").set_author("a".repeat(53));
        assert_eq!(note.author(), None);
    }

    #[test]
    fn note_vml_uses_blocks_and_reference_offsets() {
        let mut sheet = Worksheet::new();
        sheet.insert_note(1, 1, &Note::new("offset")).expect("note");
        let vml = vml_notes_xml(&sheet, 1);
        assert!(vml.contains("<x:Anchor>2,15,0,10,"), "{vml}");

        let mut sheet = Worksheet::new();
        for row in 0..1025 {
            sheet.insert_note(row, 0, &Note::new("x")).expect("note");
        }
        let vml = vml_notes_xml(&sheet, 1);
        assert!(vml.contains(r#"<o:idmap v:ext="edit" data="1,2" />"#));
        assert!(vml.contains(r#"id="_x0000_s2049""#));
    }

    #[test]
    fn note_anchor_clamps_at_the_last_cells() {
        let mut sheet = Worksheet::new();
        sheet
            .insert_note(
                crate::worksheet::MAX_ROW,
                crate::worksheet::MAX_COL,
                &Note::new("edge"),
            )
            .expect("note");
        let vml = vml_notes_xml(&sheet, 1);
        assert!(vml.contains("<x:Anchor>16380,49,1048571,14,"), "{vml}");
    }

    #[test]
    fn note_color_alt_text_and_reset_are_emitted() {
        let mut sheet = Worksheet::new();
        sheet
            .insert_note(
                0,
                0,
                &Note::new("note text")
                    .set_background_color("#80FF80")
                    .set_alt_text("a \"quote\" & <tag>"),
            )
            .expect("note");
        let vml = vml_notes_xml(&sheet, 1);
        assert!(vml.contains(r##"fillcolor="#80FF80""##), "{vml}");
        assert!(vml.contains(r##"<v:fill color2="#80FF80" />"##), "{vml}");
        assert!(
            vml.contains(r#"alt="a &quot;quote&quot; &amp; &lt;tag&gt;""#),
            "{vml}"
        );

        let mut note = Note::new("old");
        note.reset_text("new");
        assert_eq!(note.text(), "new");
        assert_eq!(
            note.clone()
                .set_background_color(Color::Theme(0, 0))
                .fill_color(),
            0x00FF_FFE1
        );
        assert_eq!(note.clone().set_alt_text("x".repeat(256)).alt_text(), "");
    }

    #[test]
    fn explicit_visibility_overrides_show_all_notes() {
        let mut sheet = Worksheet::new();
        sheet.show_all_notes(true);
        sheet
            .insert_note(0, 0, &Note::new("Hidden").set_visible(false))
            .expect("note");
        let vml = vml_notes_xml(&sheet, 1);
        assert!(vml.contains("visibility:hidden;"));
        assert!(!vml.contains("<x:Visible />"));
    }
}
