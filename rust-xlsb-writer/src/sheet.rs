//! Worksheet part serialization.

use std::collections::{BTreeSet, HashMap};

use rust_xlsb_core::prelude::{
    BrtBeginAFilter, BrtBeginCellIgnoreECs, BrtBeginColBrk, BrtBeginColInfos,
    BrtBeginCustomFilters, BrtBeginDVals, BrtBeginFilterColumn, BrtBeginFilters,
    BrtBeginHeaderFooter, BrtBeginListParts, BrtBeginMergeCells, BrtBeginRwBrk, BrtBeginSheet,
    BrtBeginSheetData, BrtBeginWsView, BrtBeginWsViews, BrtBrk, BrtCellBlank, BrtCellBool,
    BrtCellError, BrtCellIgnoreEC, BrtCellIsst, BrtCellRString, BrtCellReal, BrtCellRk, BrtColInfo,
    BrtColSpan, BrtCustomFilter, BrtEndAFilter, BrtEndCFRule, BrtEndCellIgnoreECs, BrtEndColBrk,
    BrtEndColInfos, BrtEndConditionalFormatting, BrtEndCustomFilters, BrtEndDVals,
    BrtEndFilterColumn, BrtEndFilters, BrtEndHeaderFooter, BrtEndListParts, BrtEndMergeCells,
    BrtEndRwBrk, BrtEndSheet, BrtEndSheetData, BrtEndWsView, BrtEndWsViews, BrtFilter, BrtFont,
    BrtHLink, BrtListPart, BrtMargins, BrtMergeCell, BrtPageSetup, BrtPane, BrtPrintOptions,
    BrtRowHdr, BrtSel, BrtWsDim, BrtWsFmtInfo, BrtWsProp, Cell, HeaderFooterString, RichStr,
    StrRun, StrRuns, UncheckedRfX, UncheckedSqRfX, XlNullableWideString, XlStringExt, XlWideString,
    Xnum, conditional_format_block,
};
use rust_xlsb_core::{
    BiffWrite, CellValue, NumberEncoding, SSTHolder, ToCellValue, WbContext, push_record,
};

use crate::data::CellData;
use crate::filter::{FilterCriteria, FilterData};
use crate::worksheet::{ColumnOptions, IgnoreError, MAX_COL, MAX_ROW, Worksheet};
use crate::{Error, Result};

const DEFAULT_COL_WIDTH: u32 = 2_158;
const DEFAULT_ROW_HEIGHT: u16 = 300;

struct SaveContext<'a> {
    sst: &'a SSTHolder,
    rich_fonts: &'a HashMap<BrtFont, u16>,
}

impl WbContext for SaveContext<'_> {
    fn shared_strings(&self) -> &SSTHolder {
        self.sst
    }
}

#[cfg(test)]
pub fn write_sheet(
    sheet: &Worksheet,
    style_indexes: &[u32],
    sst: &SSTHolder,
    selected: bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    write_sheet_with_fonts(
        sheet,
        style_indexes,
        sst,
        &HashMap::new(),
        &[],
        selected,
        out,
    )
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "package assembly uses this instead of the test-only wrapper"
)]
pub(crate) fn write_sheet_with_fonts(
    sheet: &Worksheet,
    style_indexes: &[u32],
    sst: &SSTHolder,
    rich_fonts: &HashMap<BrtFont, u16>,
    conditional_dxf_ids: &[Option<u32>],
    selected: bool,
    out: &mut Vec<u8>,
) -> Result<()> {
    let ctx = SaveContext { sst, rich_fonts };

    push(out, &BrtBeginSheet::default())?;
    write_ws_prop(sheet, out)?;
    push(out, &dimension(sheet))?;
    write_sheet_views(sheet, selected, out)?;
    write_sheet_format(sheet, out)?;

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
    rows.extend(sheet.filter_hidden_rows().iter().copied());

    for row in rows {
        let row_options = sheet.row_options().get(&row);
        let filter_hidden = sheet.filter_hidden_rows().contains(&row);
        let row_style = row_options
            .and_then(|options| options.format)
            .map_or(0, |index| mapped_style(index, style_indexes));
        let default_row_height = sheet
            .default_row_height()
            .map_or(DEFAULT_ROW_HEIGHT, height_to_twips);
        let row_height = if filter_hidden || row_options.is_some_and(|options| options.hidden) {
            0
        } else {
            row_options
                .and_then(|options| options.height)
                .map_or(default_row_height, height_to_twips)
        };
        let cells = sheet.cells().get(&row);
        let spans = cells.map_or_else(Vec::new, |cells| column_spans(cells.keys().copied()));

        push(
            out,
            &BrtRowHdr {
                rw: row,
                ixfe: row_style,
                miyRw: row_height,
                fDyZero: filter_hidden || row_options.is_some_and(|options| options.hidden),
                fGhostDirty: row_options.and_then(|options| options.format).is_some(),
                iOutLevel: row_options.map_or(0, |options| options.level),
                fCollapsed: row_options.is_some_and(|options| options.collapsed),
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
    if let Some(protection) = sheet.protection() {
        push(out, &protection.record())?;
    }
    write_unprotected_ranges(sheet, out)?;
    if let Some(rfx) = sheet.autofilter_range() {
        push(out, &BrtBeginAFilter { rfx })?;
        write_filter_columns(sheet, rfx, out)?;
        push(out, &BrtEndAFilter::default())?;
    }

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

    write_conditional_formats(sheet, conditional_dxf_ids, out)?;
    write_data_validations(sheet, out)?;
    write_hyperlinks(sheet, out)?;
    write_print_settings(sheet, out)?;
    write_header_footer(sheet, out)?;
    write_page_breaks(sheet, out)?;
    write_ignored_errors(sheet, out)?;
    write_list_parts(sheet, out)?;

    push(out, &BrtEndSheet::default())?;
    Ok(())
}

fn write_conditional_formats(
    sheet: &Worksheet,
    dxf_ids: &[Option<u32>],
    out: &mut Vec<u8>,
) -> Result<()> {
    let formats = sheet.conditional_formats();
    if formats.is_empty() {
        return Ok(());
    }
    if dxf_ids.len() != formats.len() {
        return Err(Error::ValueNotRepresentable(
            "conditional-format style index count does not match the worksheet",
        ));
    }

    for (index, conditional_format) in formats.iter().enumerate() {
        push(out, &conditional_format_block(conditional_format.range))?;
        let priority = i32::try_from(index + 1)
            .map_err(|_| Error::ValueNotRepresentable("too many conditional formats"))?;
        push(
            out,
            &conditional_format.begin_rule(priority, dxf_ids[index].unwrap_or(u32::MAX))?,
        )?;
        push(out, &BrtEndCFRule::default())?;
        push(out, &BrtEndConditionalFormatting::default())?;
    }
    Ok(())
}

fn write_list_parts(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let tables = sheet.tables();
    if tables.is_empty() {
        return Ok(());
    }
    push(
        out,
        &BrtBeginListParts {
            cParts: u32::try_from(tables.len())
                .map_err(|_| Error::ValueNotRepresentable("too many tables"))?,
        },
    )?;
    for index in 0..tables.len() {
        let rel = format!("rId{}", index + 1);
        push(
            out,
            &BrtListPart {
                stRelID: XlNullableWideString::construct(Some(rel.as_str())),
            },
        )?;
    }
    push(out, &BrtEndListParts::default())?;
    Ok(())
}

fn write_unprotected_ranges(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    for range in sheet.unprotected_ranges() {
        push(out, &range.record())?;
    }
    Ok(())
}

fn write_ignored_errors(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let ignored = sheet.ignored_errors();
    if ignored.is_empty() {
        return Ok(());
    }

    push(out, &BrtBeginCellIgnoreECs::default())?;
    for (&error_type, ranges) in ignored {
        for chunk in ranges.chunks(8_191) {
            push(
                out,
                &BrtCellIgnoreEC {
                    grbit: ignore_error_flag(error_type),
                    sqrfxu: UncheckedSqRfX {
                        crfx: u32::try_from(chunk.len()).map_err(|_| {
                            Error::ValueNotRepresentable("too many ignored-error ranges")
                        })?,
                        rgRefx: chunk.to_vec(),
                    },
                },
            )?;
        }
    }
    push(out, &BrtEndCellIgnoreECs::default())?;
    Ok(())
}

const fn ignore_error_flag(error_type: IgnoreError) -> u32 {
    match error_type {
        IgnoreError::FormulaError => BrtCellIgnoreEC::F_CALC_ERROR,
        IgnoreError::FormulaRefersToEmptyCells => BrtCellIgnoreEC::F_EMPTY_CELL_REF,
        IgnoreError::NumberStoredAsText => BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT,
        IgnoreError::FormulaOmitsCells => BrtCellIgnoreEC::F_INCONSIST_RANGE,
        IgnoreError::FormulaDiffers => BrtCellIgnoreEC::F_INCONSIST_FMLA,
        IgnoreError::TwoDigitTextYear => BrtCellIgnoreEC::F_TEXT_DATE_INSUFF,
        IgnoreError::UnlockedCellsWithFormula => BrtCellIgnoreEC::F_UNPROT_FMLA,
        IgnoreError::DataValidationError => BrtCellIgnoreEC::F_DATA_VALIDATION,
        IgnoreError::InconsistentColumnFormula => BrtCellIgnoreEC::F_CALC_COL,
    }
}

fn write_ws_prop(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let fit_to_page = sheet.page_setup().fit_to_pages.is_some();
    let (row_level, col_level) = sheet.outline_levels();
    let (symbols_above, symbols_left) = sheet.outline_symbols();
    let color = sheet.tab_color();

    if color.is_none()
        && !fit_to_page
        && row_level == 0
        && col_level == 0
        && !symbols_above
        && !symbols_left
    {
        return Ok(());
    }

    let mut prop = BrtWsProp {
        fShowAutoBreaks: true,
        fPublish: true,
        fRowSumsBelow: !symbols_above,
        fColSumsRight: !symbols_left,
        fFitToPage: fit_to_page,
        fShowOutlineSymbols: true,
        fCondFmtCalc: true,
        rwSync: u32::MAX,
        colSync: u32::MAX,
        ..BrtWsProp::default()
    };
    if let Some(color) = color {
        prop.brtcolorTab = color.to_brt_color()?;
    }
    push(out, &prop)
}

fn write_sheet_format(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let height = sheet.default_row_height();
    let col_width = sheet.default_col_width();
    let (row_level, col_level) = sheet.outline_levels();
    let hide_unused_rows = sheet.unused_rows_hidden();
    if height.is_none()
        && col_width.is_none()
        && row_level == 0
        && col_level == 0
        && !hide_unused_rows
    {
        return Ok(());
    }

    push(
        out,
        &BrtWsFmtInfo {
            dxGCol: col_width.map_or(DEFAULT_COL_WIDTH, width_to_units),
            miyDefRwHeight: height.map_or(DEFAULT_ROW_HEIGHT, height_to_twips),
            fUnsynced: height.is_some(),
            fDyZero: hide_unused_rows,
            iOutLevelRw: row_level,
            iOutLevelCol: col_level,
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
            fDspRuler: view.view_mode.shows_ruler(),
            rwTop: rw_top,
            colLeft: col_left,
            icvHdr: 0x40,
            wScale: view.zoom,
            wScaleNormal: 100,
            xlView: view.view_mode.xl_view(),
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

fn write_hyperlinks(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let mut rel_id = u32::try_from(sheet.tables().len())
        .map_err(|_| Error::ValueNotRepresentable("too many tables"))?;
    for (&(row, col), link) in sheet.hyperlinks() {
        // None marks an internal link; its serialized relId is the empty string.
        let rel = if link.is_internal() {
            None
        } else {
            rel_id = rel_id
                .checked_add(1)
                .ok_or(Error::ValueNotRepresentable("too many hyperlinks"))?;
            Some(format!("rId{rel_id}"))
        };
        let display = if link.is_internal() {
            link.text.as_str()
        } else {
            ""
        };
        push(
            out,
            &BrtHLink {
                rfx: UncheckedRfX {
                    rwFirst: row,
                    rwLast: row,
                    colFirst: u32::from(col),
                    colLast: u32::from(col),
                },
                relId: XlNullableWideString::construct(Some(rel.as_deref().unwrap_or_default())),
                location: XlWideString::construct(Some(link.location.as_str())),
                tooltip: XlWideString::construct(Some(link.tooltip.as_str())),
                display: XlWideString::construct(Some(display)),
            },
        )?;
    }
    Ok(())
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
    for table in sheet.tables() {
        min_row = min_row.min(table.range.rwFirst);
        max_row = max_row.max(table.range.rwLast);
        min_col = min_col.min(table.range.colFirst);
        max_col = max_col.max(table.range.colLast);
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
    let default_width = sheet
        .default_col_width()
        .map_or(DEFAULT_COL_WIDTH, width_to_units);
    let mut columns = sheet.column_options().iter().peekable();

    while let Some((&first, first_options)) = columns.next() {
        let mut last = first;
        while let Some((next, next_options)) = columns.peek() {
            if **next == last.saturating_add(1)
                && next_options.width == first_options.width
                && next_options.format == first_options.format
                && next_options.hidden == first_options.hidden
                && next_options.level == first_options.level
                && next_options.collapsed == first_options.collapsed
                && next_options.best_fit == first_options.best_fit
            {
                last = **next;
                columns.next();
            } else {
                break;
            }
        }
        infos.push(col_info(
            first,
            last,
            first_options,
            style_indexes,
            default_width,
        ));
    }

    infos
}

fn col_info(
    first: u16,
    last: u16,
    options: &ColumnOptions,
    style_indexes: &[u32],
    default_width: u32,
) -> BrtColInfo {
    BrtColInfo {
        colFirst: u32::from(first),
        colLast: u32::from(last),
        coldx: if options.hidden {
            0
        } else {
            options.width.map_or(default_width, width_to_units)
        },
        ixfe: options
            .format
            .map_or(0, |index| mapped_style(index, style_indexes)),
        fHidden: options.hidden,
        fUserSet: options.width.is_some() || options.format.is_some() || options.hidden,
        fBestFit: options.best_fit,
        iOutLevel: options.level,
        fCollapsed: options.collapsed,
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

fn write_data_validations(sheet: &Worksheet, out: &mut Vec<u8>) -> Result<()> {
    let validations = sheet.data_validations();
    if validations.is_empty() {
        return Ok(());
    }

    push(
        out,
        &BrtBeginDVals::new(
            u32::try_from(validations.len())
                .map_err(|_| Error::ValueNotRepresentable("too many data validations"))?,
        ),
    )?;
    for item in validations {
        push(out, &item.validation.record(item.range)?)?;
    }
    push(out, &BrtEndDVals::default())?;
    Ok(())
}

fn write_filter_columns(sheet: &Worksheet, range: UncheckedRfX, out: &mut Vec<u8>) -> Result<()> {
    for (&col, condition) in sheet.filter_conditions() {
        push(
            out,
            &BrtBeginFilterColumn {
                dwCol: u32::from(col) - range.colFirst,
                fHideArrow: false,
                fNoBtn: false,
            },
        )?;

        if condition.is_list_filter {
            push(
                out,
                &BrtBeginFilters {
                    fBlank: u32::from(condition.should_match_blanks),
                    unused: 0,
                },
            )?;
            for data in &condition.list {
                push(
                    out,
                    &BrtFilter {
                        rgch: XlWideString::construct(Some(data.string.as_str())),
                    },
                )?;
            }
            push(out, &BrtEndFilters::default())?;
        } else {
            push(
                out,
                &BrtBeginCustomFilters {
                    fAnd: u32::from(condition.apply_logical_or && condition.custom2.is_some()),
                },
            )?;
            if let Some(data) = &condition.custom1 {
                push(out, &custom_filter(data))?;
            }
            if let Some(data) = &condition.custom2 {
                push(out, &custom_filter(data))?;
            }
            push(out, &BrtEndCustomFilters::default())?;
        }

        push(out, &BrtEndFilterColumn::default())?;
    }

    Ok(())
}

fn custom_filter(data: &FilterData) -> BrtCustomFilter {
    let grbit_sgn = data.criteria.operator();
    if data.is_number() {
        return BrtCustomFilter {
            vts: 0x04,
            grbitSgn: grbit_sgn,
            union: data.number.to_le_bytes(),
            vtsStringXls: None,
        };
    }

    if data.string == " " && data.criteria == FilterCriteria::NotEqualTo {
        return BrtCustomFilter {
            vts: 0x0E,
            grbitSgn: grbit_sgn,
            union: [0; 8],
            vtsStringXls: None,
        };
    }

    BrtCustomFilter {
        vts: 0x06,
        grbitSgn: grbit_sgn,
        union: [0; 8],
        vtsStringXls: Some(XlWideString::construct(Some(data.value()))),
    }
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
        CellData::ExcelDateTime(value) => push_number(
            out,
            cell,
            &CellValue::Number(ctx.store_f64(value.to_excel())),
        ),
        CellData::Duration(value) => push_number(out, cell, &value.to_cell_value(ctx)?),
        CellData::RichString(rich) => {
            let runs = rich
                .runs
                .iter()
                .map(|run| {
                    Ok(StrRun {
                        ich: run.ich,
                        ifnt: *ctx.rich_fonts.get(&run.font).ok_or(
                            Error::ValueNotRepresentable(
                                "rich string font is missing from the style table",
                            ),
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            push(
                out,
                &BrtCellRString {
                    cell,
                    value: RichStr {
                        flags: RichStr::F_RICH_STR,
                        str: XlWideString::construct(Some(rich.text.as_str())),
                        rich: Some(StrRuns {
                            dwSizeStrRun: u32::try_from(runs.len()).map_err(|_| {
                                Error::ValueNotRepresentable("too many rich string runs")
                            })?,
                            rgsStrRun: runs,
                        }),
                        phonetic: None,
                    },
                },
            )
        }
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
        BrtMergeCell, BrtPageSetup, BrtPane, BrtPrintOptions, BrtRangeProtection, BrtRowHdr,
        BrtSel, BrtSheetProtection, BrtWsFmtInfo, BrtWsProp,
    };
    use rust_xlsb_core::{BiffHead, CheckedBiff, KnownID, SSTHolder, StyleRegistry};

    use super::*;
    use crate::Format;
    use crate::ProtectionOptions;
    use crate::worksheet::{MAX_COL, pixels_to_width};

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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn view_modes_emit_xlview_and_ruler() {
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());

        sheet.set_view_page_layout();
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].xlView, 2);
        assert!(views[0].fDspRuler);

        sheet.set_view_page_break_preview();
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let views = records::<BrtBeginWsView>(&out);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].xlView, 1);
        assert!(!views[0].fDspRuler);

        sheet.set_view_normal();
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        assert_eq!(records::<BrtBeginWsView>(&out).len(), 0);
    }

    #[test]
    fn hide_unused_rows_sets_zero_height() {
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.hide_unused_rows(true);
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert!(formats[0].fDyZero);
        assert!(!formats[0].fUnsynced);

        sheet.set_default_row_height(30.0).expect("height");
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert!(formats[0].fDyZero);
        assert!(formats[0].fUnsynced);
        assert_eq!(formats[0].miyDefRwHeight, 600);

        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.hide_unused_rows(true);
        sheet.hide_unused_rows(false);
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        assert_eq!(records::<BrtWsFmtInfo>(&out).len(), 0);
    }

    #[test]
    fn autofit_width_sets_best_fit_and_splits_spans() {
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.set_column_autofit_width(0, 100).expect("autofit");
        sheet.set_column_autofit_width(1, 100).expect("autofit");
        sheet
            .set_column_width(2, pixels_to_width(100))
            .expect("manual");

        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 2);
        assert_eq!((columns[0].colFirst, columns[0].colLast), (0, 1));
        assert!(columns[0].fBestFit);
        assert!(columns[0].fUserSet);
        assert_eq!(columns[0].coldx, width_to_units(pixels_to_width(100)));
        assert_eq!((columns[1].colFirst, columns[1].colLast), (2, 2));
        assert!(!columns[1].fBestFit);

        sheet.set_column_width(0, 20.0).expect("manual override");
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let columns = records::<BrtColInfo>(&out);
        assert!(!columns[0].fBestFit);

        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.set_column_autofit_width(0, 0).expect("zero hides");
        let mut out = Vec::new();
        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
        let columns = records::<BrtColInfo>(&out);
        assert!(columns[0].fHidden);
        assert_eq!(columns[0].coldx, 0);
        assert!(!columns[0].fBestFit);
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn protection_options_map_to_permission_flags() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        let options = ProtectionOptions {
            select_locked_cells: false,
            select_unlocked_cells: false,
            format_cells: true,
            format_columns: true,
            format_rows: true,
            insert_columns: true,
            insert_rows: true,
            insert_links: true,
            delete_columns: true,
            delete_rows: true,
            sort: true,
            use_autofilter: true,
            use_pivot_tables: true,
            edit_scenarios: true,
            edit_objects: true,
            contents: false,
        };
        sheet.protect_with_password("password");
        sheet.protect_with_options(&options);
        sheet.protect();
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let protections = records::<BrtSheetProtection>(&out);
        assert_eq!(protections.len(), 1);
        let record = &protections[0];
        assert_eq!(record.protpwd, 0x83AF);
        assert_eq!(record.fLocked, 1);
        assert_eq!(record.fObjects, 1);
        assert_eq!(record.fScenarios, 1);
        assert_eq!(record.fFormatCells, 1);
        assert_eq!(record.fFormatColumns, 1);
        assert_eq!(record.fFormatRows, 1);
        assert_eq!(record.fInsertColumns, 1);
        assert_eq!(record.fInsertRows, 1);
        assert_eq!(record.fInsertHyperlinks, 1);
        assert_eq!(record.fDeleteColumns, 1);
        assert_eq!(record.fDeleteRows, 1);
        assert_eq!(record.fSelLockedCells, 0);
        assert_eq!(record.fSort, 1);
        assert_eq!(record.fAutoFilter, 1);
        assert_eq!(record.fPivotTables, 1);
        assert_eq!(record.fSelUnlockedCells, 0);
    }

    #[test]
    fn protection_follows_sheet_data_and_precedes_merged_cells() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(0, 0, "x").expect("writes");
        sheet.protect();
        sheet
            .merge_range(1, 0, 1, 1, "merged", &Format::new())
            .expect("merge");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|candidate| *candidate == id);
        let data = position(KnownID::BrtEndSheetData).expect("sheet data");
        let protection = position(KnownID::BrtSheetProtection).expect("protection");
        let merged = position(KnownID::BrtBeginMergeCells).expect("merged cells");
        assert!(data < protection);
        assert!(protection < merged);
    }

    #[test]
    fn autofilter_follows_protection_and_precedes_merges() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(0, 0, "x").expect("writes");
        sheet.autofilter(0, 0, 3, 1).expect("autofilter");
        sheet.protect();
        sheet
            .merge_range(1, 0, 1, 1, "merged", &Format::new())
            .expect("merge");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|candidate| *candidate == id);
        let data = position(KnownID::BrtEndSheetData).expect("sheet data");
        let protection = position(KnownID::BrtSheetProtection).expect("protection");
        let filter = position(KnownID::BrtBeginAFilter).expect("autofilter");
        let end_filter = position(KnownID::BrtEndAFilter).expect("autofilter end");
        let merged = position(KnownID::BrtBeginMergeCells).expect("merged cells");
        assert!(data < protection);
        assert!(protection < filter);
        assert!(filter < end_filter);
        assert!(end_filter < merged);
    }

    #[test]
    fn default_sheet_view_emits_no_view_records() {
        let sheet = Worksheet::new_named("Sheet1".to_owned());
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
        assert_eq!(records::<BrtSheetProtection>(&out).len(), 0);
        assert_eq!(records::<BrtRangeProtection>(&out).len(), 0);
        assert_eq!(records::<BrtBeginCellIgnoreECs>(&out).len(), 0);
        assert_eq!(records::<BrtBeginAFilter>(&out).len(), 0);
    }

    #[test]
    fn header_footer_serializes_text_and_flags() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        assert!(columns[0].fUserSet);
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn hidden_and_formatted_columns_keep_the_user_set_flag() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.set_column_hidden(0).expect("hide");
        let format = Format::new().set_bold();
        sheet.set_column_format(1, &format).expect("format");
        let mut registry = StyleRegistry::new();
        let style = format.to_cell_style(&mut registry).expect("style");
        let index = registry.add(style).expect("adds");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[index], &sst, false, &mut out).expect("sheet");

        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 2);
        assert!(columns[0].fUserSet);
        assert_eq!(columns[0].coldx, 0);
        assert!(columns[1].fUserSet);
        assert_eq!(columns[1].coldx, DEFAULT_COL_WIDTH);
    }

    #[test]
    fn unhidden_row_restores_default_height() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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
    fn grouped_rows_set_levels_and_worksheet_maximum() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(0, 0, "head").expect("head");
        sheet.write(1, 0, 1_i32).expect("row 1");
        sheet.write(2, 0, 2_i32).expect("row 2");
        sheet.write(3, 0, 3_i32).expect("row 3");
        sheet.group_rows(1, 3).expect("outer group");
        sheet.group_rows(2, 2).expect("inner group");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        let levels: Vec<u8> = rows.iter().map(|row| row.iOutLevel).collect();
        assert_eq!(levels, [0, 1, 2, 1]);

        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].iOutLevelRw, 2);
        assert_eq!(formats[0].iOutLevelCol, 0);
        assert!(!formats[0].fUnsynced);
        assert_eq!(formats[0].miyDefRwHeight, 300);
        let props = records::<BrtWsProp>(&out);
        assert_eq!(props.len(), 1);
        assert!(props[0].fRowSumsBelow);
        assert!(props[0].fColSumsRight);
    }

    #[test]
    fn outline_levels_share_sheet_format_with_default_height() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.set_default_row_height(24.0).expect("height");
        sheet.group_columns_collapsed(2, 3).expect("group");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert!(formats[0].fUnsynced);
        assert_eq!(formats[0].miyDefRwHeight, 480);
        assert_eq!(formats[0].iOutLevelCol, 1);
    }

    #[test]
    fn collapsed_rows_hide_and_mark_the_following_row() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(1, 0, 1_i32).expect("row 1");
        sheet.write(2, 0, 2_i32).expect("row 2");
        sheet.group_rows_collapsed(1, 2).expect("group");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert_eq!(rows.len(), 3);
        for row in &rows[..2] {
            assert_eq!(row.iOutLevel, 1);
            assert!(row.fDyZero);
            assert_eq!(row.miyRw, 0);
            assert!(!row.fCollapsed);
        }
        assert_eq!(rows[2].rw, 3);
        assert_eq!(rows[2].iOutLevel, 0);
        assert!(rows[2].fCollapsed);
        assert!(!rows[2].fDyZero);
    }

    #[test]
    fn grouped_columns_split_by_level_and_collapse() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.group_columns(0, 2).expect("outer group");
        sheet.group_columns(1, 1).expect("inner group");
        sheet
            .group_columns_collapsed(4, 5)
            .expect("collapsed group");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 5);
        assert_eq!((columns[0].colFirst, columns[0].colLast), (0, 0));
        assert_eq!(columns[0].iOutLevel, 1);
        assert_eq!((columns[1].colFirst, columns[1].colLast), (1, 1));
        assert_eq!(columns[1].iOutLevel, 2);
        assert_eq!((columns[2].colFirst, columns[2].colLast), (2, 2));
        assert_eq!(columns[2].iOutLevel, 1);
        assert_eq!((columns[3].colFirst, columns[3].colLast), (4, 5));
        assert_eq!(columns[3].iOutLevel, 1);
        assert!(columns[3].fUserSet);
        assert!(columns[3].fHidden);
        assert_eq!(columns[3].coldx, 0);
        assert_eq!((columns[4].colFirst, columns[4].colLast), (6, 6));
        assert!(columns[4].fCollapsed);
        assert_eq!(columns[4].iOutLevel, 0);

        let formats = records::<BrtWsFmtInfo>(&out);
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].iOutLevelRw, 0);
        assert_eq!(formats[0].iOutLevelCol, 2);
    }

    #[test]
    fn grouped_columns_at_the_same_level_coalesce() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.group_columns(0, 3).expect("group");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 1);
        assert_eq!((columns[0].colFirst, columns[0].colLast), (0, 3));
        assert_eq!(columns[0].iOutLevel, 1);
        assert!(!columns[0].fUserSet);
    }

    #[test]
    fn grouping_rejects_depth_order_and_limits_without_partial_state() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.group_rows(0, 1).expect("level 1");
        for _ in 0..6 {
            sheet.group_rows(0, 1).expect("nested");
        }
        assert!(sheet.group_rows(0, 0).is_err(), "row already at level 7");

        assert!(sheet.group_rows(2, 1).is_err(), "reversed rows");
        assert!(sheet.group_rows(0, MAX_ROW + 1).is_err(), "row limit");
        assert!(sheet.group_columns(2, 1).is_err(), "reversed columns");
        assert!(sheet.group_columns(0, MAX_COL + 1).is_err(), "column limit");

        let rows = records::<BrtRowHdr>(&{
            let sst = SSTHolder::create_in_memory().expect("sst");
            let mut out = Vec::new();
            write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");
            out
        });
        assert!(rows.iter().all(|row| row.iOutLevel <= 7));
    }

    #[test]
    fn collapsed_groups_at_the_sheet_edge_skip_the_marker() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet
            .group_rows_collapsed(MAX_ROW, MAX_ROW)
            .expect("last row");
        sheet
            .group_columns_collapsed(MAX_COL, MAX_COL)
            .expect("last column");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let rows = records::<BrtRowHdr>(&out);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rw, MAX_ROW);
        assert!(rows[0].fDyZero);
        assert!(!rows[0].fCollapsed);
        let columns = records::<BrtColInfo>(&out);
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].colFirst, u32::from(MAX_COL));
        assert!(columns[0].fHidden);
        assert!(!columns[0].fCollapsed);
    }

    #[test]
    fn outline_symbols_set_ws_prop_summary_flags() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.group_rows(1, 2).expect("rows");
        sheet.group_columns(1, 2).expect("columns");
        sheet.group_symbols_above(true);
        sheet.group_symbols_to_left(true);
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let props = records::<BrtWsProp>(&out);
        assert_eq!(props.len(), 1);
        assert!(!props[0].fRowSumsBelow);
        assert!(!props[0].fColSumsRight);
        assert!(props[0].fShowOutlineSymbols);
    }

    #[test]
    fn grouping_keeps_default_workbook_output_unchanged() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(0, 0, 1_i32).expect("writes");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        assert_eq!(records::<BrtWsProp>(&out).len(), 0);
        assert_eq!(records::<BrtWsFmtInfo>(&out).len(), 0);
    }

    #[test]
    fn merge_range_emits_merge_records() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
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

    #[test]
    fn protected_ranges_follow_sheet_protection_and_precede_merges() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.write(0, 0, "x").expect("writes");
        sheet.protect();
        sheet
            .unprotect_range_with_options(1, 1, 3, 2, "Editable", "abc123")
            .expect("unprotect");
        sheet.unprotect_range(5, 0, 5, 1).expect("default range");
        sheet
            .merge_range(7, 0, 7, 1, "merged", &Format::new())
            .expect("merge");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let ranges = records::<BrtRangeProtection>(&out);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].protPwd, 0xC58F);
        assert_eq!(ranges[0].sqRfX.crfx, 1);
        assert_eq!(ranges[0].sqRfX.rgRefx[0].rwFirst, 1);
        assert_eq!(ranges[0].sqRfX.rgRefx[0].rwLast, 3);
        assert_eq!(ranges[0].sqRfX.rgRefx[0].colFirst, 1);
        assert_eq!(ranges[0].sqRfX.rgRefx[0].colLast, 2);
        assert_eq!(
            ranges[0]
                .RangeProtectionTitleSDRel
                .rgchTitle
                .try_to_string()
                .expect("title"),
            "Editable"
        );
        assert_eq!(ranges[0].RangeProtectionTitleSDRel.dwsdRel, 0);
        assert_eq!(
            ranges[1]
                .RangeProtectionTitleSDRel
                .rgchTitle
                .try_to_string()
                .expect("title"),
            "Range2"
        );

        let ids = record_ids(&out);
        let position = |id| ids.iter().position(|candidate| *candidate == id);
        let protection = position(KnownID::BrtSheetProtection).expect("protection");
        let first = position(KnownID::BrtRangeProtection).expect("range protection");
        let merged = position(KnownID::BrtBeginMergeCells).expect("merged cells");
        assert!(protection < first);
        assert!(first < merged);
    }

    #[test]
    fn ignored_errors_group_by_type_after_page_breaks() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        sheet.set_header("report");
        sheet.set_page_breaks(&[20, 40]).expect("breaks");
        sheet
            .ignore_error_range(0, 0, 1, 1, IgnoreError::NumberStoredAsText)
            .expect("first");
        sheet
            .ignore_error(3, 3, IgnoreError::NumberStoredAsText)
            .expect("second");
        sheet
            .ignore_error(5, 0, IgnoreError::FormulaError)
            .expect("formula");
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let ids = record_ids(&out);
        assert_eq!(
            ids.iter()
                .filter(|id| **id == KnownID::BrtBeginCellIgnoreECs)
                .count(),
            1
        );
        assert_eq!(
            ids.iter()
                .filter(|id| **id == KnownID::BrtEndCellIgnoreECs)
                .count(),
            1
        );
        let errors = records::<BrtCellIgnoreEC>(&out);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].grbit, BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT);
        assert_eq!(errors[0].sqrfxu.crfx, 2);
        assert_eq!(errors[0].sqrfxu.rgRefx[0].rwFirst, 0);
        assert_eq!(errors[0].sqrfxu.rgRefx[0].colLast, 1);
        assert_eq!(errors[0].sqrfxu.rgRefx[1].rwFirst, 3);
        assert_eq!(errors[0].sqrfxu.rgRefx[1].colFirst, 3);
        assert_eq!(errors[1].grbit, BrtCellIgnoreEC::F_CALC_ERROR);
        assert_eq!(errors[1].sqrfxu.crfx, 1);

        let position = |id| ids.iter().position(|candidate| *candidate == id);
        let breaks = position(KnownID::BrtEndRwBrk).expect("breaks");
        let begin = position(KnownID::BrtBeginCellIgnoreECs).expect("ignore begin");
        let end = position(KnownID::BrtEndCellIgnoreECs).expect("ignore end");
        let sheet_end = position(KnownID::BrtEndSheet).expect("sheet end");
        assert!(breaks < begin);
        assert!(begin < end);
        assert!(end < sheet_end);
    }

    #[test]
    fn every_ignore_error_maps_to_its_spec_flag() {
        let mappings = [
            (IgnoreError::FormulaError, BrtCellIgnoreEC::F_CALC_ERROR),
            (
                IgnoreError::FormulaRefersToEmptyCells,
                BrtCellIgnoreEC::F_EMPTY_CELL_REF,
            ),
            (
                IgnoreError::NumberStoredAsText,
                BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT,
            ),
            (
                IgnoreError::FormulaOmitsCells,
                BrtCellIgnoreEC::F_INCONSIST_RANGE,
            ),
            (
                IgnoreError::FormulaDiffers,
                BrtCellIgnoreEC::F_INCONSIST_FMLA,
            ),
            (
                IgnoreError::TwoDigitTextYear,
                BrtCellIgnoreEC::F_TEXT_DATE_INSUFF,
            ),
            (
                IgnoreError::UnlockedCellsWithFormula,
                BrtCellIgnoreEC::F_UNPROT_FMLA,
            ),
            (
                IgnoreError::DataValidationError,
                BrtCellIgnoreEC::F_DATA_VALIDATION,
            ),
            (
                IgnoreError::InconsistentColumnFormula,
                BrtCellIgnoreEC::F_CALC_COL,
            ),
        ];

        for (error_type, flag) in mappings {
            assert_eq!(ignore_error_flag(error_type), flag);
        }
    }

    #[test]
    fn ignored_errors_chunk_at_the_8191_range_limit() {
        let mut sheet = Worksheet::new_named("Sheet1".to_owned());
        for row in 0..8_192 {
            sheet
                .ignore_error(row, 0, IgnoreError::NumberStoredAsText)
                .expect("ignore");
        }
        let sst = SSTHolder::create_in_memory().expect("sst");
        let mut out = Vec::new();

        write_sheet(&sheet, &[], &sst, false, &mut out).expect("sheet");

        let errors = records::<BrtCellIgnoreEC>(&out);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].sqrfxu.crfx, 8_191);
        assert_eq!(errors[1].sqrfxu.crfx, 1);
        assert_eq!(errors[0].grbit, BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT);
        assert_eq!(errors[1].grbit, BrtCellIgnoreEC::F_NUM_STORED_AS_TEXT);
        assert_eq!(errors[1].sqrfxu.rgRefx[0].rwFirst, 8_191);
    }
}
