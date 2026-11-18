# Worksheet tabs and default row height

- Status: Implemented — reviewer found no defects
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.1, §3.3
- Baseline: `9c81bbb` (`feat: add worksheet view options and pixel units`)
- Target: `rust-xlsb-core`, `rust-xlsb-writer`

## 1. Goal

Add the remaining low-risk worksheet tab state and default row height:

- `set_hidden(enable)`
- `set_very_hidden(enable)`
- `set_active(enable)`
- `set_selected(enable)`
- `set_first_tab(enable)`
- `set_tab_color(color)`
- `set_default_row_height(height)`
- `set_default_row_height_pixels(pixels)`

A default workbook stays byte-identical: no `BOOKVIEWS`, no `BrtWsProp`, no
`BrtWsFmtInfo`, and `BrtBundleSh.hsState` remains `0`.

## 2. Scope And Acceptance

Accepted when:

- hidden and very-hidden sheets serialize `hsState` `1` and `2`;
- the last `set_active(true)` sheet becomes `BrtBookView.itabCur`, and the last
  `set_first_tab(true)` visible sheet becomes `itabFirst`;
- active sheets are selected and visible; hidden sheets clear active/selected;
- `set_selected(true)` emits `BrtBeginWsView.fSelected`;
- tab colors emit `BrtWsProp.brtcolorTab` and invalid/default colors are
  ignored or clear the color without changing prior state;
- default row height emits `BrtWsFmtInfo.miyDefRwHeight` with `fUnsynced`;
- calamine still reads cells from a workbook using all new options;
- existing tests and default output remain unchanged.

Out of scope: unhiding via a visible API, tab color themes beyond the existing
`Color` type, default column width, hidden unused rows, print settings.

## 3. Change Necessity

The gap analysis lists these as missing public behavior. No configuration or
documentation change can emit the required BIFF records; two new core record
structs and writer state are the minimum boundary.

## 4. Aegis Visibility

The public `Worksheet` contract, workbook package assembly, and binary record
catalogue all change. A durable plan keeps the API, record layout, and default
compatibility verifiable in one place.

## 5. Owners And Files

- `rust-xlsb-core/src/prelude/record/brt_book_view.rs` — new `BrtBookView`.
- `rust-xlsb-core/src/prelude/record/brt_ws_fmt_info.rs` — new `BrtWsFmtInfo`.
- `rust-xlsb-core/src/prelude/record/mod.rs`, `prelude/mod.rs` — exports and
  round-trip cases.
- `rust-xlsb-writer/src/worksheet.rs` — visibility/active/selected/first-tab,
  tab-color, default-height state and setters.
- `rust-xlsb-writer/src/workbook.rs` — choose the effective active sheet.
- `rust-xlsb-writer/src/sheet.rs` — emit `BrtWsProp` and `BrtWsFmtInfo`.
- `rust-xlsb-writer/src/package.rs` — `hsState`, `BOOKVIEWS`, `itabFirst`,
  `itabCur`.
- `rust-xlsb-writer/src/e2e/mod.rs` — package readback coverage.
- `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`.

## 6. Compatibility Boundary

- Existing workbook output without tab options keeps the same parts and record
  sequence.
- `BrtBookView` defaults reproduce the MS-XLSB example flags; only
  `itabFirst`, `itabCur`, and visible-sheet state vary.
- `BrtWsFmtInfo` is emitted only when a default row height is set.
- `BrtWsProp` is emitted only when a tab color is set.
- The selected bit is additive to `SheetView`; existing view output for
  unselected sheets is unchanged.

## 7. TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API, workbook package behavior, and binary records.
- Test posture: add record round-trip/size tests and failing writer/package
  tests before implementation, then keep them as regression coverage.
- Verification: unit tests, calamine readback, clippy, fmt.

## 8. Tasks

1. Add `BrtBookView` (29 bytes, LSB flags) and `BrtWsFmtInfo` (12 bytes, LSB
   flags). Register exports and round-trip size cases.
2. Add worksheet tab state and setters. Follow `rust_xlsxwriter` mutual
   exclusion: activating selects and unhides; hiding clears active/selected;
   selecting unhides; first-tab marks a visible tab.
3. Add default row height state with the existing `0..=409.5` validation and
   pixel conversion.
4. Serialize tab color via `BrtWsProp` before `BrtWsDim`, and default height via
   `BrtWsFmtInfo` after `WSVIEWS2`.
5. In `Workbook::prepare`, choose the last active sheet, default to index 0, and
   make it active/selected. In package assembly, choose the last flagged
   visible first tab, then emit `BOOKVIEWS` only when `itabFirst` or `itabCur`
   differs from zero.
6. Emit `hsState` from worksheet visibility and preserve the active sheet.
7. Add package tests for hidden state, book view indices, selected view, tab
   color, and default height. Add calamine readback with hidden/tab options.
8. Update the gap matrix, AGENTS counts, and plan status.

## 9. Risks And Verification

- Risk: `BrtBookView` window geometry is cosmetic. Use the MS-XLSB example
  values and do not invent a window model.
- Risk: `BrtWsProp` has fields outside tab color. Keep Excel example defaults
  or record defaults explicit.
- Risk: all sheets hidden. Excel requires one visible active sheet; make the
  chosen active sheet visible at save.
- Verification: `cargo test --workspace`, clippy, fmt; inspect decoded package
  records; calamine readback.

## 10. Execution Handoff

- Status: implemented.
- Verification: 136 core + 78 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine readback; LibreOffice preserved hidden state, tab color,
  selected tab, active tab, and default row height. Reviewer verified both new
  record layouts and the workbook/sheet record order against MS-XLSB and found
  no defects.
- LibreOffice normalized `firstSheet` from the requested index 2 to the active
  tab 1 during an XLSX round trip; the decoded `BrtBookView.itabFirst` remains 2.
- Route: inline.
- User confirmation required: no.
