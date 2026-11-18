# Defined names for literal values and absolute cell ranges

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `Workbook::define_name` and user-defined name records.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.1 `define_name`.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Workbook::define_name`, `DefinedName`, and `utility::check_name`.

## Goal

Add source-compatible workbook and worksheet-scoped defined names for the two
common formula shapes:

- constants: numbers, strings, and booleans;
- absolute A1 cell or range references, optionally sheet-qualified.

Global names use scope `0xFFFFFFFF`; local names use the worksheet index.
Names are emitted as `BrtName` records in `xl/workbook.bin`. Records and Xti
entries are absent when no name is defined, so default package bytes do not
change.

## Architecture

`Workbook` owns `Vec<DefinedName>`. `define_name` validates the name, splits
`Sheet!Name` local scopes, strips one leading `=` from the formula, and stores
the name plus formula string. The worksheet collection remains the source of
truth for names and indexes; scope resolution and duplicate detection happen in
`prepare()`.

`package::write_defined_names` becomes the single emitter. It builds name
records into a temporary buffer while lazily collecting Xti entries, then writes
the `BrtBeginExternals`/`BrtSupSelf`/`BrtExternSheet` block only when a 3D
reference exists. Built-in print-area, autofilter, and print-title names keep
their current order and values. User names follow, sorted case-insensitively.

`NameParsedFormula` forbids local `PtgRef`/`PtgArea`, so every A1 reference is
encoded as `PtgRef3d` (0x3A) or `PtgArea3d` (0x3B). An unqualified reference is
resolved against the local name's sheet; a global name must qualify the sheet.
Absolute references only: relative flags stay zero and unanchored references are
rejected rather than given incorrect semantics.

## Tech Stack

Rust 1.100.0, edition 2024, `rust-xlsb-core::prelude::{BrtName,
NameParsedFormula, XlWideString}`, existing `Xti`/`BrtExternSheet` packaging.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/workbook.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-core/src/prelude/record/brt_name.rs`
- `thirdparty/rust_xlsxwriter/src/workbook.rs::define_name`
- `thirdparty/rust_xlsxwriter/src/utility.rs::check_name`
- MS-XLSB §2.5.98.12 `NameParsedFormula`, §2.5.98.63 `PtgNum`, §2.5.98.68
  `PtgRef3d`, §2.5.98.74 `PtgStr`, §2.4.718 `BrtName`

## Compatibility Boundary

- API: `Workbook::define_name(&mut self, name: impl Into<String>, formula:
  &str) -> Result<&mut Workbook>`.
- Local scopes use the reference syntax `Sheet1!Name` or `'Sheet 1'!Name`.
- Names may be defined before the referenced worksheet exists; scope resolution
  happens at save.
- Supported formulas are one constant or one absolute A1 cell/range. Operators,
  functions, relative references, whole-row/column ranges, and names are
  rejected with `Error::Parameter`.
- `ExcelDateTime` names are not added; the value-type surface is unchanged.
- `docProps/app.xml` Named Ranges metadata is not added; print-area names have
  the same existing limitation.

## Change Necessity

`BrtName` exists and is already used for sheet-scoped print metadata, but there
is no public path for user names and no formula encoder in the writer. Adding a
string field without encoding it would produce an unreadable name record. The
minimum code boundary is one writer-private name model plus a reference/constant
encoder wired into the existing defined-name emitter.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API and package contract; reference writes records that
  Excel reads.
- Test posture: first add API/validation and package tests that fail to compile
  or fail record assertions, then implement, then add default-output and
  duplicate/scope regressions.
- Reason: scope ids, formula token bytes, and Xti presence are external
  contracts, not thin adapters.

## Tasks

### Task 1: RED — API and package contract

**Files:**
- Modify: `rust-xlsb-writer/src/workbook.rs`
- Modify: `rust-xlsb-writer/src/package.rs`

Add tests requiring `Workbook::define_name`, global/local name records, Xti
entries for qualified references, and no records by default. Add rejection tests
for duplicate names, unknown scopes, malformed names, and unsupported formulas.

Verification: tests fail to compile or fail on missing records.

### Task 2: GREEN — name model and validation

**Files:**
- Add: `rust-xlsb-writer/src/defined_name.rs`
- Modify: `rust-xlsb-writer/src/lib.rs`
- Modify: `rust-xlsb-writer/src/workbook.rs`

Add the writer-private `DefinedName`, `split_local_name`, `check_name`, A1
parser, and name sorting key. Wire `Workbook::define_name` and save-time scope
and duplicate validation.

Verification: name-validation unit tests pass.

### Task 3: GREEN — formula tokens and package emission

**Files:**
- Modify: `rust-xlsb-writer/src/defined_name.rs`
- Modify: `rust-xlsb-writer/src/package.rs`

Encode constants and 3D absolute references; lazily collect Xti indexes; emit
user `BrtName` records after built-ins. Keep default and built-in-only bytes
unchanged.

Verification: package tests assert exact token bytes and scope ids.

### Task 4: REFACTOR — docs and evidence

**Files:**
- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `AGENTS.md`
- Modify: `docs/aegis/INDEX.md`

Mark `define_name` Partial with the supported formula boundary and update test
counts. Run workspace tests, all-features tests, clippy, fmt, and diff checks.

Verification: commands listed below pass; worktree has only intended changes.

## Verification

- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `git diff --check`

Reference behavior is record-level; calamine does not expose defined names, so
the package tests are the oracle for scope, record order, and token bytes.

## Risks

- `NameParsedFormula` forbids local references; all supported refs must be 3D.
- Relative or unqualified global references would need active-cell semantics;
  they are rejected instead.
- Excel name rules include reserved names and cell-reference-shaped names; the
  validator covers the reference checks used by `check_name`.

## Review

`DefinedNamesReview` found no blocking defects. Applied fixes:

- sheet-scope lookup is case-insensitive, matching Excel and the writer's
  case-insensitive sheet-name uniqueness;
- tests now pin `itab` on the second sheet, mixed built-in/user Xti numbering,
  and escaped-quote sheet names.

The review suggested asserting the embedded Xti index equals the sheet index in
the single-reference case; that is incorrect. The Xti index is dense
(`rgXti[0].firstSheet == 1`, embedded `ixti == 0`), and the test asserts the
actual dense mapping.

## Retirement

No old public path is replaced. The built-in name emitter keeps its current
behavior and only gains a shared record helper. No follow-up shim is planned.
