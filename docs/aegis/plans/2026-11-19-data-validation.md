# Data validation

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `DataValidation`, `DataValidationRule`, `DataValidationErrorStyle`,
  `IntoDataValidationValue`, `Worksheet::add_data_validation`, and the MS-XLSB
  `DVALS` records.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4
  `add_data_validation`.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `data_validation.rs` and
  `Worksheet::add_data_validation`.

## Goal

Add worksheet data validation for the common non-formula cases: whole numbers,
decimal numbers, dates, times, text length, and fixed string lists. Validations
carry ignore-blank, dropdown, input/error message, title, and error-style
settings. The writer emits a `BrtBeginDVals` / `BrtDVal` / `BrtEndDVals`
collection in the worksheet part.

This closes the “no validation records” gap without adding `Formula`, formula
tokens, conditional formatting, tables, or multi-range input.

## Aegis Visibility

The change adds a public API, new binary records, and a new worksheet-part
contract. A plan keeps the compatibility subset, record order, and validation
limits explicit.

## Architecture

`rust-xlsb-writer/src/data_validation.rs` owns the public validation vocabulary,
rules, option state, and conversion to a `BrtDVal` body. `Worksheet` stores a
`Vec<DataValidationRange>`; `add_data_validation` validates the cell range and
appends a cloned validation.

`rust-xlsb-core/src/prelude/record/brt_dval.rs` owns `BrtBeginDVals`,
`BrtEndDVals`, `BrtDVal`, `DVals`, and `DValStrings`. `sheet::write_sheet` emits
the collection after merged cells and before hyperlinks, matching the Worksheet
ABNF order.

Validation numbers and fixed lists use `PtgNum` and `PtgStr` tokens inside
`NameParsedFormula`, reused as `DVParsedFormula`. No formula parser or `Formula`
type is introduced. Formula-valued rules remain absent.

Invalid “Any” validations with no prompt or error text are skipped, so calling
`add_data_validation` with a blank `DataValidation::new()` does not add output.

## Tech Stack

Rust 1.100.0, edition 2024, deku record pipeline, `NameParsedFormula`,
`UncheckedRfX`, `UncheckedSqRfX`, `XlNullableWideString`, `XlWideString`, and
the existing worksheet-part writer.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/worksheet.rs`
- `rust-xlsb-writer/src/sheet.rs`
- `rust-xlsb-writer/src/defined_name.rs`
- `rust-xlsb-core/src/prelude/record/brt_name.rs`
- `thirdparty/rust_xlsxwriter/src/data_validation.rs`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs` validation XML assembly
- MS-XLSB §2.4.55 `BrtBeginDVals`, §2.4.356 `BrtDVal`, §2.4.411
  `BrtEndDVals`, §2.5.36 `DVals`, §2.5.37 `DValStrings`, §2.5.98.8
  `DVParsedFormula`, §2.5.156 `UncheckedSqRfX`

## Compatibility Boundary

- Public names and signatures match the reference for the implemented subset:
  - `DataValidation::new`
  - `allow_whole_number(DataValidationRule<i32>)`
  - `allow_decimal_number(DataValidationRule<f64>)`
  - `allow_date(DataValidationRule<impl IntoExcelDateTime + IntoDataValidationValue>)`
  - `allow_time(DataValidationRule<impl IntoExcelDateTime + IntoDataValidationValue>)`
  - `allow_text_length(DataValidationRule<u32>)`
  - `allow_list_strings(&[impl AsRef<str>]) -> Result<DataValidation>`
  - `allow_any_value`
  - `ignore_blank`, `show_dropdown`, `show_input_message`,
    `show_error_message`, `set_input_title`, `set_input_message`,
    `set_error_title`, `set_error_message`, `set_error_style`
  - `Worksheet::add_data_validation(first_row, first_col, last_row, last_col,
    &DataValidation)`
- `DataValidationRule` exposes all eight comparison variants.
- `IntoDataValidationValue` is implemented for `i32`, `u32`, `f64`,
  `ExcelDateTime`, and `&ExcelDateTime`.
- Formula-based methods (`allow_*_formula`, `allow_custom`,
  `allow_list_formula`) and `set_multi_range` are out of scope until a public
  formula type exists. Their absence is explicit, not a silent conversion.
- Lists are limited to 255 UTF-16 code units including commas, matching the
  reference. `PtgStr` is limited to 255 code units.
- Input and error titles are limited to 32 UTF-16 code units; input messages to
  255; error messages to 225 per MS-XLSB §2.5.37.
- Default output is unchanged when no validation is added.
- Only one cell range per validation; the caller can add multiple validations.

## Change Necessity

The public `add_data_validation` surface and worksheet contract cannot exist
without MS-XLSB records. The minimum code boundary is one validation model, four
record/structure definitions, worksheet storage, and sheet emission.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API, new binary records, and worksheet-part output.
- Test posture: add failing record round-trip tests, rule/limit tests, worksheet
  range tests, and package decode/order tests first; implement to green; add
  regressions for absent output, invalid Any, list limits, and formula token
  shapes.
- Verification: exact record bytes, `BrtDVal` decoded fields and formulas,
  package record order, and external LibreOffice XLSX round-trip inspection.

## Ripple Signal Triage

- Canonical owner: `Worksheet` owns the validation list; `sheet` is the only
  emitter; core owns the binary records.
- Downstream consumers: workbook save/prepare (no changes needed), worksheet
  part, and `rust-xlsb-core` prelude exports/test table.
- Source-of-truth: worksheet validation list. No validation state is duplicated
  in workbook or package.
- Compatibility risk: no validation means no records; all new output is gated
  on `add_data_validation`.
- Retirement: none; this is additive.

## Tasks

1. Add core records and structures in
   `rust-xlsb-core/src/prelude/record/brt_dval.rs`, register modules/exports, and
   add `checked_test` cases. Tests cover default byte sizes (`BrtBeginDVals` 18,
   `BrtEndDVals` 0, `BrtDVal` 56), bitfield flags, range limits, string limits,
   and formula-count invariants.
2. Add `data_validation.rs` with the public API, rule normalization, limits,
   `PtgNum`/`PtgStr` encoding, and `BrtDVal` conversion. Tests cover every
   operator, list escaping, title/message limits, date serials, and Any skip.
3. Add `Worksheet::add_data_validation` and storage. Tests cover range order,
   row/column limits, multiple validations, and clone independence.
4. Emit `DVALS` from `sheet::write_sheet` in ABNF position. Package tests decode
   the record order and fields, assert `idvMac`, and assert no `DVALS` for an
   unused or invalid-Any validation.
5. Update public exports, `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`
   (`Partial`, with formula modes called out), this plan, and `INDEX.md`.
6. Run `cargo test --workspace`, all-features tests, clippy, fmt, and
   `git diff --check`. Build a scratch workbook with list, numeric, and prompt
   validations; convert it to XLSX with LibreOffice and inspect
   `dataValidation` entries.

## Verification

- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `git diff --check`
- Package record decode for one list validation, one between validation, and
  one invalid Any.
- LibreOffice XLSX conversion exposes the expected `dataValidation` type,
  operator, formula1/formula2, prompt, and error text.

## Risks

- `BrtDVal` uses one 32-bit flag word split into mixed bit widths; a wrong bit
  order silently changes validation type or flags. Record decode tests and a
  LibreOffice round-trip are the guard.
- `PtgStr` stores raw UTF-16 without outer quotes; list strings need quote
  stripping and doubled-quote unescaping.
- Formula-valued validation is deliberately absent; users needing a cell-range
  list or custom formula stay on the roadmap.
- LibreOffice may normalize or drop some options on XLSB import. Record-level
  assertions remain the primary proof; the XLSX round-trip is supplementary.

## Review

Reviewer `DataValidationReview` found two issues:

1. Chrono rule values were routed through range-checked `ExcelDateTime`
   constructors, so valid pre-1900 dates and near-second times could panic.
   Replaced with direct serial math matching the reference: epoch subtraction,
   1900 leap-day correction, and millisecond time fractions. Added pre-1900 and
   59.9995-second tests.
2. `BrtDVal::check` did not enforce the `DVParsedFormula` 16,384-byte `cce`
   limit. Added the check and a boundary test.

Record layout, flag packing, `DVals`, `DValStrings`, formula tokens, DVALS
placement, absent output, and allocation behavior were checked without
blocking findings.

Verification after fixes: 212 core + 252 writer = 464 default tests, 468
all-features; clippy, fmt, and diff check clean. LibreOffice XLSX round-trip
showed the list, whole-number, and date validations with the expected list
source, prompt/error text, warning style, allow-blank/dropdown flags, operators,
and numeric/date bounds.
