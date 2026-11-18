# ExcelDateTime value type

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: Native `ExcelDateTime` constructor, parser, serial conversion, and
  cell writes.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.7 and §5.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `datetime.rs` and the
  `ExcelDateTime` `IntoExcelData` implementations in `worksheet.rs`.

## Goal

Add the source-compatible `ExcelDateTime` value type so callers can build and
write Excel-native dates, times, and datetimes without an external date crate.
Dates, times, and datetimes are written as Excel serial numbers through the
existing cell pipeline. No package part or record changes.

## Architecture

`ExcelDateTime` is a writer value type in the new `rust-xlsb-writer/src/datetime.rs`.
It stores validated date/time fields until a serial is requested, then converts
to an Excel `f64` serial matching the reference type.

The existing chrono path remains authoritative for chrono, jiff, and
`TimeDelta` values. `ExcelDateTime` cannot delegate to it because the native
type must also represent:

- Excel's 1900-02-29 fake leap day, which `chrono::NaiveDate` rejects;
- hours at or above 24, which `chrono::NaiveTime` rejects;
- the 1899-12-31 serial zero.

The serial math therefore mirrors the reference exactly, while the writer keeps
one cell conversion path: `IntoExcelDateTime` returns
`CellData::ExcelDateTime`, and `sheet.rs` encodes its serial as Rk integers or
reals through the existing `NumberPolicy`. The variant also keeps autofit on
the same 68-pixel temporal width as chrono dates.

`parse_from_str` splits with an iterator and fixed scalars rather than
collecting tokens into a `Vec`. Construction, serial conversion, and writing
allocate nothing.

## Tech Stack

Rust 1.100.0, edition 2024, `chrono` for timestamp decomposition only, existing
`IntoExcelDateTime`, `IntoExcelData`, `CellData`, `Format`, and `Error`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/data.rs`
- `rust-xlsb-writer/src/sheet.rs`
- `rust-xlsb-writer/src/error.rs`
- `rust-xlsb-writer/src/lib.rs`
- `thirdparty/rust_xlsxwriter/src/datetime.rs`
- `thirdparty/rust_xlsxwriter/src/datetime/tests.rs`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs` `IntoExcelData for
  ExcelDateTime`

## Compatibility Boundary

- Public type: `ExcelDateTime` with `Default`, `Clone`, `Debug`, and
  `PartialEq`.
- Constructors: `from_ymd`, `from_hms`, `from_hms_milli`,
  `from_serial_datetime`, `from_timestamp`, and `parse_from_str`.
- Chaining: `and_hms`, `and_hms_milli`.
- Serial output: `to_excel`.
- Errors: `Error::DateTimeRangeError(String)` and
  `Error::DateTimeParseError(String)`, matching reference variant names.
- Writes: owned and borrowed `ExcelDateTime` implement both `IntoExcelDateTime`
  and `IntoExcelData`, formatted and unformatted.
- No dependencies, features, records, package parts, or default bytes change.
- Validation matches the reference: 1899-12-31 through 9999-12-31, including
  1900-02-29; hours are unrestricted; minutes accept through 60; fractional
  seconds must be below 60.

## Change Necessity

A docs-only change cannot supply a native value type, and reusing chrono cannot
represent the two Excel-specific cases above. The minimum code boundary is one
value type, its serial conversion, and the two cell trait implementations.

## Ripple Signal Triage

`ExcelDateTime` is a public value type and a new producer for `CellData`. The
canonical owners stay `data.rs` for cell conversion and `sheet.rs` for record
selection; no second cell path is added. Verification must cover validation,
serial boundaries, parsing without allocation, formatted writes, and a
package round-trip through calamine.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: public API and new behavior.
- Test posture: write focused unit tests for validation, serial boundaries,
  parsing, and trait mapping; add one package round-trip.
- Verification: `cargo test -p rust-xlsb-writer`, then workspace commands.

## Plan Pressure Test

Owner fit: writer-only value type; core temporal code is unchanged.
Contract fit: reference method names and error variant names.
Retirement: no old path is retained or replaced.
Executability: one new module plus `data.rs`, `error.rs`, `lib.rs`, and one e2e
test.

## Tasks

### 1. Value type and errors

Files: `rust-xlsb-writer/src/datetime.rs`, `rust-xlsb-writer/src/error.rs`,
`rust-xlsb-writer/src/lib.rs`.

- Add range and parse error variants.
- Add `ExcelDateTime` field storage, default, constructors, chaining methods,
  validators, `parse_from_str`, `from_timestamp`, and `to_excel`.
- Re-export the type from the crate root.
- Document the reference-specific 1900 leap day and unrestricted hour in the
  module docs.

Verification: `cargo check -p rust-xlsb-writer`.

### 2. Cell conversion

Files: `rust-xlsb-writer/src/data.rs`.

- Implement `IntoExcelDateTime` for owned and borrowed `ExcelDateTime`.
- Add the type to the existing datetime `IntoExcelData` macro expansion.

Verification: `cargo test -p rust-xlsb-writer data::tests`.

### 3. Tests and docs

Files: `rust-xlsb-writer/src/datetime.rs`,
`rust-xlsb-writer/src/e2e/mod.rs`, `docs/gap-analysis-rust-xlsxwriter.md`,
`AGENTS.md`, `docs/aegis/INDEX.md`.

- Assert validation boundaries, serial values including 1899-12-31 and
  1900-02-29, timestamp conversion, parsed date/time/datetime values, and
  formatted/unformatted cell mapping.
- Add a package round-trip writing date, time, and datetime values and reading
  them back through calamine.
- Move `ExcelDateTime` from Missing to Supported and update counts.
- Run `cargo test --workspace`, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features`,
  `cargo fmt --all --check`, and `git diff --check`.

## Verification

Result: default builds pass 193 core + 201 writer unit tests and 7 doctests.
`--all-features` passes 193 core + 204 writer unit tests and 7 doctests. Clippy,
fmt, and diff checks pass. Reviewer `ExcelDateTimeReview` found no blocking
defects; it differentially checked serials across 1900-1903, 1999-2100, 2400,
4000, and 9998-9999 with zero mismatches, and confirmed the parser allocates no
token storage and date-only serials remain Rk integers. A post-review advisory
noted that mapping to `CellData::Number` gave autofit a numeric width; the final
implementation keeps `CellData::ExcelDateTime`, so autofit uses the temporal
width while serialization remains unchanged.

## Risks

- The native serial math intentionally duplicates the reference formula rather
  than the core chrono path; tests pin the boundary values to catch drift.
- Sub-millisecond seconds survive `to_excel` like the reference, but the
  workbook stores the value as an Excel serial `f64`; display precision is
  caller-controlled through number formats.
- `from_timestamp` uses `chrono` to decompose seconds and then the native
  serial formula; the accepted timestamp range remains reference-compatible.

## Retirement

No old path is removed. Existing chrono, jiff, and duration conversion remain
unchanged.
