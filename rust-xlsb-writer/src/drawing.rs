//! Worksheet `DrawingML` pictures.

use std::fmt::Write;

use crate::{Error, Image, Result};

use crate::worksheet::{MAX_COL, MAX_ROW, Worksheet};

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the VML object writers"
)]
pub(crate) const EMU_PER_PIXEL: u64 = 9_525;
const EMU_PER_PIXEL_F64: f64 = 9_525.0;
const DEFAULT_COL_WIDTH: f64 = 8.43;
const DEFAULT_ROW_HEIGHT: f64 = 15.0;

/// One foreground image and its insertion point.
#[derive(Debug, Clone)]
pub struct PlacedImage {
    pub(crate) image: Image,
    pub(crate) row: u32,
    pub(crate) col: u16,
    pub(crate) x_offset: u32,
    pub(crate) y_offset: u32,
    width: f64,
    height: f64,
}

impl PlacedImage {
    pub(crate) fn new(
        image: Image,
        row: u32,
        col: u16,
        x_offset: u32,
        y_offset: u32,
    ) -> Result<Self> {
        if row > MAX_ROW || col > MAX_COL {
            return Err(Error::RowColumnLimit);
        }
        let (width, height) = image.scaled_size()?;
        Ok(Self {
            image,
            row,
            col,
            x_offset,
            y_offset,
            width,
            height,
        })
    }
}

pub fn drawing_xml(sheet: &Worksheet, images: &[PlacedImage]) -> String {
    let mut xml = String::with_capacity(640 + images.len() * 520);
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    );
    for (index, image) in images.iter().enumerate() {
        let anchor = anchor(sheet, image);
        let rel_id = index + 1;
        let width = pixels_to_emu(image.width);
        let height = pixels_to_emu(image.height);
        write!(
            xml,
            concat!(
                r#"<xdr:twoCellAnchor editAs="oneCell">"#,
                r#"<xdr:from><xdr:col>{}</xdr:col><xdr:colOff>{}</xdr:colOff>"#,
                r#"<xdr:row>{}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:from>"#,
                r#"<xdr:to><xdr:col>{}</xdr:col><xdr:colOff>{}</xdr:colOff>"#,
                r#"<xdr:row>{}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:to>"#,
                r#"<xdr:pic><xdr:nvPicPr><xdr:cNvPr id="{}" name="Picture {}"/>"#,
                r#"<xdr:cNvPicPr><a:picLocks noChangeAspect="1"/></xdr:cNvPicPr></xdr:nvPicPr>"#,
                r#"<xdr:blipFill><a:blip r:embed="rId{}"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>"#,
                r#"<xdr:spPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm>"#,
                r#"<a:prstGeom prst="rect"/></xdr:spPr></xdr:pic><xdr:clientData/></xdr:twoCellAnchor>"#
            ),
            anchor.from_col,
            anchor.from_col_off,
            anchor.from_row,
            anchor.from_row_off,
            anchor.to_col,
            anchor.to_col_off,
            anchor.to_row,
            anchor.to_row_off,
            rel_id,
            rel_id,
            rel_id,
            anchor.absolute_x,
            anchor.absolute_y,
            width,
            height,
        )
        .expect("writing to String cannot fail");
    }
    xml.push_str("</xdr:wsDr>");
    xml
}

pub fn drawing_rels(images: &[PlacedImage], media_indices: &[usize]) -> String {
    let mut xml = String::with_capacity(256 + images.len() * 160);
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (index, (image, media_index)) in images.iter().zip(media_indices).enumerate() {
        write!(
            xml,
            r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{media_index}.{}"/>"#,
            index + 1,
            image.image.extension()
        )
        .expect("writing to String cannot fail");
    }
    xml.push_str("</Relationships>");
    xml
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "scaled pixels are positive and fit u64 EMU"
)]
fn pixels_to_emu(pixels: f64) -> u64 {
    (pixels * EMU_PER_PIXEL_F64).round() as u64
}

struct Anchor {
    from_col: u16,
    from_col_off: u64,
    from_row: u32,
    from_row_off: u64,
    to_col: u16,
    to_col_off: u64,
    to_row: u32,
    to_row_off: u64,
    absolute_x: u64,
    absolute_y: u64,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "anchor offsets are non-negative and fit u64"
)]
fn anchor(sheet: &Worksheet, image: &PlacedImage) -> Anchor {
    let (to_col, to_col_off) = advance_columns(sheet, image.col, image.x_offset, image.width);
    let (to_row, to_row_off) = advance_rows(sheet, image.row, image.y_offset, image.height);
    Anchor {
        from_col: image.col,
        from_col_off: u64::from(image.x_offset) * EMU_PER_PIXEL,
        from_row: image.row,
        from_row_off: u64::from(image.y_offset) * EMU_PER_PIXEL,
        to_col,
        to_col_off,
        to_row,
        to_row_off,
        absolute_x: (absolute_column_pixels(sheet, image.col) + f64::from(image.x_offset)).round()
            as u64
            * EMU_PER_PIXEL,
        absolute_y: (absolute_row_pixels(sheet, image.row) + f64::from(image.y_offset)).round()
            as u64
            * EMU_PER_PIXEL,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "anchor offsets are non-negative and fit u64"
)]
pub fn advance_columns(sheet: &Worksheet, col: u16, offset: u32, width: f64) -> (u16, u64) {
    let mut col = col;
    let mut remaining = width + f64::from(offset);
    loop {
        let cell_width = column_pixels(sheet, col);
        if remaining <= cell_width || col == MAX_COL {
            return (col, (remaining * EMU_PER_PIXEL as f64).round() as u64);
        }
        remaining -= cell_width;
        col += 1;
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "anchor offsets are non-negative and fit u64"
)]
pub fn advance_rows(sheet: &Worksheet, row: u32, offset: u32, height: f64) -> (u32, u64) {
    let mut row = row;
    let mut remaining = height + f64::from(offset);
    loop {
        let cell_height = row_pixels(sheet, row);
        if remaining <= cell_height || row == MAX_ROW {
            return (row, (remaining * EMU_PER_PIXEL as f64).round() as u64);
        }
        remaining -= cell_height;
        row += 1;
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell dimensions are non-negative and fit u32"
)]
pub fn centered_offset(cell: f64, image: f64) -> u32 {
    ((cell - image).max(0.0) / 2.0).round() as u32
}

/// Converts a 96-DPI pixel dimension to the 72-DPI points VML uses.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the VML object writers"
)]
pub(crate) fn vml_dpi_size(dimension: f64) -> f64 {
    (dimension + 0.25).floor() * 72.0 / 96.0
}

pub fn column_pixels(sheet: &Worksheet, col: u16) -> f64 {
    let default = sheet.default_col_width().unwrap_or(DEFAULT_COL_WIDTH);
    let Some(options) = sheet.column_options().get(&col) else {
        return width_pixels(default, sheet.default_column_metrics());
    };
    if options.hidden {
        0.0
    } else {
        width_pixels(
            options.width.unwrap_or(default),
            sheet.default_column_metrics(),
        )
    }
}

pub fn row_pixels(sheet: &Worksheet, row: u32) -> f64 {
    if sheet.filter_hidden_rows().contains(&row) {
        return 0.0;
    }
    let default = sheet.default_row_height().unwrap_or(DEFAULT_ROW_HEIGHT);
    let Some(options) = sheet.row_options().get(&row) else {
        if sheet.unused_rows_hidden() && !sheet.cells().contains_key(&row) {
            return 0.0;
        }
        return default * 4.0 / 3.0;
    };
    if options.hidden {
        0.0
    } else {
        options.height.unwrap_or(default) * 4.0 / 3.0
    }
}

fn width_pixels(width: f64, (digit_width, padding): (u32, u32)) -> f64 {
    if width < 1.0 {
        width * f64::from(digit_width + padding)
    } else {
        width.mul_add(f64::from(digit_width), f64::from(padding))
    }
}

pub fn absolute_column_pixels(sheet: &Worksheet, col: u16) -> f64 {
    let metrics = sheet.default_column_metrics();
    let default = width_pixels(
        sheet.default_col_width().unwrap_or(DEFAULT_COL_WIDTH),
        metrics,
    );
    let mut total = default * f64::from(col);
    let default_width = sheet.default_col_width().unwrap_or(DEFAULT_COL_WIDTH);
    for options in sheet
        .column_options()
        .range(..col)
        .map(|(_, options)| options)
    {
        let width = if options.hidden {
            0.0
        } else {
            width_pixels(options.width.unwrap_or(default_width), metrics)
        };
        total += width - default;
    }
    total
}

pub fn absolute_row_pixels(sheet: &Worksheet, row: u32) -> f64 {
    let default = sheet
        .default_row_height()
        .map_or(DEFAULT_ROW_HEIGHT * 4.0 / 3.0, |height| height * 4.0 / 3.0);
    let default_height = sheet.default_row_height().unwrap_or(DEFAULT_ROW_HEIGHT);
    let hidden = sheet.filter_hidden_rows();
    let mut total = default * f64::from(row);
    for (&row_id, options) in sheet.row_options().range(..row) {
        let height = if options.hidden || hidden.contains(&row_id) {
            0.0
        } else {
            options.height.unwrap_or(default_height) * 4.0 / 3.0
        };
        total += height - default;
    }
    for hidden_row in hidden.range(..row) {
        if !sheet.row_options().contains_key(hidden_row) {
            total -= default;
        }
    }
    if sheet.unused_rows_hidden() {
        let mut empty_rows = f64::from(row);
        for _ in sheet.row_options().range(..row) {
            empty_rows -= 1.0;
        }
        for hidden_row in hidden.range(..row) {
            if !sheet.row_options().contains_key(hidden_row) {
                empty_rows -= 1.0;
            }
        }
        for (cell_row, _) in sheet.cells().range(..row) {
            if !sheet.row_options().contains_key(cell_row) && !hidden.contains(cell_row) {
                empty_rows -= 1.0;
            }
        }
        total = default.mul_add(-empty_rows.max(0.0), total);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Image {
        let mut png = [0u8; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        Image::new_from_buffer(&png).expect("png")
    }

    #[test]
    fn anchor_walks_custom_sized_cells() {
        let mut sheet = Worksheet::new();
        sheet.set_column_width_pixels(0, 100).expect("column");
        sheet.set_row_height_pixels(0, 40).expect("row");
        let placed = PlacedImage::new(png(120, 60), 0, 0, 0, 0).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_col, 1);
        assert_eq!(anchor.to_col_off, 190_500);
        assert_eq!(anchor.to_row, 1);
        assert_eq!(anchor.to_row_off, 190_500);
        assert_eq!(anchor.absolute_x, 0);
        assert_eq!(anchor.absolute_y, 0);
    }

    #[test]
    fn anchor_includes_offsets_in_the_end_position() {
        let sheet = Worksheet::new();
        let placed = PlacedImage::new(png(20, 20), 0, 0, 10, 5).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.from_col_off, 95_250);
        assert_eq!(anchor.from_row_off, 47_625);
        assert_eq!(anchor.to_col_off, 285_750);
        assert_eq!(anchor.to_row, 1);
        assert_eq!(anchor.to_row_off, 47_625);
    }

    #[test]
    fn anchor_uses_the_scaled_size() {
        let image = png(20, 20).set_scale_width(2.0).set_scale_height(3.0);
        let placed = PlacedImage::new(image, 0, 0, 0, 0).expect("placement");
        let sheet = Worksheet::new();
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_col, 0);
        assert_eq!(anchor.to_col_off, 381_000);
        assert_eq!(anchor.to_row, 2);
        assert_eq!(anchor.to_row_off, 190_500);
    }

    #[test]
    fn anchor_uses_narrow_column_pixels() {
        let mut sheet = Worksheet::new();
        sheet.set_column_width_pixels(0, 6).expect("width");
        let placed = PlacedImage::new(png(10, 10), 0, 0, 0, 0).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_col, 1);
        assert_eq!(anchor.to_col_off, 38_100);
    }

    #[test]
    fn width_pixels_uses_metrics_for_narrow_columns() {
        assert_eq!(width_pixels(0.5, (9, 7)), 8.0);
        assert_eq!(width_pixels(1.0, (6, 5)), 11.0);
        assert_eq!(width_pixels(1.0, (7, 5)), 12.0);
    }

    #[test]
    fn anchor_uses_the_default_column_metrics() {
        let mut sheet = Worksheet::new();
        sheet
            .set_default_format(&crate::Format::new(), 20, 80)
            .expect("default format");
        let placed = PlacedImage::new(png(100, 10), 0, 0, 0, 0).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_col, 1);
        assert_eq!(anchor.to_col_off, 190_500);
    }

    #[test]
    fn anchor_ignores_unused_row_heights() {
        let mut sheet = Worksheet::new();
        sheet.write(0, 0, "used").expect("cell");
        sheet.hide_unused_rows(true);
        assert_eq!(absolute_row_pixels(&sheet, 3), 20.0);
    }

    #[test]
    fn anchor_ignores_filter_hidden_rows() {
        let mut sheet = Worksheet::new();
        for row in 0..4 {
            sheet
                .write(row, 0, i32::try_from(row).expect("row"))
                .expect("cell");
        }
        sheet.set_row_height_pixels(1, 40).expect("row height");
        sheet.autofilter(0, 0, 3, 0).expect("filter range");
        sheet
            .filter_column(
                0,
                &crate::FilterCondition::new()
                    .add_custom_filter(crate::FilterCriteria::EqualTo, 2_i32),
            )
            .expect("filter");
        sheet.apply_filters();
        assert_eq!(absolute_row_pixels(&sheet, 2), 20.0);
    }

    #[test]
    fn anchor_uses_the_default_row_height() {
        let mut sheet = Worksheet::new();
        sheet.set_default_row_height_pixels(40).expect("height");
        let placed = PlacedImage::new(png(20, 50), 0, 0, 0, 0).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_row, 1);
        assert_eq!(anchor.to_row_off, 95_250);
    }

    #[test]
    fn anchor_skips_hidden_cells() {
        let mut sheet = Worksheet::new();
        sheet.set_column_hidden(0).expect("column");
        sheet.set_row_hidden(0).expect("row");
        let placed = PlacedImage::new(png(20, 20), 0, 0, 0, 0).expect("placement");
        let anchor = anchor(&sheet, &placed);
        assert_eq!(anchor.to_col, 1);
        assert_eq!(anchor.to_col_off, 190_500);
        assert_eq!(anchor.to_row, 1);
        assert_eq!(anchor.to_row_off, 190_500);
    }
}
