# Worksheet view options and pixel units

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.2, §3.3
- Baseline: `da83eb7`
- Target: `rust-xlsb-writer`

## 1. Goal

Add the remaining low-risk worksheet view/display controls and pixel-unit
layout setters:

- `set_zoom(zoom)`
- `set_screen_gridlines(enable)`
- `set_hidden_zeroes(enable)`
- `set_right_to_left(enable)`
- `set_freeze_panes_top_cell(row, col)`
- `set_row_height_pixels(row, pixels)`
- `set_column_width_pixels(col, pixels)`
- `set_column_range_width_pixels(first, last, pixels)`

Defaults stay byte-identical: no view records, existing row/column output.

## 2. Scope And Acceptance

Accepted when:

- view setters alter `BrtBeginWsView` flags or zoom;
- `set_freeze_panes_top_cell` moves only `BrtPane.rwTop/colLeft`;
- pixel setters convert to existing point/character storage;
- pixel value `0` hides the row/column, matching `rust_xlsxwriter`;
- invalid pixel sizes return the existing range errors;
- calamine still reads cells from a sheet using all new options;
- default sheets emit no `WSVIEWS2` records.

Out of scope: view modes, print settings, split panes, default row height, sheet
visibility/tab state, and multiple selections.

## 3. Change Necessity

The record layer already carries every affected field. No core record or
packaging change is needed. The minimum boundary is writer-local `SheetView`
state, two conversion helpers, and the existing `write_sheet_views` mapping.
Documentation alone cannot expose these controls.

## 4. Spec And Reference Basis

`BrtBeginWsView` (`2.4.313`) already exposes:

- `fDspGrid` — screen gridlines;
- `fDspZeros` — display zero values;
- `fRightToLeft` — sheet direction;
- `wScale` / `wScaleNormal` — zoom percentages.

`BrtPane` (`2.4.189`) owns `rwTop` / `colLeft`, the first visible cell of the
frozen pane.

Reference behavior from `rust_xlsxwriter` v0.102.0:

- `set_zoom` accepts `10..=400`, otherwise leaves state unchanged;
- `set_row_height_pixels(row, 0)` hides the row;
- `set_column_width_pixels(col, 0)` hides the column;
- row pixels convert at `points = pixels * 0.75`;
- column pixels convert at `chars = pixels / 12` up to 12 px, then
  `(pixels - 5) / 7`.

## 5. Design

`SheetView` gains:

```rust
freeze_top_cell: Option<(RowNum, ColNum)>,
zoom: u16,              // default 100
show_gridlines: bool,   // default true
show_zeros: bool,       // default true
right_to_left: bool,    // default false
```

`is_default()` includes every new field, except `freeze_top_cell` without a
freeze (unused and therefore not serialized).

Serialization:

- `fDspGrid = show_gridlines`
- `fDspZeros = show_zeros`
- `fRightToLeft = right_to_left`
- `wScale = zoom`, `wScaleNormal = 100`
- `BrtPane.rwTop/colLeft = freeze_top_cell.unwrap_or(freeze boundary)`

Allocations: no new allocations; conversions are scalar. Pixel range setters
reuse the existing loop and validation.

Validation:

- row/column bounds use `check_row` / `check_col`;
- pixel dimensions `0` call the existing hidden setters;
- other dimensions convert then pass through existing validators;
- zoom is a no-op outside `10..=400`, matching the reference API.

## 6. TDD Route

- Mode: auto
- Decision: strict
- Authority: public API and serialized binary behavior
- Test posture: add failing writer tests first, implement, then keep the tests
  as regression coverage.
- Verification: unit tests, calamine readback, clippy, fmt.

## 7. Tasks

1. RED: add worksheet tests for view defaults, zoom bounds, flag replacement,
   pixel conversions, zero-hides, and `freeze_top_cell`; add a `sheet.rs` test
   decoding the emitted view flags/pane.
2. Implement `SheetView` fields, setters, normalization helpers, and
   `is_default`.
3. Map fields in `write_sheet_views`; cover custom pane top-left.
4. Extend the calamine e2e view test to use all new options.
5. Docs: gap analysis matrix, AGENTS test counts, Aegis index, plan status.
6. Reviewer agent, fix findings, closing-session commit.

## 8. Verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

Record tests are the binary proof. Calamine proves the package still reads.

## 9. Risks And Retirement

- No Excel runtime is available. Flag semantics come from MS-XLSB and the
  reference writer, not live Excel.
- `fFrozenNoSplit` behavior is unchanged from the prior view batch.
- No old path is retained; the default view path remains the single early
  return.

## 10. Execution Handoff

- Status: complete.
- Verification: 132 core + 67 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine reads a sheet using every new option.
- Reviewer: three P3 findings. Removed a dead `freeze_top_cell` predicate from
  `is_default`, split the gridline and zero flag tests so a swapped mapping
  fails, and added 12 px, 13 px, and single-column zero-width cases.
- No Excel runtime is available; flag and zoom semantics remain spec- and
  reference-based.
- Route: inline.
- User confirmation required: no.
