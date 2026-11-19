# Array formulas

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::{write_array_formula, write_array_formula_with_format}`,
  static array formula records, range expansion, and cached results.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.2 and P0.
- Reference: vendored `rust_xlsxwriter` v0.102.0
  `Worksheet::{write_array_formula, write_array_formula_with_format}` and
  `store_array_formula`.
- Spec: MS-XLSB §2.4.6 (`BrtArrFmla`), §2.4.684-2.4.687 (`BrtFmla*`),
  §2.5.98.1 (`ArrayParsedFormula`), §2.5.98.40 (`PtgExp`), and
  §2.5.98.42 (`PtgExtraCol`).
- Baseline: `5419da8 feat: add scalar formula support`.

## Goal

Add the source-compatible static array formula API:

```rust
let mut workbook = Workbook::new();
let worksheet = workbook.add_worksheet();
worksheet.write_number(0, 1, 500)?;
worksheet.write_number(0, 2, 300)?;
worksheet.write_number(1, 1, 10)?;
worksheet.write_number(1, 2, 15)?;
worksheet.write_array_formula(0, 0, 0, 0, "{=SUM(B1:C1*B2:C2)}")?;
workbook.save("array_formulas.xlsb")?;
```

The formula is parsed when it is written. The top-left cell carries the cached
result and the array range record; the remaining cells in the range are filled
with numeric zero, matching the reference writer.

Out of scope: dynamic-array formulas and their Metadata part, shared formulas,
table/data-table formulas, external-workbook references, cross-sheet
references, user-defined names inside formulas, and array constants such as
`{1,2;3,4}`. Existing formula syntax limits still apply.

## Aegis Visibility

`BrtArrFmla` is a durable wire contract. A wrong range, missing `PtgExp`, or
missing `PtgExtraCol` makes Excel repair or reject the workbook. The public
array API also changes which cell records are emitted, so record ordering and
the scalar formula path need one explicit owner.

## Requirement Ready Check

- Approved source: gap analysis P0 and the vendored reference API.
- Scenarios: single-cell result, multi-cell range, formatted range, cached
  result, malformed formula, invalid range.
- Acceptance: generated workbook imports without repair, contains the expected
  `BrtFmla*` plus `BrtArrFmla` sequence, and readers expose the cached result.
- No user decision is required; the reference defines the API and defaults.

## Change Necessity

The scalar path cannot represent an array range. Adding syntax to
`write_formula` would create a second owner for range state and cached-result
placement. The minimum boundary is a new cell variant plus one record type.

## Ripple Signal Triage

- Canonical record owner remains `rust-xlsb-core`.
- Canonical worksheet/API owner remains `rust-xlsb-writer`.
- Consumers affected: sheet serialization, formula-presence detection,
  autofit width, package tests, and the worksheet API.
- No fallback or duplicate owner is added. Scalar formulas keep their existing
  path.

## Architecture

`rust-xlsb-core`:

- Extend `prelude/record/brt_fmla.rs` with `BrtArrFmla`.
- `BrtArrFmla` contains `UncheckedRfX`, the `fAlwaysCalc` bit plus seven
  unused bits, and an `ArrayParsedFormula` alias for the existing
  `CellParsedFormula` layout.
- `Checked` validates the range and formula length; `assign_id!` registers it.
- Re-export and register the record in `prelude/mod.rs`.

`rust-xlsb-writer`:

- Add `ArrayFormulaCell` containing the compiled array formula, the range, and
  the cached result.
- Add `CellData::ArrayFormula`.
- `Worksheet` gains `write_array_formula` and
  `write_array_formula_with_format`, plus `store_array_formula`.
- The top-left cell stores `ArrayFormulaCell`; remaining cells become numeric
  zero with the same format index.
- `formula.rs` gains a helper that builds the `PtgExp`-based top-left formula:
  `rgce = [0x01, row]`, `rgcb = [column]`.
- `sheet.rs` emits the result-specific `BrtFmla*` record for the top-left cell,
  immediately followed by `BrtArrFmla` with the real formula tokens.
- `has_formulas` and autofit width include the array variant.
- `package.rs` emits the existing recalculation-on-load behavior when only
  array formulas are present.

## Array Semantics

- First row/column must not follow the last row/column; otherwise return
  `Error::RowColumnOrder`.
- Rows and columns are checked with the existing worksheet limits.
- The top-left formula record's `CellParsedFormula` contains `PtgExp` with the
  top-left row and a `PtgExtraCol` with the top-left column.
- `BrtArrFmla.rfx` covers the full range and `fAlwaysCalc` is set.
- The cached result comes from `Formula::set_result`, or the worksheet default
  result, using the existing number/string/bool/error mapping.
- Non-top-left cells are zeros; the writer does not invent per-cell formulas.
- `set_formula_result` continues to target scalar formula cells only.

## TDD Route

- Mode: `auto`
- Decision: `strict`
- Authority: new public API plus a shared record contract.
- Test posture: add record round-trip and token tests before wiring the
  writer path, then add package/API tests and a LibreOffice import check.

## Tasks

1. Add `BrtArrFmla` and its record test; re-export and register it.
2. Add `ArrayFormulaCell` and the `PtgExp` helper; add token tests.
3. Add worksheet storage and public methods; add range/error/format tests.
4. Emit the record sequence in `sheet.rs`; update formula detection and
   autofit width; add package tests.
5. Add the showcase/package integration test and verify with LibreOffice.
6. Run fmt, default and all-feature tests, clippy, and diff checks.
7. Update the gap analysis, AGENTS counts, and this plan.

## Tests

Core:

- `BrtArrFmla` deku round-trip with a real formula payload.
- Default record size and `Checked` rejection for an invalid range.

Writer:

- top-left `PtgExp` bytes and `PtgExtraCol` payload;
- array formula tokenization through the existing parser;
- `first > last` row/column rejection;
- range cells become numeric zero and keep the requested format;
- `set_formula_result` replaces the cached result on array formulas;
- scalar formula behavior remains unchanged.

Package:

- `BrtArrFmla` immediately follows the top-left `BrtFmla*` record;
- `BrtCalcProp` appears for array-only workbooks;
- calamine reads the cached top-left result;
- LibreOffice imports and recalculates the array formula.

## Verification

- `cargo test --workspace`: 621 tests pass (614 unit + 7 doctests; 284 core,
  330 writer).
- `cargo test --workspace --all-features`: 625 tests pass (618 unit + 7
  doctests).
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- `/tmp/array_formulas.xlsb` contains `BrtFmlaNum` immediately followed by
  `BrtArrFmla` for both ranges.
- LibreOffice converted it to XLSX without repair, preserved array formulas
  with `ref="A1:A2"` and `ref="A4:A4"`, and recalculated them to 9500 and 825.

## Review

`ArrayFormulaReview` verified the record layout, adjacency, LSB flag, RfX
bounds, `PtgExp`/`PtgExtraCol` agreement, cached results, and range zero-fill.
It found one source-compatibility defect and one test gap:

1. `set_formula_result` ignored array formulas. Fixed by matching both formula
   cell variants and updating their cached result. Added a regression test.
2. The package test did not pin `BrtArrFmla` adjacency. Fixed with a
   record-stream assertion that the array record immediately follows
   `BrtFmlaNum`.

## Risks

- The array form is validated against the spec and LibreOffice, not native
  Excel.
- The writer fills the range with zeros rather than per-cell formula records;
  this matches the reference writer but needs import evidence.
- Array constants and dynamic arrays remain unsupported and return
  `Error::FormulaError`.
