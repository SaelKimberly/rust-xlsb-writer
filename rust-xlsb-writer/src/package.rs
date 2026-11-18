//! XLSB package assembly: parts, relationships, and store-only ZIP.

use std::fmt::Write as _;

use rust_xlsb_core::prelude::{
    BrtBeginBook, BrtBeginBookViews, BrtBeginBundleShs, BrtBeginExternals, BrtBookView,
    BrtBundleSh, BrtCalcProp, BrtEndBook, BrtEndBookViews, BrtEndBundleShs, BrtEndExternals,
    BrtExternSheet, BrtFileSharing, BrtName, BrtSupSelf, NameParsedFormula, XlNullableWideString,
    XlStringExt, XlWideString, Xnum, Xti,
};
use rust_xlsb_core::{SSTHolder, StyleRegistry, push_record};

use crate::url::escape_xml;
use crate::workbook::{CalculationMode, WorkbookSettings};
use crate::worksheet::{MAX_COL, MAX_ROW, Visibility, Worksheet};
use crate::{Error, Result};

pub fn build_package(sheets: &[Worksheet], settings: WorkbookSettings) -> Result<Vec<u8>> {
    let sst = SSTHolder::create_in_memory()?;
    let mut registry = StyleRegistry::new();
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
    }

    let has_strings = !sst.is_empty();
    let mut styles = Vec::new();
    registry.write_to_biff(&mut styles)?;

    let mut parts: Vec<(String, Vec<u8>)> = vec![
        (
            "[Content_Types].xml".to_owned(),
            content_types(sheets.len(), has_strings).into_bytes(),
        ),
        ("_rels/.rels".to_owned(), root_rels().into_bytes()),
        (
            "xl/_rels/workbook.bin.rels".to_owned(),
            workbook_rels(sheets.len(), has_strings).into_bytes(),
        ),
        (
            "xl/workbook.bin".to_owned(),
            workbook_part(sheets, settings)?,
        ),
        ("xl/styles.bin".to_owned(), styles),
    ];
    parts.extend(sheet_parts);

    if has_strings {
        let mut shared_strings = Vec::new();
        sst.write_to_biff_sst(&mut shared_strings)?;
        parts.push(("xl/sharedStrings.bin".to_owned(), shared_strings));
    }

    zip_store(&parts)
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

fn workbook_part(sheets: &[Worksheet], settings: WorkbookSettings) -> Result<Vec<u8>> {
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
    write_defined_names(sheets, &mut out)?;
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

fn write_defined_names(sheets: &[Worksheet], out: &mut Vec<u8>) -> Result<()> {
    let has_names = sheets
        .iter()
        .any(|sheet| sheet.print_area().is_some() || has_repeat_titles(sheet));
    if !has_names {
        return Ok(());
    }

    let mut xti_indexes = vec![None; sheets.len()];
    let mut xtis = Vec::new();
    for (index, sheet) in sheets.iter().enumerate() {
        if sheet.print_area().is_none() && !has_repeat_titles(sheet) {
            continue;
        }
        let sheet_index = i32::try_from(index)
            .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?;
        let xti_index = u16::try_from(xtis.len())
            .map_err(|_| Error::ValueNotRepresentable("too many print ranges"))?;
        xti_indexes[index] = Some(xti_index);
        xtis.push(Xti {
            externalLink: 0,
            firstSheet: sheet_index,
            lastSheet: sheet_index,
        });
    }

    push_record(out, &BrtBeginExternals::default())?;
    push_record(out, &BrtSupSelf::default())?;
    push_record(
        out,
        &BrtExternSheet {
            cXti: u32::try_from(xtis.len())
                .map_err(|_| Error::ValueNotRepresentable("too many print ranges"))?,
            rgXti: xtis,
        },
    )?;
    push_record(out, &BrtEndExternals::default())?;

    for (index, sheet) in sheets.iter().enumerate() {
        if let Some((first_row, first_col, last_row, last_col)) = sheet.print_area() {
            let ixti = xti_indexes[index].expect("print range has an Xti");
            push_defined_name(
                out,
                index,
                "_xlnm.Print_Area",
                area3d(ixti, first_row, first_col, last_row, last_col).to_vec(),
            )?;
        }
    }
    for (index, sheet) in sheets.iter().enumerate() {
        if has_repeat_titles(sheet) {
            let ixti = xti_indexes[index].expect("print range has an Xti");
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
            push_defined_name(out, index, "_xlnm.Print_Titles", rgce)?;
        }
    }
    Ok(())
}

const fn has_repeat_titles(sheet: &Worksheet) -> bool {
    sheet.repeat_rows().is_some() || sheet.repeat_columns().is_some()
}

fn push_defined_name(
    out: &mut Vec<u8>,
    sheet_index: usize,
    name: &'static str,
    rgce: Vec<u8>,
) -> Result<()> {
    push_record(
        out,
        &BrtName {
            flags: BrtName::F_BUILTIN,
            chKey: 0,
            itab: u32::try_from(sheet_index)
                .map_err(|_| Error::ValueNotRepresentable("too many worksheets"))?,
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

fn area3d(ixti: u16, first_row: u32, first_col: u16, last_row: u32, last_col: u16) -> [u8; 15] {
    let mut rgce = [0u8; 15];
    rgce[0] = 0x3B;
    rgce[1..3].copy_from_slice(&ixti.to_le_bytes());
    rgce[3..7].copy_from_slice(&first_row.to_le_bytes());
    rgce[7..11].copy_from_slice(&last_row.to_le_bytes());
    rgce[11..13].copy_from_slice(&first_col.to_le_bytes());
    rgce[13..15].copy_from_slice(&last_col.to_le_bytes());
    rgce
}

fn sheet_rels(sheet: &Worksheet) -> Option<String> {
    if sheet.hyperlinks().values().all(crate::Url::is_internal) {
        return None;
    }

    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    ));
    let mut rel_id = 0u32;
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

fn content_types(sheet_count: usize, has_strings: bool) -> String {
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
    body.push_str("</Types>");
    body
}

fn root_rels() -> String {
    concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.bin"/>"#,
        "</Relationships>"
    )
    .to_owned()
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

fn zip_store(parts: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    const DOS_DATE: u16 = 0x0021; // 1980-01-01, the earliest representable date.

    let mut body = Vec::new();
    let mut central = Vec::new();

    for (name, data) in parts {
        let crc = crc32(data);
        let size = as_u32(data.len())?;
        let name_len = u16::try_from(name.len())
            .map_err(|_| Error::ValueNotRepresentable("ZIP entry name is too long"))?;
        let offset = as_u32(body.len())?;

        put_u32(&mut body, 0x0403_4b50);
        put_u16(&mut body, 20);
        put_u16(&mut body, 0);
        put_u16(&mut body, 0);
        put_u16(&mut body, 0);
        put_u16(&mut body, DOS_DATE);
        put_u32(&mut body, crc);
        put_u32(&mut body, size);
        put_u32(&mut body, size);
        put_u16(&mut body, name_len);
        put_u16(&mut body, 0);
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(data);

        put_u32(&mut central, 0x0201_4b50);
        put_u16(&mut central, 20);
        put_u16(&mut central, 20);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u16(&mut central, DOS_DATE);
        put_u32(&mut central, crc);
        put_u32(&mut central, size);
        put_u32(&mut central, size);
        put_u16(&mut central, name_len);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u16(&mut central, 0);
        put_u32(&mut central, 0);
        put_u32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }

    let count = u16::try_from(parts.len())
        .map_err(|_| Error::ValueNotRepresentable("too many ZIP entries"))?;
    let central_offset = as_u32(body.len())?;
    let central_size = as_u32(central.len())?;

    put_u32(&mut central, 0x0605_4b50);
    put_u16(&mut central, 0);
    put_u16(&mut central, 0);
    put_u16(&mut central, count);
    put_u16(&mut central, count);
    put_u32(&mut central, central_size);
    put_u32(&mut central, central_offset);
    put_u16(&mut central, 0);

    body.extend_from_slice(&central);
    Ok(body)
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

fn as_u32(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| Error::ValueNotRepresentable("ZIP part is too large"))
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
            if method != 0 {
                return Err(Error::ValueNotRepresentable(
                    "only stored ZIP entries are supported",
                ));
            }
            return bytes
                .get(data_start..data_end)
                .map(<[u8]>::to_vec)
                .ok_or(Error::ValueNotRepresentable("truncated ZIP entry"));
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
        BrtBeginWsView, BrtBookView, BrtBundleSh, BrtCalcProp, BrtExternSheet, BrtFileSharing,
        BrtHLink, BrtName, BrtSheetProtection, BrtXF,
    };
    use rust_xlsb_core::{BiffHead, CheckedBiff};

    use super::*;

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
}
