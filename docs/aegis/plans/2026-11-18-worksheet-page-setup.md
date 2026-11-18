# Worksheet page setup and print options

- Status: Implemented — reviewer verified fixes
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.3
- Baseline: `3db8a86` (`test: pin BrtBookView example flags`)
- Target: `rust-xlsb-core`, `rust-xlsb-writer`

## 1. Goal

Add the low-risk worksheet printing surface:

- `set_landscape()`
- `set_portrait()`
- `set_paper_size(paper_size)`
- `set_page_order(enable)`
- `set_margins(left, right, top, bottom, header, footer)`
- `set_print_gridlines(enable)`
- `set_print_headings(enable)`
- `set_print_center_horizontally(enable)`
- `set_print_center_vertically(enable)`
- `set_print_scale(scale)`
- `set_print_fit_to_pages(width, height)`
- `set_print_black_and_white(enable)`
- `set_print_draft(enable)`
- `set_print_first_page_number(page_number)`

A default workbook stays byte-identical: no `BrtPrintOptions`, `BrtMargins`,
or `BrtPageSetup`.

## 2. Scope And Acceptance

Accepted when:

- page orientation, paper size, page order, print scale or fit-to-pages, and
  first page number serialize into `BrtPageSetup`;
- print gridlines, headings, horizontal/vertical centering, black and white,
  and draft serialize into `BrtPrintOptions`;
- margins serialize into `BrtMargins` in inches;
- records appear in ABNF order after merged cells and before `BrtEndSheet`:
  `BrtPrintOptions`, `BrtMargins`, `BrtPageSetup`;
- `set_print_scale` and `set_print_fit_to_pages` are mutually exclusive, with
  the last call winning;
- invalid scale (`10..=400`), fit dimensions above `32767`, first page above
  `32767`, and margins outside `0..49` are ignored;
- a backwards margin/options change that restores defaults removes its record;
- calamine still reads cells from a workbook using every option;
- existing tests and default output remain unchanged.

Out of scope: headers, footers, header/footer images, print areas, repeat
rows/columns, page breaks, print centering measurement, and printer settings
relationships.

## 3. Change Necessity

The gap analysis lists this surface as missing. No configuration can emit the
required BIFF records; three record structs and worksheet print state are the
minimum boundary.

## 4. Aegis Visibility

The public `Worksheet` contract and binary record catalogue change. A durable
plan records the record layouts, order, validation, and default-compatibility
boundary.

## 5. Owners And Files

- `rust-xlsb-core/src/prelude/record/brt_page_setup.rs` — new `BrtPageSetup`.
- `rust-xlsb-core/src/prelude/record/brt_margins.rs` — new `BrtMargins`.
- `rust-xlsb-core/src/prelude/record/brt_print_options.rs` — new
  `BrtPrintOptions`.
- `rust-xlsb-core/src/prelude/record/brt_ws_prop.rs` — LSB flag order and
  `fFitToPage` coverage.
- `rust-xlsb-core/src/prelude/record/mod.rs`, `prelude/mod.rs` — exports and
  round-trip cases.
- `rust-xlsb-writer/src/worksheet.rs` — page-setup state and setters.
- `rust-xlsb-writer/src/sheet.rs` — emit the three records.
- `rust-xlsb-writer/src/e2e/mod.rs` — calamine readback with all options.
- `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`.

## 6. Compatibility Boundary

- `BrtPageSetup` is emitted only when a page-layout field differs from the
  default: portrait, paper size 0, down-then-over, scale 100, no fit, no first
  page.
- `BrtPrintOptions` is emitted only when a print flag is true.
- `BrtMargins` is emitted only when a margin differs from Excel defaults:
  `0.7, 0.7, 0.75, 0.75, 0.3, 0.3`.
- Record order follows the MS-XLSB `WORKSHEET` production.
- Existing sheet data, view, merge, and style output is unchanged.

## 7. TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API and three binary records.
- Test posture: add failing record and writer/package tests first, then
  implement the minimum code to pass.
- Verification: unit tests, calamine readback, clippy, fmt.

## 8. Tasks

1. Add failing record tests for `BrtPrintOptions` (2 bytes), `BrtMargins`
   (48 bytes), and `BrtPageSetup` (38 bytes, LSB flags, null `szRelID`).
2. Implement the records with spec field names and register exports and size
   cases.
3. Add failing worksheet tests for default output, all options, validation,
   and scale/fit mutual exclusion.
4. Add `PageSetup` state with the defaults in §6 and fluent setters.
5. Emit the three records after merged cells and before `BrtEndSheet`.
6. Add calamine readback with all page/print options.
7. Update the gap matrix, AGENTS counts, and plan status.

## 9. Risks And Verification

- Risk: `BrtPageSetup` has a nullable printer-settings relationship; emit null
  and add no relationship.
- Risk: fit-to-pages has no dedicated flag in `BrtPageSetup`. Emit
  `BrtWsProp.fFitToPage` and set `iScale = 0` with `iFitWidth`/`iFitHeight`.
- Risk: margins are inches, not points. Keep the API in inches like the
  reference.
- Verification: `cargo test --workspace`, clippy, fmt; decode package records
  and inspect field values; calamine readback.

## 10. Execution Handoff

- Status: implemented.
- Verification: 143 core + 85 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine readback. Real worksheet parts from `sample.xlsb` and
  `testVarious.xlsb` decoded through the new `BrtPageSetup`, `BrtMargins`, and
  `BrtPrintOptions` structs with exact field values, validating record layout.
  LibreOffice preserved orientation, page order, margins, print
  gridlines/headings/centering, fit-to-page, first page number, and tab color.
  It normalized paper size and dropped black-and-white and draft state.
- Reviewer verified both fixes with no remaining findings: black-and-white and
  draft now force `BrtPageSetup`; fit-to-pages emits `BrtWsProp.fFitToPage`.
  `BrtWsProp` now uses LSB bit order, matching the spec and real Excel bytes; a
  fit 1x1 sheet renders as one PDF page in LibreOffice.
- Route: inline.
- User confirmation required: no.
