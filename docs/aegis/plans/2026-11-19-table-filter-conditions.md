# Table filter conditions

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Table::filter_column`, filter records inside the Table part, and
  automatic table-row hiding.
- Out of scope: table filter buttons toggled by Excel at runtime, top-10/dynamic
  table filters, table sort state, and structured table columns with spaces.

## Goal

Close the remaining `add_table` gap from
`docs/gap-analysis-rust-xlsxwriter.md`: a table AutoFilter can carry filter
conditions for its columns, not only a range. Conditions use the existing
`FilterCondition` API, so callers can add list, blank, and custom comparisons
without a second model.

```rust
let table = Table::new()
    .filter_column(1, &FilterCondition::new().add_list_filter("East"));

worksheet.add_table(0, 0, 9, 3, &table)?;
```

`Table::filter_column` takes a zero-based column index **within the table**.
`add_table` translates it to the worksheet column after the table range is
known.

## Architecture

`Table` remains the owner of table-part state. It gains a
`BTreeMap<ColNum, FilterCondition>` of table-relative conditions, matching the
existing `Worksheet` filter map. At `add_table`, conditions are validated
against the table width and translated to absolute worksheet columns in the
stored `TableRange`; the public `Table` value is not mutated.

`src/filter.rs` becomes the owner of filter-record serialization. The existing
`sheet.rs` private `write_filter_columns`/`custom_filter` move there unchanged,
taking a condition map and range. Both worksheet and table serializers call the
same function, so `BrtBeginFilterColumn`, `BrtBeginFilters`/`BrtFilter`, and
`BrtBeginCustomFilters`/`BrtCustomFilter` stay one implementation.

Table part order follows MS-XLSB `2.1.7.51`:
`BrtBeginList [AUTOFILTER] [SORTSTATE] LISTCOLS BrtTableStyleClient
BrtEndList`. The serializer writes `BrtBeginAFilter`, the filter columns, and
`BrtEndAFilter` before `BrtBeginListCols`, as it already does for the empty
AutoFilter range.

Automatic row hiding is a worksheet operation because the cells and row
records live there. `Worksheet::apply_filters` extends its existing pass to
each table's data band, using the translated absolute columns. Table filter
rows are hidden exactly like worksheet AutoFilter rows. The total row and
header row are never candidates.

## Tech Stack

Rust 1.100.0, edition 2024, existing deku record types, `FilterCondition`
matching, and `BTreeMap`. No new dependency.

## Baseline And Authority Refs

- `docs/gap-analysis-rust-xlsxwriter.md`: table row says table filter conditions
  remain.
- MS-XLSB `2.1.7.51` Table part ABNF; `2.4.100` `BrtBeginList`; `2.4.4`
  `BrtBeginAFilter`; filter-column records already used by the worksheet.
- `rust-xlsb-writer/src/table.rs`, `src/sheet.rs`, `src/filter.rs`,
  `src/worksheet.rs`.
- Existing worksheet filter tests in `src/package.rs`.

## Compatibility Boundary

- Tables without `filter_column` calls stay byte-identical.
- `Table::filter_column` is additive; `Table` remains `Clone`.
- A filter condition with no autofilter, no header row, an out-of-table column,
  or no values returns `Error::Parameter` at `add_table`, before any package
  bytes are written.
- Table filters are same-sheet only. Excel runtime filter-button behavior
  remains Excel's responsibility.
- `Worksheet::filter_automatic_off` continues to disable only worksheet
  AutoFilter hiding; table filters do not silently inherit that flag.

## TDD Route

- Mode: auto
- Decision: strict for record order, validation, and hidden-row behavior.
- Authority: new public API and producer behavior.
- Test posture: add failing tests for table filter records and row hiding
  before implementation; keep existing worksheet filter tests green.
- Verification: focused tests, then full workspace tests, clippy, and fmt.

## Change Necessity

The gap is behavioral, not documentary: the Table part currently writes an
empty `BrtBeginAFilter`/`BrtEndAFilter` pair, so Excel sees filter buttons but no
saved conditions. Reusing the existing worksheet filter serializer is the
minimum code boundary; no new filter model, record type, or package part is
needed.

## Ripple Signal Triage

- Canonical owner: `Table` for table conditions, `filter.rs` for filter-record
  serialization, `Worksheet` for row visibility.
- Downstream consumers: Table part writer, worksheet save pipeline, package
  tests.
- Source of truth: public `Table` builder value; translated absolute columns
  are internal save state.
- Retirement: delete `sheet.rs`'s private filter serializer after both callers
  use `filter.rs`.
- No fallback or compatibility branch is retained.

## Tasks

1. **Add the table API and validation.**
   - Files: `src/table.rs`, `src/worksheet.rs`, `src/filter.rs`.
   - Add `Table::filter_column(self, col: ColNum, condition: &FilterCondition)
     -> Self`, storing a clone in a table-relative `BTreeMap`.
   - Add `FilterCondition::validate()` and use it from
     `Worksheet::filter_column` and `add_table`, preserving existing worksheet
     error text.
   - In `add_table`, reject filters when `show_autofilter` or
     `show_header_row` is false, reject empty conditions and columns outside
     the table width, then translate keys to absolute columns in the stored
     `TableRange`.
   - Tests: valid table filter is stored relative before add; rejected cases
     leave worksheet tables unchanged.

2. **Move filter serialization to `filter.rs`.**
   - Files: `src/filter.rs`, `src/sheet.rs`.
   - Move `write_filter_columns` and `custom_filter` with `pub(crate)`
     visibility. Keep worksheet output byte-identical.
   - Test: existing package filter test still asserts the same record order and
     values.

3. **Emit table filter records.**
   - Files: `src/table.rs`.
   - Inside the table AutoFilter block, write filter columns before
     `BrtEndAFilter` using the shared serializer.
   - Test: decode `BrtBeginFilterColumn` and `BrtFilter` from the Table part;
     assert `BrtBeginAFilter < BrtBeginFilterColumn < BrtEndAFilter <
     BrtBeginListCols`, and `dwCol` is table-relative.

4. **Hide table-filtered rows.**
   - Files: `src/worksheet.rs`.
   - Extend `apply_filters` to run each table's conditions over
     `rwFirst + 1 ..= rwLast - total_rows` (header row always present for
     filters), using absolute columns and existing `FilterCondition::matches`.
   - Test: a filtered table hides only non-matching data rows; header and total
     rows remain visible; clearing the condition restores rows.

5. **Update docs and close the loop.**
   - Files: `docs/gap-analysis-rust-xlsxwriter.md`, `AGENTS.md`, this plan.
   - Mark table filter conditions shipped; document table-relative column
     indices and automatic hiding.
   - Run full workspace tests, clippy, fmt, and a package decode check.

## Verification

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo clippy --workspace --all-targets --all-features
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo fmt --all -- --check
```

A generated table with a list filter must decode with one table
`BrtBeginFilterColumn` and one `BrtFilter`, and its non-matching data rows must
be hidden in the worksheet part.

## Review

`TableFilterReview` found no P0/P1/P2 defects. Two P3 findings were fixed:

- `filter_automatic_off` docs implied table filters respected the flag; both
  the method and `filter_column` docs now scope it to worksheet AutoFilter
  records and state that table filters always hide non-matching data rows.
- The package test used a zero-offset table, so it could not distinguish
  relative from absolute filter columns. It now starts at row 1/column 2,
  filters table-relative column 1, and asserts `dwCol == 1` plus the hidden row.

Final verification: 667 unit tests and 7 doctests default (290 core, 377
writer); 671 unit tests and 7 doctests with all features; clippy and fmt clean.

## Risks

- Table conditions store absolute columns after `add_table`; the package test
  now starts at column 2 and proves the Table part writes the table-relative
  `dwCol`.
- `apply_filters` no longer returns early when no worksheet autofilter exists;
  existing worksheet filter tests still pass.
- LibreOffice may drop XLSB table parts on import. Record and package decode
  remain the oracle.
