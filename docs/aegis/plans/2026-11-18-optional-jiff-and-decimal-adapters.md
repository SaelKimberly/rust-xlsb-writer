# Optional jiff and rust_decimal adapters

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: Optional `jiff` civil date/time writes and optional
  `rust_decimal::Decimal` writes.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.7 and §5.
- Reference: vendored `rust_xlsxwriter` v0.102.0, `jiff` conversions in
  `datetime.rs`/`worksheet.rs` and the `rust_decimal` `IntoExcelData` impl.

## Goal

Close the two smallest data-ecosystem gaps without changing default output or
pulling either dependency into default builds. A caller that enables `jiff` can
write borrowed `jiff` civil values; a caller that enables `rust_decimal` can
write `Decimal` values.

## Architecture

`CellData` remains the single context-free value type. Both adapters are
implemented in `data.rs`, next to every other `IntoExcelData` implementation.

- `jiff` values map to the existing `chrono` `CellData` variants. The core
  temporal conversion then owns serial arithmetic, the 1900 leap-day rule,
  millisecond rounding, and range checks. No second serial implementation.
- `Decimal` maps to `CellData::Number` through `ToPrimitive::to_f64`.
  Conversion is allocation-free; a value that cannot be represented returns
  `Error::ValueNotRepresentable` with a static message.

Both dependencies are optional and tied to same-named crate features. No
default feature is added, so default package bytes and the dependency graph stay
unchanged unless requested.

## Tech Stack

Rust 1.100.0, edition 2024, `jiff 0.2` without default features,
`rust_decimal 1.40`, existing `IntoExcelData`, `IntoExcelDateTime`, `CellData`,
and `Error`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/data.rs`
- `rust-xlsb-core/src/temporal.rs`
- `thirdparty/rust_xlsxwriter/src/datetime.rs::jiff_date_to_excel`,
  `jiff_time_to_excel`, `jiff_datetime_to_excel`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs` jiff and `rust_decimal`
  `IntoExcelData` implementations
- `thirdparty/rust_xlsxwriter/Cargo.toml` feature definitions

## Compatibility Boundary

- Features: `jiff` and `rust_decimal`, both off by default.
- `jiff` support matches the reference: borrowed `&jiff::civil::Date`,
  `&jiff::civil::Time`, and `&jiff::civil::DateTime` implement
  `IntoExcelData`. `IntoExcelDateTime` covers the same borrowed types.
- `rust_decimal` support: owned `Decimal` implements `IntoExcelData`; both
  formatted and unformatted writes convert once.
- `jiff` civil values outside the chrono range cannot map to `CellData`; they
  return `Error::ValueNotRepresentable`. Values inside chrono range but outside
  Excel's 1900..9999 range follow the existing chrono path and are rejected
  during conversion.
- `Decimal` uses its exact `to_f64` value; Excel still stores an `f64`, so
  precision beyond binary64 is the caller's contract.
- No public API method, BIFF record, package part, or default feature changes.

## Change Necessity

A direct jiff-to-Excel serial implementation would duplicate the core date
system and rounding rules, and would bypass the existing `CellData::Date` /
`Time` / `DateTime` classification used by autofit. Mapping through `chrono`
adds one allocation-free conversion and preserves one source of truth.

## TDD Route

- Mode: auto
- Decision: light
- Authority: repository test convention; no strict TDD request.
- Test posture: feature-gated unit tests assert the mapped `CellData` variants
  and formatted writes.
- Reason: thin adapters over already-tested conversion paths.

## Tasks

### 1. Features and conversions

Files: `rust-xlsb-writer/Cargo.toml`, `rust-xlsb-writer/src/data.rs`,
`rust-xlsb-writer/src/lib.rs`.

- Add optional `jiff` with `default-features = false`; feature
  `jiff = ["dep:jiff"]`.
- Add optional `rust_decimal`; feature `rust_decimal = ["dep:rust_decimal"]`.
- Add `jiff` date/time/datetime helpers that build `chrono` values.
- Implement `IntoExcelDateTime` and `IntoExcelData` for the borrowed `jiff`
  types.
- Implement `IntoExcelData` for `Decimal`, formatted and unformatted, with one
  `to_f64` call per write.
- Document the feature in crate-level docs.

Verification: `cargo check -p rust-xlsb-writer --features jiff,rust_decimal`.

### 2. Tests

- Write a jiff date, time, and datetime; assert `CellData` variants and values.
- Write a referenced jiff value with a format; assert the same mapped value and
  that the format is recorded.
- Write a `Decimal` both with and without a format; assert `CellData::Number`.
- Keep tests behind the matching feature gate.

Verification: `cargo test -p rust-xlsb-writer --all-features data::tests`.

### 3. Documentation and verification

Files: `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`,
`docs/aegis/INDEX.md`.

- Mark `jiff` and `rust_decimal` as supported optional inputs.
- Update the summary and test-count wording for default versus all-feature
  tests.
- Run `cargo test --workspace`, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets --all-features`,
  `cargo fmt --all --check`, and `git diff --check`.

## Verification

Result: default builds pass 193 core + 194 writer unit tests and 6 doctests.
`--all-features` passes 193 core + 197 writer unit tests and 6 doctests. Clippy,
fmt, and diff checks pass. Reviewer `OptionalAdaptersReview` found no defects;
it confirmed feature-off builds keep `jiff`/`rust_decimal` out of the dependency
graph, jiff values reach the core serial/range rules, and `Decimal` converts
without allocating.

## Risks

- `jiff` values enter through `chrono`, whose years are wider than Excel's; the
  core range check still rejects out-of-range dates at conversion time.
- `Decimal` to `f64` is lossy beyond 15–17 significant digits. This is the
  reference behavior and an inherent Excel number limit.
- Feature-gated code is not exercised by a plain `cargo test`; all-features
  checks must run before release.

## Retirement

No old path is removed. Default builds keep the existing dependency set and
behavior.
