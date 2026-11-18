# Basic tables

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `Table`, `TableColumn`, `TableStyle`, `Worksheet::add_table`, the MS-XLSB
  Table part records, and worksheet `LISTPARTS` relationships.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4 `add_table`.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `table.rs` and
  `Worksheet::add_table`.

## Goal

Add a source-compatible subset of worksheet tables: a rectangular cell range
with a header row, optional AutoFilter, style and stripe flags, custom column
headers, and a user-defined table name. Tables are stored in their own MS-XLSB
Table part and referenced from the worksheet through `BrtBeginListParts` /
`BrtListPart` / `BrtEndListParts`.

The first release covers the common table shape only. Total rows, aggregate
functions, calculated-column formulas, per-column formats, structured-reference
formulas, XML maps, query tables, slicers, and single-cell tables remain
absent. `set_total_row`, `TableFunction`, and `TableColumn::set_formula` are not
added, so no formula parser is required.

## Aegis Visibility

The change adds a public API, a new package part, a worksheet relationship, and
six new record families. A plan keeps the table-part contract, worksheet
ordering, name rules, and record layout explicit.

## Architecture

`rust-xlsb-writer/src/table.rs` owns:

- `Table`, with name, style, column list, header/autofilter visibility, and
  banded/first/last-column flags.
- `TableColumn`, with a header string.
- `TableStyle`, covering the built-in Excel table style names.
- `TableRange`, the worksheet-owned table plus its resolved cell range, table
  id, and resolved name.
- `write_table_part`, which serializes a Table part from one `TableRange`.

`Worksheet` stores `Vec<TableRange>`. `add_table` validates row/column limits,
range order, the minimum row count with a header, name syntax, duplicate column
names, and overlap with existing tables and the worksheet AutoFilter. It writes
the resolved header strings into the worksheet cells and appends the table.

`Workbook::prepare` assigns workbook-wide table ids in sheet order, fills
default `TableN` names, and rejects duplicate custom/default names
case-insensitively. This is the only point that mutates table identity.

`package.rs` emits one Table part per table at `xl/tables/tableN.bin`, adds its
content-type override, and adds the worksheet relationship. Worksheet
relationships list table parts first, so `BrtListPart.stRelID` is `rId1..rIdN`
without an extra parameter in `sheet::write_sheet`.

`sheet.rs` writes `LISTPARTS` after ignored errors and before `BrtEndSheet`,
matching the Worksheet ABNF.

## Table Part Contract

One part per table, content type `application/vnd.ms-excel.table` (the spec
type is not `+xml`), relationship type
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableSingleCells`.

Record order inside the part:

1. `BrtBeginList`
2. `BrtBeginAFilter` / `BrtEndAFilter` when the header row and AutoFilter are on
3. `BrtBeginListCols`
4. For each column: `BrtBeginListCol`, `BrtEndListCol`
5. `BrtEndListCols`
6. `BrtTableStyleClient`
7. `BrtEndList`

Worksheet record order inside `LISTPARTS`:

1. `BrtBeginListParts { cParts }`
2. One `BrtListPart { stRelID }` per table
3. `BrtEndListParts`

`BrtBeginList` uses `lt = LTRANGE (0)`, `crwTotals = 0`, all six DXF ids
`0xFFFFFFFF`, `dwConnID = 0`, and null style/comment strings. Column records
use `ilta = 0`, all column DXF ids `0xFFFFFFFF`, and `idqsif = 0`.

## Validation Rules

- `add_table` rejects out-of-range rows/columns, reversed ranges, and a
  one-row range when the header row is enabled.
- A custom table name uses the existing defined-name character and cell
  reference rules, and must not start with `_xl`. Empty names are resolved at
  save time.
- Column headers must be non-empty, at most 255 UTF-16 code units, and unique
  case-insensitively within the table. Existing string cells in the header row
  supply defaults; otherwise headers are `Column1`, `Column2`, ...
- A table range must not overlap another table range on the same sheet.
- A table AutoFilter must not overlap the worksheet AutoFilter, and a worksheet
  AutoFilter must not overlap a table AutoFilter.
- Table names are unique across the workbook, case-insensitively.

## Tech Stack

Rust 1.100.0, edition 2024, deku record pipeline, `UncheckedRfX`,
`XlNullableWideString`, `XlStringExt`, the existing AutoFilter records, and the
existing ZIP/package writer.

## Baseline / Authority Refs

- [MS-XLSB] §2.1.7.51 Table part ABNF
- [MS-XLSB] §2.4.100 `BrtBeginList`
- [MS-XLSB] §2.4.101 `BrtBeginListCol`
- [MS-XLSB] §2.4.102 `BrtBeginListCols`
- [MS-XLSB] §2.4.103 `BrtBeginListParts`
- [MS-XLSB] §2.4.707 `BrtListPart`
- [MS-XLSB] §2.4.849 `BrtTableStyleClient`
- `rust-xlsb-writer/src/worksheet.rs`
- `rust-xlsb-writer/src/sheet.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-writer/src/defined_name.rs`

## Task 1: Core list and table records

**Files:**

- Create: `rust-xlsb-core/src/prelude/record/brt_table.rs`
- Modify: `rust-xlsb-core/src/prelude/record/mod.rs`
- Modify: `rust-xlsb-core/src/prelude/mod.rs`

**Interfaces:**

- Produces: `BrtBeginList`, `BrtEndList`, `BrtBeginListCol`,
  `BrtBeginListCols`, `BrtEndListCol`, `BrtEndListCols`, `BrtBeginListParts`,
  `BrtListPart`, `BrtEndListParts`, and `BrtTableStyleClient`.

- [ ] Add deku structs with the field order in §2.4.100-103, §2.4.454-457,
  §2.4.707, and §2.4.849.
- [ ] Add `Checked` implementations for `BrtBeginList`, `BrtBeginListCol`, and
  `BrtTableStyleClient`; use `Unchecked` for zero-sized end markers and the
  count/relationship records.
- [ ] Add flag constants for `BrtBeginList` and `BrtTableStyleClient`.
- [ ] Register every type with `assign_id!` and re-export it.
- [ ] Add `test_biff_export` cases with exact sizes.
- [ ] Add hand-written tests for range/flag checks, string limits, and the
  packed style flags.

## Task 2: Table model and worksheet API

**Files:**

- Create: `rust-xlsb-writer/src/table.rs`
- Modify: `rust-xlsb-writer/src/lib.rs`
- Modify: `rust-xlsb-writer/src/worksheet.rs`
- Modify: `rust-xlsb-writer/src/error.rs`
- Modify: `rust-xlsb-writer/src/workbook.rs`

**Interfaces:**

- Produces: `Table::new`, `set_name`, `set_style`, `set_columns`,
  `set_header_row`, `set_autofilter`, `set_banded_rows`, `set_banded_columns`,
  `set_first_column`, `set_last_column`; `TableColumn::new`, `set_header`;
  `TableStyle`; `Worksheet::add_table`.

- [ ] Add the table vocabulary and defaults. Default style is
  `TableStyle::Medium9`; header, AutoFilter, and row stripes are on.
- [ ] Add `add_table` with the validation rules above. It writes headers and
  stores the table.
- [ ] Add table overlap checks to `autofilter`.
- [ ] Add table name/id resolution and workbook-wide duplicate detection to
  `Workbook::prepare`.
- [ ] Add tests for defaults, custom headers, existing header cells, invalid
  ranges, duplicate columns, overlap, AutoFilter overlap, and save-time name
  resolution/duplicates.

## Task 3: Table part and package wiring

**Files:**

- Modify: `rust-xlsb-writer/src/table.rs`
- Modify: `rust-xlsb-writer/src/sheet.rs`
- Modify: `rust-xlsb-writer/src/package.rs`
- Modify: `rust-xlsb-writer/src/lib.rs`

**Interfaces:**

- Consumes: `TableRange` from `Worksheet::tables()`.
- Produces: `xl/tables/tableN.bin`, worksheet rels, content-type overrides,
  worksheet `LISTPARTS`, and `BrtTableStyleClient` flags.

- [ ] Serialize the table part in the record order above.
- [ ] Emit `LISTPARTS` from `sheet::write_sheet`.
- [ ] Add table relationships before hyperlink relationships and add the
  content-type override for each table part.
- [ ] Add package tests for the table part, relationship, content types, and
  worksheet list-part order.
- [ ] Add a test proving output without tables is byte-for-byte unchanged in
  shape (no new parts, no rels, no `LISTPARTS`).

## Task 4: Verification and documentation

- [ ] `cargo test --workspace`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo clippy --workspace --all-targets --all-features`
- [ ] `cargo fmt --all --check`
- [ ] `git diff --check`
- [ ] Generate a workbook with one styled table and one no-header/no-filter
  table; inspect the package; try LibreOffice XLSX conversion and SheetJS.
- [ ] Update `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`, and
  `docs/aegis/INDEX.md`.
- [ ] Run a reviewer on the diff, fix valid findings, then commit.

## Review Focus

- A table with no custom columns must still get `Column1..N` headers matching
  existing header cells, and the header row must be written once.
- A custom name that looks like `Table2` must not silently collide with a
  default name on another sheet.
- Tables without AutoFilter must not emit `BrtBeginAFilter` in the table part,
  but must still emit the table records.
- A worksheet with a table and an external hyperlink must keep table relationship
  ids and hyperlink relationship ids distinct.
- `BrtBeginList` and `BrtTableStyleClient` bitfields are little-endian fixed
  words; a wrong width shifts every later string.

## Review

Reviewer `BasicTablesReview` found a name-scope bug: table names were checked
only against other table names, not against defined names. Excel and the
reference writer keep table display names and defined names in one namespace, so
`Sales` as both a defined name and a table name must fail at save. Fixed by
seeding the table-name set with every defined-name local part before inserting
table names. Added a regression test.

The same review verified the record layouts, part ordering, relationships,
content types, header writing, and overlap checks. A spec check also caught that
`BrtBeginListCol.stName` must be null for `LTRANGE` tables; the writer now emits
only `stCaption` for column headers.

LibreOffice drops tables on XLSB import, so the Table part and worksheet
`LISTPARTS` cannot be validated through an XLSX round-trip. They are covered by
record, package, and relationship decode tests.
