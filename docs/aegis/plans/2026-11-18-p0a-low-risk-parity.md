# P0a low-risk API parity

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` (§6 P0, §8)
- Target: `rust-xlsb-writer`
- Method pack: Aegis implementation plan

## 1. Goal

Close a coherent, low-risk slice of the `rust_xlsxwriter` gap without adding new
XLSB records:

- `IntoExcelData for Result<T, E>`
- formatted row/column writes and row/column matrix writes
- cell/range format edits and clearing
- hidden rows/columns
- format-only parity setters that map to existing `BrtFont`, `BrtXF`, and
  `BrtBorder` fields

This is the P0a slice recommended by the gap analysis. Formula support,
merged cells, hyperlinks, page setup, tables, and rich objects remain out of
scope.

## 2. Requirement Ready Check

```text
Requirement Ready Check:
- Requirement source refs: docs/gap-analysis-rust-xlsxwriter.md §3.2–§3.7, §6 P0, §8; user request to implement gap features
- Goals and scope refs: this plan §1, §3
- User / scenario refs: existing rust_xlsxwriter-shaped writer callers
- Requirement item refs: Result cells, formatted/matrix writes, cell/range edits, hidden rows/columns, format parity
- Acceptance / verification criteria refs: this plan §7
- Open blocker questions: none
- Decision: ready
```

## 3. Scope And Non-Goals

### In scope

- `Result<T, E>` delegates to `Ok` or `Err` values, with and without a format.
- `write_row_with_format`, `write_column_with_format`, `write_row_matrix`,
  `write_column_matrix`.
- `set_cell_format`, `set_range_format`, `clear_cell`, `clear_cell_format`.
- `set_row_hidden`, `set_row_unhidden`, `set_column_hidden`,
  `set_column_range_hidden`.
- `FormatUnderline`, `FormatScript`, `FontScheme`, `FormatDiagonalBorder`.
- `set_unlocked`, `unset_bold`, `unset_italic`, `unset_text_wrap`,
  `set_font_strikethrough`, `unset_font_strikethrough`, `set_underline`,
  `set_font_script`, `set_indent`, `set_shrink`, `unset_shrink`,
  `set_rotation`, `set_reading_direction`, `set_font_scheme`,
  `set_font_family`, `set_font_charset`, diagonal border style/color/type,
  `set_quote_prefix`, `unset_quote_prefix`.

### Out of scope

- Formula records/tokenizer, merged ranges, hyperlinks, rich strings.
- New core record types.
- Page setup, print, tables, filters, conditional formats, validation,
  protection, images, charts, notes, shapes, checkboxes, sparklines.
- Constant-memory/low-memory writers.
- Any fallback or no-op for unsupported features.

## 4. Current Owners

| Owner | Responsibility |
|---|---|
| `rust-xlsb-writer/src/data.rs` | `IntoExcelData` / `CellData` |
| `rust-xlsb-writer/src/worksheet.rs` | Cell map, row/column options, write/layout API |
| `rust-xlsb-writer/src/format.rs` | `Format`, enums, mapping to `CellStyle` |
| `rust-xlsb-writer/src/sheet.rs` | Hidden row/column emission |
| `rust-xlsb-core/src/prelude` | Existing `BrtFont`, `BrtXF`, `BrtBorder`, `BrtRowHdr`, `BrtColInfo` fields |

No new owner. All changes use existing records.

## 5. TDD Route

```text
TDD Route:
- Mode: auto
- Decision: strict
- Authority: Aegis writing-plans route guard; public API + record output
- Test posture: add failing behavior tests before implementation
- Verification: cargo test --workspace, clippy, fmt, calamine/LibreOffice spot checks
```

## 6. Implementation Tasks

### Task 1 — Result cells and write variants

Files: `data.rs`, `worksheet.rs`.

- Implement `IntoExcelData for Result<T, E>` by delegating to `Ok`/`Err`.
- Add formatted row/column writes using the same offset and partial-write
  semantics as the existing unformatted variants.
- Add row-matrix and column-matrix writes:
  - row matrix: outer rows, inner columns;
  - column matrix: outer columns, inner rows.
- Reuse `&Format`; do not clone formats per cell.

Tests: `Ok`/`Err` with and without format; formatted row/column writes; matrix
dimensions and ordering; out-of-range failure leaves earlier cells.

### Task 2 — Cell/range format edits and clearing

Files: `worksheet.rs`.

- `set_cell_format(row, col, &Format)`: intern once; insert a formatted blank
  cell when absent.
- `set_range_format(first_row, first_col, last_row, last_col, &Format)`:
  validate order/bounds once, intern once, then update cells.
- `clear_cell(row, col)`: remove the cell and prune an empty row map.
- `clear_cell_format(row, col)`: remove the cell format; prune a blank cell with
  no format.
- No per-cell `Format` clones; only the worksheet-local format table grows once
  per distinct format.

Tests: format-only edit of an existing value, blank-cell formatting, range
format, clear value, clear format, reversed range rejection.

### Task 3 — Hidden rows and columns

Files: `worksheet.rs`, `sheet.rs`.

- Add `hidden: bool` to `RowOptions`/`ColumnOptions`.
- Add the four hidden/unhidden methods.
- Emit `BrtRowHdr.fDyZero = true` and `miyRw = 0` for hidden rows.
- Emit `BrtColInfo.fHidden = true` and `coldx = 0` for hidden columns.
- Column coalescing compares hidden state too.

Tests: record fields for hidden/unhidden rows and columns; hidden columns do
not coalesce with visible neighbors; width/format behavior is preserved.

### Task 4 — Format parity setters

Files: `format.rs`.

- Add enums and mappings:
  - underline values `0`, `1`, `2`, `0x21`, `0x22`;
  - script values `0`, `1`, `2`;
  - font scheme `None=0`, `Major=1`, `Minor=2`;
  - diagonal direction flags.
- Add setters listed in §3.
- Diagonal color is stored in `Format` and applied to `BrtBorder.blxfDiag`.
- Rotation maps `0..=90`, `-90..=-1`, and `270` to `BrtXF.trot`; out-of-range
  values are ignored.
- Reading direction maps `0..=2` to `ReadingOrder`; invalid values are ignored.
- `set_indent` accepts `0..=250`; larger values are ignored.
- `set_quote_prefix` maps to `BrtXF.f123Prefix`.
- `set_unlocked`, unset helpers, shrink, family, charset, and scheme map to
  existing fields.
- No string formatting/parsing allocation in setters.

Tests: exact `CellStyle` field mappings for every setter; unset helpers; diagonal
style/color/type; rotation/reading/direction boundaries; invalid values leave
the prior state.

### Task 5 — Verification, review, commit

- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets`.
- Run `cargo fmt --all --check`.
- Render an updated showcase or a small fixture through LibreOffice for hidden
  rows/columns and the new format properties.
- Launch reviewer agent; fix all findings.
- Use `closing-sessions` skill and commit.

## 7. Acceptance Criteria

- `worksheet.write(0, 0, Ok::<i32, &str>(42))` writes `42`;
  `Err("bad")` writes `"bad"`.
- `write_row_with_format` / `write_column_with_format` apply one format to all
  cells in the run.
- `write_row_matrix` / `write_column_matrix` place values in the documented
  row/column order.
- `set_cell_format` formats a value without rewriting it; blank cells become
  formatted blanks.
- `set_range_format` covers the inclusive rectangle and rejects reversed
  ranges.
- `clear_cell` removes value and format; `clear_cell_format` keeps the value.
- Hidden row emits `fDyZero=true`, `miyRw=0`; hidden column emits
  `fHidden=true`, `coldx=0`; unhide restores visible records.
- All format parity setters map to the expected core fields.
- No new allocations per cell in the new loops; format interning happens once
  per distinct format.
- No new core records or public stubs.
- Full workspace tests, clippy, and fmt pass.

## 8. Risks

- `BrtXF.trot` direction semantics: test the documented values; visual check
  for rotation.
- Hidden row/column width semantics vary between Excel and LibreOffice; assert
  the XLSB fields and render a sample.
- `set_range_format` can touch a large rectangle; document and test bounds.
- Existing `Format` equality/hash must include the diagonal color and new state.
- No formula/merge/URL work; do not imply those gaps are closed.

## 9. Execution Handoff

- Status: complete.
- Verification: 119 core + 52 writer unit tests, 6 doctests; clippy and fmt clean.
- Reviewer: found `set_row_unhidden` materializing absent row options; fixed to
  mutate only existing `RowOptions`.
- Next: closing-sessions commit; formula/merge/URL remain P0b/P0c.

- Route: inline; edits share `worksheet.rs`, `format.rs`, and `sheet.rs`.
- User confirmation required: no.
