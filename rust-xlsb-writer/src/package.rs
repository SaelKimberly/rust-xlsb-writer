//! XLSB package assembly: parts, relationships, and ZIP packaging.

use std::fmt::Write as _;
use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use rust_xlsb_core::prelude::{
    BrtBeginBook, BrtBeginBookViews, BrtBeginBundleShs, BrtBeginExternals, BrtBookView,
    BrtBundleSh, BrtCalcProp, BrtEndBook, BrtEndBookViews, BrtEndBundleShs, BrtEndExternals,
    BrtExternSheet, BrtFileSharing, BrtName, BrtSupSelf, NameParsedFormula, XlNullableWideString,
    XlStringExt, XlWideString, Xnum, Xti,
};
use rust_xlsb_core::{SSTHolder, StyleRegistry, push_record};

use crate::defined_name::{DefinedName, XtiCollector, area3d, sheet_index};
use crate::properties::CustomPropertyValue;
use crate::table::write_table_part;
use crate::url::{escape_xml, escape_xml_attribute, escape_xml_data};
use crate::workbook::{CalculationMode, WorkbookSettings};
use crate::worksheet::{MAX_COL, MAX_ROW, Visibility, Worksheet};
use crate::{DocProperties, Error, Format, Result};

pub fn build_package(
    sheets: &[Worksheet],
    settings: WorkbookSettings,
    properties: Option<&DocProperties>,
    default_format: Option<&Format>,
    defined_names: &[DefinedName],
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

    let mut sheet_parts = Vec::with_capacity(sheets.len());
    let mut table_parts = Vec::new();
    let mut table_ids = Vec::new();
    let view = workbook_view(sheets);
    for (index, sheet) in sheets.iter().enumerate() {
        let mut bytes = Vec::new();
        let selected = index == view.active && (view.explicit_active || view.active != 0);
        crate::sheet::write_sheet(sheet, &style_indexes[index], &sst, selected, &mut bytes)?;
        sheet_parts.push((format!("xl/worksheets/sheet{}.bin", index + 1), bytes));
        if let Some(rels) = sheet_rels(sheet) {
            sheet_parts.push((
                format!("xl/worksheets/_rels/sheet{}.bin.rels", index + 1),
                rels.into_bytes(),
            ));
        }
        for table in sheet.tables() {
            let mut bytes = Vec::new();
            write_table_part(table, &mut bytes)?;
            table_parts.push((format!("xl/tables/table{}.bin", table.table.id), bytes));
            table_ids.push(table.table.id);
        }
    }

    let has_strings = !sst.is_empty();
    let has_custom_properties =
        properties.is_some_and(|properties| !properties.custom_properties.is_empty());
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
                &table_ids,
            )
            .into_bytes(),
        ),
        (
            "_rels/.rels".to_owned(),
            root_rels(properties.is_some(), has_custom_properties).into_bytes(),
        ),
        (
            "xl/_rels/workbook.bin.rels".to_owned(),
            workbook_rels(sheets.len(), has_strings).into_bytes(),
        ),
        (
            "xl/workbook.bin".to_owned(),
            workbook_part(sheets, settings, defined_names)?,
        ),
        ("xl/styles.bin".to_owned(), styles),
    ];
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

    if has_strings {
        let mut shared_strings = Vec::new();
        sst.write_to_biff_sst(&mut shared_strings)?;
        parts.push(("xl/sharedStrings.bin".to_owned(), shared_strings));
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
    settings: WorkbookSettings,
    defined_names: &[DefinedName],
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
    write_defined_names(sheets, defined_names, &mut out)?;
    if let Some(mode) = settings.calculation_mode {
        write_calculation_properties(mode, &mut out)?;
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
    out: &mut Vec<u8>,
) -> Result<()> {
    if defined_names.is_empty()
        && !sheets.iter().any(|sheet| {
            sheet.print_area().is_some()
                || has_repeat_titles(sheet)
                || sheet.autofilter_range().is_some()
        })
    {
        return Ok(());
    }

    let mut xti = XtiCollector::new(sheets.len());
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

    let mut user_names = defined_names.iter().collect::<Vec<_>>();
    user_names.sort_by(|left, right| left.sort_name.cmp(&right.sort_name));
    for name in user_names {
        let itab = match name.scope.as_deref() {
            Some(scope) => sheet_itab(sheet_index(sheets, scope)?)?,
            None => u32::MAX,
        };
        let rgce = name.encode(sheets, &mut xti)?;
        push_defined_name(&mut names, itab, &name.name, 0, rgce)?;
    }

    if !xti.is_empty() {
        let mut xtis = Vec::with_capacity(xti.sheets().len());
        for &sheet in xti.sheets() {
            let sheet = i32::try_from(sheet)
                .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?;
            xtis.push(Xti {
                externalLink: 0,
                firstSheet: sheet,
                lastSheet: sheet,
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

fn sheet_rels(sheet: &Worksheet) -> Option<String> {
    if sheet.tables().is_empty() && sheet.hyperlinks().values().all(crate::Url::is_internal) {
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
    body.push_str("</Relationships>");
    Some(body)
}

fn content_types(
    sheet_count: usize,
    has_strings: bool,
    has_properties: bool,
    has_custom_properties: bool,
    table_ids: &[u32],
) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
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

fn workbook_rels(sheet_count: usize, has_strings: bool) -> String {
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
    if has_strings {
        let strings_id = sheet_count + 2;
        write!(
            body,
            r#"<Relationship Id="rId{strings_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.bin"/>"#
        )
        .expect("writing to String cannot fail");
    }
    body.push_str("</Relationships>");
    body
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
        Bold, BrtBeginAFilter, BrtBeginCustomFilters, BrtBeginDVals, BrtBeginFilterColumn,
        BrtBeginFilters, BrtBeginList, BrtBeginListCol, BrtBeginListParts, BrtBeginWsView,
        BrtBookView, BrtBundleSh, BrtCalcProp, BrtCustomFilter, BrtDVal, BrtExternSheet,
        BrtFileSharing, BrtFilter, BrtFont, BrtHLink, BrtListPart, BrtName, BrtRowHdr,
        BrtSheetProtection, BrtTableStyleClient, BrtWsFmtInfo, BrtXF,
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
        assert!(function.define_name("Rate", "=SUM(1,2)").is_ok());
        assert!(function.save_to_buffer().is_err());
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
        let sheet_part = part_bytes(&bytes, "xl/worksheets/sheet1.bin").expect("sheet");
        assert_eq!(records::<BrtBeginListParts>(&sheet_part).len(), 0);
        assert!(part_bytes(&bytes, "xl/worksheets/_rels/sheet1.bin.rels").is_err());
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
