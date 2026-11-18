//! Builds a minimal XLSB package and reads it back with `calamine`, so a test
//! only has to state the cells it writes and the values it expects.

use std::collections::BTreeMap;
use std::io::{BufReader, Cursor};

use calamine::{Data, Reader, open_workbook_auto_from_rs};

use crate::prelude::*;
use crate::{
    BiffHead, BiffWrite, CellValue, Error, KnownID, NumberEncoding, Result, ToCellValue, WbContext,
    Workbook, write_as_biff,
};

const SHEET: &str = "Sheet1";
/// Style index that carries the `yyyy-mm-dd` format.
pub const DATE_STYLE: u32 = 1;
const DATE_FORMAT_CODE: u16 = 164;

/// One cell to write: `(row, column, style, value)`.
pub struct CellSpec<'a> {
    pub row: u32,
    pub col: u32,
    pub style: u32,
    pub value: &'a dyn ToCellValue,
}

impl<'a> CellSpec<'a> {
    #[must_use]
    pub fn new(row: u32, col: u32, value: &'a dyn ToCellValue) -> Self {
        Self {
            row,
            col,
            style: 0,
            value,
        }
    }

    #[must_use]
    pub fn style(mut self, style: u32) -> Self {
        self.style = style;
        self
    }
}

/// Writes `cells` into a sheet of `workbook` and returns the sheet as read back.
///
/// # Panics
///
/// Panics if the package cannot be serialized, if `calamine` rejects it, or if
/// the workbook does not contain exactly one sheet named `Sheet1`.
pub fn roundtrip(workbook: &Workbook, cells: &[CellSpec<'_>]) -> calamine::Range<Data> {
    let bytes = build(workbook, cells).expect("package must serialize");
    let mut workbook =
        open_workbook_auto_from_rs(Cursor::new(bytes)).expect("calamine must open it");
    assert_eq!(workbook.sheet_names(), [SHEET.to_owned()]);
    workbook.worksheet_range(SHEET).expect("reads sheet data")
}

/// Record identifiers of the written worksheet, in file order.
pub fn cell_record_ids(workbook: &Workbook, cells: &[CellSpec<'_>]) -> Vec<KnownID> {
    let sheet = worksheet_part(workbook, cells).expect("worksheet must serialize");
    let mut reader = BufReader::new(sheet.as_slice());
    let mut ids = Vec::new();
    while let Ok((head, _)) = BiffHead::read_from_with_data(&mut reader) {
        if let Some(id) = head.known_id() {
            ids.push(id);
        }
    }
    ids
}

/// Whether the produced package carries a Shared Strings part.
///
/// # Panics
///
/// Panics if the package cannot be serialized.
pub fn includes_shared_strings_part(workbook: &Workbook, cells: &[CellSpec<'_>]) -> bool {
    const NAME: &[u8] = b"xl/sharedStrings.bin";
    let bytes = build(workbook, cells).expect("package must serialize");
    bytes.windows(NAME.len()).any(|window| window == NAME)
}

fn build(workbook: &Workbook, cells: &[CellSpec<'_>]) -> Result<Vec<u8>> {
    let has_strings = !workbook.shared_strings().is_empty();
    let mut parts: Vec<(&str, Vec<u8>)> = vec![
        (
            "[Content_Types].xml",
            content_types(has_strings).into_bytes(),
        ),
        ("_rels/.rels", root_rels().into_bytes()),
        (
            "xl/_rels/workbook.bin.rels",
            workbook_rels(has_strings).into_bytes(),
        ),
        ("xl/workbook.bin", workbook_part()?),
        ("xl/worksheets/sheet1.bin", worksheet_part(workbook, cells)?),
        ("xl/styles.bin", styles_part()?),
    ];

    if has_strings {
        let mut sst = Vec::new();
        workbook.shared_strings().write_to_biff_sst(&mut sst)?;
        parts.push(("xl/sharedStrings.bin", sst));
    }

    zip_store(&parts)
}

fn push_record<T: BiffWrite>(out: &mut Vec<u8>, record: &T) -> Result<()> {
    let mut body = Vec::new();
    record.biff_push_to(&mut body)?;
    write_as_biff(record.id(), body, out).map_err(Error::BiffWriteFailed)?;
    Ok(())
}

fn workbook_part() -> Result<Vec<u8>> {
    let mut out = Vec::new();
    push_record(&mut out, &BrtBeginBook::default())?;
    push_record(&mut out, &BrtBeginBundleShs::default())?;
    push_record(
        &mut out,
        &BrtBundleSh {
            hsState: 0,
            iTabID: 1,
            strRelID: XlNullableWideString::construct(Some("rId1")),
            strName: XlWideString::construct(Some(SHEET)),
        },
    )?;
    push_record(&mut out, &BrtEndBundleShs::default())?;
    push_record(&mut out, &BrtEndBook::default())?;
    Ok(out)
}

fn worksheet_part(workbook: &Workbook, cells: &[CellSpec<'_>]) -> Result<Vec<u8>> {
    let sheet = workbook.worksheet();
    let mut out = Vec::new();
    push_record(&mut out, &BrtBeginSheet::default())?;
    push_record(
        &mut out,
        &BrtWsDim {
            rfx: UncheckedRfX {
                rwFirst: cells.iter().map(|c| c.row).min().unwrap_or(0),
                rwLast: cells.iter().map(|c| c.row).max().unwrap_or(0),
                colFirst: cells.iter().map(|c| c.col).min().unwrap_or(0),
                colLast: cells.iter().map(|c| c.col).max().unwrap_or(0),
            },
        },
    )?;
    push_record(&mut out, &BrtBeginSheetData::default())?;

    let mut rows: BTreeMap<u32, Vec<&CellSpec<'_>>> = BTreeMap::new();
    for cell in cells {
        rows.entry(cell.row).or_default().push(cell);
    }

    for (row, mut row_cells) in rows {
        row_cells.sort_by_key(|c| c.col);

        let mut spans: Vec<BrtColSpan> = Vec::new();
        for cell in &row_cells {
            match spans.last_mut() {
                Some(span) if span.colLast / 1024 == cell.col / 1024 => span.colLast = cell.col,
                _ => spans.push(BrtColSpan {
                    colMic: cell.col,
                    colLast: cell.col,
                }),
            }
        }

        push_record(
            &mut out,
            &BrtRowHdr {
                rw: row,
                ixfe: 0,
                miyRw: 300,
                ccolspan: u32::try_from(spans.len())
                    .map_err(|_| Error::ValidationError("too many spans".to_owned()))?,
                rgBrtColspan: spans,
                ..BrtRowHdr::default()
            },
        )?;

        for cell in row_cells {
            let value = cell.value.to_cell_value(&sheet)?;
            push_cell(&mut out, cell, value)?;
        }
    }

    push_record(&mut out, &BrtEndSheetData::default())?;
    push_record(&mut out, &BrtEndSheet::default())?;
    Ok(out)
}

fn push_cell(out: &mut Vec<u8>, spec: &CellSpec<'_>, value: CellValue) -> Result<()> {
    let cell = Cell {
        column: spec.col,
        iStyleRef: spec.style,
        fPhShow: false,
    };

    match value {
        CellValue::Blank => push_record(out, &BrtCellBlank { cell }),
        CellValue::Bool(value) => push_record(out, &BrtCellBool { cell, fBool: value }),
        CellValue::Error(value) => push_record(
            out,
            &BrtCellError {
                cell,
                bError: value,
            },
        ),
        CellValue::Number(NumberEncoding::Real(xnum)) => push_record(
            out,
            &BrtCellReal {
                cell,
                xnum: Xnum::from(xnum),
            },
        ),
        CellValue::Number(NumberEncoding::Rk(value)) => {
            push_record(out, &BrtCellRk { cell, value })
        }
        CellValue::SharedString(value) => push_record(
            out,
            &BrtCellIsst {
                cell,
                isst: value.id(),
            },
        ),
        CellValue::InlineString(text) => push_record(
            out,
            &BrtCellSt {
                cell,
                value: XlWideString::construct(Some(text.as_str())),
            },
        ),
    }
}

fn styles_part() -> Result<Vec<u8>> {
    let mut out = Vec::new();
    push_record(&mut out, &BrtBeginStyleSheet::default())?;

    push_record(&mut out, &BrtBeginFmts { count: 1 })?;
    push_record(
        &mut out,
        &BrtFmt {
            ifmt: DATE_FORMAT_CODE,
            stFmtCode: XlWideString::construct(Some("yyyy-mm-dd")),
        },
    )?;
    push_record(&mut out, &BrtEndFmts::default())?;

    push_record(&mut out, &BrtBeginFonts { count: 1 })?;
    push_record(&mut out, &BrtFont::default())?;
    push_record(&mut out, &BrtEndFonts::default())?;

    push_record(&mut out, &BrtBeginFills { count: 1 })?;
    push_record(&mut out, &BrtFill::default())?;
    push_record(&mut out, &BrtEndFills::default())?;

    push_record(&mut out, &BrtBeginBorders { count: 1 })?;
    push_record(&mut out, &BrtBorder::default())?;
    push_record(&mut out, &BrtEndBorders::default())?;

    push_record(&mut out, &BrtBeginCellStyleXFs { count: 1 })?;
    push_record(&mut out, &BrtXF::default())?;
    push_record(&mut out, &BrtEndCellStyleXFs::default())?;

    push_record(&mut out, &BrtBeginCellXFs { count: 2 })?;
    push_record(&mut out, &BrtXF::default())?;
    push_record(
        &mut out,
        &BrtXF {
            iFmt: DATE_FORMAT_CODE,
            ..BrtXF::default()
        },
    )?;
    push_record(&mut out, &BrtEndCellXFs::default())?;

    push_record(&mut out, &BrtBeginStyles { count: 1 })?;
    push_record(&mut out, &BrtStyle::default())?;
    push_record(&mut out, &BrtEndStyles::default())?;

    push_record(&mut out, &BrtBeginDXFs { count: 0 })?;
    push_record(&mut out, &BrtEndDXFs::default())?;

    push_record(&mut out, &BrtBeginTableStyles::default())?;
    push_record(&mut out, &BrtEndTableStyles::default())?;

    push_record(&mut out, &BrtEndStyleSheet::default())?;
    Ok(out)
}

fn content_types(has_shared_strings: bool) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
        r#"<Default Extension="xml" ContentType="application/xml"/>"#,
        r#"<Override PartName="/xl/workbook.bin" ContentType="application/vnd.ms-excel.sheet.binary.macroEnabled.main"/>"#,
        r#"<Override PartName="/xl/worksheets/sheet1.bin" ContentType="application/vnd.ms-excel.worksheet"/>"#,
        r#"<Override PartName="/xl/styles.bin" ContentType="application/vnd.ms-excel.styles"/>"#,
    ));
    if has_shared_strings {
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

fn workbook_rels(has_shared_strings: bool) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.bin"/>"#,
        r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.bin"/>"#,
    ));
    if has_shared_strings {
        body.push_str(
            r#"<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.bin"/>"#,
        );
    }
    body.push_str("</Relationships>");
    body
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
    u32::try_from(len).map_err(|_| Error::TooLargeRecordBody(len, u32::MAX as usize))
}

fn zip_store(parts: &[(&str, Vec<u8>)]) -> Result<Vec<u8>> {
    const DOS_DATE: u16 = 0x0021; // 1980-01-01, the earliest representable date.

    let mut body = Vec::new();
    let mut central = Vec::new();

    for (name, data) in parts {
        let crc = crc32(data);
        let size = as_u32(data.len())?;
        let name_len = u16::try_from(name.len())
            .map_err(|_| Error::ValidationError("ZIP entry name is too long".to_owned()))?;
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
        .map_err(|_| Error::ValidationError("Too many ZIP entries".to_owned()))?;
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
