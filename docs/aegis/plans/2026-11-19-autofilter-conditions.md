# Autofilter column conditions

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `FilterCondition`/`FilterCriteria`/`FilterData`/`IntoFilterData`,
  `Worksheet::filter_column`, `Worksheet::filter_automatic_off`, filter records,
  and automatic row hiding for worksheet autofilters.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4 `filter_column`,
  `filter_automatic_off`.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `filter.rs`,
  `Worksheet::filter_column`, `hide_autofilter_rows`, and filter XML assembly.

## Goal

Make an existing `autofilter` range filterable. Users can set list filters,
blank-list filters, and one or two custom comparison filters per column. The
writer emits the matching MS-XLSB filter records and hides rows that do not
match, like Excel does at runtime, unless automatic hiding is turned off.

This closes the current “range record only” gap without adding tables,
conditional formatting, or a filter UI model.

## Aegis Visibility

The change adds a public API and new worksheet-part contracts (`BrtFilter`,
`BrtCustomFilter` families) plus row visibility coupling, so a plan keeps owner,
record order, and verification boundaries explicit.

## Architecture

`rust-xlsb-writer/src/filter.rs` owns the public filter vocabulary and match
logic. `Worksheet` stores `BTreeMap<ColNum, FilterCondition>` plus a transient
`BTreeSet<RowNum>` of filter-hidden rows.

`filter_column` validates the autofilter range and condition, then stores or
replaces the column condition. `autofilter` clears old conditions when its range
changes. `filter_automatic_off` suppresses row hiding but keeps records.

`Workbook::prepare` recalculates filter-hidden rows from current cell data on
every save. `sheet::write_sheet` includes those rows in the emitted row set,
marks `BrtRowHdr.fDyZero`, and uses height 0. User-hidden rows remain independent
and are never unhidden by filtering.

`package` emits `BrtBeginFilterColumn`/`BrtBeginFilters`/`BrtFilter` for list
filters and `BrtBeginCustomFilters`/`BrtCustomFilter` for custom filters between
`BrtBeginAFilter` and `BrtEndAFilter`. Filter records are absent when no
condition exists, so existing autofilter-only output stays unchanged.

## Tech Stack

Rust 1.100.0, edition 2024, existing deku record pipeline,
`XlWideString`, `BrtBeginAFilter`, `BrtRowHdr`, and `BTreeMap`/`BTreeSet`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/worksheet.rs`
- `rust-xlsb-writer/src/sheet.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-core/src/prelude/record/brt_a_filter.rs`
- `thirdparty/rust_xlsxwriter/src/filter.rs`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs` filter evaluation and XML
  assembly
- MS-XLSB §2.4.84 `BrtBeginFilterColumn`, §2.4.85 `BrtBeginFilters`,
  §2.4.348 `BrtCustomFilter`, §2.4.682 `BrtFilter`, §3.4 filter example

## Compatibility Boundary

- Public API matches the reference names and signatures:
  - `FilterCondition::new`, `add_list_filter`, `add_list_blanks_filter`,
    `add_custom_filter`, `add_custom_boolean_or`
  - `FilterCriteria` all twelve variants
  - `FilterData::new_string_and_criteria`, `FilterData::new_number_and_criteria`
  - `IntoFilterData` for `f64`, `i32`, and `&str`
  - `Worksheet::filter_column(col, &FilterCondition)`
  - `Worksheet::filter_automatic_off()`
- One condition per column; a second call replaces it.
- `filter_column` requires an existing `autofilter` and a column inside its
  range.
- Matching covers strings, integers, floating numbers, blank cells, and missing
  cells. Booleans, errors, and temporal cells do not match list/number filters;
  this is a documented subset, not a silent value conversion.
- Custom string comparisons use reference semantics: trim, lowercase, and
  Rust lexical ordering; wildcard criteria map to Excel’s `*` patterns.
- `NotEqualTo` with the single-space string is encoded as Excel’s non-blank
  custom filter. Blank matching uses `add_list_blanks_filter`, matching the
  reference API.
- Filter icon metadata, top-10/dynamic filters, tables, and date-group filters
  are out of scope.
- Default output is unchanged: no condition means no filter records and no
  filter-hidden rows.

## Change Necessity

`BrtBeginAFilter` exists but carries only the range. The public filter API
cannot be meaningful without MS-XLSB filter records, and Excel does not hide
rows from file metadata alone. The minimum code boundary is one filter model,
six record types plus markers, package emission, and save-time row visibility.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API, new binary records, and row state consumed by the
  worksheet part.
- Test posture: add failing record, API, package, and row-visibility tests
  first; implement to green; then add regressions for replacement, reset, and
  automatic-off behavior.
- Verification: exact record bytes, package record order, `BrtRowHdr` flags, and
  default-output absence.

## Ripple Signal Triage

- Canonical owner: `Worksheet` for filter conditions and hidden-row state;
  `package`/`sheet` remain the only emitters.
- Downstream consumers: workbook save/prepare, worksheet autofilter range, and
  row emission.
- Source-of-truth: worksheet cells and filter conditions. No duplicate row
  owner is added; filter-hidden rows are transient and rebuilt each save.
- Compatibility risk: hidden rows alter output only when conditions exist;
  `filter_automatic_off` and no filters preserve current bytes.

## Plan Pressure Test

The smallest complete path is filter types plus records plus hidden rows.
Exporting only or storing conditions without records would be a stub. Adding
tables or conditional formats would cross the gap-analysis feature boundary.

## Tasks

### Task 1: RED — record and public API contracts

**Files:**
- Add: `rust-xlsb-core/src/prelude/record/brt_filter.rs`
- Modify: `rust-xlsb-core/src/prelude/record/mod.rs`
- Modify: `rust-xlsb-core/src/prelude/mod.rs`
- Add: `rust-xlsb-writer/src/filter.rs`
- Modify: `rust-xlsb-writer/src/lib.rs`
- Modify: `rust-xlsb-writer/src/worksheet.rs`
- Modify: `rust-xlsb-writer/src/package.rs`

Add failing tests for exact record bytes, list/custom record order, API
validation, default absence, hidden-row flags, blanks, replacement, autofilter
reset, and automatic-off behavior.

### Task 2: GREEN — MS-XLSB filter records

**Files:** record modules and prelude exports from Task 1.

Implement `BrtBeginFilterColumn` (relative column, hide/no-button bits),
`BrtBeginFilters` (blank flag), `BrtFilter`, `BrtBeginCustomFilters`,
`BrtCustomFilter` (numeric, string, blank, non-blank), and end markers.
Register each through `assign_id!`, `Checked`, prelude re-exports, and the
`test_biff_export` table.

Verification: record table and exact-byte tests pass.

### Task 3: GREEN — filter model and worksheet integration

**Files:** `filter.rs`, `worksheet.rs`, `lib.rs`, `workbook.rs`.

Implement reference-compatible builders and matching. Store conditions, clear
them when the autofilter range changes, validate columns, and recalculate
transient hidden rows during `Workbook::prepare`.

Verification: API and row-match tests pass, including list and custom filters,
blanks, OR, missing cells, and automatic-off.

### Task 4: GREEN — package and worksheet emission

**Files:** `package.rs`, `sheet.rs`.

Emit filter columns inside the autofilter collection and include
filter-hidden rows with `fDyZero` and zero height. Preserve autofilter-only
output and record order.

Verification: package decode tests assert order, relative `dwCol`, custom `fAnd`,
string/number payloads, and hidden row flags.

### Task 5: REFACTOR — docs and evidence

**Files:**
- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `AGENTS.md`
- Modify: `docs/aegis/INDEX.md`

Mark filter conditions supported with their boundary, update test counts and
verification notes, then run the commands below.

## Verification

- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `git diff --check`
- Package tests decode filter records and row flags; calamine does not expose
  autofilter conditions, so record-level tests are the oracle. LibreOffice and
  SheetJS behavior is recorded as a limitation, not a passing UI test.

## Risks

- Filter-hidden rows are saved state; recomputation each save prevents stale
  hidden rows after data or condition changes.
- Custom string comparison uses lowercase allocations to match the reference;
  counts are bounded by rows in the autofilter range.
- Temporal and boolean cells are outside the matching subset; keeping them
  unmatched is safer than guessing a serial conversion.
- `BrtCustomFilter` string/number variants share one record id; the optional
  string field is conditional on `vts == 0x06`.

## Review

`AutofilterReview` found one blocking defect and no layout or row-state errors:

- `BrtBeginCustomFilters.fAnd` was inverted. MS-XLSB §2.4.40 defines `0` as
  AND and `1` as OR; the emitter now uses `apply_logical_or` only when there are
  two criteria, and tests cover both AND and OR.
- Package tests now assert the ordered record sequence, not just per-type
  counts.
- List filters now match numeric text cells such as `"4000"`, matching the
  reference's "numbers stored as strings" behavior.
- Test counts were corrected after the review additions.

## Retirement

No old filter path exists to retire. The existing autofilter-only emitter keeps
its behavior when no conditions are present; no compatibility shim is added.
