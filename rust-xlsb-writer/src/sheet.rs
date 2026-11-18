//! Worksheet part serialization.

use std::collections::BTreeSet;

use rust_xlsb_core::prelude::{
    BrtBeginColBrk, BrtBeginColInfos, BrtBeginHeaderFooter, BrtBeginMergeCells, BrtBeginRwBrk,
    BrtBeginSheet, BrtBeginSheetData, BrtBeginWsView, BrtBeginWsViews, BrtBrk, BrtCellBlank,
    BrtCellBool, BrtCellError, BrtCellIsst, BrtCellReal, BrtCellRk, BrtColInfo, BrtColSpan,
    BrtEndColBrk, BrtEndColInfos, BrtEndHeaderFooter, BrtEndMergeCells, BrtEndRwBrk, BrtEndSheet,
    BrtEndSheetData, BrtEndWsView, BrtEndWsViews, BrtMargins, BrtMergeCell, BrtPageSetup, BrtPane,
    BrtPrintOptions, BrtRowHdr, BrtSel, BrtWsDim, BrtWsFmtInfo, BrtWsProp, Cell,
    HeaderFooterString, UncheckedRfX, UncheckedSqRfX, XlStringExt, Xnum,
};
use rust_xlsb_core::{
    BiffWrite, CellValue, NumberEncoding, SSTHolder, ToCellValue, WbContext, push_record,
};

use crate::data::CellData;
use crate::worksheet::{ColumnOptions, MAX_COL, MAX_ROW, Worksheet};
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
    selected: bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    let ctx = SaveContext { sst };

    push(out, &BrtBeginSheet::default())?;
    write_ws_prop(sheet, out)?;
    push(out, &dimension(sheet))?;
    write_sheet_views(sheet, selected, out)?;
    write_default_row_height(sheet, out)?;

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
        let row_height = if row_options.is_some_and(|options| options.hidden) {
            0
        } else {
            row_options
                .and_then(|options| options.height)
                .map_or(DEFAULT_ROW_HEIGHT, height_to_twips)
        };
        let cells = sheet.cells().get(&row);
        let spans = cells.map_or_else(Vec::new, |cells| column_spans(cells.keys().copied()));

        push(
            out,
            &BrtRowHdr {
                rw: row,
                ixfe: row_style,
                miyRw: row_height,
                fDyZero: row_options.is_some_and(|options| options.hidden),
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

    let merged = sheet.merged_ranges();
    if !merged.is_empty() {
        push(
            out,
            &BrtBeginMergeCells {
                count: u32::try_from(merged.len())
                    .map_err(|_| Error::ValueNotRepresentable("too many merged ranges"))?,
            },
        )?;
        for range in merged {
            push(
                out,
                &BrtMergeCell {
                    rfx: UncheckedRfX {
                        rwFirst: range.first_row,
                        rwLast: range.last_row,
                        colFirst: u32::from(range.first_col),
                        colLast: u32::from(range.last_col),
                    },
                },
            )?;
        }
        push(out, &BrtEndMergeCells::default())?;
    }

    write_print_settings(sheet, out)?;
    write_header_footer(sheet, out)?;
    write_page_breaks(sheet, out)?;

    push(out, &BrtEndSheet::default())?;
    Ok(())
}

fn write_ws_prop(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let fit_to_page = sheet.page_setup().fit_to_pages.is_some();
    let Some(color) = sheet.tab_color() else {
        if !fit_to_page {
            return Ok(());
        }
        return push(
            out,
            &BrtWsProp {
                fShowAutoBreaks: true,
                fPublish: true,
                fRowSumsBelow: true,
                fColSumsRight: true,
                fFitToPage: true,
                fShowOutlineSymbols: true,
                fCondFmtCalc: true,
                rwSync: u32::MAX,
                colSync: u32::MAX,
                ..BrtWsProp::default()
            },
        );
    };

    push(
        out,
        &BrtWsProp {
            fShowAutoBreaks: true,
            fPublish: true,
            fRowSumsBelow: true,
            fColSumsRight: true,
            fFitToPage: fit_to_page,
            fShowOutlineSymbols: true,
            fCondFmtCalc: true,
            brtcolorTab: color.to_brt_color()?,
            rwSync: u32::MAX,
            colSync: u32::MAX,
            ..BrtWsProp::default()
        },
    )
}

fn write_default_row_height(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let Some(height) = sheet.default_row_height() else {
        return Ok(());
    };
    push(
        out,
        &BrtWsFmtInfo {
            dxGCol: DEFAULT_COL_WIDTH,
            miyDefRwHeight: height_to_twips(height),
            fUnsynced: true,
            ..BrtWsFmtInfo::default()
        },
    )
}

fn write_sheet_views(sheet: &Worksheet, selected: bool, out: &mut Vec<u8>) -> Result<()> {
    let view = sheet.view();
    if view.is_default() && !selected {
        return Ok(());
    }

    let active_pane = active_pane(view.freeze, view.selection);
    let (rw_top, col_left) = view
        .top_left
        .map_or((0, 0), |(row, col)| (row, u32::from(col)));
    push(out, &BrtBeginWsViews::default())?;
    push(
        out,
        &BrtBeginWsView {
            fDspGrid: view.show_gridlines,
            fDspRwCol: true,
            fDspZeros: view.show_zeros,
            fRightToLeft: view.right_to_left,
            fSelected: view.selected || selected,
            fDspGuts: true,
            fDefaultHdr: true,
            rwTop: rw_top,
            colLeft: col_left,
            icvHdr: 0x40,
            wScale: view.zoom,
            wScaleNormal: 100,
            ..BrtBeginWsView::default()
        },
    )?;

    if let Some((row, col)) = view.freeze
        && (row > 0 || col > 0)
    {
        let (top_row, left_col) = view.freeze_top_cell.unwrap_or((row, col));
        push(
            out,
            &BrtPane {
                xnumXSplit: Xnum::from(f64::from(row)),
                xnumYSplit: Xnum::from(f64::from(col)),
                rwTop: top_row,
                colLeft: u32::from(left_col),
                pnnAct: active_pane,
                fFrozen: false,
                fFrozenNoSplit: true,
            },
        )?;
    }

    if let Some(range) = view.selection {
        push(
            out,
            &BrtSel {
                pnn: active_pane,
                rwAct: range.rwFirst,
                colAct: range.colFirst,
                dwRfxAct: 0,
                sqrfx: UncheckedSqRfX {
                    crfx: 1,
                    rgRefx: vec![range],
                },
            },
        )?;
    }

    push(out, &BrtEndWsView::default())?;
    push(out, &BrtEndWsViews::default())?;
    Ok(())
}

fn active_pane(freeze: Option<(u32, u16)>, selection: Option<UncheckedRfX>) -> u32 {
    const TOP_LEFT: u32 = 3;
    const TOP_RIGHT: u32 = 1;
    const BOTTOM_LEFT: u32 = 2;
    const BOTTOM_RIGHT: u32 = 0;

    let Some((freeze_row, freeze_col)) = freeze else {
        return TOP_LEFT;
    };
    if freeze_row == 0 && freeze_col == 0 {
        return TOP_LEFT;
    }
    let (row, col) = selection.map_or_else(
        || (freeze_row, u32::from(freeze_col)),
        |range| (range.rwFirst, range.colFirst),
    );
    let below = freeze_row > 0 && row >= freeze_row;
    let right = freeze_col > 0 && col >= u32::from(freeze_col);

    match (below, right) {
        (true, true) => BOTTOM_RIGHT,
        (true, false) => BOTTOM_LEFT,
        (false, true) => TOP_RIGHT,
        (false, false) => TOP_LEFT,
    }
}

fn write_print_settings(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let setup = sheet.page_setup();

    if setup.has_print_options() {
        push(
            out,
            &BrtPrintOptions {
                fHCenter: setup.center_horizontally,
                fVCenter: setup.center_vertically,
                fPrintHeaders: setup.print_headings,
                fPrintGrid: setup.print_gridlines,
                unused: true,
            },
        )?;
    }

    if setup.has_margins() {
        push(
            out,
            &BrtMargins {
                xnumLeft: Xnum::from(setup.margin_left),
                xnumRight: Xnum::from(setup.margin_right),
                xnumTop: Xnum::from(setup.margin_top),
                xnumBottom: Xnum::from(setup.margin_bottom),
                xnumHeader: Xnum::from(setup.margin_header),
                xnumFooter: Xnum::from(setup.margin_footer),
            },
        )?;
    }

    if setup.has_page_setup() {
        let (scale, fit_width, fit_height) = if let Some((width, height)) = setup.fit_to_pages {
            (0, u32::from(width), u32::from(height))
        } else {
            (u32::from(setup.print_scale), 0, 0)
        };
        let mut grbit = 0;
        if setup.left_to_right {
            grbit |= BrtPageSetup::F_LEFT_TO_RIGHT;
        }
        if setup.landscape {
            grbit |= BrtPageSetup::F_LANDSCAPE;
        }
        if setup.black_and_white {
            grbit |= BrtPageSetup::F_NO_COLOR;
        }
        if setup.draft {
            grbit |= BrtPageSetup::F_DRAFT;
        }
        if setup.first_page.is_some() {
            grbit |= BrtPageSetup::F_USE_PAGE;
        }

        push(
            out,
            &BrtPageSetup {
                iPaperSize: u32::from(setup.paper_size),
                iScale: scale,
                iCopies: 1,
                iPageStart: setup.first_page.map_or(0, i32::from),
                iFitWidth: fit_width,
                iFitHeight: fit_height,
                grbit,
                ..BrtPageSetup::without_printer_settings()
            },
        )?;
    }

    Ok(())
}

fn write_header_footer(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let header_footer = sheet.header_footer();
    if header_footer.is_default() {
        return Ok(());
    }

    let mut grbit = 0;
    if header_footer.scale_with_doc {
        grbit |= BrtBeginHeaderFooter::F_SCALE_WITH_DOC;
    }
    if header_footer.align_with_page {
        grbit |= BrtBeginHeaderFooter::F_ALIGN_MARGINS;
    }

    push(
        out,
        &BrtBeginHeaderFooter {
            grbit,
            stHeader: HeaderFooterString::construct(header_footer.header.as_deref()),
            stFooter: HeaderFooterString::construct(header_footer.footer.as_deref()),
            ..BrtBeginHeaderFooter::default()
        },
    )?;
    push(out, &BrtEndHeaderFooter::default())
}

fn write_page_breaks(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let breaks = sheet.page_breaks();
    if !breaks.is_empty() {
        let count = u32::try_from(breaks.len())
            .map_err(|_| Error::ValueNotRepresentable("too many page breaks"))?;
        push(
            out,
            &BrtBeginRwBrk {
                ibrkMac: count,
                ibrkManMac: count,
            },
        )?;
        for &row in breaks {
            push(
                out,
                &BrtBrk {
                    unRwCol: row,
                    unColRwStrt: 0,
                    unColRwEnd: u32::from(MAX_COL),
                    fMan: 1,
                    fPivot: 0,
                },
            )?;
        }
        push(out, &BrtEndRwBrk::default())?;
    }

    let breaks = sheet.vertical_page_breaks();
    if !breaks.is_empty() {
        let count = u32::try_from(breaks.len())
            .map_err(|_| Error::ValueNotRepresentable("too many page breaks"))?;
        push(
            out,
            &BrtBeginColBrk {
                ibrkMac: count,
                ibrkManMac: count,
            },
        )?;
        for &col in breaks {
            push(
                out,
                &BrtBrk {
                    unRwCol: col,
                    unColRwStrt: 0,
                    unColRwEnd: MAX_ROW,
                    fMan: 1,
                    fPivot: 0,
                },
            )?;
        }
        push(out, &BrtEndColBrk::default())?;
    }

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
                && next_options.hidden == first_options.hidden
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
        coldx: if options.hidden {
            0
        } else {
            options.width.map_or(DEFAULT_COL_WIDTH, width_to_units)
        },
        ixfe: options
            .format
            .map_or(0, |index| mapped_style(index, style_indexes)),
        fHidden: options.hidden,
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
    use rust_xlsb_core::prelude::{
        BrtBeginColBrk, BrtBeginHeaderFooter, BrtBeginMergeCells, BrtBeginRwBrk, BrtBeginWsView,
        BrtBrk, BrtCellIsst, BrtCellRk, BrtColInfo, BrtColor, BrtEndSheet, BrtMargins,
        BrtMergeCell, BrtPageSetup, BrtPane, BrtPrintOptions, BrtRowHdr, BrtSel, BrtWsFmtInfo,
        BrtWsProp,
    };
    use rust_xlsb_core::{BiffHead, CheckedBiff, KnownID, SSTHolder, StyleRegistry};

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

    fn record_ids(part: &[u8]) -> Vec<KnownID> {
        let mut reader = BufReader::new(part);
        let mut ids = Vec::new();
        while let Ok((head, _)) = BiffHead::read_from_with_data(&mut reader) {
            ids.extend(head.known_id());
        }
        ids
    }

    #[test]
    fn freeze_and_selection_emit_view_records() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_freeze_panes(2, 3).expect("freeze");
        sheet.set_selection(3, 1, 5, 4).expect("selection");
        sheet.set_top_left_cell(4, 5).expect("top left");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert_eq!((views[0].rwTop, views[0].colLeft), (4, 5));
        assert!(views[0].fDspGrid);

        let panes = records::<BrtPane>(&out);
        assert_eq!(panes.len(), 1);
        assert_eq!(*panes[0].xnumXSplit, 2.0);
        assert_eq!(*panes[0].xnumYSplit, 3.0);
        assert_eq!((panes[0].rwTop, panes[0].colLeft), (2, 3));
        assert_eq!(panes[0].pnnAct, 2);
        assert!(panes[0].fFrozenNoSplit);

        let selections = records::<BrtSel>(&out);
        assert_eq!(selections.len(), 1);
        assert_eq!(selections[0].pnn, 2);
        assert_eq!((selections[0].rwAct, selections[0].colAct), (3, 1));
        assert_eq!(selections[0].sqrfx.crfx, 1);
        assert_eq!(
            selections[0].sqrfx.rgRefx[0],
            UncheckedRfX {
                rwFirst: 3,
                rwLast: 5,
                colFirst: 1,
                colLast: 4,
            }
        );
    }

    #[test]
    fn view_options_and_freeze_top_cell_serialize() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_freeze_panes(1, 2).expect("freeze");
        sheet.set_freeze_panes_top_cell(3, 4).expect("top cell");
        sheet.set_zoom(175);
        sheet.set_screen_gridlines(false);
        sheet.set_hidden_zeroes(false);
        sheet.set_right_to_left(true);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert!(!views[0].fDspGrid);
        assert!(views[0].fDspZeros);
        assert!(views[0].fRightToLeft);
        assert_eq!((views[0].wScale, views[0].wScaleNormal), (175, 100));

        let panes = records::<BrtPane>(&out);
        assert_eq!(panes.len(), 1);
        assert_eq!((panes[0].rwTop, panes[0].colLeft), (3, 4));
        assert_eq!(*panes[0].xnumXSplit, 1.0);
        assert_eq!(*panes[0].xnumYSplit, 2.0);
    }

    #[test]
    fn hidden_zeroes_disable_the_zero_display_flag() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_hidden_zeroes(true);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert!(views[0].fDspGrid);
        assert!(!views[0].fDspZeros);
    }

    #[test]
    fn active_pane_follows_freeze_and_selection() {
        let range = |row, col| UncheckedRfX {
            rwFirst: row,
            rwLast: row,
            colFirst: col,
            colLast: col,
        };

        assert_eq!(active_pane(None, None), 3);
        assert_eq!(active_pane(Some((2, 3)), Some(range(0, 0))), 3);
        assert_eq!(active_pane(Some((2, 3)), Some(range(3, 1))), 2);
        assert_eq!(active_pane(Some((2, 3)), Some(range(1, 4))), 1);
        assert_eq!(active_pane(Some((2, 3)), Some(range(3, 4))), 0);
        assert_eq!(active_pane(Some((2, 0)), Some(range(1, 0))), 3);
        assert_eq!(active_pane(Some((2, 0)), Some(range(2, 0))), 2);
        assert_eq!(active_pane(Some((0, 3)), Some(range(0, 2))), 3);
        assert_eq!(active_pane(Some((0, 3)), Some(range(0, 4))), 1);
    }

    #[test]
    fn tab_color_and_default_height_emit_sheet_properties() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_tab_color(crate::Color::Red);
        sheet.set_default_row_height(30.0).expect("height");
        sheet.set_selected(true);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let props = records::<BrtWsProp>(&out);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].brtcolorTab, BrtColor::rgb(0xFF, 0x00, 0x00));
        assert_eq!((props[0].rwSync, props[0].colSync), (u32::MAX, u32::MAX));

        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].miyDefRwHeight, 600);
        assert_eq!(formats[0].dxGCol, DEFAULT_COL_WIDTH);
        assert!(formats[0].fUnsynced);

        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert!(views[0].fSelected);
    }

    #[test]
    fn default_sheet_view_emits_no_view_records() {
        let sheet = Worksheet::new("Sheet1".to_owned());
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        assert_eq!(records::<BrtBeginWsView>(&out).len(), 0);
        assert_eq!(records::<BrtPane>(&out).len(), 0);
        assert_eq!(records::<BrtSel>(&out).len(), 0);
        assert_eq!(records::<BrtWsProp>(&out).len(), 0);
        assert_eq!(records::<BrtWsFmtInfo>(&out).len(), 0);
        assert_eq!(records::<BrtPrintOptions>(&out).len(), 0);
        assert_eq!(records::<BrtMargins>(&out).len(), 0);
        assert_eq!(records::<BrtPageSetup>(&out).len(), 0);
        assert_eq!(records::<BrtBeginHeaderFooter>(&out).len(), 0);
        assert_eq!(records::<BrtBeginRwBrk>(&out).len(), 0);
        assert_eq!(records::<BrtBeginColBrk>(&out).len(), 0);
        assert_eq!(records::<BrtBrk>(&out).len(), 0);
    }

    #[test]
    fn header_footer_serializes_text_and_flags() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_landscape();
        sheet.set_header("&L&[Page]");
        sheet.set_footer("&R&[File]");
        sheet.set_header_footer_scale_with_doc(false);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let headers = records::<BrtBeginHeaderFooter>(&out);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].grbit, BrtBeginHeaderFooter::F_ALIGN_MARGINS);
        assert_eq!(headers[0].stHeader.try_to_string().expect("header"), "&L&P");
        assert_eq!(headers[0].stFooter.try_to_string().expect("footer"), "&R&F");
        assert!(headers[0].stHeaderEven.is_null());
        assert!(headers[0].stFooterFirst.is_null());

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|known| *known == id).expect("record");
        assert!(
            position(BrtPageSetup::default().id()) < position(BrtBeginHeaderFooter::default().id())
        );
        assert!(
            position(BrtBeginHeaderFooter::default().id()) < position(BrtEndSheet::default().id())
        );
    }

    #[test]
    fn header_footer_preserves_non_ascii_text() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_header("&CПривет 😀");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let headers = records::<BrtBeginHeaderFooter>(&out);
        assert_eq!(
            headers[0].stHeader.try_to_string().expect("header"),
            "&CПривет 😀"
        );
        assert_eq!(headers[0].stHeader.len(), 11);
    }

    #[test]
    fn page_breaks_serialize_with_full_span() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_header("report");
        sheet.set_page_breaks(&[40, 20, 20]).expect("horizontal");
        sheet.set_vertical_page_breaks(&[5, 3]).expect("vertical");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtBeginRwBrk>(&out);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].ibrkMac, rows[0].ibrkManMac), (2, 2));

        let columns = records::<BrtBeginColBrk>(&out);
        assert_eq!(columns.len(), 1);
        assert_eq!((columns[0].ibrkMac, columns[0].ibrkManMac), (2, 2));

        let breaks = records::<BrtBrk>(&out);
        assert_eq!(breaks.len(), 4);
        assert_eq!(
            (
                breaks[0].unRwCol,
                breaks[0].unColRwStrt,
                breaks[0].unColRwEnd
            ),
            (20, 0, u32::from(MAX_COL))
        );
        assert_eq!(
            (
                breaks[2].unRwCol,
                breaks[2].unColRwStrt,
                breaks[2].unColRwEnd
            ),
            (3, 0, MAX_ROW)
        );
        assert!(
            breaks
                .iter()
                .all(|entry| entry.fMan == 1 && entry.fPivot == 0)
        );

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|known| *known == id).expect("record");
        assert!(
            position(BrtBeginHeaderFooter::default().id())
                < position(BrtBeginRwBrk::default().id())
        );
        assert!(position(BrtBeginRwBrk::default().id()) < position(BrtBeginColBrk::default().id()));
        assert!(position(BrtBeginColBrk::default().id()) < position(BrtEndSheet::default().id()));
    }

    #[test]
    fn page_setup_and_print_options_serialize_in_order() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_landscape();
        sheet.set_paper_size(9);
        sheet.set_page_order(false);
        sheet.set_margins(1.0, 1.1, 1.2, 1.3, 0.4, 0.5);
        sheet.set_print_gridlines(true);
        sheet.set_print_headings(true);
        sheet.set_print_center_horizontally(true);
        sheet.set_print_center_vertically(true);
        sheet.set_print_black_and_white(true);
        sheet.set_print_draft(true);
        sheet.set_print_first_page_number(6);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let print = records::<BrtPrintOptions>(&out);
        assert_eq!(print.len(), 1);
        assert!(print[0].fHCenter);
        assert!(print[0].fVCenter);
        assert!(print[0].fPrintHeaders);
        assert!(print[0].fPrintGrid);
        assert!(print[0].unused);

        let margins = records::<BrtMargins>(&out);
        assert_eq!(margins.len(), 1);
        assert_eq!(*margins[0].xnumLeft, 1.0);
        assert_eq!(*margins[0].xnumRight, 1.1);
        assert_eq!(*margins[0].xnumTop, 1.2);
        assert_eq!(*margins[0].xnumBottom, 1.3);
        assert_eq!(*margins[0].xnumHeader, 0.4);
        assert_eq!(*margins[0].xnumFooter, 0.5);

        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(setup.len(), 1);
        assert_eq!(setup[0].iPaperSize, 9);
        assert_eq!(setup[0].iScale, 100);
        assert_eq!(setup[0].iCopies, 1);
        assert_eq!(setup[0].iPageStart, 6);
        assert_eq!(
            setup[0].grbit,
            BrtPageSetup::F_LEFT_TO_RIGHT
                | BrtPageSetup::F_LANDSCAPE
                | BrtPageSetup::F_NO_COLOR
                | BrtPageSetup::F_DRAFT
                | BrtPageSetup::F_USE_PAGE
        );
        assert!(setup[0].szRelID.is_null());

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|known| *known == id).expect("record");
        assert!(position(BrtPrintOptions::default().id()) < position(BrtMargins::default().id()));
        assert!(position(BrtMargins::default().id()) < position(BrtPageSetup::default().id()));
    }

    #[test]
    fn print_scale_and_fit_are_mutually_exclusive() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_print_scale(50);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(
            (setup[0].iScale, setup[0].iFitWidth, setup[0].iFitHeight),
            (50, 0, 0)
        );

        sheet.set_print_fit_to_pages(2, 3);
        out.clear();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(
            (setup[0].iScale, setup[0].iFitWidth, setup[0].iFitHeight),
            (0, 2, 3)
        );

        sheet.set_print_scale(75);
        out.clear();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(
            (setup[0].iScale, setup[0].iFitWidth, setup[0].iFitHeight),
            (75, 0, 0)
        );
    }

    #[test]
    fn invalid_print_values_are_ignored() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_print_scale(9);
        sheet.set_print_fit_to_pages(32_768, 2);
        sheet.set_print_first_page_number(32_768);
        sheet.set_margins(-1.0, 50.0, 1.0, f64::NAN, 0.4, 0.5);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        assert_eq!(records::<BrtPageSetup>(&out).len(), 0);
        let margins = records::<BrtMargins>(&out);
        assert_eq!(margins.len(), 1);
        assert_eq!(*margins[0].xnumLeft, 0.7);
        assert_eq!(*margins[0].xnumRight, 0.7);
        assert_eq!(*margins[0].xnumTop, 1.0);
        assert_eq!(*margins[0].xnumHeader, 0.4);
        assert_eq!(*margins[0].xnumFooter, 0.5);
    }

    #[test]
    fn print_black_white_and_draft_emit_page_setup() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_print_black_and_white(true);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        assert_eq!(records::<BrtPrintOptions>(&out).len(), 0);
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(setup.len(), 1);
        assert_eq!(setup[0].grbit, BrtPageSetup::F_NO_COLOR);

        sheet.set_print_draft(true);
        out.clear();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(
            setup[0].grbit,
            BrtPageSetup::F_NO_COLOR | BrtPageSetup::F_DRAFT
        );
    }

    #[test]
    fn fit_to_pages_sets_wsp_prop_flag() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_print_fit_to_pages(1, 2);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let props = records::<BrtWsProp>(&out);
        assert_eq!(props.len(), 1);
        assert!(props[0].fFitToPage);
        let setup = records::<BrtPageSetup>(&out);
        assert_eq!(
            (setup[0].iScale, setup[0].iFitWidth, setup[0].iFitHeight),
            (0, 1, 2)
        );
    }

    #[test]
    fn restoring_defaults_removes_print_records() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.set_landscape();
        sheet.set_portrait();
        sheet.set_margins(0.7, 0.7, 0.75, 0.75, 0.3, 0.3);
        sheet.set_print_gridlines(true);
        sheet.set_print_gridlines(false);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        assert_eq!(records::<BrtPrintOptions>(&out).len(), 0);
        assert_eq!(records::<BrtMargins>(&out).len(), 0);
        assert_eq!(records::<BrtPageSetup>(&out).len(), 0);
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

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

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

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

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

        write_sheet(&sheet, &[index], &sst, false, &mut out).expect("sheet");

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

        write_sheet(&sheet, &indexes, &sst, false, &mut out).expect("sheet");

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

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

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

    #[test]
    fn hidden_rows_and_columns_emit_flags() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.write(0, 0, 1_i32).expect("writes");
        sheet.write(0, 1, 2_i32).expect("writes");
        sheet.set_row_hidden(0).expect("hide row");
        sheet.set_column_width(0, 10.0).expect("width");
        sheet.set_column_width(1, 10.0).expect("width");
        sheet.set_column_hidden(1).expect("hide column");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert!(rows[0].fDyZero);
        assert_eq!(rows[0].miyRw, 0);
        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 2);
        assert!(!columns[0].fHidden);
        assert!(columns[1].fHidden);
        assert_eq!(columns[1].coldx, 0);
    }

    #[test]
    fn unhidden_row_restores_default_height() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet.write(0, 0, 1_i32).expect("writes");
        sheet.set_row_hidden(0).expect("hide row");
        sheet.set_row_unhidden(0).expect("unhide row");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert!(!rows[0].fDyZero);
        assert_eq!(rows[0].miyRw, 300);
    }

    #[test]
    fn merge_range_emits_merge_records() {
        let mut sheet = Worksheet::new("Sheet1".to_owned());
        sheet
            .merge_range(0, 0, 1, 2, "Merged", &Format::new())
            .expect("merge");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let begins = records::<BrtBeginMergeCells>(&out);
        assert_eq!(begins.len(), 1);
        assert_eq!(begins[0].count, 1);
        let cells = records::<BrtMergeCell>(&out);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].rfx.rwFirst, 0);
        assert_eq!(cells[0].rfx.rwLast, 1);
        assert_eq!(cells[0].rfx.colFirst, 0);
        assert_eq!(cells[0].rfx.colLast, 2);
    }
}
