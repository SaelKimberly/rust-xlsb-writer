# Table total rows, subtotals, and per-column formats

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Table::set_total_row`, `TableFunction`, `TableColumn` total labels,
  total functions, data formats, and header formats; MS-XLSB `BrtListTrFmla`;
  the worksheet total-row cells; and the existing Table part serializer.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4 `add_table`.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `table.rs` and
  `Worksheet::add_table`.
- Related plan: `docs/aegis/plans/2026-11-19-basic-tables.md`.

## Goal

Extend the shipped table subset with the common totals and formatting modes:

- `Table::set_total_row(true)` marks the last row of the table as a total row.
- `TableColumn::set_total_label` writes a label into the total row.
- `TableColumn::set_total_function(TableFunction::*)` writes a `SUBTOTAL`
  formula into the total row and records the matching `ILTA_*` value in the
  Table part.
- `TableColumn::set_format` applies a cell format to the data rows and the
  total formula cell.
- `TableColumn::set_header_format` applies a cell format to the header cell.
- `TableFunction::Custom(Formula)` accepts a same-sheet A1 formula and writes
  `BrtListTrFmla` beside the column.

Calculated-column formulas (`TableColumn::set_formula`), structured-reference
formulas, XML maps, query tables, slicers, and single-cell tables remain out of
scope. A structured total formula such as `SUM([Column5])` is rejected by the
existing formula compiler; an A1 equivalent such as `SUM(B3:B5)` works.

## Aegis Visibility

This adds public API, changes the Table part contract (`crwTotals`,
`BrtBeginList.flags`, column `ilta`/`stTotal`, `BrtListTrFmla`), and writes
formula cells into the worksheet. A plan keeps the table row geometry and the
Table part record order explicit.

## Architecture

`rust-xlsb-writer/src/table.rs` owns:

- `Table`, gaining `show_total_row`.
- `TableColumn`, gaining `total_label`, `total_function`, `format`, and
  `header_format`, plus private compiled custom-total state.
- `TableFunction`, mapping the Excel subtotal choices to `ILTA_*` values and
  `SUBTOTAL` function numbers.
- `write_table_part`, which emits `crwTotals`, `F_SHOWN_TOTAL_ROW`, the
  total-row-free `AutoFilter` range, column `ilta`/`stTotal`, and
  `BrtListTrFmla` for custom totals.

`Worksheet::add_table` keeps its existing validation and storage flow. It
computes the data-row band as:

```text
data_first = first_row + show_header_row
data_last  = last_row - show_total_row
```

It writes headers, total labels, total formulas, and column formats before
pushing the `TableRange`.

`rust-xlsb-core` gains one record:

- `BrtListTrFmla` (`2.4.708`): one flags byte (`fArray` bit 1) followed by a
  `CellParsedFormula`-shaped `ListParsedFormula`.

`crate::formula` gains a direct `SUBTOTAL` token builder used for standard
total functions. This avoids constructing and reparsing a formula string for
each column. `SUBTOTAL` is also added to the function table so an A1 custom
total formula can use it.

## Table Part Contract

For a table with a total row:

- `BrtBeginList.crwTotals = 1`.
- `BrtBeginList.flags` includes `F_SHOWN_TOTAL_ROW`.
- `BrtBeginAFilter.rfx` excludes the final total row.
- Each `BrtBeginListCol.ilta` is the effective `TableFunction` value; a
  non-empty total label forces `ILTA_NONE`.
- `stTotal` carries a total label when present, otherwise it is null.
- A custom total emits `BrtListTrFmla` after `BrtBeginListCol` and before
  `BrtEndListCol`.

The table `rfxList` continues to cover the whole table, including the total
row.

## Compatibility Boundary

- Existing tables without a total row keep byte-identical Table parts.
- `Table` and `TableColumn` gain builders; existing constructors and methods
  keep their signatures.
- `TableFunction` and the new `TableColumn` methods mirror the reference names.
- `TableColumn::set_formula` remains absent; calculated columns stay a gap.
- Custom total formulas use the existing formula subset: same-sheet A1
  references, operators, and supported functions.

## Tech Stack

Rust 1.100.0, edition 2024, deku records, `CellParsedFormula`,
`XlNullableWideString`, the existing Table part writer, `Format` interning, and
the existing worksheet formula cells.

## Baseline / Authority Refs

- [MS-XLSB] §2.4.100 `BrtBeginList`
- [MS-XLSB] §2.4.101 `BrtBeginListCol`
- [MS-XLSB] §2.4.708 `BrtListTrFmla`
- [MS-XLSB] §2.5.88 `ListTotalRowFunction`
- [MS-XLSB] §2.5.98.11 `ListParsedFormula`
- [MS-XLSB] §2.1.7.51 Table part ABNF
- `thirdparty/rust_xlsxwriter/src/table.rs`
- `rust-xlsb-writer/src/table.rs`
- `rust-xlsb-writer/src/worksheet.rs`
- `rust-xlsb-writer/src/formula.rs`

## TDD Route

- Mode: `auto`
- Decision: `strict`
- Authority: project test suite and this plan; behavior and public API change.
- Test posture: add focused tests for the record layout, table part fields,
  total-row cell formulas, custom totals, formats, and validation before
  treating the feature as complete.
- Verification: `cargo test --workspace`, all features, clippy, fmt.

## Tasks

### Task 1: `BrtListTrFmla` and SUBTOTAL tokens

**Files:**

- Modify: `rust-xlsb-core/src/prelude/record/brt_table.rs`
- Modify: `rust-xlsb-core/src/prelude/record/mod.rs`
- Modify: `rust-xlsb-core/src/prelude/mod.rs`
- Modify: `rust-xlsb-writer/src/formula.rs`

**Change:**

- Add `BrtListTrFmla { flags: u8, formula: CellParsedFormula }` with
  `assign_id!`, `Default`, and `Checked` (`flags & !0x02 == 0`, formula check).
- Re-export it and add a `test_biff_export` case with the default 12-byte size.
- Add `formula::subtotal_formula(function, first_row, last_row, col) ->
  CellParsedFormula` producing `PtgInt`, `PtgArea`, `PtgFuncVar` for
  `SUBTOTAL`.
- Add `SUBTOTAL` (id `0x0158`, two arguments) to the function table.

**Acceptance:**

- The new record round-trips and rejects reserved flag bits.
- `subtotal_formula(109, 2, 4, 1)` emits the expected token bytes.
- `compile("SUBTOTAL(109,B3:B5)")` succeeds.

### Task 2: Table model and table part

**Files:**

- Modify: `rust-xlsb-writer/src/table.rs`
- Modify: `rust-xlsb-writer/src/lib.rs`

**Change:**

- Add `Table::set_total_row`.
- Add `TableFunction` with `None`, `Average`, `Count`, `CountNumbers`, `Max`,
  `Min`, `Sum`, `StdDev`, `Var`, and `Custom(Formula)`.
- Add `TableColumn::set_total_label`, `set_total_function`, `set_format`, and
  `set_header_format`.
- Add `ILTA_*` mapping and `SUBTOTAL` number mapping.
- Emit `crwTotals`, `F_SHOWN_TOTAL_ROW`, the reduced AutoFilter range, column
  `ilta`/`stTotal`, and `BrtListTrFmla`.
- Compile custom total formulas once during `add_table` into private
  `total_formula` and `total_result` fields.

**Acceptance:**

- Default tables keep `crwTotals = 0`, `ilta = 0`, and no `BrtListTrFmla`.
- A total table emits `crwTotals = 1`, the total flag, and the right `ilta`.
- A custom total emits `BrtListTrFmla` with the same formula as the cell.

### Task 3: Worksheet total row and formats

**Files:**

- Modify: `rust-xlsb-writer/src/worksheet.rs`

**Change:**

- Validate `rows > header_rows + total_rows`; reject total rows without a data
  row.
- Validate total labels at 8189 UTF-16 code units.
- Compute the effective total function, preferring a label.
- Write total labels and standard/custom total formulas into the last row.
- Apply header formats to header cells and data formats to every data cell.
- Apply the column data format to a total formula cell.

**Acceptance:**

- Total labels and formulas land in the last table row with the expected cached
  result and formats.
- Data and header cells receive the configured formats without losing values.
- An invalid table shape leaves worksheet state unchanged.

### Task 4: Docs and verification

**Files:**

- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `AGENTS.md`

**Change:**

- Mark table totals/labels/subtotals and per-column formats shipped; keep
  calculated columns listed as missing.
- Document the new API and record.
- Run the full default and all-feature suites, clippy, and fmt.

## Verification

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo clippy --workspace --all-targets --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo fmt --all -- --check
```

## Review

`TableTotalsReview` found one P1 defect: `subtotal_formula` emitted `PtgArea`
with the VALUE data type (`0x45`). MS-XLSB §2.5.88 requires the area `type` to
be REFERENCE (`0x01`), which is token `0x25`. The builder now uses
`PTG_AREA_REF` and the test locks the corrected byte. The reviewer otherwise
confirmed the record layout, `crwTotals`, `F_SHOWN_TOTAL_ROW`, AutoFilter
range, `ILTA_*` mapping, geometry validation, format application, custom-total
compilation, and failure atomicity.

LibreOffice 7.3 drops XLSB tables on import, so the Table part itself remains
verified by record and package decode tests. LibreOffice does preserve and
calculate the total-row cells: the generated workbook round-trips as
`SUBTOTAL(109,$B$2:$B$4)` = 90 and `SUM(C2:C4)` = 120.

## Risks

- Excel may require the total row cell formula and `ilta` formula to match
  byte-for-byte. The implementation uses one `subtotal_formula` builder for
  both.
- A custom total formula using structured references is rejected; this is an
  explicit subset limit, not a silent fallback.
- Per-column formats are applied as cell formats. Table DXFs stay
  `0xFFFFFFFF`; the visible format comes from the cell XF.

## Retirement

No feature is retired. The existing no-total path stays the default and its
byte output is unchanged.
