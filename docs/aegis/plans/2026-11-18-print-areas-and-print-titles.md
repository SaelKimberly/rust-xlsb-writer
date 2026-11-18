# Print Areas And Print Titles

- Status: Implemented — reviewer found no defects
- Date: 2026-11-18
- Scope: worksheet print area, repeat rows, and repeat columns
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` section 3.3; `rust_xlsxwriter`
  `set_print_area`, `set_repeat_rows`, `set_repeat_columns`

## 1. Goal

Add the three missing print-range APIs:

- `set_print_area(first_row, first_col, last_row, last_col)`
- `set_repeat_rows(first_row, last_row)`
- `set_repeat_columns(first_col, last_col)`

The output must be a valid workbook with defined names that Excel resolves as
the sheet print area and repeated print titles. Calamine reads the same names
and formulas independently. Default output stays byte-identical: no new records
unless one of these APIs is used.

## 2. Architecture

Print ranges are workbook-level defined names, not worksheet records:

- `_xlnm.Print_Area` is sheet-scoped and holds one `PtgArea3d`.
- `_xlnm.Print_Titles` is sheet-scoped and holds the repeat-row area, the
  repeat-column area, or both joined by `PtgUnion` (`0x10`).
- The workbook part gains `BrtBeginExternals`, `BrtSupSelf`,
  `BrtExternSheet`, `BrtEndExternals` before the `BrtName` records.
- `BrtExternSheet` contains one `Xti` per worksheet that has print settings;
  each `PtgArea3d.ixti` indexes that array.

No package part, relationship, or content type is added.

## 3. Core Records

Add under `prelude/record/`:

- `brt_name.rs`: `NameParsedFormula { cce, rgce, cb, rgcb }` and `BrtName`.
- `brt_extern_sheet.rs`: `Xti`, `BrtExternSheet`, `BrtSupSelf`.
- Extend `scope.rs` with `Externals`.

`BrtName` stores the spec bit layout in a raw `flags: u32` word; the writer
sets only `F_BUILTIN`. The layout is:

| Field | Bits |
|---|---:|
| `fHidden` | 1 |
| `fFunc` | 1 |
| `fOB` | 1 |
| `fProc` | 1 |
| `fCalcExp` | 1 |
| `fBuiltin` | 1 |
| `fgrp` | 9 |
| `fPublished` | 1 |
| `fWorkbookParam` | 1 |
| `fFutureFunction` | 1 |
| `reserved` | 14 |

Then `chKey: u8`, `itab: u32`, `name: XlWideString`,
`formula: NameParsedFormula`, `comment: XlNullableWideString`. The record is
`Unchecked`, matching other variable-string records.

`BrtExternSheet` is `cXti: u32` plus `rgXti: Vec<Xti>`. `Xti` is
`externalLink: u32`, `firstSheet: i32`, `lastSheet: i32`. `BrtSupSelf` is a
zero-byte record.

Round-trip tests pin 25-byte default `BrtName`, 4-byte empty `BrtExternSheet`,
12-byte `Xti`, and 0-byte externals markers. Raw-bit tests pin `fBuiltin` and
`fHidden` positions.

## 4. Worksheet State And Validation

Add three `Option` fields to `Worksheet`:

```rust
print_area: Option<(RowNum, ColNum, RowNum, ColNum)>,
repeat_rows: Option<(RowNum, RowNum)>,
repeat_columns: Option<(ColNum, ColNum)>,
```

No allocation is added for worksheets that never set them.

Validation follows the reference order: bounds first, then ordering. Invalid
calls return `RowColumnLimit` or `RowColumnOrder` and leave prior state
unchanged. A print area covering the entire worksheet clears the setting,
matching the reference default.

`set_repeat_rows` and `set_repeat_columns` replace the previous repeat range of
that axis. They do not clear each other.

## 5. Package Emission

After `BrtEndBundleShs`, when any sheet has a print setting:

1. Build an Xti list in sheet order for sheets with print settings.
2. Emit `BrtBeginExternals`, `BrtSupSelf`, `BrtExternSheet`, `BrtEndExternals`.
3. Emit `BrtName` for each print area, then each print-title range.

`BrtName` construction:

- `fBuiltin = true`, `fHidden = false`, `chKey = 0`, `fgrp = 0`.
- `itab` = sheet index for sheet-scoped names.
- `name` = `_xlnm.Print_Area` or `_xlnm.Print_Titles`.
- `comment` = null.
- formula: `cce` plus one or two 15-byte `PtgArea3d` structures.

`PtgArea3d` layout:

```
0x3B | ixti u16 | first_row u32 | last_row u32 | first_col u16 | last_col u16
```

Repeat rows use columns `0..=MAX_COL`; repeat columns use rows
`0..=MAX_ROW`. When both axes exist, the second area is followed by `0x10`.

All `usize` to `u32` conversions use `Error::ValueNotRepresentable`.

## 6. Verification

Unit tests:

- print area replaces and clears; full-sheet area clears.
- repeat rows/columns replace independently; invalid values preserve state.
- single-axis print titles emit no union; non-zero sheet scope keeps the
  matching `itab` and Xti index.
- `PtgArea3d` bytes and print-title union bytes.
- externals and names emitted only when configured.
- multiple sheets map to the correct `Xti`.

E2E through `calamine`:

- `defined_names()` contains `_xlnm.Print_Area` with the expected
  `Sheet1!$A$1:$C$5` formula.
- repeat rows/columns and a two-sheet print area resolve to the right sheet.
- cell values remain unchanged.

LibreOffice imports the defined names but leaves their XLSB expressions empty,
so it cannot verify print ranges from XLSB. Calamine's parsed name/formula
output is the independent reader oracle; raw record tests remain authoritative
for framing.

Commands: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`cargo fmt --all --check`, `git diff --check`.

Result: 171 core + 120 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. Reviewer found no code defects and requested
single-axis title coverage, which is included.

## 7. Compatibility And Risks

- No record emitted by default.
- No worksheet API other than the three new methods changes.
- Defined names are added before any calc records; this package currently has
  no calc records.
- `PtgArea3d` is encoded with absolute row/column flags clear.
- Built-in names are stored with the `_xlnm.` prefix and `fBuiltin = true`,
  matching OOXML defined-name spelling.
- LibreOffice's XLSB importer drops defined-name expressions, so it cannot
  validate page layout from these records.

## 8. Retirement

None. These are additive APIs; no old path is replaced.
