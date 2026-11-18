//! XLSB package assembly: parts, relationships, and store-only ZIP.

use std::fmt::Write as _;

use rust_xlsb_core::prelude::{
    BrtBeginBook, BrtBeginBookViews, BrtBeginBundleShs, BrtBookView, BrtBundleSh, BrtEndBook,
    BrtEndBookViews, BrtEndBundleShs, XlNullableWideString, XlStringExt, XlWideString,
};
use rust_xlsb_core::{SSTHolder, StyleRegistry, push_record};

use crate::worksheet::{Visibility, Worksheet};
use crate::{Error, Result};

pub fn build_package(sheets: &[Worksheet]) -> Result<Vec<u8>> {
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
        ("xl/workbook.bin".to_owned(), workbook_part(sheets)?),
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

fn workbook_part(sheets: &[Worksheet]) -> Result<Vec<u8>> {
    let view = workbook_view(sheets);
    let mut out = Vec::new();
    push_record(&mut out, &BrtBeginBook::default())?;
    if view.first != 0 || view.active != 0 {
        push_record(&mut out, &BrtBeginBookViews::default())?;
        push_record(
            &mut out,
            &BrtBookView {
                xWn: 120,
                yWn: 30,
                dxWn: 23_760,
                dyWn: 14_640,
                iTabRatio: 600,
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
    push_record(&mut out, &BrtEndBook::default())?;
    Ok(out)
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
    use rust_xlsb_core::prelude::{BrtBeginWsView, BrtBookView, BrtBundleSh};
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
