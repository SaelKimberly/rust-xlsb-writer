//! Worksheet part serialization.

use std::collections::BTreeSet;

use rust_xlsb_core::prelude::{
    BrtBeginColInfos, BrtBeginSheet, BrtBeginSheetData, BrtCellBlank, BrtCellBool, BrtCellError,
    BrtCellIsst, BrtCellReal, BrtCellRk, BrtColInfo, BrtColSpan, BrtEndColInfos, BrtEndSheet,
    BrtEndSheetData, BrtRowHdr, BrtWsDim, Cell, UncheckedRfX, Xnum,
};
use rust_xlsb_core::{
    BiffWrite, CellValue, NumberEncoding, SSTHolder, ToCellValue, WbContext, push_record,
};

use crate::data::CellData;
use crate::worksheet::{ColumnOptions, Worksheet};
use crate::{Error, Result};

const DEFAULT_COL_WIDTH: u32 = 2_158;
const DEFAULT_ROW_HEIGHT: u16 = 300;

struct SaveContext<'a> {
    sst: &'a SSTHolder,
}

impl WbContext for SaveContext<'_> {
    fn shared_strings(&self) -> &SSTHolder {
        self.sst
    }
}

pub fn write_sheet(
    sheet: &Worksheet,
    style_indexes: &[u32],
    sst: &SSTHolder,
    out: &mut Vec<u8>,
) -> Result<()> {
    let ctx = SaveContext { sst };

    push(out, &BrtBeginSheet::default())?;
    push(out, &dimension(sheet))?;

    let columns = column_infos(sheet, style_indexes);
    if !columns.is_empty() {
        push(
            out,
            &BrtBeginColInfos {
                count: u32::try_from(columns.len())
                    .map_err(|_| Error::ValueNotRepresentable("too many column infos"))?,
            },
        )?;
        for column in &columns {
            push(out, column)?;
        }
        push(out, &BrtEndColInfos::default())?;
    }

    push(out, &BrtBeginSheetData::default())?;
    let mut rows = BTreeSet::new();
    rows.extend(sheet.cells().keys().copied());
    rows.extend(sheet.row_options().keys().copied());

    for row in rows {
        let row_options = sheet.row_options().get(&row);
        let row_style = row_options
            .and_then(|options| options.format)
            .map_or(0, |index| mapped_style(index, style_indexes));
        let row_height = row_options
            .and_then(|options| options.height)
            .map_or(DEFAULT_ROW_HEIGHT, height_to_twips);
        let cells = sheet.cells().get(&row);
        let spans = cells.map_or_else(Vec::new, |cells| column_spans(cells.keys().copied()));

        push(
            out,
            &BrtRowHdr {
                rw: row,
                ixfe: row_style,
                miyRw: row_height,
                fGhostDirty: row_options.and_then(|options| options.format).is_some(),
                ccolspan: u32::try_from(spans.len())
                    .map_err(|_| Error::ValueNotRepresentable("too many column spans"))?,
                rgBrtColspan: spans,
                ..BrtRowHdr::default()
            },
        )?;

        if let Some(cells) = cells {
            for (&col, cell) in cells {
                let column_style = sheet
                    .column_options()
                    .get(&col)
                    .and_then(|options| options.format);
                let style = cell
                    .format
                    .or_else(|| row_options.and_then(|options| options.format))
                    .or(column_style)
                    .map_or(0, |index| mapped_style(index, style_indexes));

                push_cell(out, &ctx, col, style, &cell.data)?;
            }
        }
    }

    push(out, &BrtEndSheetData::default())?;
    push(out, &BrtEndSheet::default())?;
    Ok(())
}

fn dimension(sheet: &Worksheet) -> BrtWsDim {
    let mut min_row = u32::MAX;
    let mut max_row = 0;
    let mut min_col = u32::MAX;
    let mut max_col = 0;

    for (&row, cells) in sheet.cells() {
        min_row = min_row.min(row);
        max_row = max_row.max(row);
        for &col in cells.keys() {
            let col = u32::from(col);
            min_col = min_col.min(col);
            max_col = max_col.max(col);
        }
    }
    for &row in sheet.row_options().keys() {
        min_row = min_row.min(row);
        max_row = max_row.max(row);
    }
    for &col in sheet.column_options().keys() {
        let col = u32::from(col);
        min_col = min_col.min(col);
        max_col = max_col.max(col);
    }

    if min_row == u32::MAX {
        min_row = 0;
    }
    if min_col == u32::MAX {
        min_col = 0;
    }

    BrtWsDim {
        rfx: UncheckedRfX {
            rwFirst: min_row,
            rwLast: max_row,
            colFirst: min_col,
            colLast: max_col,
        },
    }
}

fn column_infos(sheet: &Worksheet, style_indexes: &[u32]) -> Vec<BrtColInfo> {
    let mut infos = Vec::new();
    let mut columns = sheet.column_options().iter().peekable();

    while let Some((&first, first_options)) = columns.next() {
        let mut last = first;
        while let Some((next, next_options)) = columns.peek() {
            if **next == last.saturating_add(1)
                && next_options.width == first_options.width
                && next_options.format == first_options.format
            {
                last = **next;
                columns.next();
            } else {
                break;
            }
        }
        infos.push(col_info(first, last, first_options, style_indexes));
    }

    infos
}

fn col_info(first: u16, last: u16, options: &ColumnOptions, style_indexes: &[u32]) -> BrtColInfo {
    BrtColInfo {
        colFirst: u32::from(first),
        colLast: u32::from(last),
        coldx: options.width.map_or(DEFAULT_COL_WIDTH, width_to_units),
        ixfe: options
            .format
            .map_or(0, |index| mapped_style(index, style_indexes)),
        fUserSet: true,
        ..BrtColInfo::default()
    }
}

fn column_spans(columns: impl Iterator<Item = u16>) -> Vec<BrtColSpan> {
    let mut spans: Vec<BrtColSpan> = Vec::new();
    for col in columns {
        let col = u32::from(col);
        match spans.last_mut() {
            Some(span) if span.colLast / 1024 == col / 1024 => span.colLast = col,
            _ => spans.push(BrtColSpan {
                colMic: col,
                colLast: col,
            }),
        }
    }
    spans
}

fn push<T: BiffWrite>(out: &mut Vec<u8>, record: &T) -> Result<()> {
    push_record(out, record).map_err(Error::from)
}

fn push_cell(
    out: &mut Vec<u8>,
    ctx: &SaveContext<'_>,
    col: u16,
    style: u32,
    data: &CellData,
) -> Result<()> {
    let cell = Cell {
        column: u32::from(col),
        iStyleRef: style,
        fPhShow: false,
    };

    match data {
        CellData::Blank => push(out, &BrtCellBlank { cell }),
        CellData::Bool(value) => push(
            out,
            &BrtCellBool {
                cell,
                fBool: *value,
            },
        ),
        CellData::Error(value) => push(
            out,
            &BrtCellError {
                cell,
                bError: *value,
            },
        ),
        CellData::String(text) => push(
            out,
            &BrtCellIsst {
                cell,
                isst: ctx.shared_string(text)?.id(),
            },
        ),
        CellData::Integer(value) => {
            push_number(out, cell, &CellValue::Number(ctx.store_integer(*value)?))
        }
        CellData::Number(value) => {
            push_number(out, cell, &CellValue::Number(ctx.store_f64(*value)))
        }
        CellData::Date(value) => push_number(out, cell, &value.to_cell_value(ctx)?),
        CellData::Time(value) => push_number(out, cell, &value.to_cell_value(ctx)?),
        CellData::DateTime(value) => push_number(out, cell, &value.to_cell_value(ctx)?),
        CellData::Duration(value) => push_number(out, cell, &value.to_cell_value(ctx)?),
    }
}

fn push_number(out: &mut Vec<u8>, cell: Cell, value: &CellValue) -> Result<()> {
    match value {
        CellValue::Number(NumberEncoding::Real(value)) => push(
            out,
            &BrtCellReal {
                cell,
                xnum: Xnum::from(*value),
            },
        ),
        CellValue::Number(NumberEncoding::Rk(value)) => push(
            out,
            &BrtCellRk {
                cell,
                value: *value,
            },
        ),
        _ => Err(Error::ValueNotRepresentable(
            "temporal or numeric conversion did not produce a number",
        )),
    }
}

fn mapped_style(index: u32, style_indexes: &[u32]) -> u32 {
    style_indexes
        .get(usize::try_from(index).expect("style index fits usize"))
        .copied()
        .unwrap_or(0)
}

fn width_to_units(width: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "width is validated to 0..=255 before reaching this conversion"
    )]
    {
        (width * 256.0).round() as u32
    }
}

fn height_to_twips(height: f64) -> u16 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "height is validated to 0..=409.5 before reaching this conversion"
    )]
    {
        (height * 20.0).round() as u16
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use deku::DekuContainerRead;
    use rust_xlsb_core::prelude::{BrtCellIsst, BrtCellRk, BrtColInfo, BrtRowHdr};
    use rust_xlsb_core::{BiffHead, CheckedBiff, SSTHolder, StyleRegistry};

    use super::*;
    use crate::Format;
    use crate::worksheet::MAX_COL;

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
    fn emits_column_widths_row_heights_and_cell_records() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_column_width(0, 22.0).expect("width");
        sheet.set_column_range_width(1, 2, 10.0).expect("width");
        sheet.set_row_height(0, 30.0).expect("height");
        sheet.write(0, 0, 42_i32).expect("writes");
        sheet.write(0, 1, "x").expect("writes");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, &mut out).expect("sheet");

        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 2);
        assert_eq!((columns[0].colFirst, columns[0].colLast), (0, 0));
        assert_eq!(columns[0].coldx, 5_632);
        assert_eq!((columns[1].colFirst, columns[1].colLast), (1, 2));
        assert_eq!(columns[1].coldx, 2_560);

        let rows = records::<BrtRowHdr>(&out);
        assert_eq!(rows[0].miyRw, 600);

        let rk = records::<BrtCellRk>(&out);
        assert_eq!(rk.len(), 1);
        assert_eq!(rk[0].cell.column, 0);
        let isst = records::<BrtCellIsst>(&out);
        assert_eq!(isst.len(), 1);
        assert_eq!(isst[0].isst, 0);
    }

    #[test]
    fn coalesces_columns_and_splits_on_override() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_column_range_width(0, 3, 10.0).expect("width");
        sheet.set_column_width(1, 20.0).expect("override");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, &mut out).expect("sheet");

        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 3);
        assert_eq!((columns[0].colFirst, columns[0].colLast), (0, 0));
        assert_eq!((columns[1].colFirst, columns[1].colLast), (1, 1));
        assert_eq!((columns[2].colFirst, columns[2].colLast), (2, 3));
    }

    #[test]
    fn row_format_sets_ghost_dirty_and_style() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        let format = Format::new().set_bold();
        sheet.set_row_format(0, &format).expect("row");
        let mut registry = StyleRegistry::new();
        let style = format.to_cell_style(&mut registry).expect("style");
        let index = registry.add(style).expect("adds");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[index], &sst, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert_eq!(rows[0].ixfe, index);
        assert!(rows[0].fGhostDirty);
    }

    #[test]
    fn effective_style_prefers_cell_then_row_then_column() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        let column_format = Format::new().set_text_wrap();
        let row_format = Format::new().set_italic();
        let cell_format = Format::new().set_bold();
        sheet.set_column_format(0, &column_format).expect("column");
        sheet.set_row_format(0, &row_format).expect("row");
        sheet
            .write_with_format(0, 0, 1_i32, &cell_format)
            .expect("cell");
        sheet.write(0, 1, 2_i32).expect("row style");
        sheet.write(1, 0, 3_i32).expect("column style");

        let mut registry = StyleRegistry::new();
        let mut indexes = Vec::new();
        for format in sheet.formats() {
            let style = format.to_cell_style(&mut registry).expect("style");
            indexes.push(registry.add(style).expect("adds"));
        }
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &indexes, &sst, &mut out).expect("sheet");

        let cells = records::<BrtCellRk>(&out);
        assert_eq!(cells[0].cell.iStyleRef, indexes[2]);
        assert_eq!(cells[1].cell.iStyleRef, indexes[1]);
        assert_eq!(cells[2].cell.iStyleRef, indexes[0]);
    }

    #[test]
    fn full_width_row_stays_within_span_limit() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.write(0, 0, 1_i32).expect("writes");
        sheet.write(0, MAX_COL, 2_i32).expect("writes");
        for block in 0_u16..16 {
            sheet.write(1, block * 1_024, 1_i32).expect("writes");
        }
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert_eq!(rows[0].ccolspan, 2);
        assert_eq!(
            (
                rows[0].rgBrtColspan[0].colMic,
                rows[0].rgBrtColspan[0].colLast
            ),
            (0, 0)
        );
        assert_eq!(
            (
                rows[0].rgBrtColspan[1].colMic,
                rows[0].rgBrtColspan[1].colLast
            ),
            (16_383, 16_383)
        );
        assert_eq!(rows[1].ccolspan, 16);
    }
}
