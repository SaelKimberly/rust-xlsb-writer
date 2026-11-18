# Worksheet default format

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: `Worksheet::set_default_format` for detached worksheets.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.3.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Worksheet::set_default_format` and `initialize_default_format`.

## Goal

Add the worksheet-level `set_default_format` API so a detached `Worksheet` can
carry the same default style and dimensions as its parent workbook. Keep
existing package output unchanged when the API is unused.

## Architecture

The worksheet already owns the default style, row height, and converted column
width after a workbook applies its defaults. The new public method validates
and writes that same state before any other format exists.

`Workbook::set_default_format` and `Worksheet::set_default_format` share one
supported-width table. Move that table to `worksheet.rs` and import it from
`workbook.rs`; no duplicate conversion logic.

The package still owns one global XF 0. A detached worksheet's default format
becomes effective only when the parent workbook is given the same default,
matching the reference contract.

## Tech Stack

Rust 1.100.0, edition 2024, existing `Format`, `Worksheet`, `Workbook`, and
`Error` types.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/worksheet.rs::initialize_default_format`
- `rust-xlsb-writer/src/workbook.rs::set_default_format`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs::set_default_format`
- `thirdparty/rust_xlsxwriter/src/workbook.rs::set_default_format`

## Compatibility Boundary

- Public signature:
  `pub fn set_default_format(&mut self, format: &Format, row_height: u32, col_width: u32) -> Result<&mut Self>`.
- Rejects unsupported column pixel widths with `Error::Parameter`.
- Rejects row heights above 409.5 points with `Error::ValueNotRepresentable`.
- Rejects the call after any cell or row/column format has been added, matching
  the reference's `xf_formats` guard. Unformatted data may already exist.
- The parent workbook must receive the same default via
  `Workbook::set_default_format`; otherwise style 0 remains the workbook
  default. This matches the reference documentation.
- No new record, package, or style-registry path.

## Change Necessity

The workbook path already has the conversion and validation rules, but they are
private to `workbook.rs`. The smallest complete change is one public worksheet
method plus moving the shared width table; adding a package path would create a
second source of truth for global XF 0.

## TDD Route

- Mode: auto
- Decision: light
- Authority: repository test convention; no strict TDD request.
- Test posture: add focused tests for the format guard, width/height
  validation, and applied dimensions/hyperlink style.
- Reason: the method changes public worksheet state but reuses an existing
  validated path.

## Tasks

### 1. Shared validation and public method

Files: `rust-xlsb-writer/src/worksheet.rs`,
`rust-xlsb-writer/src/workbook.rs`.

- Move `default_column_metrics` from `workbook.rs` to `worksheet.rs` as
  `pub(crate)`.
- Add `Worksheet::set_default_format` with the reference guard order: reject
  when any format exists, then validate the column width and row height.
- Store the cloned format, row height in points, and converted column width via
  `initialize_default_format`.
- Keep `Workbook::set_default_format` on the same helper.

Verification: `cargo test -p rust-xlsb-writer worksheet::tests`.

### 2. Tests

- Accept the method before any format, even after unformatted writes.
- Reject after `write_with_format`.
- Reject unsupported column widths and over-limit row heights.
- Assert default row/column dimensions and that hyperlinks use the worksheet
  default style.

Verification: targeted writer tests.

### 3. Documentation and verification

Files: `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`,
`docs/aegis/INDEX.md`, `docs/aegis/plans/2026-11-18-workbook-default-format.md`.

- Mark `Worksheet::set_default_format` supported.
- Note the worksheet API in the public writer summary.
- Update the earlier plan's stale “remains missing” boundary.
- Update test counts.
- Run `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `cargo fmt --all --check`, and `git diff --check`.

## Verification

Result: 193 core + 187 writer unit tests and 6 doctests pass. Clippy, fmt, and
diff checks pass. Reviewer `WorksheetDefaultReview` found no code defects; its
only finding was this plan's pending status.

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- Target tests: guard after a formatted write, unsupported width, over-limit
  row height, applied dimensions, and hyperlink style.

## Risks

- A worksheet default without the matching workbook default does not change XF
  0. The reference has the same requirement; the docs state it.
- The guard uses the worksheet's interned-format table, so any formatted write
  blocks later default changes.
- Default format never changes default output unless called.

## Retirement

No old path is removed. The shared width table replaces the former
workbook-private copy.
