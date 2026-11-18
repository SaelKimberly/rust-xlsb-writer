# Worksheet outline groups

- Status: Implemented — reviewer finding fixed
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.3, §6 P2
- Baseline: `da4343c` (`feat: add worksheet headers footers and page breaks`)
- Target: `rust-xlsb-writer`

## 1. Goal

Add Excel outline groups for rows and columns:

- `group_rows(first, last)`
- `group_rows_collapsed(first, last)`
- `group_columns(first, last)`
- `group_columns_collapsed(first, last)`
- `group_symbols_above(enable)`
- `group_symbols_to_left(enable)`

A default workbook remains byte-identical: no `BrtWsProp` or `BrtWsFmtInfo`
appears unless an outline or default-height change requires it.

## 2. Scope And Acceptance

Accepted when:

- grouping a range increments every row/column outline level by one;
- an outline level above 7 is rejected with `Error::MaxGroupLevelExceeded`;
- a reversed range is rejected with `Error::RowColumnOrder`, and an
  out-of-range endpoint with `Error::RowColumnLimit`;
- collapsed groups hide their rows/columns and mark the summary row/column
  immediately after the range with `fCollapsed`; the marker is skipped when
  the range ends at the worksheet limit;
- row headers carry `iOutLevel`, `fCollapsed`, and `fDyZero`/`miyRw = 0` for
  collapsed rows;
- column infos carry `iOutLevel`, `fCollapsed`, and hidden width zero;
- adjacent columns merge only when width, format, hidden state, outline level,
  and collapsed state match;
- `BrtWsFmtInfo.iOutLevelRw` and `iOutLevelCol` carry the highest levels;
  the record is emitted when either level is nonzero, and combines cleanly
  with `set_default_row_height`;
- `group_symbols_above(true)` clears `BrtWsProp.fRowSumsBelow`;
  `group_symbols_to_left(true)` clears `fColSumsRight`;
- `BrtWsProp.fShowOutlineSymbols` is true when the record is emitted;
- calamine still reads cells from a grouped workbook;
- LibreOffice preserves outline levels and hidden state on XLSX round-trip,
  and collapsed groups render hidden; marker placement is verified by raw
  record bytes because LibreOffice drops the marker on export;
- existing tests and default output remain unchanged.

Out of scope: ungrouping, automatic outline styles, `hide_unused_rows`, and
print areas/repeat titles.

## 3. Change Necessity

The gap analysis lists row/column grouping and outline symbol placement as
missing. No existing setter changes row/column outline levels; the minimum
boundary is new row/column state plus existing `BrtRowHdr`, `BrtColInfo`,
`BrtWsFmtInfo`, and `BrtWsProp` fields.

## 4. Aegis Visibility

The public `Worksheet` API and observable BIFF output change while the record
catalogue stays unchanged. A durable plan records the level semantics,
collapsed-marker placement, worksheet flags, and default-compatibility
boundary.

## 5. Owners And Files

- `rust-xlsb-writer/src/worksheet.rs` — outline levels on `RowOptions` and
  `ColumnOptions`, worksheet maxima, symbol flags, grouping methods.
- `rust-xlsb-writer/src/error.rs` — `MaxGroupLevelExceeded`.
- `rust-xlsb-writer/src/sheet.rs` — row/column outline fields, collapsed
  markers, `BrtWsProp` symbol flags, `BrtWsFmtInfo` maxima.
- `rust-xlsb-writer/src/e2e/mod.rs` — calamine readback through grouped data.
- `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`.

## 6. Record Semantics

`BrtRowHdr` (2.4.770) and `BrtColInfo` (2.4.336) already expose `iOutLevel`
and `fCollapsed`; `BrtWsFmtInfo` (2.4.875) exposes `iOutLevelRw` and
`iOutLevelCol`; `BrtWsProp` (2.4.71) exposes `fRowSumsBelow`, `fColSumsRight`,
and `fShowOutlineSymbols`.

- Grouping `first..=last` adds one to each existing level.
- Collapsed groups set `fDyZero`/hidden and `miyRw`/`coldx` to zero.
- The row/column after a collapsed range gets `fCollapsed = true` and keeps
  its current level, matching Excel's summary marker.
- `BrtWsFmtInfo` is emitted when a default height or an outline level exists.
  `fUnsynced` is true only when the default height was set.
- `BrtWsProp` is emitted for a tab color, fit-to-page, outline levels, or a
  non-default summary direction.

## 7. Compatibility Boundary

- No grouping and no default-height change leaves the worksheet bytes
  unchanged.
- `fUserSet` on `BrtColInfo` stays `true` for an explicit width, format, or
  hidden state, so existing column output is preserved while outline-only
  columns do not claim a custom width.
- Existing hidden rows/columns and formats are preserved when grouping.
- The collapsed marker is skipped at the last row/column instead of wrapping
  past the worksheet limit.

## 8. TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API and observable outline records.
- Test posture: add failing writer tests for levels, collapsed markers,
  maxima, flags, validation, and default output; then implement.
- Verification: unit tests, calamine readback, LibreOffice round-trip and
  render, clippy, fmt.

## 9. Tasks

1. Add failing tests for grouped rows: levels, hidden collapsed rows, marker,
   `BrtWsFmtInfo` maxima, and depth/order/limit errors.
2. Add failing tests for grouped columns: coalescing by level, hidden width,
   marker, and `BrtWsFmtInfo` maxima.
3. Add failing tests for symbol flags and for default output with no outline
   records.
4. Add `level` and `collapsed` to `RowOptions` and `ColumnOptions`; add
   worksheet maxima and symbol flags.
5. Implement grouping validation and mutation; set collapsed markers.
6. Emit row/column outline fields and collapse markers.
7. Emit `BrtWsFmtInfo` when outline levels exist, combining default height.
8. Emit `BrtWsProp` symbol flags; preserve tab-color and fit-to-page paths.
9. Add calamine readback and LibreOffice round-trip evidence.
10. Update the gap matrix, AGENTS counts, index, and plan status.

## 10. Risks And Verification

- Risk: a collapsed group at `MAX_ROW`/`MAX_COL` has no summary cell; skip
  the marker rather than overflowing.
- Risk: `BrtWsFmtInfo` is shared by default height and outline levels; one
  record must carry both and `fUnsynced` must stay false for outline-only.
- Risk: grouping mutates row/column state; validate the whole range before
  applying so a depth error does not leave a partial group.
- Risk: LibreOffice drops the collapsed summary marker on XLSX export; raw
  record tests, not the round-trip, prove the marker bits.
- Verification: `cargo test --workspace`, clippy, fmt; decode generated
  records; calamine readback; LibreOffice XLSX round-trip attributes and
  collapsed render.

## 11. Execution Handoff

- Route: inline.
- User confirmation required: no.
- Status: implemented; reviewer found one plan-text contradiction, fixed; no code defects.
- Verification: 161 core + 104 writer unit tests and 6 doctests; clippy and
  fmt clean; calamine reads grouped sheets. Raw record tests pin the row,
  column, and worksheet summary flag bytes. LibreOffice XLSX round-trip
  preserves `outlineLevelRow`/`outlineLevelCol`, row/column levels, and hidden
  state; collapsed rows disappear from the rendered PDF. LibreOffice drops
  the collapsed summary marker on XLSX export, so marker placement is proven
  by raw `BrtRowHdr`/`BrtColInfo` bytes.
