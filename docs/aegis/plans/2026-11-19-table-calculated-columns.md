# Table calculated-column formulas

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `TableColumn::set_formula`, structured-reference formula encoding,
  `BrtListCCFmla` emission, and the table/Xti package wiring.
- Out of scope: shared formulas, dynamic arrays, calculated columns that write
  cell values, and structured references to external workbooks.

## Goal

Close the remaining `add_table` gap from
`docs/gap-analysis-rust-xlsxwriter.md`: a table column can carry a calculated
formula, serialized as `BrtListCCFmla` with a `ListParsedFormula` token stream.
The public API matches `rust_xlsxwriter`:

```rust
TableColumn::new()
    .set_header("Totals")
    .set_formula("SUM(Table1[@[Quarter 1]:[Quarter 4]])")
```

The common structured-reference forms are supported:

- `Table1[Column]` — the data region of one column.
- `Table1[@Column]` and `Table1[@[Column]]` — the current row.
- `Table1[@[First]:[Last]]` — the current row across a column range.
- `Table1[[#Headers],[Column]]`, `[#Data]`, `[#Totals]`, `[#All]`, and
  `[#This Row]` row selectors, including column ranges.
- `Table1[[#All]]` — all columns.

## Architecture

`rust-xlsb-writer` already writes table parts in `src/table.rs`. Calculated
formulas are a table-column property, so they belong in the same owner rather
than the worksheet cell pipeline. `rust_xlsxwriter` also stores them only in the
Table part; it does not write cell formulas for the calculated column.

Formula compilation is deferred until package assembly, after
`Workbook::prepare()` assigns table ids and default names. A table formula needs
the target table's `idList` and the `BrtExternSheet` Xti index for its sheet.
Compiling at `add_table()` is therefore impossible without moving id and Xti
assignment earlier.

`BrtListCCFmla` (MS-XLSB 2.4.706) is the same shape as the shipped
`BrtListTrFmla`: a one-byte flags field followed by `ListParsedFormula`.
Structured references encode as `PtgList` (2.5.98.52), not `PtgRef`:
`ptg=0x18`, `eptg=0x19`, `ixti`, two flag bytes, `listIndex`, `colFirst`, and
`colLast`. This keeps the existing A1 formula parser unchanged for cell
formulas.

## Tech Stack

Rust 1.100.0, edition 2024, deku record derives, existing `formula` parser, and
existing `XtiCollector`. No new dependency.

## Baseline And Authority Refs

- `docs/gap-analysis-rust-xlsxwriter.md` table row: calculated-column formulas
  missing.
- `thirdparty/rust_xlsxwriter/src/table.rs`: `TableColumn::set_formula`,
  `calculatedColumnFormula`.
- MS-XLSB 2.4.706 `BrtListCCFmla`, 2.5.98.11 `ListParsedFormula`, 2.5.98.52
  `PtgList`, 2.5.98.73 `PtgRowType`, 2.5.98.103 `XtiIndex`.
- `rust-xlsb-writer/src/table.rs`, `src/formula.rs`, `src/package.rs`,
  `src/defined_name.rs`.

## Compatibility Boundary

- Existing table output without calculated formulas stays byte-identical.
- `TableColumn::set_formula` accepts `impl Into<Formula>`, matching the
  reference signature.
- Structured references are same-workbook only; external-workbook tables and
  table names with quoted spaces remain errors. Table names follow the existing
  defined-name grammar, so unquoted names are the supported subset.
- The formula is metadata for Excel; this writer does not write calculated
  result cells. Cached values remain caller-written data cells.
- A conflicting or missing table name, column name, or column range returns
  `Error::FormulaError` before any package bytes are emitted.

## Change Necessity

- User-visible need: source-compatible table calculated formulas; the prior
  table feature explicitly left them out.
- No-change option: docs-only would keep a known gap and cannot produce a valid
  `BrtListCCFmla`.
- Minimum code boundary: one core record, one parser extension, table-column
  storage, and package Xti/table-part wiring.
- Decision: code-change.

## TDD Route

- Mode: auto
- Decision: strict
- Strict authority: recorded auto decision for a new public API and new
  serialized record path.
- Test posture: strict RED/GREEN for the record, parser, and serialization
  slices, then package and regression tests.
- Verification: focused tests, workspace tests, all features, clippy, fmt.

## Task 1: Add `BrtListCCFmla`

Files: `rust-xlsb-core/src/prelude/record/brt_table.rs`,
`record/mod.rs`, `prelude/mod.rs`.

- RED: add a `checked_test::<BrtListCCFmla>(12)` case and a flags/layout test.
- Add the record with `flags: u8`, `formula: CellParsedFormula`, `F_ARRAY`,
  and reserved-bit validation.
- Re-export from both prelude modules through `assign_id!`.
- GREEN: focused core tests.

## Task 2: Encode structured references as `PtgList`

Files: `rust-xlsb-writer/src/formula.rs`.

- RED: add parser tests for:
  - `SUM(Table1[@[Q1]:[Q4]])` → 14-byte `PtgList` with `rowType=CURRENT`,
    `columns=2`, `colFirst=1`, `colLast=4`, `listIndex=1`, `ixti=0`.
  - `Table1[[#Headers],[Q1]]` → `rowType=HEADERS`, `columns=1`.
  - `Table1[[#All]]` → `rowType=ALL`, `columns=0`.
  - `Table1[Q1]` → `rowType=DATA`, `columns=1`.
  - unknown table, unknown column, and inverted range errors.
- Add a crate-private `TableInfo` lookup slice and
  `compile_table_formula(formula, tables)`.
- Extend `Parser` with an optional table slice. `parse_name_or_ref` detects an
  identifier followed by `[` and delegates to a structured-reference parser.
- Build `PtgList` with `ptg=0x18`, `eptg=0x19`, zero `ixti` from the table,
  flag byte `columns | rowType << 2`, second flag byte `0x02`, then
  `listIndex`, `colFirst`, `colLast`.
- Keep cell formula `compile` behavior unchanged.
- GREEN: focused formula tests.

## Task 3: Store and serialize table column formulas

Files: `rust-xlsb-writer/src/table.rs`,
`rust-xlsb-writer/src/worksheet.rs`, `rust-xlsb-writer/src/package.rs`,
`rust-xlsb-writer/src/lib.rs`, `rust-xlsb-writer/src/defined_name.rs`.

- RED: add table-part test asserting `BrtListCCFmla` appears before
  `BrtListTrFmla`, decodes to the expected tokens, and no record appears for a
  column without a formula.
- Add `TableColumn::set_formula(Formula)` and private `formula` storage.
- Add `TableInfo<'a>` (name, id, Xti, columns) and pass the table slice into
  `write_table_part`.
- In `add_table`, validate formula result length like custom totals; do not
  compile yet.
- In `build_package`, after table ids/names are assigned:
  - create one `XtiCollector`;
  - if any table column has a formula, index every sheet that owns a table;
  - build `TableInfo` entries with the sheet's Xti;
  - pass the slice to table-part writing.
- Refactor `workbook_part` / `write_defined_names` to accept the shared
  `XtiCollector`; table formula sheets therefore get `BrtExternSheet` entries
  even when no defined names exist. Existing no-table-formula output remains
  byte-identical.
- Emit `BrtListCCFmla` before `BrtListTrFmla`.
- GREEN: focused writer tests.

## Task 4: Package-level and failure tests

Files: `rust-xlsb-writer/src/package.rs`,
`rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/table.rs`.

- Assert a table part contains `BrtListCCFmla` and the workbook part contains
  `BrtExternSheet` for a formula-only workbook.
- Assert unknown table/column and cross-table missing-sheet errors surface from
  save, and existing table output without formulas is unchanged.
- Assert table names are resolved after default `TableN` assignment, so a
  formula may use the resolved name.
- Update docs and the gap analysis.

## Verification

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo test -p rust-xlsb-core prelude::tests::test_biff_export
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo test -p rust-xlsb-writer formula::tests
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo test -p rust-xlsb-writer table::tests
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo test --workspace
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo test --workspace --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo clippy --workspace --all-targets --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" \
  cargo fmt --all -- --check
```

Record-level oracle: decode the generated Table part and assert the
`BrtListCCFmla` token bytes. LibreOffice drops XLSB tables on import, so the
Table part is proven by record/package decode, not an XLSX round-trip.

## Review

`TableCalculatedReview` found four issues; all are fixed and covered:

1. P1: `PtgList` always used `type=reference` (`0x02`), producing non-conforming
   value contexts such as `ABS(Table1[@Q1])`. The second flag byte is now
   `0x06` (`unused=1`, `type=value`), matching the MS-XLSB 3.3.11 example. Tests
   pin both `SUM` and `ABS` token streams.
2. P2: table names beginning with an A1 reference (`Q1Sales`) were parsed as a
   cell first. Table names are now resolved before the A1 branch.
3. P3: non-ASCII table names (`Vendasé`) passed validation but not the parser.
   The prefix lookup now accepts the validated table name before identifier
   scanning.
4. P4: the record-order test did not prove `BrtBeginListCol < BrtListCCFmla <
   BrtListTrFmla < BrtEndListCol`. The test now puts both formulas on one column
   and asserts their indexes.

LibreOffice 7.3 imports the generated XLSB without error but drops the table
part, as with previous table work. The Table part and `BrtExternSheet` remain
proven by record/package decode. Package tests cover same-sheet, other-sheet,
default-name, and unknown-name cases.

## Risks

- `PtgList` bit packing must match 2.5.98.52. The test pins the full byte
  sequence; a wrong flag byte is a visible Excel formula error.
- `BrtExternSheet` is required even without defined names. The package test
  decodes it; otherwise Excel resolves `ixti` to an invalid entry.
- Structured-reference column names with `]` or external workbook qualifiers
  remain unsupported and return a formula error.
- Calculated formulas do not write cell values; a workbook that never
  recalculates still shows caller-supplied data, matching the reference API.

## Retirement

No old owner is retained. The existing cell formula parser stays the only A1
token encoder; table structured references add one explicit path and do not
fork cell formula behavior.
