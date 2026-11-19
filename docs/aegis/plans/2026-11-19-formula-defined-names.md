# Formula-valued defined names

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Workbook::define_name` formulas beyond single literals and absolute
  A1 cell/range references.
- Out of scope: cell formulas that refer to defined names, external-workbook
  references, multi-sheet `Sheet1:Sheet3!` ranges, future/`_xlfn` functions,
  name-to-name references, and arguments or volatile functions those features
  need.

## Goal

Close the defined-name gap in `docs/gap-analysis-rust-xlsxwriter.md`: a name
can be defined as an arbitrary workbook formula, not only a constant, an
absolute A1 reference, or a single sheet-qualified absolute A1 range.

```rust
workbook.define_name("Total", "=SUM(Data!$B$2:$B$10)")?;
workbook.define_name("Flag", "=IF(Data!$A$1>0,1,0)")?;
workbook.define_name("Sheet1!Rate", "=$A$1*2")?;
```

Formulas use the existing US-English expression grammar: numbers, strings,
booleans, errors, `+ - * / ^ & %`, comparisons, parentheses, and the function
table already used by cell formulas. Same-workbook sheet-qualified absolute A1
cells and ranges become `PtgRef3d`/`PtgArea3d`; unqualified absolute references
in a worksheet-scoped name resolve to that sheet.

## Architecture

`formula.rs` remains the owner of expression parsing and token encoding. It
gains one internal compile entry point for names. The existing `Parser` gains
an optional name context:

- the workbook sheet slice,
- the mutable `XtiCollector` owned by defined-name serialization,
- the default sheet for a worksheet-scoped name.

When that context is present, an explicit `Sheet1!$A$1` or
`'Data Sheet'!$A$1:$B$2` is parsed into a 3D token. An unqualified `$A$1` uses
the default sheet; a global name has no default, so an unqualified reference is
rejected. Relative A1 references are rejected because a defined name has no
single evaluation cell to anchor them. Cell formulas,
table calculated formulas, data validation, and conditional formatting keep
passing `None` and therefore keep rejecting cross-sheet syntax exactly as
before.

`defined_name.rs` keeps its current fast paths for constants and bare A1
references. Formulas that need the expression parser fall back to the new
entry point. The existing `XtiCollector` and `BrtExternSheet` output are
unchanged.

Reference tokens use the same column-word flag encoding as same-sheet cell
formulas. `PtgRef3d` and `PtgArea3d` therefore carry the absolute column-word
flags. The A1 parser already enforces Excel row/column limits.

## Tech Stack

Rust 1.100.0, edition 2024, existing `formula.rs` parser and
`defined_name.rs` encoder, existing `XtiCollector`. No new dependency and no
new record or package part.

## Baseline And Authority Refs

- `docs/gap-analysis-rust-xlsxwriter.md`: user-defined names support literals
  and absolute A1 references only.
- MS-XLSB `2.5.98.60`/`2.5.98.69`/`2.5.98.19`: `PtgName`, `PtgRef3d`, and
  `PtgArea3d`; `2.5.98.12` `NameParsedFormula`.
- `rust-xlsb-writer/src/formula.rs`: existing expression parser and function
  table.
- `rust-xlsb-writer/src/defined_name.rs`: `XtiCollector`, `area3d`, and current
  formula encoding.
- `rust-xlsb-writer/src/package.rs`: `write_defined_names` and `BrtName`
  serialization.

## Compatibility Boundary

- Existing constants and bare A1 references keep their exact token bytes.
- Existing calls that rely on cross-sheet syntax being rejected from cell
  formulas continue to fail; only defined-name encoding opts into 3D parsing.
- Existing rejection of global unqualified references is preserved.
- Invalid sheet names, unknown functions, malformed syntax, and unsupported
  name-to-name references fail at save before any package bytes are returned.
- No new public type or method is added; `Workbook::define_name` only accepts
  more formula text.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: new producer behavior and token-level public output.
- Test posture: add failing defined-name tests for a function over a 3D range,
  an expression over a 3D cell, a quoted sheet name, a local unqualified
  reference, and rejected external workbooks before implementation.
- Verification: focused module tests, package test, full workspace tests,
  clippy, and fmt.

## Change Necessity

The gap is behavioral: `define_name` already accepts a formula string but
rejects every expression that is not a constant or a single absolute reference.
The minimum code boundary is the existing expression parser plus a name-only
sheet context; no new formula model, record, or public API is needed.

## Ripple Signal Triage

- Canonical owner: `formula.rs` for formula tokenization; `defined_name.rs` for
  workbook-scoped encoding and Xti collection.
- Downstream consumers: `package.rs` `BrtName` output and `BrtExternSheet`.
- Source of truth: the stored formula text; token bytes are produced at save.
- Compatibility risk: parser changes must not alter cell-formula output.
- Retirement: no second parser; the defined-name fast paths stay because they
  avoid allocation for the common constant/reference case.

## Tasks

1. **Write failing defined-name formula tests.**
   - Files: `src/defined_name.rs`.
   - Add tests that call `encode_formula` for:
     - `=SUM(Sheet1!$A$1:$A$3)`;
     - `=Sheet1!$A$1*2`;
     - `='Data Sheet'!$A$1+1`;
     - local `=$A$1+1` resolving to the scope sheet;
     - global `=$A$1+1` rejected;
     - `=[Book]Sheet1!$A$1` rejected.
   - Assert exact `PtgRef3d`/`PtgArea3d` bytes and Xti sheet order.
   - Keep the existing constant and bare-reference tests unchanged.
   - Verify these tests fail before implementation.

2. **Add the name-only parser context.**
   - Files: `src/formula.rs`.
   - Add `compile_name_formula(formula, sheets, xti, default_sheet)`.
   - Add a parser context carrying sheets, Xti collector, and default sheet.
   - Parse quoted and unquoted sheet prefixes and emit `PtgRef3d`/`PtgArea3d`.
   - Resolve unqualified absolute references through the default sheet; reject
     when no default exists or a reference is relative.
   - Keep existing parser calls passing no context.

3. **Route defined-name expressions through the parser.**
   - Files: `src/defined_name.rs`.
   - Preserve current constant and bare-reference fast paths.
   - Fall back to `compile_name_formula` for expressions, passing the scope
     sheet index.
   - Update `Workbook::define_name` docs to describe expression support.
   - Verify focused tests now pass.

4. **Add package-level proof and update the gap record.**
   - Files: `src/package.rs`, `docs/gap-analysis-rust-xlsxwriter.md`,
     `AGENTS.md`, `docs/aegis/INDEX.md`.
   - Add a save test that decodes `BrtName` and `BrtExternSheet` for
     `=SUM(Sheet1!$A$1:$A$3)` and asserts the token stream.
   - Add a `calamine` round-trip test for a global function formula and a local
     expression.
   - Mark formula-valued defined names shipped/partial with the remaining
     exclusions.
   - Update test counts after the final run.

## Verification

- `cargo test -p rust-xlsb-writer defined_name` and the new package test.
- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`

Result: 674 unit tests and 7 doctests in default builds (290 core, 384 writer);
678 unit tests and 7 doctests with all features; clippy and fmt clean.

## Review

`FormulaNamesReview` found two non-blocking defects; both are fixed and
covered:

1. P2: every formula starting with a string literal took the bare-string fast
   path, so `="Total: "&Sheet1!$A$1` and `="a"="b"` were rejected. The fast path
   now returns only when `encode_string` succeeds as a complete literal;
   expressions fall through to the parser. Tests pin both token streams.
2. P3: the expression path accepted reversed 3D areas such as `$B$2:$A$1`,
   while the existing fast path returned `Error::RowColumnOrder`. `push_area3d`
   now validates the same order before writing; local and quoted-sheet tests
   cover the rejection.

The reviewer confirmed the 3D token layout against MS-XLSB 2.5.98.69,
2.5.98.19, 2.5.98.91, and 2.5.98.92, and confirmed that cell formulas, table
formulas, validation, and conditional formats still reject cross-sheet syntax.
Relative defined-name references are rejected because a name has no evaluation
cell to anchor them.

Calamine 0.38 decodes `PtgGe`/`PtgGt` (`0x0C`/`0x0D`) as `>`/`>=` reversed, so
the calamine round-trip test exercises function and arithmetic formulas;
record tests pin the comparison tokens against the spec.

## Risks And Mitigations

- **Parser regression:** cell formulas must keep rejecting cross-sheet syntax.
  Mitigation: context is optional; existing parser tests cover the error.
- **Xti order:** `write_defined_names` seeds built-in names before user names.
  Mitigation: reuse `XtiCollector` and assert sheet order in tests.
- **Relative semantics:** relative references have no evaluation anchor in a
  defined name. Mitigation: reject them and test `$`-anchored references only.
- **Future functions:** `_xlfn` calls remain unsupported. Mitigation: document
  the boundary; do not silently emit a bad `PtgFunc`.

## Self-Review

- Scope covers one public behavior: more accepted `define_name` formulas.
- Files are existing owners; no new module.
- Verification names the user-visible seam (`BrtName` bytes and package save).
- Out-of-scope features are explicit.
