# Worksheet display modes, unused-row hiding, and autofit-width hints

- Date: 2026-11-18
- Status: Implemented — reviewer finding fixed
- Owner: `rust-xlsb-writer`
- Baseline: `708d960 feat: add worksheet outline groups`
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.3

## 1. Goal

Close three bounded layout/view gaps without adding a second owner:

- `set_view_normal`, `set_view_page_layout`, `set_view_page_break_preview`
- `hide_unused_rows(enable)`
- `set_column_autofit_width(col, pixels)`

The result must remain source-compatible with the corresponding
`rust_xlsxwriter` methods, emit no records for default state, and serialize the
correct MS-XLSB flags.

## 2. Architecture

All public state stays on `Worksheet`; `sheet.rs` remains the only serializer.
No new module and no new dependency.

| Concern | Owner | Record |
|---|---|---|
| View mode | `Worksheet::view` (`SheetView`) | `BrtBeginWsView.xlView`, `fDspRuler` |
| Unused rows | `Worksheet.hide_unused_rows` | `BrtWsFmtInfo.fDyZero` |
| Autofit hint | `ColumnOptions.best_fit` | `BrtColInfo.fBestFit` |

`BrtBeginWsView`, `BrtWsFmtInfo`, and `BrtColInfo` already carry these fields.
No core record or `KnownID` change is required.

## 3. Baseline And Authority

- `rust-xlsb-core`: record layouts and `Checked` limits are already defined.
- Existing worksheet view emission: `rust-xlsb-writer/src/sheet.rs::write_sheet_views`.
- Existing format emission: `write_sheet_format`.
- Existing column assembly: `column_infos` / `col_info`.
- Reference behavior: vendored `rust_xlsxwriter` `set_view_*`,
  `hide_unused_rows`, and `set_column_autofit_width`.
- MS-XLSB: `XLView` 2.5.168; `BrtBeginWsView.fDspRuler` 2.4.307;
  `BrtWsFmtInfo.fDyZero` 2.4.875; `BrtColInfo.fBestFit` 2.4.336.

## 4. Compatibility Boundary

- Default workbook output is byte-identical: normal view, visible rows, and no
  autofit hint emit no new records and no changed bytes.
- `set_view_normal()` restores default state; if no other view state remains,
  `WSVIEWS2` is omitted.
- Page Layout sets `xlView = 2` and `fDspRuler = true`. Page Break Preview sets
  `xlView = 1` and `fDspRuler = false`.
- `hide_unused_rows(false)` restores the default and cannot force an otherwise
  empty `BrtWsFmtInfo`.
- `set_column_autofit_width(col, 0)` hides the column, matching the reference
  and the existing pixel-width setter.
- Manual `set_column_width*` clears `fBestFit`; autofit width sets
  `fBestFit = true` and `fUserSet = true`.
- Column coalescing includes `best_fit`, so a best-fit column cannot merge into
  a manual-width span.
- No formula, print-area, or defined-name work is included.

## 5. Change Necessity

No existing API or record combination covers these behaviors. View modes need
`xlView`; unused-row hiding needs `fDyZero`; best-fit widths need `fBestFit`.
The minimum boundary is the writer state plus three serializer branches.

## 6. TDD Route

- Mode: `auto`
- Decision: `strict`
- Authority: public API and BIFF-visible behavior; regression risk in
  column coalescing and default output.
- Test posture: add focused failing writer tests first for raw record bits and
  default omission, then implement, then run workspace tests/clippy/fmt.

## 7. Tasks

### Task 1 — View modes

Files: `rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/sheet.rs`.

- Add private `ViewMode { Normal, PageLayout, PageBreakPreview }` with
  `Default = Normal` and an `xl_view()` mapping `0/2/1`.
- Add `SheetView.view_mode`; include it in `SheetView::is_default`.
- Add public setters returning `&mut Self`.
- Emit `xlView` and `fDspRuler = (view_mode == PageLayout)` in
  `write_sheet_views`.
- Tests: each mode's raw `xlView`/`fDspRuler`; `set_view_normal` after a
  non-normal mode removes the view record.

### Task 2 — Hide unused rows

Files: `rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/sheet.rs`.

- Add `Worksheet.hide_unused_rows: bool` and public `hide_unused_rows(bool)`.
- Include the flag in `write_sheet_format`'s emission predicate and set
  `BrtWsFmtInfo.fDyZero`.
- Tests: enabled emits `BrtWsFmtInfo` with `fDyZero`; disabled emits no record;
  combined default height + hidden unused rows keeps `fUnsynced` tied only to
  the explicit height.

### Task 3 — Autofit-width hint

Files: `rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/sheet.rs`.

- Add `ColumnOptions.best_fit: bool`.
- Add `set_column_autofit_width(col, pixels)`:
  - zero hides the column;
  - non-zero converts pixels with the existing `pixels_to_width`;
  - validates through the existing width range.
- Add a private width setter carrying `best_fit`; manual width setters pass
  `false`.
- Emit `BrtColInfo.fBestFit`; add `best_fit` to coalescing equality.
- Tests: autofit width sets `fBestFit` and width; manual width clears it;
  zero hides; differing `best_fit` splits adjacent columns.

### Task 4 — Documentation and regression sweep

Files: `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`,
`docs/aegis/INDEX.md`.

- Mark the three APIs supported and update test counts.
- Add this plan to the index.
- Run the full verification commands below.

## 8. Verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

Expected: 163 core + 107 writer unit tests and 6 doctests pass, no clippy warnings, fmt clean.
LibreOffice rendered a workbook whose first and last data rows remained
visible with unused rows hidden between them; raw record tests pin `fDyZero`.
Writer tests decode `BrtBeginWsView`, `BrtWsFmtInfo`, and `BrtColInfo` from
written bytes; calamine/LO is not used because none of these flags are exposed
by calamine or preserved reliably by LibreOffice.

## 9. Risks And Retirement

- Risk: adding a field to column equality can split existing coalesced spans.
  The field defaults to `false`, preserving every existing default path.
- Risk: `fDspRuler` must not be set outside Page Layout. The mapping is
  centralized in `write_sheet_views`.
- Risk: hidden unused rows and a default height share `BrtWsFmtInfo`.
  `fUnsynced` remains controlled only by `default_row_height`.
- No old path is retained; these are new methods with default-off state.
- Review: one documentation contradiction in the gap summary was found and
  fixed; no code defects.
