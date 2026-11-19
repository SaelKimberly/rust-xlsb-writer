//! XLSB package assembly: parts, relationships, and ZIP packaging.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use rust_xlsb_core::prelude::{
    BrtBeginBook, BrtBeginBookViews, BrtBeginBundleShs, BrtBeginDynamicArrayPr, BrtBeginEsfmd,
    BrtBeginEsmdb, BrtBeginEsmdtinfo, BrtBeginExternals, BrtBeginFmd, BrtBeginMetadata,
    BrtBookView, BrtBundleSh, BrtCalcProp, BrtEndBook, BrtEndBookViews, BrtEndBundleShs,
    BrtEndDynamicArrayPr, BrtEndEsfmd, BrtEndEsmdb, BrtEndEsmdtinfo, BrtEndExternals, BrtEndFmd,
    BrtEndMetadata, BrtExternSheet, BrtFRTBegin, BrtFRTEnd, BrtFileSharing, BrtMdb, BrtMdtinfo,
    BrtName, BrtSupSelf, BrtWbProp, CodeName, FrtProductVersion, Mdir, NameParsedFormula,
    XlNullableWideString, XlStringExt, XlWideString, Xnum, Xti,
};
use rust_xlsb_core::{SSTHolder, StyleRegistry, push_record};

use crate::data::CellData;
use crate::defined_name::{
    DefinedName, NameTable, XtiCollector, area3d, area3d_value, sheet_index,
};
use crate::drawing::{drawing_rels, drawing_xml};
use crate::formula::TableInfo;
use crate::properties::CustomPropertyValue;
use crate::sparkline::{SparklineSaveCell, SparklineSaveGroup};
use crate::table::write_table_part;
use crate::theme::Theme;
use crate::url::{escape_xml, escape_xml_attribute, escape_xml_data};
use crate::workbook::{CalculationMode, WorkbookSettings};
use crate::worksheet::{MAX_COL, MAX_ROW, Visibility, Worksheet};
use crate::{DocProperties, Error, Format, Result};

pub fn build_package(
    sheets: &[Worksheet],
    settings: &WorkbookSettings,
    properties: Option<&DocProperties>,
    default_format: Option<&Format>,
    defined_names: &[DefinedName],
    vba_project: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let sst = SSTHolder::create_in_memory()?;
    let mut registry = StyleRegistry::new();
    if let Some(format) = default_format {
        let style = format.to_cell_style(&mut registry)?;
        registry.set_default_style(style)?;
    }
    let mut style_indexes = Vec::with_capacity(sheets.len());

    for sheet in sheets {
        let mut indexes = Vec::with_capacity(sheet.formats().len());
        for format in sheet.formats() {
            let style = format.to_cell_style(&mut registry)?;
            indexes.push(registry.add(style)?);
        }
        style_indexes.push(indexes);
    }

    // Rebuild from current cells so overwritten rich text cannot leave a stale
    // font. The worksheet flag skips the pass entirely for plain worksheets.
    let mut rich_fonts = HashMap::new();
    for sheet in sheets.iter().filter(|sheet| sheet.has_rich_strings()) {
        for row in sheet.cells().values() {
            for cell in row.values() {
                if let CellData::RichString(rich) = &cell.data {
                    for run in &rich.runs {
                        if !rich_fonts.contains_key(&run.font) {
                            let id = registry.register_font(run.font.clone())?;
                            rich_fonts.insert(run.font.clone(), id);
                        }
                    }
                }
            }
        }
    }

    let mut conditional_dxf_ids = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let mut ids = Vec::with_capacity(sheet.conditional_formats().len());
        for conditional_format in sheet.conditional_formats() {
            if let Some(format) = conditional_format.dxf_format() {
                let dxf = format.to_dxf()?;
                ids.push(Some(registry.register_dxf(dxf)?));
            } else {
                ids.push(None);
            }
        }
        conditional_dxf_ids.push(ids);
    }

    let mut xti = XtiCollector::new(sheets.len());
    let mut table_infos = Vec::new();
    if sheets.iter().any(|sheet| {
        sheet.tables().iter().any(|table| {
            table
                .table
                .columns
                .iter()
                .any(|column| column.formula.is_some())
        })
    }) {
        for (index, sheet) in sheets.iter().enumerate() {
            if !sheet.tables().is_empty() {
                xti.index(index)?;
            }
        }
        for (index, sheet) in sheets.iter().enumerate() {
            if sheet.tables().is_empty() {
                continue;
            }
            let ixti = xti.index(index)?;
            for table in sheet.tables() {
                table_infos.push(TableInfo {
                    name: &table.table.name,
                    id: table.table.id,
                    ixti,
                    columns: &table.table.columns,
                });
            }
        }
    }

    let mut sheet_sparklines = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let mut groups = Vec::with_capacity(sheet.sparkline_groups().len());
        for group in sheet.sparkline_groups() {
            let name = group
                .cells
                .first()
                .and_then(|cell| cell.range.sheet.as_deref())
                .unwrap_or_else(|| sheet.name_ref());
            let ixti = xti.index(sheet_index(sheets, name)?)?;
            let mut cells = Vec::with_capacity(group.cells.len());
            for cell in &group.cells {
                let range = &cell.range;
                cells.push(SparklineSaveCell {
                    row: cell.row,
                    col: cell.col,
                    rgce: area3d_value(
                        ixti,
                        range.first_row,
                        range.first_col,
                        range.last_row,
                        range.last_col,
                    ),
                });
            }
            groups.push(SparklineSaveGroup {
                sparkline: &group.sparkline,
                cells,
            });
        }
        sheet_sparklines.push(groups);
    }

    let mut sheet_parts = Vec::with_capacity(sheets.len());
    let mut table_parts = Vec::new();
    let mut table_ids = Vec::new();
    let mut drawing_parts = Vec::new();
    let mut drawing_ids = Vec::new();
    let mut media_parts = Vec::new();
    let mut image_types = Vec::new();
    let mut drawing_number = 0u32;
    let view = workbook_view(sheets);
    for (index, sheet) in sheets.iter().enumerate() {
        let image_index = sheet.background_image().map(|image| {
            let image_index = media_parts.len() + 1;
            let extension = image.extension();
            media_parts.push((
                format!("xl/media/image{image_index}.{extension}"),
                image.data().to_vec(),
            ));
            if !image_types.iter().any(|(known, _)| *known == extension) {
                image_types.push((extension, image.content_type()));
            }
            image_index
        });
        let mut next_rel_id = next_sheet_rel_id(sheet)?;
        let drawing = if sheet.images().is_empty() {
            None
        } else {
            drawing_number += 1;
            drawing_ids.push(drawing_number);
            let images = sheet.images();
            let mut media_indices = Vec::with_capacity(images.len());
            for image in images {
                let media_index = media_parts.len() + 1;
                let extension = image.image.extension();
                media_parts.push((
                    format!("xl/media/image{media_index}.{extension}"),
                    image.image.data().to_vec(),
                ));
                if !image_types.iter().any(|(known, _)| *known == extension) {
                    image_types.push((extension, image.image.content_type()));
                }
                media_indices.push(media_index);
            }
            drawing_parts.push((
                format!("xl/drawings/drawing{drawing_number}.xml"),
                drawing_xml(sheet, images).into_bytes(),
            ));
            drawing_parts.push((
                format!("xl/drawings/_rels/drawing{drawing_number}.xml.rels"),
                drawing_rels(images, &media_indices).into_bytes(),
            ));
            let rel_id = next_rel_id;
            next_rel_id += 1;
            Some((drawing_number, rel_id))
        };
        let background_rel_id = if image_index.is_some() {
            let rel_id = next_rel_id;
            Some(rel_id)
        } else {
            None
        };
        let mut bytes = Vec::new();
        let selected = index == view.active && (view.explicit_active || view.active != 0);
        crate::sheet::write_sheet_with_fonts(
            sheet,
            &style_indexes[index],
            &sst,
            &rich_fonts,
            &conditional_dxf_ids[index],
            &sheet_sparklines[index],
            selected,
            drawing.map(|(_, rel_id)| rel_id),
            background_rel_id,
            &mut bytes,
        )?;
        sheet_parts.push((format!("xl/worksheets/sheet{}.bin", index + 1), bytes));
        if let Some(rels) = sheet_rels(sheet, drawing, image_index.zip(background_rel_id)) {
            sheet_parts.push((
                format!("xl/worksheets/_rels/sheet{}.bin.rels", index + 1),
                rels.into_bytes(),
            ));
        }
        for table in sheet.tables() {
            let mut bytes = Vec::new();
            write_table_part(table, &table_infos, &mut bytes)?;
            table_parts.push((format!("xl/tables/table{}.bin", table.table.id), bytes));
            table_ids.push(table.table.id);
        }
    }

    let has_strings = !sst.is_empty();
    let has_dynamic_arrays = sheets.iter().any(Worksheet::has_dynamic_arrays);
    let has_custom_properties =
        properties.is_some_and(|properties| !properties.custom_properties.is_empty());
    let has_theme = settings.theme.is_some();
    let mut styles = Vec::new();
    registry.write_to_biff(&mut styles)?;

    let mut parts: Vec<(String, Vec<u8>)> = vec![
        (
            "[Content_Types].xml".to_owned(),
            content_types(
                sheets.len(),
                has_strings,
                properties.is_some(),
                has_custom_properties,
                settings.theme.as_ref(),
                &table_ids,
                &drawing_ids,
                &image_types,
                has_dynamic_arrays,
                vba_project.is_some(),
            )
            .into_bytes(),
        ),
        (
            "_rels/.rels".to_owned(),
            root_rels(properties.is_some(), has_custom_properties).into_bytes(),
        ),
        (
            "xl/_rels/workbook.bin.rels".to_owned(),
            workbook_rels(
                sheets.len(),
                has_strings,
                has_theme,
                has_dynamic_arrays,
                vba_project.is_some(),
            )
            .into_bytes(),
        ),
        (
            "xl/workbook.bin".to_owned(),
            workbook_part(sheets, settings, defined_names, &mut xti)?,
        ),
        ("xl/styles.bin".to_owned(), styles),
    ];
    if let Some(theme) = settings.theme.as_ref() {
        parts.push((
            "xl/theme/theme1.xml".to_owned(),
            theme.xml().as_bytes().to_vec(),
        ));
    }
    if let Some(properties) = properties {
        parts.push((
            "docProps/core.xml".to_owned(),
            core_xml(properties).into_bytes(),
        ));
        parts.push((
            "docProps/app.xml".to_owned(),
            app_xml(sheets, properties).into_bytes(),
        ));
        if has_custom_properties {
            parts.push((
                "docProps/custom.xml".to_owned(),
                custom_xml(properties).into_bytes(),
            ));
        }
    }
    parts.extend(sheet_parts);
    parts.extend(table_parts);
    parts.extend(drawing_parts);
    parts.extend(media_parts);

    if has_strings {
        let mut shared_strings = Vec::new();
        sst.write_to_biff_sst(&mut shared_strings)?;
        parts.push(("xl/sharedStrings.bin".to_owned(), shared_strings));
    }
    if has_dynamic_arrays {
        let mut metadata = Vec::new();
        write_metadata_part(&mut metadata)?;
        parts.push(("xl/metadata.bin".to_owned(), metadata));
    }
    if let Some(project) = vba_project {
        parts.push(("xl/vbaProject.bin".to_owned(), project.to_vec()));
    }

    zip_store(
        &parts,
        settings.zip_compression_level,
        settings.zip_large_file,
    )
}

struct WorkbookView {
    active: usize,
    first: usize,
    explicit_active: bool,
}

fn workbook_view(sheets: &[Worksheet]) -> WorkbookView {
    let explicit_active = sheets.iter().rposition(Worksheet::is_active);
    let active = explicit_active
        .or_else(|| {
            sheets
                .iter()
                .position(|sheet| sheet.visibility() == Visibility::Visible)
        })
        .unwrap_or(0);
    let requested = sheets
        .iter()
        .rposition(Worksheet::is_first_tab)
        .unwrap_or(0);
    let first = (requested..sheets.len())
        .chain(0..requested)
        .find(|&index| index == active || sheets[index].visibility() == Visibility::Visible)
        .unwrap_or(active);
    WorkbookView {
        active,
        first,
        explicit_active: explicit_active.is_some(),
    }
}

const fn visible_state(sheet: &Worksheet, index: usize, active: usize) -> u32 {
    if index == active {
        return 0;
    }
    match sheet.visibility() {
        Visibility::Visible => 0,
        Visibility::Hidden => 1,
        Visibility::VeryHidden => 2,
    }
}

fn workbook_part(
    sheets: &[Worksheet],
    settings: &WorkbookSettings,
    defined_names: &[DefinedName],
    xti: &mut XtiCollector,
) -> Result<Vec<u8>> {
    let view = workbook_view(sheets);
    let mut out = Vec::new();
    push_record(&mut out, &BrtBeginBook::default())?;
    if settings.read_only_recommended {
        push_record(
            &mut out,
            &BrtFileSharing {
                fReadOnlyRec: 1,
                stUserName: XlNullableWideString::construct(None::<&str>),
                ..BrtFileSharing::default()
            },
        )?;
    }
    if settings.theme.is_some() || settings.vba_codename.is_some() {
        push_record(
            &mut out,
            &BrtWbProp {
                dwThemeVersion: settings.theme.as_ref().map_or(0, Theme::version),
                strName: settings
                    .vba_codename
                    .as_deref()
                    .map_or_else(CodeName::default, |name| CodeName::construct(Some(name))),
                ..BrtWbProp::default()
            },
        )?;
    }
    if view.first != 0 || view.active != 0 || settings.tab_ratio.is_some() {
        push_record(&mut out, &BrtBeginBookViews::default())?;
        push_record(
            &mut out,
            &BrtBookView {
                xWn: 120,
                yWn: 30,
                dxWn: 23_760,
                dyWn: 14_640,
                iTabRatio: settings
                    .tab_ratio
                    .map_or(600, |ratio| u32::from(ratio) * 10),
                itabFirst: u32::try_from(view.first)
                    .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?,
                itabCur: u32::try_from(view.active)
                    .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?,
                fDspHScroll: true,
                fDspVScroll: true,
                fBotAdornment: true,
                fAFDateGroup: true,
                ..BrtBookView::default()
            },
        )?;
        push_record(&mut out, &BrtEndBookViews::default())?;
    }
    push_record(&mut out, &BrtBeginBundleShs::default())?;
    for (index, sheet) in sheets.iter().enumerate() {
        let tab_id = u32::try_from(index + 1)
            .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?;
        let rel_id = format!("rId{}", index + 1);
        push_record(
            &mut out,
            &BrtBundleSh {
                hsState: visible_state(sheet, index, view.active),
                iTabID: tab_id,
                strRelID: XlNullableWideString::construct(Some(rel_id.as_str())),
                strName: XlWideString::construct(Some(sheet.name_ref())),
            },
        )?;
    }
    push_record(&mut out, &BrtEndBundleShs::default())?;
    write_defined_names(sheets, defined_names, xti, &mut out)?;
    if let Some(mode) = settings.calculation_mode {
        write_calculation_properties(mode, &mut out)?;
    } else if sheets.iter().any(Worksheet::has_formulas) {
        write_calculation_properties(CalculationMode::Automatic, &mut out)?;
    }
    push_record(&mut out, &BrtEndBook::default())?;
    Ok(out)
}

fn write_calculation_properties(mode: CalculationMode, out: &mut Vec<u8>) -> Result<()> {
    let (auto_recalc, recalc) = match mode {
        CalculationMode::Automatic => (1, true),
        CalculationMode::Manual => (0, false),
        CalculationMode::Partial => (2, true),
    };
    push_record(
        out,
        &BrtCalcProp {
            recalcID: 124_519,
            fAutoRecalc: auto_recalc,
            cCalcCount: 100,
            xnumDelta: Xnum::from(0.001),
            cUserThreadCount: 1,
            fFullCalcOnLoad: recalc,
            fRefA1: true,
            fFullPrec: true,
            fSaveRecalc: recalc,
            ..BrtCalcProp::default()
        },
    )?;
    Ok(())
}

fn write_defined_names(
    sheets: &[Worksheet],
    defined_names: &[DefinedName],
    xti: &mut XtiCollector,
    out: &mut Vec<u8>,
) -> Result<()> {
    if defined_names.is_empty()
        && xti.is_empty()
        && !sheets.iter().any(|sheet| {
            sheet.print_area().is_some()
                || has_repeat_titles(sheet)
                || sheet.autofilter_range().is_some()
        })
    {
        return Ok(());
    }

    let builtin_count = sheets
        .iter()
        .filter(|sheet| sheet.print_area().is_some())
        .count()
        + sheets
            .iter()
            .filter(|sheet| sheet.autofilter_range().is_some())
            .count()
        + sheets
            .iter()
            .filter(|sheet| has_repeat_titles(sheet))
            .count();

    let mut names = Vec::new();

    // Preserve the historical Xti order: sheet order, one entry per sheet that
    // already had built-in name metadata.
    for (index, sheet) in sheets.iter().enumerate() {
        if sheet.print_area().is_some()
            || has_repeat_titles(sheet)
            || sheet.autofilter_range().is_some()
        {
            xti.index(index)?;
        }
    }

    for (index, sheet) in sheets.iter().enumerate() {
        if let Some((first_row, first_col, last_row, last_col)) = sheet.print_area() {
            let ixti = xti.index(index)?;
            push_defined_name(
                &mut names,
                sheet_itab(index)?,
                "_xlnm.Print_Area",
                BrtName::F_BUILTIN,
                area3d(ixti, first_row, first_col, last_row, last_col).to_vec(),
            )?;
        }
    }
    for (index, sheet) in sheets.iter().enumerate() {
        if let Some(rfx) = sheet.autofilter_range() {
            let ixti = xti.index(index)?;
            let first_col = u16::try_from(rfx.colFirst)
                .map_err(|_| Error::ValueNotRepresentable("autofilter column out of range"))?;
            let last_col = u16::try_from(rfx.colLast)
                .map_err(|_| Error::ValueNotRepresentable("autofilter column out of range"))?;
            push_defined_name(
                &mut names,
                sheet_itab(index)?,
                "_xlnm._FilterDatabase",
                BrtName::F_BUILTIN | BrtName::F_HIDDEN,
                area3d(ixti, rfx.rwFirst, first_col, rfx.rwLast, last_col).to_vec(),
            )?;
        }
    }
    for (index, sheet) in sheets.iter().enumerate() {
        if has_repeat_titles(sheet) {
            let ixti = xti.index(index)?;
            let mut rgce = Vec::with_capacity(31);
            if let Some((first_row, last_row)) = sheet.repeat_rows() {
                rgce.extend_from_slice(&area3d(ixti, first_row, 0, last_row, MAX_COL));
            }
            if let Some((first_col, last_col)) = sheet.repeat_columns() {
                rgce.extend_from_slice(&area3d(ixti, 0, first_col, MAX_ROW, last_col));
            }
            if sheet.repeat_rows().is_some() && sheet.repeat_columns().is_some() {
                rgce.push(0x10);
            }
            push_defined_name(
                &mut names,
                sheet_itab(index)?,
                "_xlnm.Print_Titles",
                BrtName::F_BUILTIN,
                rgce,
            )?;
        }
    }

    let user_names = NameTable::new(sheets, defined_names, builtin_count)?;
    for entry in user_names.entries() {
        let itab = match entry.scope {
            Some(scope) => sheet_itab(scope)?,
            None => u32::MAX,
        };
        let rgce = entry.defined.encode(sheets, xti, &user_names)?;
        push_defined_name(&mut names, itab, &entry.defined.name, 0, rgce)?;
    }

    if !xti.is_empty() {
        let mut xtis = Vec::with_capacity(xti.ranges().len());
        for &(first, last) in xti.ranges() {
            let first_sheet = i32::try_from(first)
                .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?;
            let last_sheet = i32::try_from(last)
                .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?;
            xtis.push(Xti {
                externalLink: 0,
                firstSheet: first_sheet,
                lastSheet: last_sheet,
            });
        }
        push_record(out, &BrtBeginExternals::default())?;
        push_record(out, &BrtSupSelf::default())?;
        push_record(
            out,
            &BrtExternSheet {
                cXti: u32::try_from(xtis.len())
                    .map_err(|_| Error::ValueNotRepresentable("too many defined-name ranges"))?,
                rgXti: xtis,
            },
        )?;
        push_record(out, &BrtEndExternals::default())?;
    }

    out.extend_from_slice(&names);
    Ok(())
}

fn sheet_itab(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(|_| Error::ValueNotRepresentable("too many worksheets"))
}

const fn has_repeat_titles(sheet: &Worksheet) -> bool {
    sheet.repeat_rows().is_some() || sheet.repeat_columns().is_some()
}

fn push_defined_name(
    out: &mut Vec<u8>,
    itab: u32,
    name: &str,
    flags: u32,
    rgce: Vec<u8>,
) -> Result<()> {
    push_record(
        out,
        &BrtName {
            flags,
            chKey: 0,
            itab,
            name: XlWideString::construct(Some(name)),
            formula: NameParsedFormula {
                cce: u32::try_from(rgce.len())
                    .map_err(|_| Error::ValueNotRepresentable("formula too long"))?,
                rgce,
                cb: 0,
                rgcb: Vec::new(),
            },
            comment: XlNullableWideString::construct(None::<&str>),
        },
    )?;
    Ok(())
}

fn next_sheet_rel_id(sheet: &Worksheet) -> Result<u32> {
    let mut rel_id = u32::try_from(sheet.tables().len())
        .map_err(|_| Error::ValueNotRepresentable("too many tables"))?;
    for link in sheet.hyperlinks().values() {
        if !link.is_internal() {
            rel_id = rel_id
                .checked_add(1)
                .ok_or(Error::ValueNotRepresentable("too many hyperlinks"))?;
        }
    }
    rel_id.checked_add(1).ok_or(Error::ValueNotRepresentable(
        "too many worksheet relationships",
    ))
}

fn sheet_rels(
    sheet: &Worksheet,
    drawing: Option<(u32, u32)>,
    background: Option<(usize, u32)>,
) -> Option<String> {
    if sheet.tables().is_empty()
        && sheet.hyperlinks().values().all(crate::Url::is_internal)
        && drawing.is_none()
        && background.is_none()
    {
        return None;
    }

    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    ));
    let mut rel_id = 0u32;
    for table in sheet.tables() {
        rel_id += 1;
        write!(
            body,
            r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableSingleCells" Target="../tables/table{}.bin"/>"#,
            table.table.id
        )
        .expect("writing to String cannot fail");
    }
    for link in sheet.hyperlinks().values() {
        if link.is_internal() {
            continue;
        }
        rel_id += 1;
        write!(
            body,
            r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
            escape_xml(&link.target)
        )
        .expect("writing to String cannot fail");
    }
    if let Some((drawing_number, drawing_rel_id)) = drawing {
        write!(
            body,
            r#"<Relationship Id="rId{drawing_rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing{drawing_number}.xml"/>"#
        )
        .expect("writing to String cannot fail");
    }
    if let Some((image_index, image_rel_id)) = background {
        let extension = sheet
            .background_image()
            .map_or("png", crate::Image::extension);
        write!(
            body,
            r#"<Relationship Id="rId{image_rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{image_index}.{extension}"/>"#
        )
        .expect("writing to String cannot fail");
    }
    body.push_str("</Relationships>");
    Some(body)
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "package flags mirror independent part-presence decisions"
)]
fn content_types(
    sheet_count: usize,
    has_strings: bool,
    has_properties: bool,
    has_custom_properties: bool,
    theme: Option<&crate::theme::Theme>,
    table_ids: &[u32],
    drawing_ids: &[u32],
    image_types: &[(&str, &str)],
    has_metadata: bool,
    has_vba: bool,
) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
    ));
    for (index, (extension, content_type)) in image_types.iter().enumerate() {
        if image_types[..index]
            .iter()
            .any(|(known, _)| known == extension)
        {
            continue;
        }
        write!(
            body,
            r#"<Default Extension="{extension}" ContentType="{content_type}"/>"#
        )
        .expect("writing to String cannot fail");
    }
    body.push_str(concat!(
        r#"<Override PartName="/xl/workbook.bin" ContentType="application/vnd.ms-excel.sheet.binary.macroEnabled.main"/>"#,
        r#"<Override PartName="/xl/styles.bin" ContentType="application/vnd.ms-excel.styles"/>"#,
    ));
    for index in 1..=sheet_count {
        write!(
            body,
            r#"<Override PartName="/xl/worksheets/sheet{index}.bin" ContentType="application/vnd.ms-excel.worksheet"/>"#
        )
        .expect("writing to String cannot fail");
    }
    if has_strings {
        body.push_str(
            r#"<Override PartName="/xl/sharedStrings.bin" ContentType="application/vnd.ms-excel.sharedStrings"/>"#,
        );
    }
    if has_metadata {
        body.push_str(
            r#"<Override PartName="/xl/metadata.bin" ContentType="application/vnd.ms-excel.sheetMetadata"/>"#,
        );
    }
    if has_vba {
        body.push_str(
            r#"<Override PartName="/xl/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/>"#,
        );
    }
    if theme.is_some() {
        body.push_str(
            r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>"#,
        );
    }
    if has_properties {
        body.push_str(
            concat!(
                r#"<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>"#,
                r#"<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#,
            ),
        );
    }
    if has_custom_properties {
        body.push_str(
            r#"<Override PartName="/docProps/custom.xml" ContentType="application/vnd.openxmlformats-officedocument.custom-properties+xml"/>"#,
        );
    }
    for id in table_ids {
        write!(
            body,
            r#"<Override PartName="/xl/tables/table{id}.bin" ContentType="application/vnd.ms-excel.table"/>"#
        )
        .expect("writing to String cannot fail");
    }
    for id in drawing_ids {
        write!(
            body,
            r#"<Override PartName="/xl/drawings/drawing{id}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#
        )
        .expect("writing to String cannot fail");
    }
    body.push_str("</Types>");
    body
}

fn root_rels(has_properties: bool, has_custom_properties: bool) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.bin"/>"#,
    ));
    if has_properties {
        body.push_str(concat!(
            r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>"#,
            r#"<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>"#,
        ));
    }
    if has_custom_properties {
        body.push_str(
            r#"<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/>"#,
        );
    }
    body.push_str("</Relationships>");
    body
}

fn core_xml(properties: &DocProperties) -> String {
    let mut body = String::with_capacity(512);
    body.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#,
    ));
    if !properties.title.is_empty() {
        push_xml_element(&mut body, "dc:title", &properties.title);
    }
    if !properties.subject.is_empty() {
        push_xml_element(&mut body, "dc:subject", &properties.subject);
    }
    push_xml_element(&mut body, "dc:creator", &properties.author);
    if !properties.keywords.is_empty() {
        push_xml_element(&mut body, "cp:keywords", &properties.keywords);
    }
    if !properties.comment.is_empty() {
        push_xml_element(&mut body, "dc:description", &properties.comment);
    }
    push_xml_element(&mut body, "cp:lastModifiedBy", &properties.author);
    if let Some(creation_time) = &properties.creation_time {
        push_dcterms(&mut body, "dcterms:created", creation_time);
        push_dcterms(&mut body, "dcterms:modified", creation_time);
    }
    if !properties.category.is_empty() {
        push_xml_element(&mut body, "cp:category", &properties.category);
    }
    if !properties.status.is_empty() {
        push_xml_element(&mut body, "cp:contentStatus", &properties.status);
    }
    body.push_str("</cp:coreProperties>");
    body
}

fn app_xml(sheets: &[Worksheet], properties: &DocProperties) -> String {
    let count = sheets
        .iter()
        .filter(|sheet| sheet.visibility() != Visibility::VeryHidden)
        .count();

    let mut body = String::with_capacity(1024);
    body.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">"#,
        "<Application>Microsoft Excel</Application><DocSecurity>0</DocSecurity><ScaleCrop>false</ScaleCrop>",
        r#"<HeadingPairs><vt:vector size="2" baseType="variant"><vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant><vt:variant><vt:i4>"#,
    ));
    write!(body, "{count}").expect("writing to String cannot fail");
    body.push_str(
        "</vt:i4></vt:variant></vt:vector></HeadingPairs><TitlesOfParts><vt:vector size=\"",
    );
    write!(body, "{count}").expect("writing to String cannot fail");
    body.push_str("\" baseType=\"lpstr\">");
    for sheet in sheets
        .iter()
        .filter(|sheet| sheet.visibility() != Visibility::VeryHidden)
    {
        push_xml_element(&mut body, "vt:lpstr", sheet.name_ref());
    }
    body.push_str("</vt:vector></TitlesOfParts>");
    if !properties.manager.is_empty() {
        push_xml_element(&mut body, "Manager", &properties.manager);
    }
    push_xml_element(&mut body, "Company", &properties.company);
    body.push_str("<LinksUpToDate>false</LinksUpToDate><SharedDoc>false</SharedDoc>");
    if !properties.hyperlink_base.is_empty() {
        push_xml_element(&mut body, "HyperlinkBase", &properties.hyperlink_base);
    }
    body.push_str(
        "<HyperlinksChanged>false</HyperlinksChanged><AppVersion>12.0000</AppVersion></Properties>",
    );
    body
}

fn custom_xml(properties: &DocProperties) -> String {
    let mut body = String::with_capacity(256 + properties.custom_properties.len() * 96);
    body.push_str(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">"#,
    ));
    for (index, property) in properties.custom_properties.iter().enumerate() {
        body.push_str(r#"<property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid=""#);
        write!(body, "{}", index + 2).expect("writing to String cannot fail");
        body.push_str(r#"" name=""#);
        body.push_str(escape_xml_attribute(&property.name).as_ref());
        body.push_str(r#"">"#);
        match &property.value {
            CustomPropertyValue::Text(value) => push_xml_element(&mut body, "vt:lpwstr", value),
            CustomPropertyValue::Int(value) => {
                body.push_str("<vt:i4>");
                write!(body, "{value}").expect("writing to String cannot fail");
                body.push_str("</vt:i4>");
            }
            CustomPropertyValue::Real(value) => push_vt_r8(&mut body, *value),
            CustomPropertyValue::Bool(value) => {
                body.push_str("<vt:bool>");
                body.push_str(if *value { "true" } else { "false" });
                body.push_str("</vt:bool>");
            }
            CustomPropertyValue::DateTime(value) => {
                push_xml_element(&mut body, "vt:filetime", value);
            }
        }
        body.push_str("</property>");
    }
    body.push_str("</Properties>");
    body
}

fn push_vt_r8(out: &mut String, value: f64) {
    out.push_str("<vt:r8>");
    if value.is_nan() {
        out.push_str("NaN");
    } else if value == f64::INFINITY {
        out.push_str("INF");
    } else if value == f64::NEG_INFINITY {
        out.push_str("-INF");
    } else {
        write!(out, "{value}").expect("writing to String cannot fail");
    }
    out.push_str("</vt:r8>");
}

fn push_dcterms(out: &mut String, tag: &str, value: &str) {
    out.push('<');
    out.push_str(tag);
    out.push_str(r#" xsi:type="dcterms:W3CDTF">"#);
    out.push_str(escape_xml_data(value).as_ref());
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn push_xml_element(out: &mut String, tag: &str, value: &str) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    out.push_str(escape_xml_data(value).as_ref());
    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "relationship flags mirror independent part-presence decisions"
)]
fn workbook_rels(
    sheet_count: usize,
    has_strings: bool,
    has_theme: bool,
    has_metadata: bool,
    has_vba: bool,
) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    ));
    for index in 1..=sheet_count {
        write!(
            body,
            r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.bin"/>"#
        )
        .expect("writing to String cannot fail");
    }
    let style_id = sheet_count + 1;
    write!(
        body,
        r#"<Relationship Id="rId{style_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.bin"/>"#
    )
    .expect("writing to String cannot fail");
    let mut next_id = style_id + 1;
    if has_strings {
        write!(
            body,
            r#"<Relationship Id="rId{next_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.bin"/>"#
        )
        .expect("writing to String cannot fail");
        next_id += 1;
    }
    if has_theme {
        write!(
            body,
            r#"<Relationship Id="rId{next_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#
        )
        .expect("writing to String cannot fail");
        next_id += 1;
    }
    if has_metadata {
        write!(
            body,
            r#"<Relationship Id="rId{next_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sheetMetadata" Target="metadata.bin"/>"#
        )
        .expect("writing to String cannot fail");
        next_id += 1;
    }
    if has_vba {
        write!(
            body,
            r#"<Relationship Id="rId{next_id}" Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject" Target="vbaProject.bin"/>"#
        )
        .expect("writing to String cannot fail");
    }
    body.push_str("</Relationships>");
    body
}

fn write_metadata_part(out: &mut Vec<u8>) -> Result<()> {
    push_record(out, &BrtBeginMetadata::default())?;
    push_record(out, &BrtBeginEsmdtinfo::default())?;
    push_record(out, &BrtMdtinfo::default())?;
    push_record(out, &BrtEndEsmdtinfo::default())?;
    push_record(out, &BrtBeginEsfmd::default())?;
    push_record(out, &BrtBeginFmd::default())?;
    push_record(
        out,
        &BrtFRTBegin {
            productVersion: FrtProductVersion {
                version: 2,
                product: 2,
                reserved: false,
            },
        },
    )?;
    push_record(out, &BrtBeginDynamicArrayPr::default())?;
    push_record(out, &BrtEndDynamicArrayPr::default())?;
    push_record(out, &BrtFRTEnd::default())?;
    push_record(out, &BrtEndFmd::default())?;
    push_record(out, &BrtEndEsfmd::default())?;
    push_record(out, &BrtBeginEsmdb::default())?;
    push_record(
        out,
        &BrtMdb {
            cMdir: 1,
            rgMdir: vec![Mdir::default()],
        },
    )?;
    push_record(out, &BrtEndEsmdb::default())?;
    push_record(out, &BrtEndMetadata::default())?;
    Ok(())
}

const ZIP64_LIMIT: u64 = u32::MAX as u64;
const ZIP_ENTRY_ZIP64: &str = "ZIP entry exceeds 32-bit limit; call use_zip_large_file(true)";
const ZIP_ARCHIVE_ZIP64: &str = "ZIP archive exceeds 32-bit limit; call use_zip_large_file(true)";

fn zip_store(
    parts: &[(String, Vec<u8>)],
    compression_level: Option<u8>,
    large_file: bool,
) -> Result<Vec<u8>> {
    const DOS_DATE: u16 = 0x0021; // 1980-01-01, the earliest representable date.

    let mut body = Vec::new();
    let mut central = Vec::new();

    for (name, data) in parts {
        let crc = crc32(data);
        let uncompressed = u64::try_from(data.len())
            .map_err(|_| Error::ValueNotRepresentable("ZIP part is too large"))?;
        let name = name.as_bytes();
        let name_len = u16::try_from(name.len())
            .map_err(|_| Error::ValueNotRepresentable("ZIP entry name is too long"))?;
        let offset = u64::try_from(body.len())
            .map_err(|_| Error::ValueNotRepresentable("ZIP archive is too large"))?;
        let method = if compression_level.is_some() {
            8_u16
        } else {
            0
        };
        let extra_len = if large_file { 20 } else { 0 };
        let header_offset = body.len();
        body.resize(header_offset + 30 + name.len() + extra_len, 0);

        let data_start = body.len();
        if let Some(level) = compression_level {
            let mut encoder = DeflateEncoder::new(&mut body, Compression::new(u32::from(level)));
            encoder.write_all(data)?;
            encoder.finish()?;
        } else {
            body.extend_from_slice(data);
        }
        let compressed = u64::try_from(body.len() - data_start)
            .map_err(|_| Error::ValueNotRepresentable("ZIP part is too large"))?;

        let compressed32 = if large_file {
            u32::MAX
        } else {
            zip_u32(compressed, ZIP_ENTRY_ZIP64)?
        };
        let uncompressed32 = if large_file {
            u32::MAX
        } else {
            zip_u32(uncompressed, ZIP_ENTRY_ZIP64)?
        };
        let offset32 = if large_file && offset >= ZIP64_LIMIT {
            u32::MAX
        } else {
            zip_u32(offset, ZIP_ARCHIVE_ZIP64)?
        };
        let version = if large_file { 45_u16 } else { 20 };

        let header = &mut body[header_offset..header_offset + 30];
        header[0..4].copy_from_slice(&0x0403_4b50_u32.to_le_bytes());
        header[4..6].copy_from_slice(&version.to_le_bytes());
        header[6..8].copy_from_slice(&0_u16.to_le_bytes());
        header[8..10].copy_from_slice(&method.to_le_bytes());
        header[10..12].copy_from_slice(&0_u16.to_le_bytes());
        header[12..14].copy_from_slice(&DOS_DATE.to_le_bytes());
        header[14..18].copy_from_slice(&crc.to_le_bytes());
        header[18..22].copy_from_slice(&compressed32.to_le_bytes());
        header[22..26].copy_from_slice(&uncompressed32.to_le_bytes());
        header[26..28].copy_from_slice(&name_len.to_le_bytes());
        header[28..30].copy_from_slice(
            &u16::try_from(extra_len)
                .expect("ZIP64 extra is 20 bytes")
                .to_le_bytes(),
        );
        let name_start = header_offset + 30;
        body[name_start..name_start + name.len()].copy_from_slice(name);
        if large_file {
            let extra = name_start + name.len();
            body[extra..extra + 4].copy_from_slice(&[0x01, 0x00, 0x10, 0x00]);
            body[extra + 4..extra + 12].copy_from_slice(&uncompressed.to_le_bytes());
            body[extra + 12..extra + 20].copy_from_slice(&compressed.to_le_bytes());
        }

        let central_extra_len = if large_file {
            if offset >= ZIP64_LIMIT { 28 } else { 20 }
        } else {
            0
        };
        put_u32(&mut central, 0x0201_4b50);
        put_u16(&mut central, version);
        put_u16(&mut central, version);
        put_u16(&mut central, 0);
        put_u16(&mut central, method);
        put_u16(&mut central, 0);
        put_u16(&mut central, DOS_DATE);
        put_u32(&mut central, crc);
        put_u32(&mut central, compressed32);
        put_u32(&mut central, uncompressed32);
        put_u16(&mut central, name_len);
        put_u16(&mut central, central_extra_len);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u32(&mut central, 0);
        put_u32(&mut central, offset32);
        central.extend_from_slice(name);
        if large_file {
            put_u16(&mut central, 0x0001);
            put_u16(&mut central, if offset >= ZIP64_LIMIT { 24 } else { 16 });
            put_u64(&mut central, uncompressed);
            put_u64(&mut central, compressed);
            if offset >= ZIP64_LIMIT {
                put_u64(&mut central, offset);
            }
        }
    }

    let count = u64::try_from(parts.len())
        .map_err(|_| Error::ValueNotRepresentable("too many ZIP entries"))?;
    if !large_file && count > u64::from(u16::MAX) {
        return Err(Error::ValueNotRepresentable("too many ZIP entries"));
    }
    let central_offset = u64::try_from(body.len())
        .map_err(|_| Error::ValueNotRepresentable("ZIP archive is too large"))?;
    let central_size = u64::try_from(central.len())
        .map_err(|_| Error::ValueNotRepresentable("ZIP archive is too large"))?;
    body.extend_from_slice(&central);
    write_zip_end(&mut body, count, central_size, central_offset);
    Ok(body)
}

fn write_zip_end(out: &mut Vec<u8>, count: u64, central_size: u64, central_offset: u64) {
    let needs_zip64 = count >= u64::from(u16::MAX)
        || central_size >= ZIP64_LIMIT
        || central_offset >= ZIP64_LIMIT;
    if needs_zip64 {
        let zip64_offset = out.len();
        put_u32(out, 0x0606_4b50);
        put_u64(out, 44);
        put_u16(out, 45);
        put_u16(out, 45);
        put_u32(out, 0);
        put_u32(out, 0);
        put_u64(out, count);
        put_u64(out, count);
        put_u64(out, central_size);
        put_u64(out, central_offset);

        put_u32(out, 0x0706_4b50);
        put_u32(out, 0);
        put_u64(out, zip64_offset as u64);
        put_u32(out, 1);
    }

    put_u32(out, 0x0605_4b50);
    put_u16(out, 0);
    put_u16(out, 0);
    put_u16(
        out,
        u16::try_from(count.min(u64::from(u16::MAX))).expect("clamped entry count"),
    );
    put_u16(
        out,
        u16::try_from(count.min(u64::from(u16::MAX))).expect("clamped entry count"),
    );
    put_u32(
        out,
        u32::try_from(central_size.min(ZIP64_LIMIT)).expect("clamped central size"),
    );
    put_u32(
        out,
        u32::try_from(central_offset.min(ZIP64_LIMIT)).expect("clamped central offset"),
    );
    put_u16(out, 0);
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                (crc >> 1) ^ 0xedb8_8320
            };
        }
    }
    !crc
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn zip_u32(value: u64, message: &'static str) -> Result<u32> {
    if value >= ZIP64_LIMIT {
        return Err(Error::ValueNotRepresentable(message));
    }
    u32::try_from(value).map_err(|_| Error::ValueNotRepresentable(message))
}

#[cfg(test)]
fn part_names(bytes: &[u8]) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut offset = 0;
    while offset + 30 <= bytes.len() && read_u32(bytes, offset)? == 0x0403_4b50 {
        let name_len = usize::from(read_u16(bytes, offset + 26)?);
        let extra_len = usize::from(read_u16(bytes, offset + 28)?);
        let compressed = usize::try_from(read_u32(bytes, offset + 18)?)
            .map_err(|_| Error::ValueNotRepresentable("ZIP entry too large"))?;
        let name_start = offset + 30;
        let name_end = name_start + name_len;
        names.push(
            std::str::from_utf8(&bytes[name_start..name_end])
                .map_err(|_| Error::ValueNotRepresentable("ZIP name is not UTF-8"))?
                .to_owned(),
        );
        offset = name_end + extra_len + compressed;
    }
    Ok(names)
}

#[cfg(test)]
pub fn part_bytes(bytes: &[u8], wanted: &str) -> Result<Vec<u8>> {
    let mut offset = 0;
    while offset + 30 <= bytes.len() && read_u32(bytes, offset)? == 0x0403_4b50 {
        let method = read_u16(bytes, offset + 8)?;
        let name_len = usize::from(read_u16(bytes, offset + 26)?);
        let extra_len = usize::from(read_u16(bytes, offset + 28)?);
        let compressed = usize::try_from(read_u32(bytes, offset + 18)?)
            .map_err(|_| Error::ValueNotRepresentable("ZIP entry too large"))?;
        let name_start = offset + 30;
        let name_end = name_start + name_len;
        let data_start = name_end + extra_len;
        let data_end = data_start + compressed;
        let name = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|_| Error::ValueNotRepresentable("ZIP name is not UTF-8"))?;
        if name == wanted {
            let data = bytes
                .get(data_start..data_end)
                .ok_or(Error::ValueNotRepresentable("truncated ZIP entry"))?;
            return match method {
                0 => Ok(data.to_vec()),
                8 => {
                    let mut decoded = Vec::new();
                    std::io::Read::read_to_end(
                        &mut flate2::read::DeflateDecoder::new(data),
                        &mut decoded,
                    )?;
                    Ok(decoded)
                }
                _ => Err(Error::ValueNotRepresentable(
                    "unsupported ZIP compression method",
                )),
            };
        }
        offset = data_end;
    }
    Err(Error::ValueNotRepresentable("ZIP part not found"))
}

#[cfg(test)]
fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = bytes
        .get(offset..offset + 2)
        .and_then(|slice| <[u8; 2]>::try_from(slice).ok())
        .ok_or(Error::ValueNotRepresentable("truncated ZIP header"))?;
    Ok(u16::from_le_bytes(value))
}

#[cfg(test)]
fn read_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    let value = bytes
        .get(offset..offset + 8)
        .and_then(|slice| <[u8; 8]>::try_from(slice).ok())
        .ok_or(Error::ValueNotRepresentable("truncated ZIP header"))?;
    Ok(u64::from_le_bytes(value))
}

#[cfg(test)]
fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .ok_or(Error::ValueNotRepresentable("truncated ZIP header"))?;
    Ok(u32::from_le_bytes(value))
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use deku::DekuContainerRead;
    use rust_xlsb_core::prelude::{
        Bold, BrtArrFmla, BrtBeginAFilter, BrtBeginCFRule, BrtBeginConditionalFormatting,
        BrtBeginCustomFilters, BrtBeginDVals, BrtBeginDatabar, BrtBeginFilterColumn,
        BrtBeginFilters, BrtBeginIconSet, BrtBeginList, BrtBeginListCol, BrtBeginListParts,
        BrtBeginWsView, BrtBkHim, BrtBookView, BrtBundleSh, BrtCFVO, BrtCalcProp, BrtCellMeta,
        BrtCellRString, BrtColor, BrtCustomFilter, BrtDVal, BrtDXF, BrtDrawing, BrtExternSheet,
        BrtFileSharing, BrtFilter, BrtFmlaBool, BrtFmlaError, BrtFmlaNum, BrtFmlaString, BrtFont,
        BrtHLink, BrtListPart, BrtListTrFmla, BrtMdb, BrtMdtinfo, BrtName, BrtRowHdr,
        BrtSheetProtection, BrtSparkline, BrtTableStyleClient, BrtWbProp, BrtWsFmtInfo, BrtWsProp,
        BrtXF, CF_TEMPLATE_CONTAINS_BLANKS, CF_TEMPLATE_CONTAINS_NO_ERRORS,
        CF_TEMPLATE_CONTAINS_TEXT, CF_TEMPLATE_DATABAR, CF_TEMPLATE_DUPLICATE_VALUES,
        CF_TEMPLATE_EQUAL_BELOW_AVERAGE, CF_TEMPLATE_FILTER, CF_TEMPLATE_FMLA,
        CF_TEMPLATE_GRADIENT, CF_TEMPLATE_MULTISTATE, CF_TEMPLATE_TIME_PERIOD_LAST_MONTH,
        CF_TEMPLATE_TIME_PERIOD_TODAY, CF_TEMPLATE_UNIQUE_VALUES, CF_TEXTOPER_CONTAINS,
        CF_TYPE_DATABAR, CF_TYPE_EXPRESSION, CF_TYPE_FILTER, CF_TYPE_GRADIENT, CF_TYPE_MULTISTATE,
        CFVO_MAX, CFVO_MIN, CFVO_NUM, CFVO_PERCENT, CFVO_PERCENTILE,
    };
    use rust_xlsb_core::{BiffHead, CheckedBiff, KnownID};

    use super::*;
    use crate::filter::{FilterCondition, FilterCriteria};
    use crate::{Table, TableColumn, TableStyle};

    fn record_ids(part: &[u8]) -> Vec<KnownID> {
        let mut reader = BufReader::new(part);
        let mut ids = Vec::new();
        while let Ok((head, _)) = BiffHead::read_from_with_data(&mut reader) {
            if let Some(id) = head.known_id() {
                ids.push(id);
            }
        }
        ids
    }

    fn records<T>(part: &[u8]) -> Vec<T>
    where
        T: for<'a> DekuContainerRead<'a> + Default + CheckedBiff,
    {
        let target = T::default().id();
        let mut reader = BufReader::new(part);
        let mut records = Vec::new();
        while let Ok((head, body)) = BiffHead::read_from_with_data(&mut reader) {
            if head.known_id() == Some(target)
                && let Some(body) = body
            {
                let (_, record) = T::from_bytes((&body, 0)).expect("record decodes");
                records.push(record);
            }
        }
        records
    }

    #[test]
    fn rich_string_emits_utf16_runs_and_fonts() {
        let mut workbook = crate::Workbook::new();
        let bold = crate::Format::new().set_bold();
        let large = crate::Format::new().set_font_size(14.0);
        workbook
            .add_worksheet()
            .write_rich_string(
                0,
                0,
                &[
                    (&crate::Format::default(), "A"),
                    (&bold, "😀"),
                    (&large, "B"),
                ],
            )
            .expect("rich string");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let cells = records::<BrtCellRString>(&sheet);
        assert_eq!(cells.len(), 1);
        assert_eq!(
            cells[0].value.str.to_optional_string().as_deref(),
            Some("A😀B")
        );
        let rich = cells[0].value.rich.as_ref().expect("formatted runs");
        assert_eq!(rich.dwSizeStrRun, 3);
        assert_eq!(
            rich.rgsStrRun.iter().map(|run| run.ich).collect::<Vec<_>>(),
            [0, 1, 3]
        );

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        let fonts = records::<BrtFont>(&styles);
        assert_eq!(fonts[rich.rgsStrRun[0].ifnt as usize], BrtFont::default());
        assert_eq!(fonts[rich.rgsStrRun[1].ifnt as usize].bls, Bold::BLSBOLD);
        assert_eq!(fonts[rich.rgsStrRun[2].ifnt as usize].dyHeight, 280);
    }

    #[test]
    fn conditional_format_emits_rule_and_dxf() {
        let mut workbook = crate::Workbook::new();
        let format = crate::Format::new()
            .set_background_color(crate::Color::Yellow)
            .set_font_color(crate::Color::Red);
        let rule = crate::ConditionalFormatCell::new()
            .set_rule(crate::ConditionalFormatCellRule::GreaterThan(5))
            .set_format(format);
        let worksheet = workbook.add_worksheet();
        worksheet.write_number(1, 0, 10).expect("value");
        worksheet
            .add_conditional_format(1, 0, 4, 0, &rule)
            .expect("conditional format");
        worksheet
            .add_conditional_format(
                5,
                0,
                6,
                0,
                &crate::ConditionalFormatCell::new()
                    .set_rule(crate::ConditionalFormatCellRule::EqualTo(0)),
            )
            .expect("conditional format without a format");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let blocks = records::<BrtBeginConditionalFormatting>(&sheet);
        let rules = records::<BrtBeginCFRule>(&sheet);
        let ids = record_ids(&sheet);
        let first_block = ids
            .iter()
            .position(|id| *id == KnownID::BrtBeginConditionalFormatting)
            .expect("conditional-format block");
        assert_eq!(ids[first_block - 1], KnownID::BrtEndSheetData);
        assert_eq!(ids[first_block + 1], KnownID::BrtBeginCFRule);
        assert!(!ids.contains(&KnownID::BrtBeginConditionalFormattings));
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].ccf, 1);
        assert_eq!(blocks[0].sqrfx.crfx, 1);
        assert_eq!(blocks[0].sqrfx.rgRefx[0].rwFirst, 1);
        assert_eq!(blocks[0].sqrfx.rgRefx[0].rwLast, 4);
        assert_eq!(blocks[0].sqrfx.rgRefx[0].colFirst, 0);
        assert_eq!(blocks[0].sqrfx.rgRefx[0].colLast, 0);

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].dxfId, 0);
        assert_eq!(rules[0].iPri, 1);
        assert_eq!(rules[0].iParam, 5);
        assert_eq!(rules[0].cbFmla1, 9);
        let formula = rules[0].formula1.as_ref().expect("formula");
        assert_eq!(formula.cce, 9);
        assert_eq!(formula.rgce[0], 0x1F);
        assert_eq!(&formula.rgce[1..], &5.0_f64.to_le_bytes());
        assert_eq!(rules[1].dxfId, u32::MAX);
        assert_eq!(rules[1].iPri, 2);
        assert_eq!(rules[1].iParam, 3);

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        let dxfs = records::<BrtDXF>(&styles);
        assert_eq!(dxfs.len(), 1);
        assert_eq!(dxfs[0].xfprops.cprops, 2);
        assert_eq!(dxfs[0].xfprops.props[0].xfPropType, 0x0002);
        assert_eq!(dxfs[0].xfprops.props[1].xfPropType, 0x0005);
    }

    #[test]
    fn conditional_format_rule_families_emit_expected_records() {
        let mut workbook = crate::Workbook::new();
        let red = crate::Format::new().set_background_color(crate::Color::Red);
        let green = crate::Format::new().set_background_color(crate::Color::Green);
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatTop::new()
                    .set_rule(crate::ConditionalFormatTopRule::BottomPercent(20))
                    .set_format(red.clone())
                    .set_stop_if_true(true),
            )
            .expect("top rule");
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatAverage::new()
                    .set_rule(crate::ConditionalFormatAverageRule::EqualOrBelowAverage)
                    .set_format(&red),
            )
            .expect("average rule");
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatDuplicate::new().set_format(green),
            )
            .expect("duplicate rule");
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatDuplicate::new().invert(),
            )
            .expect("unique rule");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].iType, CF_TYPE_FILTER);
        assert_eq!(rules[0].iTemplate, CF_TEMPLATE_FILTER);
        assert_eq!(rules[0].iParam, 20);
        assert!(rules[0].flags & BrtBeginCFRule::F_BOTTOM != 0);
        assert!(rules[0].flags & BrtBeginCFRule::F_PERCENT != 0);
        assert_eq!(rules[0].flags & BrtBeginCFRule::F_ABOVE, 0);
        assert!(rules[0].stop_if_true());
        assert_eq!(rules[1].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[1].iTemplate, CF_TEMPLATE_EQUAL_BELOW_AVERAGE);
        assert_eq!(rules[1].iParam, 0);
        assert_eq!(rules[1].flags & BrtBeginCFRule::F_ABOVE, 0);
        assert_eq!(rules[2].iTemplate, CF_TEMPLATE_DUPLICATE_VALUES);
        assert_eq!(rules[3].iTemplate, CF_TEMPLATE_UNIQUE_VALUES);
        for (index, rule) in rules.iter().enumerate() {
            assert_eq!(rule.iPri, i32::try_from(index + 1).expect("priority"));
            assert_eq!(rule.cbFmla1, 0);
            assert_eq!(rule.cbFmla2, 0);
            assert!(rule.formula1.is_none());
            assert!(rule.formula2.is_none());
            assert!(rule.strParam.is_null());
        }
        assert_eq!(rules[0].dxfId, 0);
        assert_eq!(rules[1].dxfId, 0);
        assert_eq!(rules[2].dxfId, 1);
        assert_eq!(rules[3].dxfId, u32::MAX);

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles).len(), 2);
    }

    #[test]
    fn text_blank_and_error_rules_emit_formulas_and_dxfs() {
        let mut workbook = crate::Workbook::new();
        let red = crate::Format::new().set_background_color(crate::Color::Red);
        let green = crate::Format::new().set_background_color(crate::Color::Green);
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatText::new()
                    .set_rule(crate::ConditionalFormatTextRule::Contains(
                        "rust".to_owned(),
                    ))
                    .set_format(red)
                    .set_stop_if_true(true),
            )
            .expect("text rule");
        worksheet
            .add_conditional_format(
                1,
                1,
                20,
                1,
                &crate::ConditionalFormatBlank::new().set_format(green),
            )
            .expect("blank rule");
        worksheet
            .add_conditional_format(1, 2, 20, 2, &crate::ConditionalFormatError::new().invert())
            .expect("error rule");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ids = record_ids(&sheet);
        assert_eq!(
            ids.iter()
                .filter(|id| **id == KnownID::BrtBeginConditionalFormatting)
                .count(),
            3
        );
        assert!(!ids.contains(&KnownID::BrtBeginConditionalFormattings));
        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 3);

        assert_eq!(rules[0].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[0].iTemplate, CF_TEMPLATE_CONTAINS_TEXT);
        assert_eq!(rules[0].iParam, CF_TEXTOPER_CONTAINS);
        assert_eq!(rules[0].dxfId, 0);
        assert_eq!(rules[0].iPri, 1);
        assert!(rules[0].stop_if_true());
        assert_eq!(
            rules[0].strParam.to_optional_string().as_deref(),
            Some("rust")
        );
        let formula = rules[0].formula1.as_ref().expect("text formula");
        assert_eq!(rules[0].cbFmla1, 1);
        assert_eq!(formula.rgce[0], 0x17);
        assert_eq!(&formula.rgce[1..3], &[4, 0]);
        assert_eq!(&formula.rgce[3..11], &[b'r', 0, b'u', 0, b's', 0, b't', 0]);
        assert!(rules[0].formula2.is_none());

        assert_eq!(rules[1].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[1].iTemplate, CF_TEMPLATE_CONTAINS_BLANKS);
        assert_eq!(rules[1].iParam, 0);
        assert_eq!(rules[1].dxfId, 1);
        assert_eq!(rules[1].iPri, 2);
        assert!(rules[1].strParam.is_null());
        let formula = rules[1].formula1.as_ref().expect("blank formula");
        assert_eq!(rules[1].cbFmla1, 1);
        assert_eq!(
            formula.rgce,
            [
                0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 0x76, 0, 0x41, 0x20, 0, 0x1E, 0, 0, 0x0B,
            ]
        );

        assert_eq!(rules[2].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[2].iTemplate, CF_TEMPLATE_CONTAINS_NO_ERRORS);
        assert_eq!(rules[2].iParam, 0);
        assert_eq!(rules[2].dxfId, u32::MAX);
        assert_eq!(rules[2].iPri, 3);
        assert!(rules[2].strParam.is_null());
        let formula = rules[2].formula1.as_ref().expect("error formula");
        assert_eq!(rules[2].cbFmla1, 1);
        assert_eq!(
            formula.rgce,
            [0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x41, 3, 0, 0x41, 0x26, 0]
        );

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles).len(), 2);
    }

    #[test]
    fn date_rules_emit_templates_parameters_and_formulas() {
        let mut workbook = crate::Workbook::new();
        let red = crate::Format::new().set_background_color(crate::Color::Red);
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(
                1,
                0,
                20,
                0,
                &crate::ConditionalFormatDate::new()
                    .set_rule(crate::ConditionalFormatDateRule::Today)
                    .set_format(red),
            )
            .expect("today rule");
        worksheet
            .add_conditional_format(
                1,
                1,
                20,
                1,
                &crate::ConditionalFormatDate::new()
                    .set_rule(crate::ConditionalFormatDateRule::LastMonth),
            )
            .expect("last month rule");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ids = record_ids(&sheet);
        assert_eq!(
            ids.iter()
                .filter(|id| **id == KnownID::BrtBeginConditionalFormatting)
                .count(),
            2
        );
        assert!(!ids.contains(&KnownID::BrtBeginConditionalFormattings));
        assert!(!ids.contains(&KnownID::BrtBeginCFRule14));
        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 2);

        assert_eq!(rules[0].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[0].iTemplate, CF_TEMPLATE_TIME_PERIOD_TODAY);
        assert_eq!(rules[0].iParam, 0);
        assert_eq!(rules[0].dxfId, 0);
        assert_eq!(rules[0].iPri, 1);
        assert!(rules[0].strParam.is_null());
        assert_eq!(rules[0].cbFmla1, 1);
        assert_eq!(
            rules[0].formula1.as_ref().expect("today formula").rgce,
            [
                0x4C, 0, 0, 0, 0, 0x00, 0xC0, 0x1E, 1, 0, 0x41, 0x1D, 1, 0x41, 0xDD, 0, 0x0B,
            ]
        );

        assert_eq!(rules[1].iTemplate, CF_TEMPLATE_TIME_PERIOD_LAST_MONTH);
        assert_eq!(rules[1].iParam, 5);
        assert_eq!(rules[1].dxfId, u32::MAX);
        assert_eq!(rules[1].iPri, 2);
        assert!(rules[1].strParam.is_null());
        let formula = rules[1].formula1.as_ref().expect("last month formula");
        assert_eq!(formula.rgce[0], 0x4C);
        assert_eq!(&formula.rgce[7..10], &[0x41, 0x44, 0]);
        assert_eq!(
            formula.cce,
            u32::try_from(formula.rgce.len()).expect("length")
        );

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles).len(), 1);
    }

    #[test]
    fn color_scales_emit_nested_records_without_dxf() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(0, 0, 4, 0, &crate::ConditionalFormat2ColorScale::new())
            .expect("two-color scale");
        worksheet
            .add_conditional_format(
                0,
                2,
                4,
                2,
                &crate::ConditionalFormat3ColorScale::new()
                    .set_minimum(crate::ConditionalFormatType::Number, 2)
                    .set_midpoint(crate::ConditionalFormatType::Percent, 60)
                    .set_maximum(crate::ConditionalFormatType::Percentile, 90),
            )
            .expect("three-color scale");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ids = record_ids(&sheet);
        let first = ids
            .iter()
            .position(|id| *id == KnownID::BrtBeginConditionalFormatting)
            .expect("first block");
        assert_eq!(
            &ids[first..first + 10],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginColorScale,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtEndColorScale,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );
        let second = first + 10;
        assert_eq!(
            &ids[second..second + 12],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginColorScale,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtColor,
                KnownID::BrtEndColorScale,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );

        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 2);
        for (index, rule) in rules.iter().enumerate() {
            assert_eq!(rule.iType, CF_TYPE_GRADIENT);
            assert_eq!(rule.iTemplate, CF_TEMPLATE_GRADIENT);
            assert_eq!(rule.dxfId, u32::MAX);
            assert_eq!(rule.iPri, i32::try_from(index + 1).expect("priority"));
            assert_eq!(rule.iParam, 0);
            assert_eq!(rule.flags, 0);
            assert_eq!(rule.cbFmla1, 0);
            assert_eq!(rule.cbFmla2, 0);
            assert!(rule.formula1.is_none());
            assert!(rule.formula2.is_none());
        }

        let cfvos = records::<BrtCFVO>(&sheet);
        assert_eq!(
            cfvos.iter().map(|cfvo| cfvo.iType).collect::<Vec<_>>(),
            [CFVO_MIN, CFVO_MAX, CFVO_NUM, CFVO_PERCENT, CFVO_PERCENTILE]
        );
        assert_eq!(f64::from(cfvos[2].numParam), 2.0);
        assert_eq!(f64::from(cfvos[3].numParam), 60.0);
        assert_eq!(f64::from(cfvos[4].numParam), 90.0);
        assert_eq!(records::<BrtColor>(&sheet).len(), 5);
        assert_eq!(
            ids.iter()
                .filter(|id| **id == KnownID::BrtEndColorScale)
                .count(),
            2
        );

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles), Vec::new());
    }

    #[test]
    fn data_bars_emit_nested_records_without_dxf() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(0, 0, 4, 0, &crate::ConditionalFormatDataBar::new())
            .expect("default data bar");
        worksheet
            .add_conditional_format(
                0,
                2,
                4,
                2,
                &crate::ConditionalFormatDataBar::new()
                    .set_minimum(crate::ConditionalFormatType::Number, 2)
                    .set_maximum(crate::ConditionalFormatType::Percentile, 90)
                    .set_fill_color(crate::Color::Red)
                    .set_bar_only(true),
            )
            .expect("custom data bar");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ids = record_ids(&sheet);
        let first = ids
            .iter()
            .position(|id| *id == KnownID::BrtBeginConditionalFormatting)
            .expect("first block");
        assert_eq!(
            &ids[first..first + 9],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginDatabar,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtEndDatabar,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );
        assert_eq!(
            &ids[first + 9..first + 18],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginDatabar,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtColor,
                KnownID::BrtEndDatabar,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );

        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 2);
        for (index, rule) in rules.iter().enumerate() {
            assert_eq!(rule.iType, CF_TYPE_DATABAR);
            assert_eq!(rule.iTemplate, CF_TEMPLATE_DATABAR);
            assert_eq!(rule.dxfId, u32::MAX);
            assert_eq!(rule.iPri, i32::try_from(index + 1).expect("priority"));
            assert_eq!(rule.iParam, 0);
            assert_eq!(rule.flags, 0);
            assert_eq!(rule.cbFmla1, 0);
            assert!(rule.formula1.is_none());
        }

        let bars = records::<BrtBeginDatabar>(&sheet);
        assert_eq!(bars.len(), 2);
        assert_eq!(
            (bars[0].bLenMin, bars[0].bLenMax, bars[0].fShowValue),
            (10, 90, 1)
        );
        assert_eq!(
            (bars[1].bLenMin, bars[1].bLenMax, bars[1].fShowValue),
            (10, 90, 0)
        );

        let cfvos = records::<BrtCFVO>(&sheet);
        assert_eq!(
            cfvos.iter().map(|cfvo| cfvo.iType).collect::<Vec<_>>(),
            [CFVO_MIN, CFVO_MAX, CFVO_NUM, CFVO_PERCENTILE]
        );
        assert_eq!(f64::from(cfvos[2].numParam), 2.0);
        assert_eq!(f64::from(cfvos[3].numParam), 90.0);

        let colors = records::<BrtColor>(&sheet);
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0], BrtColor::rgb(0x63, 0x8E, 0xC6));
        assert_eq!(colors[1], BrtColor::rgb(0xFF, 0x00, 0x00));

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles), Vec::new());
    }

    #[test]
    fn icon_sets_emit_nested_records_without_dxf() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(
                0,
                0,
                4,
                0,
                &crate::ConditionalFormatIconSet::new()
                    .set_icon_type(crate::ConditionalFormatIconType::ThreeTrafficLights),
            )
            .expect("default icon set");
        worksheet
            .add_conditional_format(
                0,
                0,
                4,
                0,
                &crate::ConditionalFormatIconSet::new()
                    .set_icon_type(crate::ConditionalFormatIconType::FiveArrows)
                    .reverse_icons(true)
                    .show_icons_only(true),
            )
            .expect("five-icon set");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ids = record_ids(&sheet);
        let first = ids
            .iter()
            .position(|id| *id == KnownID::BrtBeginConditionalFormatting)
            .expect("first block");
        assert_eq!(
            &ids[first..first + 9],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginIconSet,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtEndIconSet,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );
        assert_eq!(
            &ids[first + 9..first + 20],
            &[
                KnownID::BrtBeginConditionalFormatting,
                KnownID::BrtBeginCFRule,
                KnownID::BrtBeginIconSet,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtCFVO,
                KnownID::BrtEndIconSet,
                KnownID::BrtEndCFRule,
                KnownID::BrtEndConditionalFormatting,
            ]
        );

        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 2);
        for (index, rule) in rules.iter().enumerate() {
            assert_eq!(rule.iType, CF_TYPE_MULTISTATE);
            assert_eq!(rule.iTemplate, CF_TEMPLATE_MULTISTATE);
            assert_eq!(rule.dxfId, u32::MAX);
            assert_eq!(rule.iPri, i32::try_from(index + 1).expect("priority"));
            assert_eq!(rule.iParam, 0);
            assert_eq!(rule.flags, 0);
            assert_eq!(rule.cbFmla1, 0);
            assert!(rule.formula1.is_none());
        }

        let sets = records::<BrtBeginIconSet>(&sheet);
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].iSet, 3);
        assert_eq!(sets[0].flags, 0);
        assert_eq!(sets[1].iSet, 13);
        assert_eq!(
            sets[1].flags,
            BrtBeginIconSet::F_ICON | BrtBeginIconSet::F_REVERSE
        );

        let cfvos = records::<BrtCFVO>(&sheet);
        assert_eq!(cfvos.len(), 8);
        assert!(cfvos.iter().all(|cfvo| cfvo.iType == CFVO_PERCENT));
        assert!(
            cfvos
                .iter()
                .all(|cfvo| cfvo.fSaveGTE == 1 && cfvo.fGTE == 1)
        );
        assert_eq!(
            cfvos
                .iter()
                .map(|cfvo| f64::from(cfvo.numParam))
                .collect::<Vec<_>>(),
            [0.0, 33.0, 67.0, 0.0, 20.0, 40.0, 60.0, 80.0]
        );

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles), Vec::new());
    }

    #[test]
    fn data_bar_and_formatted_rule_keep_dxf_alignment() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_conditional_format(
                0,
                0,
                4,
                0,
                &crate::ConditionalFormatCell::new()
                    .set_rule(crate::ConditionalFormatCellRule::EqualTo(1))
                    .set_format(crate::Format::new().set_background_color(crate::Color::Yellow)),
            )
            .expect("formatted rule");
        worksheet
            .add_conditional_format(0, 0, 4, 0, &crate::ConditionalFormatDataBar::new())
            .expect("data bar");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].dxfId, 0);
        assert_eq!(rules[1].dxfId, u32::MAX);

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        assert_eq!(records::<BrtDXF>(&styles).len(), 1);
    }

    #[test]
    fn conditional_format_priorities_restart_on_each_sheet() {
        let mut workbook = crate::Workbook::new();
        for _ in 0..2 {
            let worksheet = workbook.add_worksheet();
            worksheet
                .add_conditional_format(
                    0,
                    0,
                    9,
                    0,
                    &crate::ConditionalFormatAverage::new()
                        .set_rule(crate::ConditionalFormatAverageRule::AboveAverage),
                )
                .expect("conditional format");
        }

        let bytes = workbook.save_to_buffer().expect("saves");
        for part in ["xl/worksheets/sheet1.bin", "xl/worksheets/sheet2.bin"] {
            let sheet = part_bytes(&bytes, part).expect("sheet");
            let rules = records::<BrtBeginCFRule>(&sheet);
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].iPri, 1);
        }
    }

    #[test]
    fn default_format_replaces_style_zero_and_sets_sheet_dimensions() {
        let mut workbook = crate::Workbook::new();
        workbook
            .set_default_format(&crate::Format::new().set_bold().set_font_size(14.0), 24, 80)
            .expect("default format");
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        let fonts = records::<BrtFont>(&styles);
        let xfs = records::<BrtXF>(&styles);

        assert_eq!(fonts.len(), 2);
        assert_eq!(fonts[1].bls, Bold::BLSBOLD);
        assert_eq!(fonts[1].dyHeight, 280);
        assert_eq!(xfs[0].ixfeParent, 0xFFFF);
        assert_eq!(xfs[0].iFont, 1);
        assert_eq!(xfs[1].ixfeParent, 0);
        assert_eq!(xfs[1].iFont, 1);

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let formats = records::<BrtWsFmtInfo>(&sheet);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].miyDefRwHeight, 360);
        assert_eq!(formats[0].dxGCol, 2_076);
    }

    #[test]
    fn print_ranges_are_absent_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        assert_eq!(records::<BrtName>(&workbook_part).len(), 0);
        assert_eq!(records::<BrtExternSheet>(&workbook_part).len(), 0);
    }

    #[test]
    fn print_ranges_emit_scoped_defined_names() {
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_print_area(0, 0, 4, 2).expect("print area");
        sheet.set_repeat_rows(0, 1).expect("repeat rows");
        sheet.set_repeat_columns(0, 1).expect("repeat columns");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].itab, 0);
        assert_eq!(names[0].flags, BrtName::F_BUILTIN);
        assert_eq!(
            names[0].name.to_optional_string().as_deref(),
            Some("_xlnm.Print_Area")
        );
        assert_eq!(names[0].formula.rgce, area3d(0, 0, 0, 4, 2).to_vec());
        assert_eq!(
            names[1].name.to_optional_string().as_deref(),
            Some("_xlnm.Print_Titles")
        );
        let mut expected = area3d(0, 0, 0, 1, MAX_COL).to_vec();
        expected.extend_from_slice(&area3d(0, 0, 0, MAX_ROW, 1));
        expected.push(0x10);
        assert_eq!(names[1].formula.rgce, expected);
    }

    #[test]
    fn print_titles_emit_single_axis_formulas_with_sheet_scope() {
        let mut workbook = crate::Workbook::new();
        let rows = workbook.add_worksheet();
        rows.set_repeat_rows(2, 3).expect("repeat rows");
        let columns = workbook.add_worksheet();
        columns.set_repeat_columns(2, 3).expect("repeat columns");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[1].firstSheet, 1);
        assert_eq!(names[0].itab, 0);
        assert_eq!(names[0].formula.rgce, area3d(0, 2, 0, 3, MAX_COL).to_vec());
        assert_eq!(names[1].itab, 1);
        assert_eq!(names[1].formula.rgce, area3d(1, 0, 2, MAX_ROW, 3).to_vec());
    }

    #[test]
    fn user_defined_names_emit_global_and_local_records() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, "x").expect("writes");
        workbook
            .define_name("Exchange_rate", "=0.96")
            .expect("constant");
        workbook
            .define_name("Sheet1!Rate", "=$A$1")
            .expect("local reference");
        workbook
            .define_name("Sales", "=Sheet1!$A$1:$B$2")
            .expect("global reference");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(names.len(), 3);

        let constant = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Exchange_rate"))
            .expect("constant name");
        assert_eq!(constant.itab, u32::MAX);
        assert_eq!(constant.flags, 0);
        let mut expected = vec![0x1F];
        expected.extend_from_slice(&0.96_f64.to_le_bytes());
        assert_eq!(constant.formula.rgce, expected);

        let local = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Rate"))
            .expect("local name");
        assert_eq!(local.itab, 0);
        assert_eq!(local.formula.rgce, [0x3A, 0, 0, 0, 0, 0, 0, 0, 0]);

        let global = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Sales"))
            .expect("global name");
        assert_eq!(global.itab, u32::MAX);
        assert_eq!(global.formula.rgce, area3d(0, 0, 0, 1, 1).to_vec());
    }

    #[test]
    fn formula_defined_names_emit_three_dimensional_tokens() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();
        workbook
            .define_name("Total", "=SUM(Sheet1!$A$1:$A$3)")
            .expect("formula name");
        workbook
            .define_name("Sheet2!Flag", "=$A$1>0")
            .expect("local formula name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 2);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 1);
        assert_eq!(extern_sheets[0].rgXti[1].firstSheet, 0);

        let total = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Total"))
            .expect("global formula name");
        let mut expected = area3d(1, 0, 0, 2, 0).to_vec();
        expected.extend_from_slice(&[0x42, 1, 0x04, 0]);
        assert_eq!(total.itab, u32::MAX);
        assert_eq!(total.formula.rgce, expected);

        let flag = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Flag"))
            .expect("local formula name");
        assert_eq!(flag.itab, 1);
        assert_eq!(
            flag.formula.rgce,
            [0x3A, 0, 0, 0, 0, 0, 0, 0, 0, 0x1E, 0, 0, 0x0D]
        );
    }

    #[test]
    fn name_references_use_the_btname_collection_order() {
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.write(0, 0, 1).expect("writes");
        sheet.set_print_area(0, 0, 1, 1).expect("print area");
        workbook
            .define_name("Base", "=Sheet1!$A$1")
            .expect("base name");
        workbook
            .define_name("Double", "=Base*2")
            .expect("referencing name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);

        assert_eq!(names.len(), 3);
        assert_eq!(
            names[0].name.to_optional_string().as_deref(),
            Some("_xlnm.Print_Area")
        );
        assert_eq!(names[1].name.to_optional_string().as_deref(), Some("Base"));
        assert_eq!(
            names[2].name.to_optional_string().as_deref(),
            Some("Double")
        );
        assert_eq!(names[2].formula.rgce, [0x23, 2, 0, 0, 0, 0x1E, 2, 0, 0x05]);
    }

    #[test]
    fn worksheet_range_names_share_one_extern_sheet_entry() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();
        workbook.add_worksheet();
        workbook
            .define_name("Total", "=SUM(Sheet1:Sheet3!$A$1:$A$3)")
            .expect("worksheet range name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[0].lastSheet, 2);

        let total = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Total"))
            .expect("range name");
        let mut expected = area3d(0, 0, 0, 2, 0).to_vec();
        expected.extend_from_slice(&[0x42, 1, 0x04, 0]);
        assert_eq!(total.itab, u32::MAX);
        assert_eq!(total.formula.rgce, expected);
    }

    #[test]
    fn sparkline_ranges_share_the_extern_sheet_collection() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();
        let first = crate::Sparkline::new().set_range(("Sheet1", 0, 0, 0, 4));
        let second = crate::Sparkline::new().set_range(("Sheet2", 1, 0, 1, 4));
        let repeat = crate::Sparkline::new().set_range(("Sheet1", 2, 0, 2, 4));
        let sheet1 = workbook.worksheet_from_index(0).expect("Sheet1");
        sheet1.add_sparkline(0, 5, &first).expect("first");
        sheet1.add_sparkline(1, 5, &repeat).expect("repeat");
        let cross = crate::Sparkline::new().set_range(("Sheet2", 2, 0, 2, 4));
        sheet1.add_sparkline(2, 5, &cross).expect("cross sheet");
        let group = crate::Sparkline::new().set_range(("Sheet2", 0, 0, 2, 4));
        sheet1
            .add_sparkline_group(0, 6, 2, 6, &group)
            .expect("group");
        workbook
            .worksheet_from_index(1)
            .expect("Sheet2")
            .add_sparkline(0, 5, &second)
            .expect("second");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);
        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 2);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[1].firstSheet, 1);

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let sparklines = records::<BrtSparkline>(&sheet);
        assert_eq!(sparklines.len(), 6);
        let formula = |record: &BrtSparkline| {
            record.frt.rgFormulas.as_ref().expect("formula").array[0]
                .formula
                .rgce
                .clone()
        };
        let sqref = |record: &BrtSparkline| {
            let range = &record.frt.rgSqrefs.as_ref().expect("sqrefs").array[0]
                .sqrfx
                .rgRefx[0];
            (range.rwFirst, range.colFirst)
        };
        assert_eq!(&formula(&sparklines[0])[1..3], &[0, 0]);
        assert_eq!(&formula(&sparklines[1])[1..3], &[0, 0]);
        assert_eq!(&formula(&sparklines[2])[1..3], &[1, 0]);
        for (index, row) in (0_u32..=2).enumerate() {
            let record = &sparklines[3 + index];
            assert_eq!(&formula(record)[1..3], &[1, 0]);
            assert_eq!(&formula(record)[3..7], &row.to_le_bytes());
            assert_eq!(sqref(record), (row, 6));
        }
    }

    #[test]
    fn sparkline_with_an_unknown_sheet_fails_at_save() {
        let mut workbook = crate::Workbook::new();
        let sparkline = crate::Sparkline::new().set_range(("Missing", 0, 0, 0, 4));
        workbook
            .add_worksheet()
            .add_sparkline(0, 5, &sparkline)
            .expect("stores");
        let error = workbook.save_to_buffer().expect_err("unknown sheet");
        assert!(matches!(error, Error::UnknownWorksheetNameOrIndex(name) if name == "Missing"));
    }

    #[test]
    fn local_name_on_a_second_sheet_uses_its_sheet_index() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();
        workbook
            .define_name("sheet2!Rate", "=$A$1")
            .expect("case-insensitive scope");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let name = records::<BrtName>(&workbook_part)
            .into_iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Rate"))
            .expect("name");
        assert_eq!(name.itab, 1);
        assert_eq!(&name.formula.rgce[1..3], &[0, 0]);

        let extern_sheets = records::<BrtExternSheet>(&workbook_part);
        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 1);
    }

    #[test]
    fn built_in_and_user_names_share_one_extern_sheet_collection() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .set_print_area(0, 0, 1, 1)
            .expect("print area");
        workbook.add_worksheet();
        workbook.define_name("Other", "=Sheet2!$A$1").expect("name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let names = records::<BrtName>(&workbook_part);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);

        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 2);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[1].firstSheet, 1);

        let user = names
            .iter()
            .find(|name| name.name.to_optional_string().as_deref() == Some("Other"))
            .expect("user name");
        assert_eq!(&user.formula.rgce[1..3], &[1, 0]);
    }

    #[test]
    fn constant_only_names_skip_the_external_sheet_block() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.define_name("Rate", "0.5").expect("name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        assert_eq!(records::<BrtName>(&workbook_part).len(), 1);
        assert_eq!(records::<BrtExternSheet>(&workbook_part).len(), 0);
    }

    #[test]
    fn defined_name_scopes_and_duplicates_are_checked_at_save() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.define_name("Rate", "=1").expect("name");
        assert!(workbook.define_name("rate", "=2").is_ok());
        assert!(workbook.save_to_buffer().is_err());

        let mut scoped = crate::Workbook::new();
        scoped.add_worksheet();
        scoped.add_worksheet();
        scoped.define_name("Sheet1!Rate", "=1").expect("local");
        scoped.define_name("Sheet2!Rate", "=2").expect("local");
        assert!(scoped.save_to_buffer().is_ok());

        let mut missing = crate::Workbook::new();
        missing.add_worksheet();
        missing.define_name("Missing!Rate", "=1").expect("stores");
        assert!(missing.save_to_buffer().is_err());
    }

    #[test]
    fn invalid_names_and_unsupported_formulas_are_rejected() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        assert!(workbook.define_name("A1", "=1").is_err());

        assert!(workbook.define_name("Rate", "=A1").is_ok());
        assert!(workbook.save_to_buffer().is_err());

        let mut function = crate::Workbook::new();
        function.add_worksheet();
        assert!(function.define_name("Rate", "=NOPE(1)").is_ok());
        assert!(function.save_to_buffer().is_err());

        let mut external = crate::Workbook::new();
        external.add_worksheet();
        assert!(
            external
                .define_name("Rate", "=SUM([Book]Sheet1!$A$1)")
                .is_ok()
        );
        assert!(external.save_to_buffer().is_err());
    }

    #[test]
    fn hyperlinks_emit_records_and_relationships() {
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet
            .write_url(0, 0, "https://example.com/a?x=1&y=2")
            .expect("url");
        sheet
            .write_url_with_text(1, 0, "internal:Sheet1!B2", "Jump")
            .expect("internal");
        sheet
            .write_url_with_options(
                2,
                0,
                "https://example.com/docs#intro",
                "Docs",
                "Open docs",
                None,
            )
            .expect("options");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let links = records::<BrtHLink>(&sheet_part);

        assert_eq!(links.len(), 3);
        assert_eq!(
            links[0].rfx,
            rust_xlsb_core::prelude::UncheckedRfX {
                rwFirst: 0,
                rwLast: 0,
                colFirst: 0,
                colLast: 0,
            }
        );
        assert_eq!(links[0].relId.to_optional_string().as_deref(), Some("rId1"));
        assert!(links[0].location.is_empty());
        assert!(links[0].tooltip.is_empty());
        assert!(links[0].display.is_empty());

        assert_eq!(links[1].relId.to_optional_string().as_deref(), Some(""));
        assert_eq!(
            links[1].location.to_optional_string().as_deref(),
            Some("Sheet1!B2")
        );
        assert_eq!(
            links[1].display.to_optional_string().as_deref(),
            Some("Jump")
        );

        assert_eq!(links[2].relId.to_optional_string().as_deref(), Some("rId2"));
        assert_eq!(
            links[2].location.to_optional_string().as_deref(),
            Some("intro")
        );
        assert_eq!(
            links[2].tooltip.to_optional_string().as_deref(),
            Some("Open docs")
        );

        let rels = part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels");
        let rels = String::from_utf8(rels).expect("xml");
        assert!(rels.contains(
            r#"Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/a?x=1&amp;y=2" TargetMode="External""#
        ));
        assert!(rels.contains(r#"Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com/docs" TargetMode="External""#));
        assert!(!rels.contains("Sheet1!B2"));
    }

    #[test]
    fn internal_hyperlinks_omit_sheet_relationships() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write_url(0, 0, "internal:Sheet1!A1")
            .expect("internal");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(!names.contains(&"xl/worksheets/_rels/sheet1.bin.rels".to_owned()));

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let links = records::<BrtHLink>(&sheet);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relId.to_optional_string().as_deref(), Some(""));
    }

    #[test]
    fn hyperlinks_are_absent_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(!names.iter().any(|name| name.contains("_rels/sheet")));
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtHLink>(&sheet).len(), 0);
    }

    #[test]
    fn worksheet_protection_is_written_to_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write(0, 0, "locked").expect("writes");
        worksheet.protect_with_password("abc123");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let protections = records::<BrtSheetProtection>(&sheet);
        assert_eq!(protections.len(), 1);
        assert_eq!(protections[0].protpwd, 0xC58F);
        assert_eq!(protections[0].fLocked, 1);
        assert_eq!(protections[0].fSelLockedCells, 1);
        assert_eq!(protections[0].fSelUnlockedCells, 1);

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        let xfs = records::<BrtXF>(&styles);
        assert!(xfs.len() >= 2);
        assert!(xfs[..2].iter().all(|xf| xf.fLocked));
    }

    #[test]
    fn protected_ranges_and_ignored_errors_reach_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write(0, 0, "x").expect("writes");
        worksheet.protect();
        worksheet
            .unprotect_range_with_options(1, 1, 2, 2, "Editable", "abc123")
            .expect("unprotect");
        worksheet
            .ignore_error_range(0, 0, 1, 1, crate::IgnoreError::NumberStoredAsText)
            .expect("ignore");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let ranges = records::<rust_xlsb_core::prelude::BrtRangeProtection>(&sheet);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].protPwd, 0xC58F);
        assert_eq!(ranges[0].sqRfX.rgRefx[0].rwFirst, 1);
        assert_eq!(
            ranges[0]
                .RangeProtectionTitleSDRel
                .rgchTitle
                .try_to_string()
                .expect("title"),
            "Editable"
        );

        let errors = records::<rust_xlsb_core::prelude::BrtCellIgnoreEC>(&sheet);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].grbit,
            rust_xlsb_core::prelude::BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT
        );
        assert_eq!(errors[0].sqrfxu.crfx, 1);
    }

    #[test]
    fn package_has_expected_parts() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "hello")
            .expect("writes");
        workbook.add_worksheet().set_name("Second").expect("name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");

        assert!(names.contains(&"[Content_Types].xml".to_owned()));
        assert!(names.contains(&"_rels/.rels".to_owned()));
        assert!(names.contains(&"xl/workbook.bin".to_owned()));
        assert!(names.contains(&"xl/worksheets/sheet1.bin".to_owned()));
        assert!(names.contains(&"xl/worksheets/sheet2.bin".to_owned()));
        assert!(names.contains(&"xl/styles.bin".to_owned()));
        assert!(names.contains(&"xl/sharedStrings.bin".to_owned()));
    }

    #[test]
    fn autofilter_reaches_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(1, 2, 5, 4).expect("autofilter");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let begin = records::<BrtBeginAFilter>(&sheet);
        assert_eq!(begin.len(), 1);
        assert_eq!(begin[0].rfx.rwFirst, 1);
        assert_eq!(begin[0].rfx.rwLast, 5);
        assert_eq!(begin[0].rfx.colFirst, 2);
        assert_eq!(begin[0].rfx.colLast, 4);

        let workbook = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let names = records::<BrtName>(&workbook);
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].flags, BrtName::F_BUILTIN | BrtName::F_HIDDEN);
        assert_eq!(
            names[0].name.to_optional_string().as_deref(),
            Some("_xlnm._FilterDatabase")
        );
        assert_eq!(names[0].formula.rgce, area3d(0, 1, 2, 5, 4).to_vec());
    }

    #[test]
    fn filter_conditions_emit_records_and_hide_rows() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(0, 1, 3, 2).expect("autofilter");
        worksheet.write_string(0, 1, "Region").expect("header");
        worksheet.write_string(0, 2, "Sales").expect("header");
        worksheet.write_string(1, 1, "East").expect("east");
        worksheet.write_string(2, 1, "West").expect("west");
        worksheet.write_number(1, 2, 4).expect("number");
        worksheet.write_number(2, 2, 9).expect("number");
        worksheet
            .filter_column(
                1,
                &FilterCondition::new()
                    .add_list_filter("East")
                    .add_list_blanks_filter(),
            )
            .expect("list filter");
        worksheet
            .filter_column(
                2,
                &FilterCondition::new()
                    .add_custom_filter(FilterCriteria::GreaterThanOrEqualTo, 4_i32)
                    .add_custom_filter(FilterCriteria::LessThanOrEqualTo, 8_i32),
            )
            .expect("custom filter");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");

        let columns = records::<BrtBeginFilterColumn>(&sheet);
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].dwCol, 0);
        assert_eq!(columns[1].dwCol, 1);

        let list = records::<BrtBeginFilters>(&sheet);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].fBlank, 1);
        let filters = records::<BrtFilter>(&sheet);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].rgch.try_to_string().expect("string"), "East");

        let custom = records::<BrtBeginCustomFilters>(&sheet);
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].fAnd, 0);
        let custom = records::<BrtCustomFilter>(&sheet);
        assert_eq!(custom.len(), 2);
        assert_eq!((custom[0].vts, custom[0].grbitSgn), (0x04, 0x06));
        assert_eq!(f64::from_le_bytes(custom[0].union), 4.0);
        assert_eq!((custom[1].vts, custom[1].grbitSgn), (0x04, 0x03));
        assert_eq!(f64::from_le_bytes(custom[1].union), 8.0);

        let ids = record_ids(&sheet);
        let position = |id: KnownID| {
            ids.iter()
                .position(|record_id| *record_id == id)
                .expect("record")
        };
        let list_start = position(KnownID::BrtBeginFilters);
        let list_end = position(KnownID::BrtEndFilters);
        let custom_start = position(KnownID::BrtBeginCustomFilters);
        let custom_end = position(KnownID::BrtEndCustomFilters);
        assert!(position(KnownID::BrtBeginAFilter) < position(KnownID::BrtBeginFilterColumn));
        assert!(list_start < list_end && list_end < custom_start);
        assert!(custom_start < custom_end && custom_end < position(KnownID::BrtEndAFilter));
        let column_ends: Vec<_> = ids
            .iter()
            .enumerate()
            .filter_map(|(index, id)| (*id == KnownID::BrtEndFilterColumn).then_some(index))
            .collect();
        assert_eq!(column_ends.len(), 2);
        assert!(column_ends[0] > list_end && column_ends[1] > custom_end);

        let rows = records::<BrtRowHdr>(&sheet);
        let hidden = |row: u32| {
            rows.iter()
                .find(|record| record.rw == row)
                .is_some_and(|record| record.fDyZero && record.miyRw == 0)
        };
        assert!(!hidden(1));
        assert!(hidden(2));
        assert!(hidden(3));
    }

    #[test]
    fn rich_strings_participate_in_autofilter_matching() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let bold = crate::Format::new().set_bold();
        worksheet.autofilter(0, 0, 3, 0).expect("autofilter");
        worksheet.write_string(0, 0, "Region").expect("header");
        worksheet
            .write_rich_string(1, 0, &[(&bold, "East")])
            .expect("east");
        worksheet
            .write_rich_string(2, 0, &[(&bold, "West")])
            .expect("west");
        worksheet
            .filter_column(0, &FilterCondition::new().add_list_filter("East"))
            .expect("filter");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let rows = records::<BrtRowHdr>(&sheet);
        let hidden = |row: u32| {
            rows.iter()
                .find(|record| record.rw == row)
                .is_some_and(|record| record.fDyZero && record.miyRw == 0)
        };
        assert!(!hidden(1));
        assert!(hidden(2));
        assert!(hidden(3));
    }

    #[test]
    fn data_validations_reach_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let list = crate::DataValidation::new()
            .allow_list_strings(&["East", "West"])
            .expect("list")
            .set_input_title("Region")
            .expect("input title")
            .set_input_message("Pick one")
            .expect("input message");
        worksheet
            .add_data_validation(1, 2, 2, 2, &list)
            .expect("list validation");

        let whole = crate::DataValidation::new()
            .allow_whole_number(crate::DataValidationRule::Between(1, 10))
            .ignore_blank(false)
            .set_error_style(crate::DataValidationErrorStyle::Warning);
        worksheet
            .add_data_validation(4, 2, 4, 2, &whole)
            .expect("whole validation");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let begin = records::<BrtBeginDVals>(&sheet);
        assert_eq!(begin.len(), 1);
        assert_eq!(begin[0].dVals.idvMac, 2);

        let validations = records::<BrtDVal>(&sheet);
        assert_eq!(validations.len(), 2);
        assert_eq!(validations[0].val_type(), 3);
        assert_eq!(validations[0].operator(), 0);
        assert_eq!(validations[0].sqrfx.rgRefx[0].rwFirst, 1);
        assert_eq!(validations[0].sqrfx.rgRefx[0].colFirst, 2);
        assert_eq!(
            validations[0]
                .strings
                .strPromptTitle
                .to_optional_string()
                .as_deref(),
            Some("Region")
        );
        assert_eq!(
            validations[0]
                .strings
                .strPrompt
                .to_optional_string()
                .as_deref(),
            Some("Pick one")
        );
        assert_eq!(validations[0].formula1.rgce[0], 0x17);
        assert_eq!(&validations[0].formula1.rgce[1..3], &9_u16.to_le_bytes());
        let units = validations[0].formula1.rgce[3..]
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        assert_eq!(String::from_utf16(&units).expect("utf16"), "East,West");

        assert_eq!(validations[1].val_type(), 1);
        assert_eq!(validations[1].operator(), 0);
        assert_eq!(validations[1].flags & BrtDVal::F_ALLOW_BLANK, 0);
        assert_eq!(validations[1].err_style(), 1);
        assert_eq!(
            f64::from_le_bytes(
                validations[1].formula1.rgce[1..9]
                    .try_into()
                    .expect("8 bytes")
            ),
            1.0
        );
        assert_eq!(
            f64::from_le_bytes(
                validations[1].formula2.rgce[1..9]
                    .try_into()
                    .expect("8 bytes")
            ),
            10.0
        );

        let ids = record_ids(&sheet);
        let position = |id: KnownID| {
            ids.iter()
                .position(|record_id| *record_id == id)
                .expect("record")
        };
        assert!(position(KnownID::BrtBeginDVals) < position(KnownID::BrtDVal));
        assert!(position(KnownID::BrtDVal) < position(KnownID::BrtEndDVals));
    }

    #[test]
    fn formula_validations_and_conditional_formats_reach_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();

        let limit = crate::DataValidation::new().allow_whole_number_formula(
            crate::DataValidationRule::LessThanOrEqualTo(crate::Formula::new("=D1")),
        );
        worksheet
            .add_data_validation(1, 0, 10, 0, &limit)
            .expect("formula limit");

        let list = crate::DataValidation::new().allow_list_formula(crate::Formula::new("=F1:F9"));
        worksheet
            .add_data_validation(1, 1, 10, 1, &list)
            .expect("formula list");

        let custom = crate::DataValidation::new().allow_custom(crate::Formula::new("=A1>0"));
        worksheet
            .add_data_validation(1, 2, 10, 2, &custom)
            .expect("custom formula");

        let rule = crate::ConditionalFormatFormula::new()
            .set_rule("=ISODD(B3)")
            .set_format(crate::Format::new().set_background_color(crate::Color::Yellow))
            .set_stop_if_true(true);
        worksheet
            .add_conditional_format(2, 1, 11, 1, &rule)
            .expect("formula conditional format");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");

        let validations = records::<BrtDVal>(&sheet);
        assert_eq!(validations.len(), 3);
        assert_eq!(
            validations
                .iter()
                .map(BrtDVal::val_type)
                .collect::<Vec<_>>(),
            [1, 3, 7]
        );
        assert_eq!(
            validations
                .iter()
                .map(BrtDVal::operator)
                .collect::<Vec<_>>(),
            [7, 0, 0]
        );
        assert_eq!(
            validations[0].formula1.rgce,
            [0x4C, 0xFF, 0xFF, 0x0F, 0x00, 3, 0xC0]
        );
        assert_eq!(
            validations[1].formula1.rgce,
            [0x4D, 0xFF, 0xFF, 0x0F, 0x00, 7, 0, 0, 0, 4, 0xC0, 4, 0xC0]
        );
        assert_eq!(
            validations[2].formula1.rgce,
            [0x4C, 0xFF, 0xFF, 0x0F, 0x00, 0xFE, 0xFF, 0x1E, 0, 0, 0x0D]
        );

        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].iType, CF_TYPE_EXPRESSION);
        assert_eq!(rules[0].iTemplate, CF_TEMPLATE_FMLA);
        assert_eq!(rules[0].iParam, 0);
        assert_eq!(rules[0].cbFmla1, 1);
        assert!(rules[0].stop_if_true());
        let formula = rules[0].formula1.as_ref().expect("formula");
        assert_eq!(formula.rgce, [0x4C, 0, 0, 0, 0, 0, 0xC0, 0x41, 0xA5, 0x01]);
        assert_eq!(formula.cce as usize, formula.rgce.len());

        let ids = record_ids(&sheet);
        let position = |id: KnownID| {
            ids.iter()
                .position(|record_id| *record_id == id)
                .expect("record")
        };
        assert!(position(KnownID::BrtBeginDVals) < position(KnownID::BrtDVal));
        assert!(position(KnownID::BrtDVal) < position(KnownID::BrtEndDVals));
        assert!(ids.contains(&KnownID::BrtBeginConditionalFormatting));
    }

    #[test]
    fn multi_range_validations_and_conditional_formats_reach_the_sheet_part() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();

        let validation = crate::DataValidation::new()
            .allow_whole_number(crate::DataValidationRule::GreaterThan(0))
            .set_multi_range("B3:D6 I3:K6");
        worksheet
            .add_data_validation(0, 0, 0, 0, &validation)
            .expect("multi-range validation");

        let rule = crate::ConditionalFormatFormula::new()
            .set_rule("=A1")
            .set_multi_range("$I$3:$K$6,$B$9:$D$12");
        worksheet
            .add_conditional_format(0, 0, 0, 0, &rule)
            .expect("multi-range conditional format");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");

        let validations = records::<BrtDVal>(&sheet);
        assert_eq!(validations.len(), 1);
        assert_eq!(validations[0].sqrfx.crfx, 2);
        assert_eq!(
            validations[0]
                .sqrfx
                .rgRefx
                .iter()
                .map(|range| (range.rwFirst, range.rwLast, range.colFirst, range.colLast))
                .collect::<Vec<_>>(),
            [(2, 5, 1, 3), (2, 5, 8, 10)]
        );

        let blocks = records::<BrtBeginConditionalFormatting>(&sheet);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].sqrfx.crfx, 2);
        assert_eq!(
            blocks[0]
                .sqrfx
                .rgRefx
                .iter()
                .map(|range| (range.rwFirst, range.rwLast, range.colFirst, range.colLast))
                .collect::<Vec<_>>(),
            [(2, 5, 8, 10), (8, 11, 1, 3)]
        );

        let rules = records::<BrtBeginCFRule>(&sheet);
        assert_eq!(rules.len(), 1);
        let formula = rules[0].formula1.as_ref().expect("formula");
        let expected = crate::formula::compile_relative("=A1", 2, 1).expect("compiled");
        assert_eq!(formula.rgce, expected.rgce);
    }

    #[test]
    fn invalid_any_and_unused_validations_emit_no_dvals() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_data_validation(0, 0, 0, 0, &crate::DataValidation::new())
            .expect("empty any");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert!(record_ids(&sheet).iter().all(|id| !matches!(
            id,
            KnownID::BrtBeginDVals | KnownID::BrtDVal | KnownID::BrtEndDVals
        )));
    }

    #[test]
    fn custom_filter_or_sets_the_or_relationship() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(0, 0, 2, 0).expect("autofilter");
        worksheet
            .filter_column(
                0,
                &FilterCondition::new()
                    .add_custom_filter(FilterCriteria::EqualTo, 3000_i32)
                    .add_custom_filter(FilterCriteria::EqualTo, 9000_i32)
                    .add_custom_boolean_or(),
            )
            .expect("filter");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let custom = records::<BrtBeginCustomFilters>(&sheet);
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].fAnd, 1);
    }

    #[test]
    fn non_blank_custom_filters_use_the_special_value_type() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(0, 0, 2, 1).expect("autofilter");
        worksheet
            .filter_column(
                0,
                &FilterCondition::new().add_custom_filter(FilterCriteria::NotEqualTo, " "),
            )
            .expect("nonblank");
        worksheet
            .filter_column(
                1,
                &FilterCondition::new().add_custom_filter(FilterCriteria::EqualTo, " "),
            )
            .expect("blank");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let custom = records::<BrtCustomFilter>(&sheet);
        assert_eq!(custom.len(), 2);
        assert_eq!(custom[0].vts, 0x0E);
        assert_eq!(custom[1].vts, 0x06);
        assert_eq!(
            custom[1]
                .vtsStringXls
                .as_ref()
                .expect("string")
                .try_to_string()
                .expect("value"),
            " "
        );
    }

    #[test]
    fn filter_automatic_off_keeps_records_without_hiding_rows() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(0, 0, 2, 0).expect("autofilter");
        worksheet.write_string(1, 0, "East").expect("east");
        worksheet.write_string(2, 0, "West").expect("west");
        worksheet
            .filter_column(0, &FilterCondition::new().add_list_filter("East"))
            .expect("filter");
        worksheet.filter_automatic_off();

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtBeginFilterColumn>(&sheet).len(), 1);
        assert!(
            records::<BrtRowHdr>(&sheet)
                .iter()
                .all(|record| !record.fDyZero)
        );
    }

    #[test]
    fn autofilter_and_print_area_share_one_external_sheet_index() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.autofilter(0, 0, 3, 1).expect("autofilter");
        worksheet.set_print_area(0, 0, 3, 1).expect("print area");

        let bytes = workbook.save_to_buffer().expect("saves");
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);
        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);

        let names = records::<BrtName>(&workbook_part);
        assert_eq!(names.len(), 2);
        let name = |index: usize| names[index].name.to_optional_string().expect("name");
        assert_eq!(name(0), "_xlnm.Print_Area");
        assert_eq!(name(1), "_xlnm._FilterDatabase");
        assert_eq!(names[0].formula.rgce, area3d(0, 0, 0, 3, 1).to_vec());
        assert_eq!(names[1].formula.rgce, area3d(0, 0, 0, 3, 1).to_vec());
    }

    #[test]
    fn tables_emit_part_reference_relationship_and_content_type() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let table = Table::new()
            .set_name("Sales")
            .set_style(TableStyle::Light11)
            .set_columns(&[
                TableColumn::new().set_header("Product"),
                TableColumn::new().set_header("Q1"),
            ]);
        worksheet.add_table(1, 0, 4, 1, &table).expect("table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        let list = records::<BrtBeginList>(&table_part);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].idList, 1);
        assert_eq!(
            list[0].stName.to_optional_string().as_deref(),
            Some("Sales")
        );
        assert_eq!(list[0].crwHeader, 1);
        assert_eq!(
            list[0].stDisplayName.to_optional_string().as_deref(),
            Some("Sales")
        );

        let columns = records::<BrtBeginListCol>(&table_part);
        assert_eq!(columns.len(), 2);
        assert!(columns[0].stName.is_null());
        assert_eq!(
            columns[0].stCaption.to_optional_string().as_deref(),
            Some("Product")
        );

        let styles = records::<BrtTableStyleClient>(&table_part);
        assert_eq!(styles.len(), 1);
        assert_eq!(
            styles[0].stStyleName.to_optional_string().as_deref(),
            Some("TableStyleLight11")
        );

        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let parts = records::<BrtBeginListParts>(&sheet_part);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].cParts, 1);
        let table_refs = records::<BrtListPart>(&sheet_part);
        assert_eq!(table_refs.len(), 1);
        assert_eq!(
            table_refs[0].stRelID.to_optional_string().as_deref(),
            Some("rId1")
        );

        let rels_xml = String::from_utf8(
            part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels"),
        )
        .expect("utf8");
        assert!(rels_xml.contains(r#"Target="../tables/table1.bin""#));
        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("content types"))
                .expect("utf8");
        assert!(content_types.contains(
            r#"<Override PartName="/xl/tables/table1.bin" ContentType="application/vnd.ms-excel.table"/>"#
        ));
    }

    #[test]
    fn tables_and_hyperlinks_use_distinct_relationship_ids() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .add_table(0, 0, 2, 0, &Table::new())
            .expect("table");
        worksheet
            .write_url(4, 0, "https://example.com")
            .expect("url");

        let bytes = workbook.save_to_buffer().expect("saves");
        let rels = String::from_utf8(
            part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels"),
        )
        .expect("utf8");
        assert!(rels.contains(r#"Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableSingleCells""#));
        assert!(rels.contains(r#"Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink""#));

        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let links = records::<BrtHLink>(&sheet_part);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].relId.to_optional_string().as_deref(), Some("rId2"));
    }

    #[test]
    fn table_totals_emit_flags_functions_and_formulas() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let table =
            Table::new().set_total_row(true).set_columns(&[
                TableColumn::new()
                    .set_header("Name")
                    .set_total_label("Totals"),
                TableColumn::new()
                    .set_header("Q1")
                    .set_total_function(crate::TableFunction::Sum),
                TableColumn::new().set_header("Q2").set_total_function(
                    crate::TableFunction::Custom(crate::Formula::new("=SUM(C2:C3)")),
                ),
            ]);
        worksheet.add_table(0, 0, 3, 2, &table).expect("table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        let list = records::<BrtBeginList>(&table_part);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].crwTotals, 1);
        assert_eq!(list[0].flags, BrtBeginList::F_SHOWN_TOTAL_ROW);

        let filters = records::<BrtBeginAFilter>(&table_part);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].rfx.rwLast, 2);

        let columns = records::<BrtBeginListCol>(&table_part);
        assert_eq!(columns.len(), 3);
        assert_eq!(columns[0].ilta, 0);
        assert_eq!(
            columns[0].stTotal.to_optional_string().as_deref(),
            Some("Totals")
        );
        assert_eq!(columns[1].ilta, 6);
        assert!(columns[1].stTotal.is_null());
        assert_eq!(columns[2].ilta, 9);
        assert!(columns[2].stTotal.is_null());

        let formulas = records::<BrtListTrFmla>(&table_part);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0].formula,
            crate::formula::compile("=SUM(C2:C3)").expect("compiled")
        );
    }

    #[test]
    fn table_filters_emit_records_and_hide_data_rows() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_string(1, 2, "Region").expect("header");
        worksheet.write_string(2, 3, "East").expect("east");
        worksheet.write_string(3, 3, "West").expect("west");
        worksheet.write_string(4, 3, "East").expect("east");
        let table = Table::new()
            .set_total_row(true)
            .set_columns(&[
                TableColumn::new().set_header("Region"),
                TableColumn::new().set_header("Q1"),
                TableColumn::new().set_header("Q2"),
            ])
            .filter_column(1, &FilterCondition::new().add_list_filter("East"));
        worksheet.add_table(1, 2, 5, 4, &table).expect("table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        let columns = records::<BrtBeginFilterColumn>(&table_part);
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].dwCol, 1);
        let filters = records::<BrtFilter>(&table_part);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].rgch.try_to_string().expect("string"), "East");

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let rows = records::<BrtRowHdr>(&sheet);
        let hidden = |row: u32| {
            rows.iter()
                .find(|record| record.rw == row)
                .is_some_and(|record| record.fDyZero && record.miyRw == 0)
        };
        assert!(!hidden(1));
        assert!(!hidden(2));
        assert!(hidden(3));
        assert!(!hidden(4));
        assert!(!hidden(5));
    }

    #[test]
    fn table_calculated_columns_emit_formula_and_external_sheet() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let table = Table::new().set_name("Sales").set_columns(&[
            TableColumn::new().set_header("Product"),
            TableColumn::new().set_header("Q1"),
            TableColumn::new()
                .set_header("Q2")
                .set_formula("SUM(Sales[@[Q1]:[Q2]])"),
        ]);
        worksheet.add_table(0, 0, 3, 2, &table).expect("table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        let formulas = records::<rust_xlsb_core::prelude::BrtListCCFmla>(&table_part);
        assert_eq!(formulas.len(), 1);

        let columns = [
            TableColumn::new().set_header("Product"),
            TableColumn::new().set_header("Q1"),
            TableColumn::new().set_header("Q2"),
        ];
        let tables = [crate::formula::TableInfo {
            name: "Sales",
            id: 1,
            ixti: 0,
            columns: &columns,
        }];
        assert_eq!(
            formulas[0].formula,
            crate::formula::compile_table_formula("SUM(Sales[@[Q1]:[Q2]])", &tables)
                .expect("compiled")
        );

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        assert_eq!(records::<BrtName>(&workbook_part).len(), 0);
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);
        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 1);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[0].lastSheet, 0);
    }

    #[test]
    fn table_calculated_columns_use_default_table_names() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_columns(&[
                    TableColumn::new().set_header("Name"),
                    TableColumn::new()
                        .set_header("Value")
                        .set_formula("Table1[Value]"),
                ]),
            )
            .expect("table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        assert_eq!(
            records::<rust_xlsb_core::prelude::BrtListCCFmla>(&table_part).len(),
            1
        );
    }

    #[test]
    fn table_calculated_columns_reference_tables_on_other_sheets() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_name("Sales").set_columns(&[
                    TableColumn::new().set_header("Name"),
                    TableColumn::new()
                        .set_header("Total")
                        .set_formula("Other[Value]"),
                ]),
            )
            .expect("first table");
        workbook
            .add_worksheet()
            .add_table(
                0,
                0,
                2,
                1,
                &Table::new().set_name("Other").set_columns(&[
                    TableColumn::new().set_header("Name"),
                    TableColumn::new().set_header("Value"),
                ]),
            )
            .expect("second table");

        let bytes = workbook.save_to_buffer().expect("saves");
        let table_part = part_bytes(&bytes, "xl/tables/table1.bin").expect("table part");
        let formulas = records::<rust_xlsb_core::prelude::BrtListCCFmla>(&table_part);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            &formulas[0].formula.rgce[..4],
            &[0x18, 0x19, 1, 0],
            "table on the second sheet uses Xti 1"
        );

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let extern_sheets = records::<BrtExternSheet>(&workbook_part);
        assert_eq!(extern_sheets.len(), 1);
        assert_eq!(extern_sheets[0].cXti, 2);
        assert_eq!(extern_sheets[0].rgXti[0].firstSheet, 0);
        assert_eq!(extern_sheets[0].rgXti[1].firstSheet, 1);
    }

    #[test]
    fn table_calculated_columns_reject_unknown_names() {
        for formula in ["Missing[Value]", "Table1[Missing]"] {
            let mut workbook = crate::Workbook::new();
            workbook
                .add_worksheet()
                .add_table(
                    0,
                    0,
                    2,
                    1,
                    &Table::new().set_columns(&[
                        TableColumn::new().set_header("Name"),
                        TableColumn::new().set_header("Value").set_formula(formula),
                    ]),
                )
                .expect("table");
            assert!(
                matches!(
                    workbook.save_to_buffer(),
                    Err(crate::Error::FormulaError(_))
                ),
                "{formula}"
            );
        }
    }

    #[test]
    fn table_ids_and_default_names_are_workbook_scoped() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new())
            .expect("first");
        workbook
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new())
            .expect("second");

        let bytes = workbook.save_to_buffer().expect("saves");
        let first = records::<BrtBeginList>(
            &part_bytes(&bytes, "xl/tables/table1.bin").expect("first part"),
        );
        let second = records::<BrtBeginList>(
            &part_bytes(&bytes, "xl/tables/table2.bin").expect("second part"),
        );
        assert_eq!(first[0].idList, 1);
        assert_eq!(second[0].idList, 2);
        assert_eq!(
            first[0].stDisplayName.to_optional_string().as_deref(),
            Some("Table1")
        );
        assert_eq!(
            second[0].stDisplayName.to_optional_string().as_deref(),
            Some("Table2")
        );
    }

    #[test]
    fn table_names_cannot_collide_with_defined_names() {
        let mut workbook = crate::Workbook::new();
        workbook.define_name("Sales", "=1").expect("name");
        workbook
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new().set_name("Sales"))
            .expect("table");
        assert!(matches!(
            workbook.save_to_buffer(),
            Err(Error::TableError(_))
        ));
    }

    #[test]
    fn duplicate_table_names_are_rejected_at_save() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new().set_name("Sales"))
            .expect("first");
        workbook
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new().set_name("sales"))
            .expect("second");
        assert!(matches!(
            workbook.save_to_buffer(),
            Err(Error::TableError(_))
        ));

        let mut default_collision = crate::Workbook::new();
        default_collision
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new().set_name("Table2"))
            .expect("custom");
        default_collision
            .add_worksheet()
            .add_table(0, 0, 2, 0, &Table::new())
            .expect("default");
        assert!(matches!(
            default_collision.save_to_buffer(),
            Err(Error::TableError(_))
        ));
    }

    #[test]
    fn tables_are_absent_from_default_output() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "plain")
            .expect("write");
        let bytes = workbook.save_to_buffer().expect("saves");
        assert!(part_bytes(&bytes, "xl/tables/table1.bin").is_err());
        assert!(part_bytes(&bytes, "xl/theme/theme1.xml").is_err());
        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("content types UTF-8");
        assert!(!content_types.contains("/xl/theme/theme1.xml"));
        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("rels UTF-8");
        assert!(!rels.contains("relationships/theme"));
        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        assert_eq!(records::<BrtWbProp>(&workbook_part).len(), 0);
        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtBeginListParts>(&sheet_part).len(), 0);
        assert!(part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").is_err());
    }

    #[test]
    fn excel_2023_theme_emits_part_version_and_default_font() {
        let mut workbook = crate::Workbook::new();
        workbook.use_excel_2023_theme().expect("theme");
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let theme = part_bytes(&bytes, "xl/theme/theme1.xml").expect("theme part");
        let theme = String::from_utf8(theme).expect("theme UTF-8");
        assert!(theme.contains(r#"<a:latin typeface="Aptos Narrow""#));
        assert!(theme.contains(r#"<a:srgbClr val="156082"/>"#));

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("content types UTF-8");
        assert!(content_types.contains(
            r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>"#
        ));

        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("rels UTF-8");
        assert!(rels.contains(
            r#"Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml""#
        ));

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let props = records::<BrtWbProp>(&workbook_part);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].flags, 0);
        assert_eq!(props[0].dwThemeVersion, 202_300);
        assert_eq!(props[0].strName.to_optional_string().as_deref(), Some(""));

        let styles = part_bytes(&bytes, "xl/styles.bin").expect("styles");
        let fonts = records::<BrtFont>(&styles);
        let default = fonts
            .iter()
            .find(|font| font.name.to_optional_string().as_deref() == Some("Aptos Narrow"))
            .expect("Aptos Narrow font");
        assert_eq!(default.dyHeight, 220);
        assert_eq!(default.bFontScheme, 2);

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let formats = records::<BrtWsFmtInfo>(&sheet);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].miyDefRwHeight, 300);
        assert_eq!(formats[0].dxGCol, 2_158);
    }

    #[test]
    fn custom_theme_emits_verbatim_xml_and_zero_version() {
        let theme_xml = r#"<?xml version="1.0"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Custom"/>"#;
        let path =
            std::env::temp_dir().join(format!("rust_xlsb_custom_theme_{}.xml", std::process::id()));
        std::fs::write(&path, theme_xml).expect("theme file");

        let mut workbook = crate::Workbook::new();
        workbook.use_custom_theme(&path).expect("theme");
        workbook.add_worksheet().write(0, 0, "x").expect("writes");
        std::fs::remove_file(path).expect("remove theme file");

        let bytes = workbook.save_to_buffer().expect("saves");
        let theme = part_bytes(&bytes, "xl/theme/theme1.xml").expect("theme part");
        assert_eq!(String::from_utf8(theme).expect("theme UTF-8"), theme_xml);

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("content types UTF-8");
        assert!(content_types.contains(
            r#"<Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>"#
        ));

        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("rels UTF-8");
        assert!(rels.contains(
            r#"Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml""#
        ));

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let props = records::<BrtWbProp>(&workbook_part);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].dwThemeVersion, 0);
    }

    #[test]
    fn theme_relationship_uses_the_first_free_id() {
        let mut workbook = crate::Workbook::new();
        workbook.use_excel_2023_theme().expect("theme");
        workbook.add_worksheet().write(0, 0, 1).expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        assert!(part_bytes(&bytes, "xl/sharedStrings.bin").is_err());
        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("rels UTF-8");
        assert!(rels.contains(
            r#"Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme""#
        ));
    }

    #[test]
    fn autofilter_is_absent_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, "x").expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtBeginAFilter>(&sheet).len(), 0);
    }

    #[test]
    fn document_properties_add_core_and_app_parts() {
        let mut workbook = crate::Workbook::new();
        let properties = crate::DocProperties::new()
            .set_title("R&D <2026>")
            .set_subject("Subject")
            .set_author("A <B>")
            .set_manager("Manager")
            .set_company("Company & Co")
            .set_category("Category")
            .set_keywords("one, two")
            .set_comment("Comment")
            .set_status("Final")
            .set_hyperlink_base("https://example.com/?a=1&b=2");
        workbook.set_properties(&properties);
        workbook.add_worksheet().set_name("First").expect("name");
        workbook
            .add_worksheet()
            .set_name("Hidden")
            .expect("name")
            .set_hidden(true);
        workbook
            .add_worksheet()
            .set_name("Secret")
            .expect("name")
            .set_very_hidden(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(names.contains(&"docProps/core.xml".to_owned()));
        assert!(names.contains(&"docProps/app.xml".to_owned()));

        let core = String::from_utf8(part_bytes(&bytes, "docProps/core.xml").expect("core"))
            .expect("core UTF-8");
        assert!(core.contains("<dc:title>R&amp;D &lt;2026&gt;</dc:title>"));
        assert!(core.contains("<dc:creator>A &lt;B&gt;</dc:creator>"));
        assert!(core.contains("<cp:contentStatus>Final</cp:contentStatus>"));
        assert!(!core.contains("dcterms:created"));
        assert!(!core.contains("dcterms:modified"));

        let app = String::from_utf8(part_bytes(&bytes, "docProps/app.xml").expect("app"))
            .expect("app UTF-8");
        assert!(app.contains("<vt:i4>2</vt:i4>"));
        assert!(app.contains("<vt:lpstr>First</vt:lpstr>"));
        assert!(app.contains("<vt:lpstr>Hidden</vt:lpstr>"));
        assert!(!app.contains("Secret"));
        assert!(app.contains("<Company>Company &amp; Co</Company>"));
        assert!(app.contains("<HyperlinkBase>https://example.com/?a=1&amp;b=2</HyperlinkBase>"));

        let types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("content types"))
                .expect("types UTF-8");
        assert!(types.contains("/docProps/core.xml"));
        assert!(types.contains("/docProps/app.xml"));

        let rels = String::from_utf8(part_bytes(&bytes, "_rels/.rels").expect("rels"))
            .expect("rels UTF-8");
        assert!(rels.contains("Target=\"docProps/core.xml\""));
        assert!(rels.contains("Target=\"docProps/app.xml\""));
    }

    #[test]
    fn document_custom_properties_add_custom_part() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 18).expect("date");
        let utc = chrono::DateTime::from_naive_utc_and_offset(
            date.and_hms_opt(12, 34, 56).expect("time"),
            chrono::Utc,
        );
        let properties = crate::DocProperties::new()
            .set_creation_datetime(&date)
            .set_custom_property("Text", "A & B <C>")
            .set_custom_property("Checked by", true)
            .set_custom_property("Document number", 55_301_i32)
            .set_custom_property("Ratio", 1.5_f64)
            .set_custom_property("Review date", &date)
            .set_custom_property("A\u{1}B", "control")
            .set_custom_property("Quote\"\nLine", "quote")
            .set_custom_property("UTC review", &utc);
        let mut workbook = crate::Workbook::new();
        workbook.set_properties(&properties);
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(names.contains(&"docProps/custom.xml".to_owned()));

        let custom = String::from_utf8(part_bytes(&bytes, "docProps/custom.xml").expect("custom"))
            .expect("custom UTF-8");
        assert!(custom.contains(
            r#"<property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2" name="Text"><vt:lpwstr>A &amp; B &lt;C&gt;</vt:lpwstr></property>"#
        ));
        assert!(custom.contains(r#"pid="3" name="Checked by"><vt:bool>true</vt:bool>"#));
        assert!(custom.contains(r#"pid="4" name="Document number"><vt:i4>55301</vt:i4>"#));
        assert!(custom.contains(r#"pid="5" name="Ratio"><vt:r8>1.5</vt:r8>"#));
        assert!(custom.contains(
            r#"pid="6" name="Review date"><vt:filetime>2026-11-18T00:00:00Z</vt:filetime>"#
        ));
        assert!(custom.contains(r#"pid="7" name="A_x0001_B"><vt:lpwstr>control</vt:lpwstr>"#));
        assert!(
            custom.contains(r#"pid="8" name="Quote&quot;&#xA;Line"><vt:lpwstr>quote</vt:lpwstr>"#)
        );
        assert!(custom.contains(
            r#"pid="9" name="UTC review"><vt:filetime>2026-11-18T12:34:56Z</vt:filetime>"#
        ));

        let core = String::from_utf8(part_bytes(&bytes, "docProps/core.xml").expect("core"))
            .expect("core UTF-8");
        assert!(core.contains(
            r#"<dcterms:created xsi:type="dcterms:W3CDTF">2026-11-18T00:00:00Z</dcterms:created>"#
        ));

        let types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("content types"))
                .expect("types UTF-8");
        assert!(types.contains(
            r#"<Override PartName="/docProps/custom.xml" ContentType="application/vnd.openxmlformats-officedocument.custom-properties+xml"/>"#
        ));

        let rels = String::from_utf8(part_bytes(&bytes, "_rels/.rels").expect("rels"))
            .expect("rels UTF-8");
        assert!(rels.contains(
            r#"<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties" Target="docProps/custom.xml"/>"#
        ));
    }

    #[test]
    fn custom_properties_write_non_finite_real_values() {
        let properties = crate::DocProperties::new()
            .set_custom_property("Not a number", f64::NAN)
            .set_custom_property("Positive infinity", f64::INFINITY)
            .set_custom_property("Negative infinity", f64::NEG_INFINITY);
        let mut workbook = crate::Workbook::new();
        workbook.set_properties(&properties);
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let custom = String::from_utf8(part_bytes(&bytes, "docProps/custom.xml").expect("custom"))
            .expect("custom UTF-8");
        assert!(custom.contains(r#"pid="2" name="Not a number"><vt:r8>NaN</vt:r8>"#));
        assert!(custom.contains(r#"pid="3" name="Positive infinity"><vt:r8>INF</vt:r8>"#));
        assert!(custom.contains(r#"pid="4" name="Negative infinity"><vt:r8>-INF</vt:r8>"#));
    }

    #[test]
    fn custom_properties_part_is_absent_without_custom_values() {
        let mut workbook = crate::Workbook::new();
        workbook.set_properties(&crate::DocProperties::new().set_title("Title"));
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(!names.contains(&"docProps/custom.xml".to_owned()));

        let types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("content types"))
                .expect("types UTF-8");
        assert!(!types.contains("/docProps/custom.xml"));

        let rels = String::from_utf8(part_bytes(&bytes, "_rels/.rels").expect("rels"))
            .expect("rels UTF-8");
        assert!(!rels.contains("Target=\"docProps/custom.xml\""));
    }

    #[test]
    fn document_creation_time_adds_created_and_modified() {
        let date =
            chrono::NaiveDateTime::parse_from_str("2026-11-18 12:34:56", "%Y-%m-%d %H:%M:%S")
                .expect("datetime");
        let mut workbook = crate::Workbook::new();
        workbook.set_properties(&crate::DocProperties::new().set_creation_datetime(&date));
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let core = String::from_utf8(part_bytes(&bytes, "docProps/core.xml").expect("core"))
            .expect("core UTF-8");
        assert!(core.contains(
            r#"<dcterms:created xsi:type="dcterms:W3CDTF">2026-11-18T12:34:56Z</dcterms:created>"#
        ));
        assert!(core.contains(
            r#"<dcterms:modified xsi:type="dcterms:W3CDTF">2026-11-18T12:34:56Z</dcterms:modified>"#
        ));
    }

    #[test]
    fn document_properties_escape_xml_control_characters() {
        let mut workbook = crate::Workbook::new();
        workbook.set_properties(&crate::DocProperties::new().set_title("a\u{1}b\r\nc"));
        workbook.add_worksheet().set_name("A\u{1}B").expect("name");

        let bytes = workbook.save_to_buffer().expect("saves");
        let core = String::from_utf8(part_bytes(&bytes, "docProps/core.xml").expect("core"))
            .expect("core UTF-8");
        assert!(core.contains("<dc:title>a_x0001_b_x000D_\nc</dc:title>"));
        assert!(!core.contains('\u{1}'));
        assert!(!core.contains('\r'));

        let app = String::from_utf8(part_bytes(&bytes, "docProps/app.xml").expect("app"))
            .expect("app UTF-8");
        assert!(app.contains("<vt:lpstr>A_x0001_B</vt:lpstr>"));
        assert!(!app.contains('\u{1}'));
    }

    #[test]
    fn document_properties_are_absent_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(!names.contains(&"docProps/core.xml".to_owned()));
        assert!(!names.contains(&"docProps/app.xml".to_owned()));
    }

    #[test]
    fn package_omits_shared_strings_when_unused() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, 42).expect("writes");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");

        assert!(!names.contains(&"xl/sharedStrings.bin".to_owned()));
    }

    #[test]
    fn default_package_omits_book_views() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        assert_eq!(records::<BrtBookView>(&part).len(), 0);
    }

    #[test]
    fn workbook_behavior_records_are_absent_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        assert_eq!(records::<BrtFileSharing>(&part).len(), 0);
        assert_eq!(records::<BrtCalcProp>(&part).len(), 0);
        assert_eq!(records::<BrtBookView>(&part).len(), 0);
    }

    #[test]
    fn read_only_recommended_emits_file_sharing_record() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.read_only_recommended();

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let sharing = records::<BrtFileSharing>(&part);

        assert_eq!(sharing.len(), 1);
        assert_eq!(sharing[0].fReadOnlyRec, 1);
        assert_eq!(sharing[0].wResPass, 0);
        assert_eq!(sharing[0].stUserName.to_optional_string(), None);
    }

    #[test]
    fn formulas_emit_result_records_and_force_recalculation() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .write_formula(0, 0, "=1+2")
            .expect("formula")
            .set_formula_result(0, 0, "3");
        worksheet
            .write_formula(1, 0, crate::Formula::new("=A1+1").set_result("TRUE"))
            .expect("bool formula");
        worksheet
            .write_formula(2, 0, crate::Formula::new("=A1+1").set_result("#DIV/0!"))
            .expect("error formula");
        worksheet
            .write_formula(3, 0, crate::Formula::new("=A1&\"x\"").set_result("done"))
            .expect("string formula");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let numbers = records::<BrtFmlaNum>(&sheet);
        assert_eq!(numbers.len(), 1);
        assert_eq!(*numbers[0].xnum, 3.0);
        assert_eq!(&numbers[0].formula.rgce, &[0x1E, 1, 0, 0x1E, 2, 0, 0x03]);

        let booleans = records::<BrtFmlaBool>(&sheet);
        assert_eq!(booleans.len(), 1);
        assert!(booleans[0].bBool);

        let errors = records::<BrtFmlaError>(&sheet);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].bError, rust_xlsb_core::prelude::BErr::Div0);

        let strings = records::<BrtFmlaString>(&sheet);
        assert_eq!(strings.len(), 1);
        assert_eq!(
            strings[0].value.to_optional_string().as_deref(),
            Some("done")
        );

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let calc = records::<BrtCalcProp>(&workbook_part);
        assert_eq!(calc.len(), 1);
        assert_eq!(calc[0].fAutoRecalc, 1);
        assert!(calc[0].fFullCalcOnLoad);
        assert!(calc[0].fSaveRecalc);
    }

    #[test]
    fn array_formulas_emit_an_expansion_formula_and_range_record() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write(0, 1, 500).expect("number");
        worksheet.write(0, 2, 300).expect("number");
        worksheet.write(1, 1, 10).expect("number");
        worksheet.write(1, 2, 15).expect("number");
        worksheet
            .write_array_formula(
                0,
                0,
                1,
                0,
                crate::Formula::new("{=SUM(B1:C1*B2:C2)}").set_result("9500"),
            )
            .expect("array formula");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let arrays = records::<BrtArrFmla>(&sheet);
        assert_eq!(arrays.len(), 1);
        assert_eq!(
            arrays[0].rfx,
            rust_xlsb_core::prelude::UncheckedRfX {
                rwFirst: 0,
                rwLast: 1,
                colFirst: 0,
                colLast: 0,
            }
        );
        assert!(arrays[0].fAlwaysCalc);
        assert_eq!(records::<BrtCellMeta>(&sheet).len(), 0);
        assert_eq!(arrays[0].formula.cce, 31);

        let numbers = records::<BrtFmlaNum>(&sheet);
        assert_eq!(numbers.len(), 1);
        assert_eq!(*numbers[0].xnum, 9500.0);
        assert_eq!(numbers[0].formula.rgce, [0x01, 0, 0, 0, 0]);
        assert_eq!(numbers[0].formula.rgcb, [0, 0, 0, 0]);

        let mut reader = BufReader::new(sheet.as_slice());
        let mut previous = None;
        let mut followed_by_array = false;
        while let Ok((head, _)) = BiffHead::read_from_with_data(&mut reader) {
            if head.known_id() == Some(KnownID::BrtArrFmla) && previous == Some(KnownID::BrtFmlaNum)
            {
                followed_by_array = true;
            }
            previous = head.known_id();
        }
        assert!(followed_by_array);

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        assert_eq!(records::<BrtCalcProp>(&workbook_part).len(), 1);
        assert!(part_bytes(&bytes, "xl/metadata.bin").is_err());
    }

    #[test]
    fn dynamic_array_formula_emits_metadata_and_cell_meta() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_string(0, 0, "Foo").expect("string");
        worksheet.write_string(1, 0, "Food").expect("string");
        worksheet.write_string(2, 0, "Frood").expect("string");
        worksheet
            .write_dynamic_array_formula(0, 1, 2, 1, "=LEN(A1:A3)")
            .expect("dynamic formula");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let arrays = records::<BrtArrFmla>(&sheet);
        assert_eq!(arrays.len(), 1);
        assert!(!arrays[0].fAlwaysCalc);
        assert_eq!(
            arrays[0].rfx,
            rust_xlsb_core::prelude::UncheckedRfX {
                rwFirst: 0,
                rwLast: 2,
                colFirst: 1,
                colLast: 1,
            }
        );
        assert_eq!(arrays[0].formula.cce, 16);

        let cells = records::<BrtCellMeta>(&sheet);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].icmb, 1);

        let ids = record_ids(&sheet);
        let meta = ids
            .iter()
            .position(|id| *id == KnownID::BrtCellMeta)
            .expect("cell meta");
        assert_eq!(ids.get(meta + 1), Some(&KnownID::BrtFmlaNum));
        assert_eq!(ids.get(meta + 2), Some(&KnownID::BrtArrFmla));

        let metadata = part_bytes(&bytes, "xl/metadata.bin").expect("metadata part");
        assert_eq!(metadata.len(), 117);
        assert_eq!(
            record_ids(&metadata),
            [
                KnownID::BrtBeginMetadata,
                KnownID::BrtBeginEsmdtinfo,
                KnownID::BrtMdtinfo,
                KnownID::BrtEndEsmdtinfo,
                KnownID::BrtBeginEsfmd,
                KnownID::BrtBeginFmd,
                KnownID::BrtFRTBegin,
                KnownID::BrtBeginDynamicArrayPr,
                KnownID::BrtEndDynamicArrayPr,
                KnownID::BrtFRTEnd,
                KnownID::BrtEndFmd,
                KnownID::BrtEndEsfmd,
                KnownID::BrtBeginEsmdb,
                KnownID::BrtMdb,
                KnownID::BrtEndEsmdb,
                KnownID::BrtEndMetadata,
            ]
        );
        let types = records::<BrtMdtinfo>(&metadata);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].grbit, 0x506a_c0b0);
        assert_eq!(types[0].metadataID, 0x0001_d4c0);
        assert_eq!(
            types[0].stName.to_optional_string().as_deref(),
            Some("XLDAPR")
        );
        let blocks = records::<BrtMdb>(&metadata);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rgMdir.len(), 1);
        assert_eq!(blocks[0].rgMdir[0].iMdt, 1);

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(content_types.contains(
            r#"<Override PartName="/xl/metadata.bin" ContentType="application/vnd.ms-excel.sheetMetadata"/>"#
        ));
        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("utf8");
        assert!(rels.contains(
            r#"Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sheetMetadata" Target="metadata.bin""#
        ));
    }

    #[test]
    fn dynamic_formula_single_cell_uses_one_cell_range() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet
            .write_dynamic_formula(1, 2, "=LEN(A1:A3)")
            .expect("dynamic formula");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let arrays = records::<BrtArrFmla>(&sheet);
        assert_eq!(arrays.len(), 1);
        assert_eq!(
            arrays[0].rfx,
            rust_xlsb_core::prelude::UncheckedRfX {
                rwFirst: 1,
                rwLast: 1,
                colFirst: 2,
                colLast: 2,
            }
        );
        assert_eq!(records::<BrtCellMeta>(&sheet).len(), 1);
    }

    #[test]
    fn formula_result_oversize_string_is_rejected_on_save() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_formula(0, 0, "=1+1").expect("formula");
        worksheet.set_formula_result(0, 0, "x".repeat(32_768));

        assert!(matches!(
            workbook.save_to_buffer(),
            Err(crate::Error::StringTooLong)
        ));
    }

    #[test]
    fn formula_result_only_updates_formula_cells() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_number(0, 0, 1).expect("number");
        worksheet.set_formula_result(0, 0, "2");

        assert_eq!(
            worksheet
                .cells()
                .get(&0)
                .and_then(|row| row.get(&0))
                .map(|cell| &cell.data),
            Some(&CellData::Number(1.0))
        );
    }

    #[test]
    fn formatted_formula_uses_the_registered_style() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        let bold = crate::Format::new().set_bold();
        worksheet
            .write_formula_with_format(0, 0, "=1+2", &bold)
            .expect("formatted formula");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let numbers = records::<BrtFmlaNum>(&sheet);

        assert_eq!(numbers.len(), 1);
        assert_ne!(numbers[0].cell.iStyleRef, 0);
    }

    #[test]
    fn formula_defaults_control_the_cached_result_type() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_formula(0, 0, "=1+2").expect("default");
        worksheet.set_formula_result_default("");
        worksheet.write_formula(1, 0, "=1+2").expect("empty");

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtFmlaNum>(&sheet).len(), 1);
        let strings = records::<BrtFmlaString>(&sheet);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].value.to_optional_string().as_deref(), Some(""));
    }

    #[test]
    fn explicit_calculation_mode_wins_over_formula_defaults() {
        let mut workbook = crate::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.write_formula(0, 0, "=1+2").expect("formula");
        workbook.set_calculation_mode(crate::CalculationMode::Manual);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let calc = records::<BrtCalcProp>(&part);

        assert_eq!(calc.len(), 1);
        assert_eq!(calc[0].fAutoRecalc, 0);
        assert!(!calc[0].fFullCalcOnLoad);
    }

    #[test]
    fn calculation_mode_emits_calc_prop_values() {
        for (mode, auto_recalc, recalc) in [
            (crate::CalculationMode::Automatic, 1, true),
            (crate::CalculationMode::Manual, 0, false),
            (crate::CalculationMode::Partial, 2, true),
        ] {
            let mut workbook = crate::Workbook::new();
            workbook.add_worksheet();
            workbook.set_calculation_mode(mode);

            let bytes = workbook.save_to_buffer().expect("saves");
            let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
            let props = records::<BrtCalcProp>(&part);

            assert_eq!(props.len(), 1);
            assert_eq!(props[0].recalcID, 124_519);
            assert_eq!(props[0].fAutoRecalc, auto_recalc);
            assert_eq!(props[0].cCalcCount, 100);
            assert_eq!(*props[0].xnumDelta, 0.001);
            assert_eq!(props[0].cUserThreadCount, 1);
            assert_eq!(props[0].fFullCalcOnLoad, recalc);
            assert!(props[0].fRefA1);
            assert!(props[0].fFullPrec);
            assert_eq!(props[0].fSaveRecalc, recalc);
        }
    }

    #[test]
    fn tab_ratio_forces_book_view() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.set_tab_ratio(25);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let views = records::<BrtBookView>(&part);

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].iTabRatio, 250);
        assert_eq!((views[0].itabFirst, views[0].itabCur), (0, 0));
    }

    #[test]
    fn out_of_range_tab_ratio_leaves_default_output() {
        let mut expected = crate::Workbook::new();
        expected.add_worksheet();
        let expected = expected.save_to_buffer().expect("saves");

        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.set_tab_ratio(101);
        let bytes = workbook.save_to_buffer().expect("saves");

        assert_eq!(bytes, expected);
    }

    #[test]
    fn workbook_behavior_records_follow_spec_order() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .set_print_area(0, 0, 1, 1)
            .expect("area");
        workbook
            .read_only_recommended()
            .set_calculation_mode(crate::CalculationMode::Manual)
            .set_tab_ratio(40);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        let mut reader = BufReader::new(part.as_slice());
        let mut ids = Vec::new();
        while let Ok((head, _body)) = BiffHead::read_from_with_data(&mut reader) {
            if let Some(id) = head.known_id() {
                ids.push(id);
            }
        }

        let position = |id| ids.iter().position(|candidate| *candidate == id);
        let begin = position(rust_xlsb_core::KnownID::BrtBeginBook).expect("begin");
        let sharing = position(rust_xlsb_core::KnownID::BrtFileSharing).expect("sharing");
        let end = position(rust_xlsb_core::KnownID::BrtEndBook).expect("end");
        let calc = position(rust_xlsb_core::KnownID::BrtCalcProp).expect("calc");
        let name = position(rust_xlsb_core::KnownID::BrtName).expect("name");
        let externals = position(rust_xlsb_core::KnownID::BrtExternSheet).expect("externals");

        assert_eq!(sharing, begin + 1);
        assert!(externals < name);
        assert!(name < calc);
        assert_eq!(calc, end - 1);
    }

    #[test]
    fn tab_state_sets_sheet_visibility_and_book_view() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().set_hidden(true);
        workbook.add_worksheet().set_active(true);
        workbook.add_worksheet().set_first_tab(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        let sheets = records::<BrtBundleSh>(&part);
        assert_eq!(
            sheets.iter().map(|sheet| sheet.hsState).collect::<Vec<_>>(),
            [1, 0, 0]
        );

        let views = records::<BrtBookView>(&part);
        assert_eq!(views.len(), 1);
        assert_eq!((views[0].itabFirst, views[0].itabCur), (2, 1));
        assert!(views[0].fDspHScroll);
        assert!(views[0].fBotAdornment);

        let active = part_bytes(&bytes, "xl/worksheets/sheet2.bin").expect("sheet part");
        assert!(records::<BrtBeginWsView>(&active)[0].fSelected);
    }

    #[test]
    fn selected_sheet_emits_view_without_book_view() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet().set_selected(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");
        assert_eq!(records::<BrtBookView>(&part).len(), 0);

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet2.bin").expect("sheet part");
        assert!(records::<BrtBeginWsView>(&sheet)[0].fSelected);
    }

    #[test]
    fn hidden_first_sheet_makes_next_visible_sheet_active() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().set_hidden(true);
        workbook.add_worksheet();

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        let views = records::<BrtBookView>(&part);
        assert_eq!(views.len(), 1);
        assert_eq!((views[0].itabFirst, views[0].itabCur), (1, 1));

        let sheet = part_bytes(&bytes, "xl/worksheets/sheet2.bin").expect("sheet part");
        assert!(records::<BrtBeginWsView>(&sheet)[0].fSelected);
    }

    #[test]
    fn all_hidden_sheets_keep_one_visible() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().set_hidden(true);
        workbook.add_worksheet().set_very_hidden(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        assert_eq!(
            records::<BrtBundleSh>(&part)
                .iter()
                .map(|sheet| sheet.hsState)
                .collect::<Vec<_>>(),
            [0, 2]
        );
    }

    #[test]
    fn very_hidden_sheet_serializes_state_two() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet().set_very_hidden(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        let part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook part");

        assert_eq!(
            records::<BrtBundleSh>(&part)
                .iter()
                .map(|sheet| sheet.hsState)
                .collect::<Vec<_>>(),
            [0, 2]
        );
    }

    #[test]
    fn zip_large_file_writes_zip64_headers() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "large file")
            .expect("cell");
        workbook.use_zip_large_file(true);

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(read_u16(&bytes, 4).expect("version"), 45);
        assert_eq!(read_u16(&bytes, 8).expect("method"), 0);
        assert_eq!(read_u32(&bytes, 18).expect("compressed"), u32::MAX);
        assert_eq!(read_u32(&bytes, 22).expect("uncompressed"), u32::MAX);
        let name_len = usize::from(read_u16(&bytes, 26).expect("name length"));
        assert_eq!(read_u16(&bytes, 28).expect("extra length"), 20);
        let extra = 30 + name_len;
        assert_eq!(read_u16(&bytes, extra).expect("ZIP64 tag"), 1);
        assert_eq!(read_u16(&bytes, extra + 2).expect("ZIP64 size"), 16);
        assert_eq!(
            read_u64(&bytes, extra + 4).expect("uncompressed size"),
            read_u64(&bytes, extra + 12).expect("compressed size")
        );

        let central_offset =
            usize::try_from(read_u32(&bytes, bytes.len() - 6).expect("central offset"))
                .expect("offset fits");
        assert_eq!(
            read_u32(&bytes, central_offset).expect("central"),
            0x0201_4b50
        );
        assert_eq!(read_u16(&bytes, central_offset + 6).expect("version"), 45);
        assert_eq!(
            read_u32(&bytes, central_offset + 20).expect("compressed"),
            u32::MAX
        );
        assert_eq!(
            read_u32(&bytes, central_offset + 24).expect("uncompressed"),
            u32::MAX
        );
        let central_name = usize::from(read_u16(&bytes, central_offset + 28).expect("name"));
        assert_eq!(read_u16(&bytes, central_offset + 30).expect("extra"), 20);
        let central_extra = central_offset + 46 + central_name;
        assert_eq!(read_u16(&bytes, central_extra).expect("tag"), 1);
        assert_eq!(read_u16(&bytes, central_extra + 2).expect("size"), 16);
    }

    #[test]
    fn disabling_zip64_restores_default_output() {
        let mut expected = crate::Workbook::new();
        expected.add_worksheet();
        let expected = expected.save_to_buffer().expect("saves");

        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.use_zip_large_file(true).use_zip_large_file(false);
        let bytes = workbook.save_to_buffer().expect("saves");

        assert_eq!(bytes, expected);
    }

    #[test]
    fn zip32_fields_reject_zip64_sentinels() {
        let error = zip_u32(ZIP64_LIMIT, ZIP_ENTRY_ZIP64).expect_err("sentinel rejected");
        assert!(matches!(
            error,
            Error::ValueNotRepresentable(ZIP_ENTRY_ZIP64)
        ));
        assert_eq!(
            zip_u32(ZIP64_LIMIT - 1, ZIP_ENTRY_ZIP64).expect("largest non-sentinel"),
            u32::MAX - 1
        );
    }

    #[test]
    fn zip64_end_records_cover_threshold_values() {
        let mut bytes = Vec::new();
        write_zip_end(&mut bytes, u64::from(u16::MAX), ZIP64_LIMIT, ZIP64_LIMIT);

        assert_eq!(read_u32(&bytes, 0).expect("signature"), 0x0606_4b50);
        assert_eq!(read_u64(&bytes, 40).expect("central size"), ZIP64_LIMIT);
        assert_eq!(read_u64(&bytes, 48).expect("central offset"), ZIP64_LIMIT);
        assert_eq!(read_u16(&bytes, 84).expect("entry count"), u16::MAX);
        assert_eq!(
            read_u32(&bytes, 88).expect("central size sentinel"),
            u32::MAX
        );
        assert_eq!(
            read_u32(&bytes, 92).expect("central offset sentinel"),
            u32::MAX
        );
    }

    #[test]
    fn zip64_end_records_cover_oversized_directory() {
        let mut bytes = Vec::new();
        write_zip_end(&mut bytes, u64::from(u16::MAX) + 1, 0, 0);

        assert_eq!(read_u32(&bytes, 0).expect("signature"), 0x0606_4b50);
        assert_eq!(read_u64(&bytes, 4).expect("record size"), 44);
        assert_eq!(read_u64(&bytes, 24).expect("entry count"), 65_536);
        assert_eq!(read_u64(&bytes, 32).expect("total count"), 65_536);
        assert_eq!(read_u32(&bytes, 56).expect("locator"), 0x0706_4b50);
        assert_eq!(read_u64(&bytes, 64).expect("zip64 offset"), 0);
        assert_eq!(read_u32(&bytes, 76).expect("eocd"), 0x0605_4b50);
        assert_eq!(read_u16(&bytes, 84).expect("entry count"), u16::MAX);
    }

    #[test]
    fn zip_compression_level_uses_deflate_entries() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "compress me")
            .expect("cell");
        let stored = workbook.save_to_buffer().expect("stored");
        workbook.set_zip_compression_level(6);

        let compressed = workbook.save_to_buffer().expect("saves");
        assert_eq!(read_u16(&compressed, 8).expect("method"), 8);
        let names = part_names(&stored).expect("stored parts");
        assert_eq!(names, part_names(&compressed).expect("compressed parts"));
        for name in names {
            assert_eq!(
                part_bytes(&stored, &name).expect("stored part"),
                part_bytes(&compressed, &name).expect("compressed part")
            );
        }
    }

    #[test]
    fn zip_compression_is_stored_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "store me")
            .expect("cell");

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(read_u16(&bytes, 8).expect("method"), 0);
    }

    #[test]
    fn invalid_zip_compression_level_is_ignored_by_default() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "store me")
            .expect("cell");
        workbook.set_zip_compression_level(0);
        workbook.set_zip_compression_level(10);

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(read_u16(&bytes, 8).expect("method"), 0);
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut png = [0u8; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        png.to_vec()
    }

    #[test]
    fn image_insertion_rejects_bad_dimensions_and_cells() {
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        let bad = crate::Image::new_from_buffer(b"\x89PNG\r\n\x1a\nshort").expect("png");
        assert!(matches!(
            sheet.insert_image(0, 0, &bad),
            Err(Error::ImageError(_))
        ));

        let image = crate::Image::new_from_buffer(&png(1, 1)).expect("png");
        assert!(matches!(
            sheet.insert_image(MAX_ROW + 1, 0, &image),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_image(0, MAX_COL + 1, &image),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_image_fit_to_cell(MAX_ROW + 1, 0, &image, true),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_image_fit_to_cell_centered(0, MAX_COL + 1, &image),
            Err(Error::RowColumnLimit)
        ));
        assert!(matches!(
            sheet.insert_image_fit_to_cell(0, 0, &bad, true),
            Err(Error::ImageError(_))
        ));
    }

    #[test]
    fn each_image_sheet_gets_its_own_drawing_part() {
        let image = crate::Image::new_from_buffer(&png(1, 1)).expect("png");
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .insert_image(0, 0, &image)
            .expect("first sheet");
        workbook
            .add_worksheet()
            .insert_image(0, 0, &image)
            .expect("second sheet");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(names.contains(&"xl/drawings/drawing1.xml".to_owned()));
        assert!(names.contains(&"xl/drawings/drawing2.xml".to_owned()));
        assert!(names.contains(&"xl/media/image1.png".to_owned()));
        assert!(names.contains(&"xl/media/image2.png".to_owned()));
        for sheet in 1..=2 {
            let rels = String::from_utf8(
                part_bytes(
                    &bytes,
                    &format!("xl/worksheets/_rels/sheet{sheet}.bin.rels"),
                )
                .expect("rels"),
            )
            .expect("utf8");
            assert!(rels.contains(&format!(r#"Target="../drawings/drawing{sheet}.xml""#)));
        }
    }

    #[test]
    fn no_images_emit_no_drawing_parts() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet().write(0, 0, "plain").expect("cell");
        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(!names.iter().any(|name| name.contains("drawings/")));
        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(!content_types.contains("drawing+xml"));
    }

    #[test]
    fn foreground_images_emit_drawing_media_and_relationships() {
        let first = crate::Image::new_from_buffer(&png(3, 2)).expect("png");
        let second = crate::Image::new_from_buffer(&png(4, 5)).expect("png");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet
            .insert_image_with_offset(2, 3, &first, 10, 5)
            .expect("first image");
        sheet.insert_image(4, 5, &second).expect("second image");

        let bytes = workbook.save_to_buffer().expect("saves");
        let names = part_names(&bytes).expect("parts");
        assert!(names.contains(&"xl/drawings/drawing1.xml".to_owned()));
        assert!(names.contains(&"xl/drawings/_rels/drawing1.xml.rels".to_owned()));
        assert_eq!(
            part_bytes(&bytes, "xl/media/image1.png").expect("first media"),
            png(3, 2)
        );
        assert_eq!(
            part_bytes(&bytes, "xl/media/image2.png").expect("second media"),
            png(4, 5)
        );

        let drawing =
            String::from_utf8(part_bytes(&bytes, "xl/drawings/drawing1.xml").expect("drawing"))
                .expect("utf8");
        assert!(drawing.contains(
            "<xdr:from><xdr:col>3</xdr:col><xdr:colOff>95250</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>47625</xdr:rowOff></xdr:from>"
        ));
        assert!(drawing.contains(
            "<xdr:to><xdr:col>3</xdr:col><xdr:colOff>123825</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>66675</xdr:rowOff></xdr:to>"
        ));
        assert!(drawing.contains(r#"<xdr:cNvPr id="1" name="Picture 1"/>"#));
        assert!(drawing.contains(r#"<xdr:cNvPr id="2" name="Picture 2"/>"#));
        assert!(drawing.contains(r#"<a:blip r:embed="rId1"/>"#));
        assert!(drawing.contains(r#"<a:blip r:embed="rId2"/>"#));
        assert_eq!(drawing.matches("<xdr:twoCellAnchor").count(), 2);

        let drawing_rels = String::from_utf8(
            part_bytes(&bytes, "xl/drawings/_rels/drawing1.xml.rels").expect("drawing rels"),
        )
        .expect("utf8");
        assert!(drawing_rels.contains(r#"Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png""#));
        assert!(drawing_rels.contains(r#"Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image2.png""#));

        let sheet_bytes = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("worksheet");
        let drawings = records::<BrtDrawing>(&sheet_bytes);
        assert_eq!(drawings.len(), 1);
        assert_eq!(
            drawings[0].rgb.to_optional_string().as_deref(),
            Some("rId1")
        );
        let ids = record_ids(&sheet_bytes);
        assert!(ids.contains(&KnownID::BrtDrawing));

        let rels = String::from_utf8(
            part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels"),
        )
        .expect("utf8");
        assert_eq!(
            rels,
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/>"#,
                r#"</Relationships>"#,
            )
        );

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(content_types.contains(r#"<Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#));
    }

    #[test]
    fn foreground_images_scale_and_fit_to_cell() {
        let image = crate::Image::new_from_buffer(&png(100, 50)).expect("png");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_column_width_pixels(0, 100).expect("column");
        sheet.set_row_height_pixels(2, 40).expect("row");
        sheet.set_row_height_pixels(4, 40).expect("row");
        sheet
            .insert_image(
                0,
                0,
                &image.clone().set_scale_width(0.5).set_scale_height(0.5),
            )
            .expect("scaled image");
        sheet
            .insert_image_fit_to_cell(2, 0, &image, true)
            .expect("fit image");
        sheet
            .insert_image_fit_to_cell_centered(4, 0, &image)
            .expect("centered image");

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(
            part_bytes(&bytes, "xl/media/image1.png").expect("media"),
            png(100, 50)
        );
        let drawing =
            String::from_utf8(part_bytes(&bytes, "xl/drawings/drawing1.xml").expect("drawing"))
                .expect("utf8");
        assert!(
            drawing.contains(r#"<a:ext cx="476250" cy="238125"/>"#),
            "{drawing}"
        );
        assert_eq!(
            drawing
                .matches(r#"<a:ext cx="762000" cy="381000"/>"#)
                .count(),
            2
        );
        assert!(drawing.contains(
            "<xdr:from><xdr:col>0</xdr:col><xdr:colOff>95250</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"
        ));
        assert!(drawing.contains(
            "<xdr:to><xdr:col>0</xdr:col><xdr:colOff>857250</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>381000</xdr:rowOff></xdr:to>"
        ));
    }

    #[test]
    fn vba_project_and_code_names_are_packaged() {
        let path =
            std::env::temp_dir().join(format!("rust_xlsb_vba_project_{}.bin", std::process::id()));
        std::fs::write(&path, b"test vba project").expect("project");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_vba_name("DataSheet").expect("sheet name");
        workbook.set_vba_name("ProjectBook").expect("book name");
        workbook.add_vba_project(&path).expect("vba project");
        let bytes = workbook.save_to_buffer().expect("saves");
        std::fs::remove_file(path).expect("cleanup");

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(content_types.contains(
            r#"<Override PartName="/xl/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/>"#
        ));
        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("utf8");
        assert!(rels.contains(
            r#"Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject" Target="vbaProject.bin""#
        ));
        assert_eq!(
            part_bytes(&bytes, "xl/vbaProject.bin").expect("project"),
            b"test vba project"
        );

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let props = records::<BrtWbProp>(&workbook_part);
        assert_eq!(props.len(), 1);
        assert_eq!(
            props[0].strName.to_optional_string().as_deref(),
            Some("ProjectBook")
        );

        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        let props = records::<BrtWsProp>(&sheet_part);
        assert_eq!(props.len(), 1);
        assert_eq!(
            props[0].strName.to_optional_string().as_deref(),
            Some("DataSheet")
        );
    }

    #[test]
    fn vba_project_defaults_workbook_and_worksheet_code_names() {
        let path =
            std::env::temp_dir().join(format!("rust_xlsb_vba_default_{}.bin", std::process::id()));
        std::fs::write(&path, b"project").expect("project");
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        workbook.add_worksheet();
        workbook
            .worksheet_from_index(0)
            .expect("sheet")
            .set_name("Data")
            .expect("name");
        workbook.add_vba_project(&path).expect("vba project");
        let bytes = workbook.save_to_buffer().expect("saves");
        std::fs::remove_file(path).expect("cleanup");

        let workbook_part = part_bytes(&bytes, "xl/workbook.bin").expect("workbook");
        let props = records::<BrtWbProp>(&workbook_part);
        assert_eq!(
            props[0].strName.to_optional_string().as_deref(),
            Some("ThisWorkbook")
        );
        for index in 1..=2 {
            let sheet_part =
                part_bytes(&bytes, &format!("xl/worksheets/sheet{index}.bin")).expect("sheet");
            let props = records::<BrtWsProp>(&sheet_part);
            assert_eq!(props.len(), 1);
            assert_eq!(
                props[0].strName.to_optional_string().as_deref(),
                Some(format!("Sheet{index}").as_str())
            );
        }
    }

    #[test]
    fn invalid_vba_names_are_rejected() {
        let mut workbook = crate::Workbook::new();
        let mut sheet = crate::Worksheet::new();
        for name in ["", "9Book", "Has Space", "Has-Dash"] {
            assert!(matches!(
                workbook.set_vba_name(name),
                Err(crate::Error::VbaNameError(_))
            ));
            assert!(matches!(
                sheet.set_vba_name(name),
                Err(crate::Error::VbaNameError(_))
            ));
        }
        assert!(matches!(
            workbook.set_vba_name("A".repeat(32)),
            Err(crate::Error::VbaNameError(_))
        ));
    }

    #[test]
    fn workbook_without_vba_has_no_project_part() {
        let mut workbook = crate::Workbook::new();
        workbook.add_worksheet();
        let bytes = workbook.save_to_buffer().expect("saves");
        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(!content_types.contains("vbaProject"));
        assert!(part_bytes(&bytes, "xl/vbaProject.bin").is_err());
        let rels =
            String::from_utf8(part_bytes(&bytes, "xl/_rels/workbook.bin.rels").expect("rels"))
                .expect("utf8");
        assert!(!rels.contains("vbaProject"));
    }

    #[test]
    fn fit_to_cell_leaves_a_hidden_target_unscaled() {
        let image = crate::Image::new_from_buffer(&png(10, 10)).expect("png");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.set_column_hidden(0).expect("column");
        sheet
            .insert_image_fit_to_cell(0, 0, &image, true)
            .expect("image");

        let bytes = workbook.save_to_buffer().expect("saves");
        let drawing =
            String::from_utf8(part_bytes(&bytes, "xl/drawings/drawing1.xml").expect("drawing"))
                .expect("utf8");
        assert!(drawing.contains(r#"<a:ext cx="95250" cy="95250"/>"#));
        assert!(drawing.contains(
            "<xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"
        ));
    }

    #[test]
    fn foreground_and_background_images_keep_distinct_relationships() {
        let foreground = crate::Image::new_from_buffer(&png(3, 2)).expect("png");
        let background = crate::Image::new_from_buffer(&png(1, 1)).expect("background");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.insert_image(0, 0, &foreground).expect("image");
        sheet.insert_background_image(&background);

        let bytes = workbook.save_to_buffer().expect("saves");
        let sheet_bytes = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("worksheet");
        let drawings = records::<BrtDrawing>(&sheet_bytes);
        let backgrounds = records::<BrtBkHim>(&sheet_bytes);
        assert_eq!(drawings.len(), 1);
        assert_eq!(
            drawings[0].rgb.to_optional_string().as_deref(),
            Some("rId1")
        );
        assert_eq!(backgrounds.len(), 1);
        assert_eq!(
            backgrounds[0].rgb.to_optional_string().as_deref(),
            Some("rId2")
        );
        let ids = record_ids(&sheet_bytes);
        let drawing_index = ids
            .iter()
            .position(|id| *id == KnownID::BrtDrawing)
            .expect("drawing");
        let background_index = ids
            .iter()
            .position(|id| *id == KnownID::BrtBkHim)
            .expect("background");
        assert!(drawing_index < background_index);

        let drawing_rels = String::from_utf8(
            part_bytes(&bytes, "xl/drawings/_rels/drawing1.xml.rels").expect("drawing rels"),
        )
        .expect("utf8");
        assert!(drawing_rels.contains(r#"Target="../media/image2.png""#));
        let rels = String::from_utf8(
            part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels"),
        )
        .expect("utf8");
        assert!(rels.contains(r#"Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing""#));
        assert!(rels.contains(r#"Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png""#));
    }

    #[test]
    fn background_image_emits_media_part_and_relationship() {
        const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbackground";
        const JPEG: &[u8] = b"\xFF\xD8\xFF\xE0background";
        let image = crate::Image::new_from_buffer(PNG).expect("png");
        let mut workbook = crate::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.add_table(2, 0, 4, 0, &Table::new()).expect("table");
        sheet
            .write_url(0, 0, "https://example.com")
            .expect("hyperlink");
        sheet.insert_background_image(&image);

        let jpeg = crate::Image::new_from_buffer(JPEG).expect("jpeg");
        workbook.add_worksheet().insert_background_image(&jpeg);

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(
            part_bytes(&bytes, "xl/media/image1.png").expect("media part"),
            PNG
        );
        assert_eq!(
            part_bytes(&bytes, "xl/media/image2.jpg").expect("jpeg media part"),
            JPEG
        );

        let sheet_bytes = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("worksheet part");
        let background = records::<BrtBkHim>(&sheet_bytes);
        assert_eq!(background.len(), 1);
        assert_eq!(
            background[0].rgb.to_optional_string().as_deref(),
            Some("rId3")
        );
        let ids = record_ids(&sheet_bytes);
        let background_index = ids
            .iter()
            .position(|id| *id == KnownID::BrtBkHim)
            .expect("background record");
        let list_index = ids
            .iter()
            .position(|id| *id == KnownID::BrtBeginListParts)
            .expect("list part record");
        assert!(background_index < list_index);

        let rels = String::from_utf8(
            part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").expect("rels"),
        )
        .expect("utf8");
        assert!(rels.contains(r#"Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png""#));
        assert_eq!(rels.matches(r#"Id="rId3""#).count(), 1);

        let content_types =
            String::from_utf8(part_bytes(&bytes, "[Content_Types].xml").expect("types"))
                .expect("utf8");
        assert!(content_types.contains(r#"<Default Extension="png" ContentType="image/png"/>"#));
        assert!(content_types.contains(r#"<Default Extension="jpg" ContentType="image/jpeg"/>"#));
    }

    #[test]
    fn invalid_zip_compression_level_keeps_previous_setting() {
        let mut workbook = crate::Workbook::new();
        workbook
            .add_worksheet()
            .write(0, 0, "compress me")
            .expect("cell");
        workbook.set_zip_compression_level(3);
        workbook.set_zip_compression_level(10);

        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(read_u16(&bytes, 8).expect("method"), 8);
    }
}
