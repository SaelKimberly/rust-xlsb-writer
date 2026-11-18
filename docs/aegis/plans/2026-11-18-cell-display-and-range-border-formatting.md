# Worksheet display strings and range border formatting

- Date: 2026-11-18
- Status: Implemented — reviewer findings fixed
- Owner: `rust-xlsb-writer`
- Baseline: `c9e56a2 feat: add worksheet display and autofit hints`
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.2 and §3.3

## 1. Goal

Close four bounded cell-formatting gaps:

- `set_nan_value(value)`
- `set_infinity_value(value)`
- `set_neg_infinity_value(value)`
- `set_range_format_with_border(first_row, first_col, last_row, last_col,
  cell_format, border_format)`

Non-finite numbers must serialize as display strings because XLSB has no NaN
or infinity number value. Range formatting must apply the cell format to every
cell and the border format only to the range's outer edges; interior borders
come from the cell format, matching `rust_xlsxwriter`.

## 2. Architecture

Both features stay inside `rust-xlsb-writer`. No new core record, `KnownID`
variant, package part, or dependency is required.

| Concern | Owner | Output |
|---|---|---|
| Non-finite text | `Worksheet` fields plus `Worksheet::store` | Existing `CellData::String` path |
| Range borders | `Worksheet::set_range_format_with_border` plus `Format::copy_border_side` | Existing format IDs and `BrtBorder` |

`Worksheet::store` is the single entry point for every cell write, so
converting non-finite numbers there covers scalar, row, column, matrix, and
typed number writes. The conversion happens before format interning and before
`store_with_index`, so existing string-length validation still applies.

Range formatting builds at most nine edge variants from the supplied
`cell_format`, then calls the existing `set_cell_format` for each cell. A
`Format` helper copies only one border side at a time; it does not merge whole
formats or change font, fill, alignment, or number-format state.

## 3. Baseline And Authority

- Reference behavior: vendored `rust_xlsxwriter` `set_nan_value`,
  `set_infinity_value`, `set_neg_infinity_value`, and
  `set_range_format_with_border`.
- Default non-finite strings: `NAN`, `INF`, `-INF`.
- Existing write path: `rust-xlsb-writer/src/data.rs` builds `CellData`;
  `Worksheet::store` validates coordinates and strings.
- Existing style path: `Worksheet::intern_format` deduplicates `Format`
  values; `Format::to_cell_style` maps borders into `BrtBorder`.
- MS-XLSB: cell strings use `BrtCellSt`/`BrtCellIsst`; border sides are
  `BrtBorder.blxfTop`, `blxfBottom`, `blxfLeft`, and `blxfRight`.

## 4. Compatibility Boundary

- Default workbook output is byte-identical when no non-finite value is
  written and the new range-border method is unused.
- `set_nan_value`, `set_infinity_value`, and `set_neg_infinity_value` accept
  `impl Into<String>`, return `&mut Self`, and allocate only when called.
- Unset display strings use static `NAN`, `INF`, and `-INF`; no default
  `String` allocation per worksheet.
- A non-finite value written with a format keeps that format. A display
  string longer than 32,767 UTF-16 code units returns `Error::StringTooLong`,
  the same as `write_string`.
- `set_range_format_with_border` validates the range before mutating cells:
  reversed ranges return `Error::RowColumnOrder`; out-of-range coordinates
  return `Error::RowColumnLimit`.
- `cell_format` applies to every cell. `border_format` supplies only the top,
  bottom, left, and right border style and color for the relevant edges.
  Diagonal borders and all non-border properties come from `cell_format`.
- Single-cell, single-row, single-column, and rectangular ranges use the same
  edge rules as the reference implementation.
- No `Format::merge`, print areas, defined names, formulas, or new BIFF
  records are included.

## 5. Change Necessity

There is no existing API for either behavior. Non-finite numbers currently
reach `NumberPolicy` as `Real` values, which XLSB cannot display as numbers.
Range formatting currently has no border-aware variant; callers would have to
apply nine formats manually. The minimum boundary is three scalar setters, one
worksheet field group, one `Format` helper, and one worksheet method.

## 6. TDD Route

- Mode: `auto`
- Decision: `strict`
- Authority: public API and serializer-visible behavior.
- Test posture: add focused failing tests first for default and custom display
  strings, formatting preservation, range validation, and every border
  position. Then implement, then run the workspace verification commands.

## 7. Tasks

### Task 1 — Non-finite display strings

Files: `rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/e2e/mod.rs`.

- Add `nan_value`, `infinity_value`, and `neg_infinity_value` as
  `Option<String>` fields on `Worksheet`, defaulting to `None`.
- Add public setters:
  - `set_nan_value(&mut self, value: impl Into<String>) -> &mut Self`
  - `set_infinity_value(&mut self, value: impl Into<String>) -> &mut Self`
  - `set_neg_infinity_value(&mut self, value: impl Into<String>) -> &mut Self`
- Add a private `non_finite_text(&self, value: f64) -> Option<&str>` helper
  returning the configured value or the static default.
- In `store`, replace `CellData::Number(value)` when `value.is_nan()` or
  `value.is_infinite()` with `CellData::String(text.to_owned())` before
  interning the format.
- Tests in `worksheet.rs`:
  - default NaN, positive infinity, and negative infinity become `NAN`,
    `INF`, and `-INF` strings;
  - custom values replace each default;
  - a formatted non-finite write keeps the format;
  - a custom display string longer than 32,767 UTF-16 code units returns
    `StringTooLong`.
- E2E test in `e2e/mod.rs`: write default and custom non-finite values, read
  them back with calamine as strings.

### Task 2 — Border-aware range formatting

Files: `rust-xlsb-writer/src/format.rs`,
`rust-xlsb-writer/src/worksheet.rs`.

- Add `pub(crate) enum BorderSide { Top, Bottom, Left, Right }` in
  `format.rs`.
- Add `Format::copy_border_side(&mut self, other: &Format, side: BorderSide)`:
  copy `blxf*.border_type` and the matching `border_*_color` option.
- Add `Worksheet::set_range_format_with_border` with the reference signature
  and the validation order from §4.
- Add a private `combined_border_format(cell_format, border_format, sides)`
  helper that clones `cell_format` and applies each listed side from
  `border_format`; `sides` is a stack slice such as `&[BorderSide::Top,
  BorderSide::Left]`.
- Precompute the needed variants once, then loop:
  - one cell: all four sides;
  - one row: row left, row center, row right;
  - one column: column top, column center, column bottom;
  - rectangle: nine positions (four corners, four edges, center).
- Tests in `worksheet.rs`:
  - 3x3 range: `(0,0)` top+left, `(0,1)` top, `(0,2)` top+right,
    `(1,0)` left, `(1,1)` none, `(1,2)` right, `(2,0)` bottom+left,
    `(2,1)` bottom, `(2,2)` bottom+right;
  - single cell gets all four borders;
  - single row gets top/bottom on every cell, plus left on the first and right
    on the last;
  - single column gets left/right on every cell, plus top on the first and
    bottom on the last;
  - fill and font from `cell_format` survive on interior and edge cells;
  - a border color is copied to every applied side;
  - reversed and out-of-range inputs return the documented errors without
    changing existing cells.

### Task 3 — Documentation and regression sweep

Files: `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`,
`docs/aegis/INDEX.md`.

- Mark the four APIs supported.
- Add this plan to the index.
- Update test counts after the final run.
- Run the full verification commands below.

## 8. Verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
git diff --check
```

Expected: 163 core + 114 writer = 277 unit tests and 6 doctests pass, no
clippy warnings, fmt clean.
Calamine reads the non-finite strings back from a real package. Border
positions are verified from the in-memory interning table and from
`Format::to_cell_style` border sides. LibreOffice rendered a workbook showing
`NAN`/`INF`/`-INF` and custom strings, and its XLSX export preserved the
medium/red outer border sides; raw style tests remain authoritative for
interior cells.

## 9. Risks And Retirement

- Risk: converting non-finite numbers at storage changes the stored cell type
  from `Number` to `String`. This is required by XLSB and matches the
  reference; finite numbers are untouched.
- Risk: a very long custom display string fails at write time, not at setter
  time. This matches existing string writes and avoids validating unused
  defaults.
- Risk: range-border variants multiply format intern entries by at most nine
  per call. Only reached variants are generated, and identical formats
  deduplicate through `intern_format`.
- Risk: border colors are optional in `Format`; copying `None` correctly
  clears a cell-format border color on that side, matching the reference.
- No old path is retained; both features are new methods with default-off
  state.
