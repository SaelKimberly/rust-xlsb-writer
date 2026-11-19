# Defined names: worksheet ranges in expressions

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-formula-defined-names.md`

## Goal

Accept same-workbook worksheet ranges in defined-name expressions:
`=SUM(Sheet1:Sheet3!$A$1)`, `=SUM('Q1':'Q4'!$B$2:$B$9)`.

The range is a 3D reference: one `Xti` spans the first and last worksheet,
and `PtgArea3d`/`PtgRef3d` consumes that index. Cell formulas, table
calculated-column formulas, validation, and conditional formatting keep
rejecting cross-sheet syntax.

## Aegis Visibility

One shared owner changes: `XtiCollector` currently maps one worksheet to one
`Xti`. Supporting a sheet range changes that contract and the formula
prefix parser, so the byte-level package test must prove the `BrtExternSheet`
entry spans sheets.

## Baseline / Authority

- MS-XLSB `2.5.173 Xti`: `externalLink`, `firstSheet`, `lastSheet`.
- MS-XLSB `2.5.98.19 PtgArea3d`: 16-bit `ixti` plus two cell areas.
- MS-XLSB `2.4.672 BrtExternSheet`: ordered `Xti` array.
- Existing owners:
  - `rust-xlsb-writer/src/defined_name.rs`: `XtiCollector`, name encoding.
  - `rust-xlsb-writer/src/formula.rs`: sheet-prefix detection, 3D token
    emission.
  - `rust-xlsb-writer/src/package.rs`: `BrtExternSheet` serialization and
    `BrtName` package tests.

## Compatibility Boundary

- No public API changes. `Workbook::define_name` accepts more strings.
- Single-sheet references keep the exact existing bytes and Xti order.
- `Sheet3:Sheet1` is rejected: `Xti.firstSheet` must not exceed `lastSheet`.
- Unknown sheets, relative references, external workbooks, and name-to-name
  references remain rejected.
- Worksheet names containing `:` are impossible in Excel, so `:` remains the
  range separator.

## Change Necessity

The formula parser already recognizes sheet-qualified references, but
`XtiCollector` only stores a single sheet. Extending that existing owner is
smaller and safer than adding a second name encoder. The minimum code boundary
is `XtiCollector` plus the sheet-prefix branch of the existing parser.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: no explicit TDD request; repository convention is test-after
  regression coverage.
- Test posture: add focused parser, collector, and package-decoding tests with
  the implementation.

## Tasks

1. **Represent worksheet ranges in `XtiCollector`.**
   - Replace `sheets: Vec<usize>` with `ranges: Vec<(usize, usize)>`.
   - Keep the per-sheet index cache for single-sheet references.
   - Add `range(first, last) -> Result<u16>`: reuse `index` when endpoints are
     equal, deduplicate an existing pair, otherwise append.
   - Rename `sheets()` to `ranges()`.
   - Update `package.rs` to emit `Xti { firstSheet: first, lastSheet: last }`.
   - Update existing collector assertions.

2. **Parse a worksheet range before the `!`.**
   - Teach `sheet_prefix_end` to accept `sheet`, optional `: sheet`, then `!`,
     for both bare and quoted names.
   - Centralize reading one sheet token; `parse_sheet_reference` reads the
     first, optional second, then `!`.
   - Resolve both names through `sheet_index`, reject descending ranges with
     `Error::Parameter`, and call `XtiCollector::range`.
   - Keep `push_ref3d`/`push_area3d` unchanged.
   - Fix `split_sheet_prefix` so a quoted sheet followed by `:` falls through
     to the parser instead of returning `unsupported`.

3. **Prove the feature.**
   - Parser test: three-sheet range produces one `PtgArea3d` with `ixti = 0`
     and collector ranges `[(0, 2)]`.
   - Parser test: quoted range `'Data Sheet':'Sheet3'!$A$1` resolves.
   - Parser test: descending range is rejected.
   - Package test: `Total = SUM(Sheet1:Sheet3!$A$1:$A$3)` emits one
     `BrtExternSheet` entry with `firstSheet = 0`, `lastSheet = 2`, and a
     `BrtName` area token using `ixti = 0`.
   - Existing single-sheet formula-name tests stay unchanged.

4. **Update boundaries.**
   - `docs/gap-analysis-rust-xlsxwriter.md`: remove multi-sheet ranges from the
     defined-name missing list in the matrix, hidden-gaps table, and P4 row.
   - `AGENTS.md`: defined names accept same-workbook sheet ranges.
   - Plan status and `docs/aegis/INDEX.md`.

## Verification

```bash
cargo test -p rust-xlsb-writer defined_name
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

Result: 681 unit tests and 7 doctests in default builds (290 core, 391
writer); 685 unit tests and 7 doctests with all features; clippy and fmt clean.
Calamine 0.38 ignores `Xti.lastSheet` when rendering defined names, so the
package record decode is the range oracle. LibreOffice opens the generated
workbook and exports a CSV without error.

## Review

`SheetRangeReview` found no actionable defects. It independently decoded
`xl/workbook.bin` for bare, quoted, mixed, escaped-quote, `first == last`,
deduped, range-plus-single, and built-in print-area cases. All produced the
expected `Xti { firstSheet, lastSheet }` and matching 16-bit `ixti`.

The reviewer also confirmed that cell formulas, table calculated formulas,
validation, and conditional formats still pass `name_context == None`, so
cross-sheet syntax remains rejected there. Range validation lives only in
`XtiCollector::range`, avoiding a second owner.

## Risks And Mitigations

- **Xti order regression:** built-in print names and user names share the
  collector. Mitigation: single-sheet path keeps `index` and existing tests.
- **Range deduplication changes indexes:** only exact `(first, last)` pairs are
  reused. Mitigation: package test pins index 0, collector test pins order.
- **Parser ambiguity:** a sheet named like a function already wins through
  `is_simple_sheet_name`; the range prefix must not treat `SUM(...)` as a sheet.
  Mitigation: keep the existing function test and add a quoted-range test.
- **Reverse ranges:** Excel treats `Sheet3:Sheet1` as invalid. Mitigation:
  reject before emitting, with a test.

## Retirement

No second owner. `XtiCollector` remains the only `BrtExternSheet` index
allocator, and `formula.rs` remains the only 3D token encoder.

## Self-Review

- Scope covers one public behavior: more accepted `define_name` formulas.
- Files are existing owners; no new module.
- Verification names the user-visible seam (`BrtExternSheet` and `BrtName`).
- Unsupported boundaries stay explicit.
