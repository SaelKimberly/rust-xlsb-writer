//! Workbook style registry: deduplicates fonts, fills, borders, number formats
//! and cell formats, then emits the Styles part.

use std::collections::HashMap;

use crate::biff::push_record;
use crate::prelude::{
    BrtBeginBorders, BrtBeginCellStyleXFs, BrtBeginCellXFs, BrtBeginDXFs, BrtBeginFills,
    BrtBeginFmts, BrtBeginFonts, BrtBeginStyleSheet, BrtBeginStyles, BrtBeginTableStyles,
    BrtBorder, BrtDXF, BrtEndBorders, BrtEndCellStyleXFs, BrtEndCellXFs, BrtEndDXFs, BrtEndFills,
    BrtEndFmts, BrtEndFonts, BrtEndStyleSheet, BrtEndStyles, BrtEndTableStyles, BrtFill, BrtFmt,
    BrtFont, BrtStyle, BrtXF, GrbitAtr, XlStringExt, XlWideString,
};
use crate::{Error, Result};

const FIRST_CUSTOM_FORMAT: u16 = 164;
const MAX_FONTS: usize = 65_491;
const MAX_FILLS: usize = 65_431;
const MAX_BORDERS: usize = 65_430;
const MAX_CELL_XFS: usize = 65_430;
const MAX_DXFS: usize = 65_535;

/// A complete cell format: font, fill, border and the XF attributes that are
/// stored directly in the record.
///
/// [`StyleRegistry::add`] replaces `xf.iFont`, `xf.iFill`, `xf.ixBorder` and
/// `xf.ixfeParent` with the indexes it assigns.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellStyle {
    /// Font properties.
    pub font: BrtFont,
    /// Fill properties.
    pub fill: BrtFill,
    /// Border properties.
    pub border: BrtBorder,
    /// Number format, alignment, protection and the remaining direct XF bits.
    pub xf: BrtXF,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl CellStyle {
    /// Creates the default style: default font, fill, border and a locked XF.
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
        }
    }
}

/// Owns the workbook style tables and assigns stable `BrtXF` indexes.
///
/// Index 0 is always the default style. Adding an equal [`CellStyle`] again
/// returns the existing index instead of growing the table.
pub struct StyleRegistry {
    formats: Vec<BrtFmt>,
    fonts: Vec<BrtFont>,
    fills: Vec<BrtFill>,
    borders: Vec<BrtBorder>,
    xfs: Vec<BrtXF>,
    dxfs: Vec<BrtDXF>,
    style_xf: BrtXF,
    format_ids: HashMap<String, u16>,
    font_ids: HashMap<BrtFont, u16>,
    fill_ids: HashMap<BrtFill, u16>,
    border_ids: HashMap<BrtBorder, u16>,
    xf_ids: HashMap<BrtXF, u32>,
    dxf_ids: HashMap<BrtDXF, u32>,
}

impl Default for StyleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleRegistry {
    /// Creates a registry containing only the default style at index 0.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            formats: Vec::new(),
            fonts: Vec::new(),
            fills: Vec::new(),
            borders: Vec::new(),
            xfs: Vec::new(),
            dxfs: Vec::new(),
            style_xf: BrtXF {
                ixfeParent: 0xFFFF,
                ..CellStyle::default().xf
            },
            format_ids: HashMap::new(),
            font_ids: HashMap::new(),
            fill_ids: HashMap::new(),
            border_ids: HashMap::new(),
            xf_ids: HashMap::new(),
            dxf_ids: HashMap::new(),
        };

        let font = BrtFont::default();
        registry.font_ids.insert(font.clone(), 0);
        registry.fonts.push(font);

        let fill = BrtFill::default();
        registry.fill_ids.insert(fill.clone(), 0);
        registry.fills.push(fill);

        let border = BrtBorder::default();
        registry.border_ids.insert(border.clone(), 0);
        registry.borders.push(border);

        let xf = BrtXF {
            xfGrbitAtr: explicit_grbit(),
            ..CellStyle::default().xf
        };
        registry.xf_ids.insert(xf.clone(), 0);
        registry.xfs.push(xf);

        registry
    }

    /// Registers a font-only entry and returns its `BrtFont` index.
    ///
    /// Rich-string runs reference fonts directly and do not need an XF.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if the font table reaches its
    /// MS-XLSB record limit.
    pub fn register_font(&mut self, font: BrtFont) -> Result<u16> {
        self.add_font(font)
    }

    /// Registers a differential format and returns its zero-based `BrtDXF`
    /// index.
    ///
    /// Equal DXFs are deduplicated.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if the DXF table reaches its
    /// MS-XLSB record limit.
    pub fn register_dxf(&mut self, dxf: BrtDXF) -> Result<u32> {
        if let Some(&index) = self.dxf_ids.get(&dxf) {
            return Ok(index);
        }
        if self.dxfs.len() >= MAX_DXFS {
            return Err(Error::ValidationError(
                "too many differential formats".to_owned(),
            ));
        }
        let index = u32::try_from(self.dxfs.len())
            .map_err(|_| Error::ValidationError("too many differential formats".to_owned()))?;
        self.dxf_ids.insert(dxf.clone(), index);
        self.dxfs.push(dxf);
        Ok(index)
    }

    /// Replaces the default cell style at XF index 0 and the Normal style XF.
    ///
    /// Call this before registering worksheet formats. The style's font, fill,
    /// border and number format are registered in the usual tables, then cell
    /// XF 0 and the Normal style XF are replaced.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if a style table would exceed the
    /// MS-XLSB record limit, or if the number format is not registered.
    pub fn set_default_style(&mut self, style: CellStyle) -> Result<()> {
        if !self.has_number_format(style.xf.iFmt) {
            return Err(Error::ValidationError(
                "number format is not registered".to_owned(),
            ));
        }

        let i_font = self.add_font(style.font)?;
        let i_fill = self.add_fill(style.fill)?;
        let ix_border = self.add_border(style.border)?;

        let mut xf = style.xf;
        xf.ixfeParent = 0;
        xf.iFont = i_font;
        xf.iFill = i_fill;
        xf.ixBorder = ix_border;
        xf.xfGrbitAtr = explicit_grbit();

        let old_xf = self.xfs[0].clone();
        self.xf_ids.remove(&old_xf);
        self.xf_ids.insert(xf.clone(), 0);
        self.style_xf = BrtXF {
            ixfeParent: 0xFFFF,
            ..xf.clone()
        };
        self.xfs[0] = xf;
        Ok(())
    }

    /// Number of cell formats, including the default style.
    #[must_use]
    pub fn len(&self) -> u32 {
        u32::try_from(self.xfs.len()).expect("the cell XF limit is below u32::MAX")
    }

    /// Whether the registry has no cell formats. Always false after [`new`].
    ///
    /// [`new`]: Self::new
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.xfs.is_empty()
    }

    /// Registers a custom number format and returns its `iFmt` id.
    ///
    /// Custom ids start at 164; registering the same code again returns the
    /// existing id.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if the id space is exhausted.
    pub fn add_number_format(&mut self, code: &str) -> Result<u16> {
        if let Some(&id) = self.format_ids.get(code) {
            return Ok(id);
        }

        let id = FIRST_CUSTOM_FORMAT
            .checked_add(
                u16::try_from(self.formats.len())
                    .map_err(|_| Error::ValidationError("too many number formats".to_owned()))?,
            )
            .ok_or_else(|| Error::ValidationError("too many number formats".to_owned()))?;

        self.formats.push(BrtFmt {
            ifmt: id,
            stFmtCode: XlWideString::construct(Some(code)),
        });
        self.format_ids.insert(code.to_owned(), id);
        Ok(id)
    }

    /// Registers a cell style and returns its zero-based `BrtXF` index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ValidationError`] if a style table would exceed the
    /// MS-XLSB record limit.
    pub fn add(&mut self, style: CellStyle) -> Result<u32> {
        if !self.has_number_format(style.xf.iFmt) {
            return Err(Error::ValidationError(
                "number format is not registered".to_owned(),
            ));
        }

        let i_font = self.add_font(style.font)?;
        let i_fill = self.add_fill(style.fill)?;
        let ix_border = self.add_border(style.border)?;

        let mut xf = style.xf;
        xf.ixfeParent = 0;
        xf.iFont = i_font;
        xf.iFill = i_fill;
        xf.ixBorder = ix_border;
        // A registered style is self-contained; do not inherit later changes
        // from the Normal style XF.
        xf.xfGrbitAtr = explicit_grbit();

        if let Some(&index) = self.xf_ids.get(&xf) {
            return Ok(index);
        }
        if self.xfs.len() >= MAX_CELL_XFS {
            return Err(Error::ValidationError("too many cell formats".to_owned()));
        }

        let index = u32::try_from(self.xfs.len())
            .map_err(|_| Error::ValidationError("too many cell formats".to_owned()))?;
        self.xf_ids.insert(xf.clone(), index);
        self.xfs.push(xf);
        Ok(index)
    }

    fn has_number_format(&self, id: u16) -> bool {
        id < FIRST_CUSTOM_FORMAT || self.format_ids.values().any(|&value| value == id)
    }

    fn add_font(&mut self, font: BrtFont) -> Result<u16> {
        if let Some(&id) = self.font_ids.get(&font) {
            return Ok(id);
        }
        if self.fonts.len() >= MAX_FONTS {
            return Err(Error::ValidationError("too many fonts".to_owned()));
        }
        let id = u16::try_from(self.fonts.len())
            .map_err(|_| Error::ValidationError("too many fonts".to_owned()))?;
        self.font_ids.insert(font.clone(), id);
        self.fonts.push(font);
        Ok(id)
    }

    fn add_fill(&mut self, fill: BrtFill) -> Result<u16> {
        if let Some(&id) = self.fill_ids.get(&fill) {
            return Ok(id);
        }
        if self.fills.len() >= MAX_FILLS {
            return Err(Error::ValidationError("too many fills".to_owned()));
        }
        let id = u16::try_from(self.fills.len())
            .map_err(|_| Error::ValidationError("too many fills".to_owned()))?;
        self.fill_ids.insert(fill.clone(), id);
        self.fills.push(fill);
        Ok(id)
    }

    fn add_border(&mut self, border: BrtBorder) -> Result<u16> {
        if let Some(&id) = self.border_ids.get(&border) {
            return Ok(id);
        }
        if self.borders.len() >= MAX_BORDERS {
            return Err(Error::ValidationError("too many borders".to_owned()));
        }
        let id = u16::try_from(self.borders.len())
            .map_err(|_| Error::ValidationError("too many borders".to_owned()))?;
        self.border_ids.insert(border.clone(), id);
        self.borders.push(border);
        Ok(id)
    }

    /// Writes the complete Styles part record sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if a record cannot be serialized.
    pub fn write_to_biff(&self, out: &mut Vec<u8>) -> Result<()> {
        push_record(out, &BrtBeginStyleSheet::default())?;

        if !self.formats.is_empty() {
            push_record(
                out,
                &BrtBeginFmts {
                    count: count(self.formats.len())?,
                },
            )?;
            for format in &self.formats {
                push_record(out, format)?;
            }
            push_record(out, &BrtEndFmts::default())?;
        }

        push_record(
            out,
            &BrtBeginFonts {
                count: count(self.fonts.len())?,
            },
        )?;
        for font in &self.fonts {
            push_record(out, font)?;
        }
        push_record(out, &BrtEndFonts::default())?;

        push_record(
            out,
            &BrtBeginFills {
                count: count(self.fills.len())?,
            },
        )?;
        for fill in &self.fills {
            push_record(out, fill)?;
        }
        push_record(out, &BrtEndFills::default())?;

        push_record(
            out,
            &BrtBeginBorders {
                count: count(self.borders.len())?,
            },
        )?;
        for border in &self.borders {
            push_record(out, border)?;
        }
        push_record(out, &BrtEndBorders::default())?;

        push_record(out, &BrtBeginCellStyleXFs { count: 1 })?;
        push_record(out, &self.style_xf)?;
        push_record(out, &BrtEndCellStyleXFs::default())?;

        push_record(out, &BrtBeginCellXFs { count: self.len() })?;
        for xf in &self.xfs {
            push_record(out, xf)?;
        }
        push_record(out, &BrtEndCellXFs::default())?;

        push_record(out, &BrtBeginStyles { count: 1 })?;
        push_record(out, &BrtStyle::default())?;
        push_record(out, &BrtEndStyles::default())?;

        push_record(
            out,
            &BrtBeginDXFs {
                count: count(self.dxfs.len())?,
            },
        )?;
        for dxf in &self.dxfs {
            push_record(out, dxf)?;
        }
        push_record(out, &BrtEndDXFs::default())?;

        push_record(out, &BrtBeginTableStyles::default())?;
        push_record(out, &BrtEndTableStyles::default())?;

        push_record(out, &BrtEndStyleSheet::default())?;
        Ok(())
    }
}

const fn explicit_grbit() -> GrbitAtr {
    GrbitAtr {
        bit_1: true,
        bit_2: true,
        bit_3: true,
        bit_4: true,
        bit_5: true,
        bit_6: true,
    }
}

fn count(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| Error::ValidationError("style table is too large".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Bold;

    fn bold_style() -> CellStyle {
        CellStyle {
            font: BrtFont {
                bls: Bold::BLSBOLD,
                ..BrtFont::default()
            },
            ..CellStyle::default()
        }
    }

    #[test]
    fn default_style_is_index_zero() {
        let registry = StyleRegistry::new();
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.xfs[0].fLocked);
        assert!(CellStyle::default().xf.fLocked);
    }

    #[test]
    fn default_style_can_be_replaced() {
        let mut registry = StyleRegistry::new();
        let style = CellStyle {
            font: BrtFont {
                bls: Bold::BLSBOLD,
                ..BrtFont::default()
            },
            ..CellStyle::default()
        };

        registry
            .set_default_style(style.clone())
            .expect("sets default");
        assert_eq!(registry.style_xf.ixfeParent, 0xFFFF);
        assert_eq!(registry.style_xf.iFont, 1);
        assert_eq!(registry.xfs[0].ixfeParent, 0);
        assert_eq!(registry.xfs[0].iFont, 1);
        assert_eq!(registry.add(style).expect("deduplicates"), 0);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn rich_fonts_deduplicate_without_a_cell_format() {
        let mut registry = StyleRegistry::new();
        let bold = BrtFont {
            bls: Bold::BLSBOLD,
            ..BrtFont::default()
        };

        assert_eq!(
            registry.register_font(BrtFont::default()).expect("default"),
            0
        );
        assert_eq!(registry.register_font(bold.clone()).expect("bold"), 1);
        assert_eq!(registry.register_font(bold).expect("deduplicates"), 1);
        assert_eq!(registry.fonts.len(), 2);
        assert_eq!(registry.xfs.len(), 1);
    }

    #[test]
    fn equal_styles_share_an_index() {
        let mut registry = StyleRegistry::new();

        let first = registry.add(bold_style()).expect("adds");
        let second = registry.add(bold_style()).expect("deduplicates");

        assert_eq!(first, 1);
        assert_eq!(second, first);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.fonts.len(), 2);
    }

    #[test]
    fn number_formats_deduplicate_and_start_at_164() {
        let mut registry = StyleRegistry::new();

        let date = registry.add_number_format("yyyy-mm-dd").expect("adds");
        let again = registry
            .add_number_format("yyyy-mm-dd")
            .expect("deduplicates");
        let time = registry.add_number_format("hh:mm:ss").expect("adds");

        assert_eq!(date, 164);
        assert_eq!(again, date);
        assert_eq!(time, 165);
        assert_eq!(registry.formats.len(), 2);
    }

    #[test]
    fn custom_formats_must_be_registered_before_use() {
        let mut registry = StyleRegistry::new();
        let style = CellStyle {
            xf: BrtXF {
                iFmt: 164,
                ..BrtXF::default()
            },
            ..CellStyle::default()
        };

        assert!(matches!(
            registry.add(style),
            Err(Error::ValidationError(_))
        ));

        registry
            .add_number_format("yyyy-mm-dd")
            .expect("registers 164");
        let index = registry
            .add(CellStyle {
                xf: BrtXF {
                    iFmt: 164,
                    ..BrtXF::default()
                },
                ..CellStyle::default()
            })
            .expect("adds after registration");
        assert_eq!(index, 1);
    }
    #[test]
    fn dxfs_deduplicate_by_value() {
        let mut registry = StyleRegistry::new();
        let dxf = BrtDXF::new(crate::prelude::XFProps::new(vec![
            crate::prelude::XFProp::new(0x0002, vec![5, 0, 0, 0, 0xFF, 0xFF, 0x00, 0xFF]),
        ]));
        assert_eq!(registry.register_dxf(dxf.clone()).expect("dxf"), 0);
        assert_eq!(registry.register_dxf(dxf).expect("dxf"), 0);
    }
}
