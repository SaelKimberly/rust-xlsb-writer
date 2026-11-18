# Workbook Behavior: Calculation Mode, Read-Only Recommendation, Tab Ratio

- Status: Implemented — reviewer found no defects
- Date: 2026-11-18
- Scope: `Workbook::set_calculation_mode`, `Workbook::read_only_recommended`,
  `Workbook::set_tab_ratio`, plus the `BrtCalcProp` and `BrtFileSharing`
  records
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` sections 3.1 and P4;
  `rust_xlsxwriter` `Workbook::set_calculation_mode`,
  `Workbook::read_only_recommended`, `Workbook::set_tab_ratio`

## 1. Goal

Close the three small workbook-behavior gaps with reference signatures:

```rust
pub enum CalculationMode {
    Automatic,
    Manual,
    Partial,
}

impl Workbook {
    pub fn read_only_recommended(&mut self) -> &mut Workbook;
    pub fn set_calculation_mode(&mut self, mode: CalculationMode) -> &mut Workbook;
    pub fn set_tab_ratio(&mut self, ratio: u8) -> &mut Workbook;
}
```

The methods only affect output when called. A default workbook keeps its
current parts and record sequence.

## 2. Architecture

`Workbook` currently owns only `Vec<Worksheet>`. Add a crate-private
`WorkbookSettings` value:

```rust
#[derive(Clone, Copy, Default)]
pub struct WorkbookSettings {
    pub calculation_mode: Option<CalculationMode>,
    pub read_only_recommended: bool,
    pub tab_ratio: Option<u8>,
}
```

`Option` distinguishes "not requested" from an explicit default, so untouched
workbooks do not grow a calculation record or a book-view record. The settings
are `Copy` and passed by value to `build_package`; no heap allocation is added.

`BrtFileSharing` (MS-XLSB 2.4.678) is emitted immediately after `BrtBeginBook`
when read-only is recommended:

- `fReadOnlyRec: u16 = 1`
- `wResPass: u16 = 0`
- `stUserName: XlNullableWideString = null`

`BrtFileSharingIso` is omitted: the workbook ABNF makes the pair optional, and
there is no write-reservation password to describe.

`BrtCalcProp` (2.4.318) is emitted after defined names and before
`BrtEndBook` when a mode was set. Fixed fields: `recalcID: u32`,
`fAutoRecalc: u32`, `cCalcCount: u32`, `xnumDelta: Xnum`,
`cUserThreadCount: i32`, then nine flag bits and seven reserved bits. Mode maps
to `fAutoRecalc`: automatic `1`, manual `0`, partial `2`. Manual sets
`fFullCalcOnLoad = false` and `fSaveRecalc = false`; the other modes set both
true. `fRefA1` and `fFullPrec` are always true.

`set_tab_ratio` stores a percentage in `0..=100`; values above 100 print the
reference warning and leave the previous state unchanged. The package stores
`iTabRatio = ratio * 10` because `BrtBookView` uses 0.1% units. A custom ratio
forces `BrtBeginBookViews` / `BrtBookView` / `BrtEndBookViews` even when the
active and first sheet indexes are zero.

## 3. Compatibility Boundary

- Signatures and return types match `rust_xlsxwriter`.
- `CalculationMode` variants match: `Automatic`, `Manual`, `Partial`.
- Tab ratio >100 is ignored with the reference warning, not an error.
- Read-only recommendation has no password and no ISO companion record.
- Default workbooks emit no `BrtFileSharing`, no `BrtCalcProp`, and no
  `BrtBookView` unless sheet state already forced one.

## 4. Change Necessity

The three capabilities need workbook-level state and records; worksheet methods
cannot express them. Existing `BrtBookView` covers the tab ratio, so only two
record structs are new.

## 5. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
record round-trip tests for both new records, package decode tests for all three
settings, a default-output test, and a calamine open test.

## 6. Tasks

1. Core records
   - Add `BrtFileSharing` (`fReadOnlyRec: u16`, `wResPass: u16`,
     `stUserName: XlNullableWideString`) with `assign_id!` and a size test.
   - Add `BrtCalcProp` with the fixed fields, nine LSB-first flag bits, and
     seven reserved bits; `assign_id!` and a size test.
   - Re-export both and register rstest cases.
2. Workbook API
   - Add `CalculationMode` and `WorkbookSettings` in `workbook.rs`.
   - Add the three methods; validate tab ratio against 100.
   - Re-export `CalculationMode`.
3. Package
   - Pass settings into `build_package` and `workbook_part`.
   - Emit `BrtFileSharing`, `BrtCalcProp`, and the custom `iTabRatio`.
4. Tests
   - Read back both records from `xl/workbook.bin`.
   - Assert mode-to-`fAutoRecalc` mapping and manual flags.
   - Assert tab ratio scaling and that ratio alone creates a book view.
   - Assert invalid ratio leaves default output unchanged.
   - Open the saved buffer with calamine.
5. Docs
   - Mark the three capabilities supported in the gap analysis.
   - Update `AGENTS.md` API inventory and test counts.
   - Add this plan to `docs/aegis/INDEX.md`.

## 7. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- Calamine opens the generated workbook.
- Workbook record order matches the ABNF.

Result: 177 core + 139 writer unit tests and 6 doctests pass; clippy, fmt,
and `git diff --check` are clean. Reviewer found no functional defect and
confirmed both record layouts, the ABNF ordering, the omitted
`BrtFileSharingIso`, and reference parity. Its only finding was the stale plan
status, now fixed.

## 8. Risks

- `BrtCalcProp` flag packing is proven by a 26-byte round-trip and field
  assertions; no Excel fixture is available to cross-check `recalcID`.
- LibreOffice drops the custom tab ratio, manual calculation mode, and read-only
  recommendation on XLSB import, so the record decode is the primary oracle.
- A custom tab ratio emits a book view that the old default path omitted; this
  only happens when the method is used.

## 9. Retirement

No existing API is removed. The new record vocabulary is reused if workbook
properties expand later.
