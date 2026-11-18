# Unprotected Ranges and Ignored Worksheet Errors

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: `Worksheet::unprotect_range`,
  `Worksheet::unprotect_range_with_options`, `Worksheet::ignore_error`,
  `Worksheet::ignore_error_range`, `IgnoreError`, `BrtRangeProtection`, and
  `BrtCellIgnoreEC`
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` sections 3.4 and P1;
  `rust_xlsxwriter` worksheet and `IgnoreError` implementations

## 1. Goal

Close two related worksheet-analysis gaps with reference signatures:

```rust
pub enum IgnoreError {
    NumberStoredAsText,
    FormulaError,
    FormulaDiffers,
    FormulaRefersToEmptyCells,
    FormulaOmitsCells,
    DataValidationError,
    TwoDigitTextYear,
    UnlockedCellsWithFormula,
    InconsistentColumnFormula,
}

impl Worksheet {
    pub fn unprotect_range(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<&mut Worksheet>;

    pub fn unprotect_range_with_options(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        name: &str,
        password: &str,
    ) -> Result<&mut Worksheet>;

    pub fn ignore_error(
        &mut self,
        row: RowNum,
        col: ColNum,
        error_type: IgnoreError,
    ) -> Result<&mut Worksheet>;

    pub fn ignore_error_range(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
        error_type: IgnoreError,
    ) -> Result<&mut Worksheet>;
}
```

Uncalled methods add no records and keep current output unchanged.

## 2. Architecture

`unprotect_range` stores one private tuple per call:

```rust
struct UnprotectedRange {
    range: UncheckedRfX,
    name: String,
    password_hash: u16,
}
```

An empty name becomes `RangeN` using the current count, matching the
reference. The password uses the existing legacy verifier. `Range` validation
reuses the worksheet's row/column limits and order check.

`ignore_error_range` stores ranges grouped by error type:

```rust
BTreeMap<IgnoreError, Vec<UncheckedRfX>>
```

A separate `BTreeSet<(u32, u16, u32, u16)>` rejects a repeated exact range with
the new `Error::Parameter`; the reference rejects duplicate ranges because
Excel allows one ignored-error rule per cell. Overlapping but non-identical
ranges remain accepted, as in the reference. Grouping means one
`BrtCellIgnoreEC` per error type, with all its ranges in one `sqrfxu`.

`IgnoreError` derives `Ord` for the `BTreeMap`; it stays a `Copy` enum with no
allocation. Its iterator is not added because nothing in the writer consumes
it.

## 3. Records

### `BrtRangeProtection` (MS-XLSB 2.4.765)

- `protPwd: u16`
- `sqRfX: UncheckedSqRfX` with exactly one `UncheckedRfX`
- `RangeProtectionTitleSDRel` (2.5.113): `rgchTitle: XlWideString`,
  `dwsdRel: u32`, `pSDRel: Vec<u8>`

The title is required to contain 1–255 UTF-16 code units and to be unique per
sheet. The writer supplies `RangeN` when the caller passes an empty name.
`dwsdRel` is zero because this writer does not emit a security descriptor.
`BrtRangeProtectionIso` is omitted, which the worksheet ABNF permits.

Record order is after `BrtSheetProtection` and before scenario/merge records:

`... [[BrtSheetProtectionIso] BrtSheetProtection] *([BrtRangeProtectionIso]
BrtRangeProtection) [SCENMAN] ... [MERGECELLS] ...`

### `BrtCellIgnoreEC` (MS-XLSB 2.4.322)

Nine one-bit flags in an `u32`: calculation error, empty-cell reference,
number-stored-as-text, inconsistent range, inconsistent formula, insufficient
text date, unprotected formula, data validation, calculated column; 23 reserved
zero bits, followed by `sqrfxu: UncheckedSqRfX`.

Each error type maps to one bit:

| `IgnoreError` | Field |
|---|---|
| `NumberStoredAsText` | `ffecNumStoredAsText` |
| `FormulaError` | `ffecCalcError` |
| `FormulaDiffers` | `ffecInconsistFmla` |
| `FormulaRefersToEmptyCells` | `ffecEmptyCellRef` |
| `FormulaOmitsCells` | `ffecInconsistRange` |
| `DataValidationError` | `ffecDataValidation` |
| `TwoDigitTextYear` | `ffecTextDateInsuff` |
| `UnlockedCellsWithFormula` | `ffecUnprotFmla` |
| `InconsistentColumnFormula` | `ffecCalcCol` |

The records are wrapped in the empty scope markers
`BrtBeginCellIgnoreECs`/`BrtEndCellIgnoreECs` and emitted at `IGNOREECS`,
after page breaks and before `BrtEndSheet`.

## 4. Compatibility Boundary

- Public names, parameters, return types, default range names, and duplicate
  range rejection match the reference.
- The XLSB protected-range title cannot be empty, so the reference default
  `RangeN` is always materialized before serialization.
- `IgnoreError` maps to MS-XLSB flag names, not XLSX `ignoredErrors`
  attribute names.
- Macros, worksheets, and existing unprotected-range or ignored-error records
  are not read back; the holder is write-session state.

## 5. Change Necessity

`Worksheet` has no state or API for either feature, and core has neither
record. The existing protection record proves the password path but cannot
express editable ranges. The existing range structures can be reused without
new abstractions.

## 6. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
record round-trip and exact bit/byte layout, protected-range order and title
defaults, ignored-error grouping and duplicate rejection, package-level decode,
and LibreOffice XLSX round-trip inspection for `protectedRange` and
`ignoredErrors`.

## 7. Tasks

1. Core records
   - Add `RangeProtectionTitleSDRel` with a validating length/descriptor check.
   - Add `BrtRangeProtection` and `BrtCellIgnoreEC` with `assign_id!`.
   - Add `BrtBeginCellIgnoreECs`/`BrtEndCellIgnoreECs` scope markers.
   - Add exact-byte and round-trip tests and the record-table cases.
2. Writer API
   - Add `IgnoreError`, range validation, and the four methods.
   - Add the private unprotected-range and ignored-error state.
   - Add `Error::Parameter` for a repeated ignored-error range.
3. Serialization
   - Emit protected ranges after sheet protection, before merges.
   - Emit the ignored-error collection after page breaks, before end sheet.
4. Tests
   - Default names `Range1`, `Range2`; explicit name and password verifier.
   - Unique titles, including default-name collisions.
   - Ignored-error chunking at the 8,191-range record limit.
   - Row/column limit and order errors.
   - Flag mapping for all nine error types and grouping in one record.
   - Duplicate-range rejection and default-output absence.
   - Record order for both record families.
5. Docs
   - Mark both API groups supported in the gap analysis.
   - Update `AGENTS.md` API inventory and test counts.
   - Add this plan to `docs/aegis/INDEX.md`.

## 8. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- LibreOffice conversion to XLSX exposes `protectedRange` names/passwords and
  `ignoredErrors`, or the normalization limitation is documented.

Result: 189 core + 155 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. Reviewer found one P1: ignored-error records could
exceed the spec's 8,191-range limit; the writer now chunks per error type. It
also found one P2: protected-range titles were not unique; explicit duplicates
are rejected and default `RangeN` names skip collisions. LibreOffice opens the
generated XLSB and keeps `sheetProtection`, but drops both `protectedRange` and
`ignoredErrors` on XLSB import, so those records are proven by exact-byte,
record-order, and package decode tests rather than the XLSX round-trip.

## 9. Risks

- No Excel fixture is available; record layout is proven by the spec and
  decode tests.
- LibreOffice drops `protectedRange` and `ignoredErrors` on XLSB import. The
  record decode remains the primary oracle.
- Ignored-error ranges are not checked for overlap, matching the reference.
- Protected-range title grammar is not enforced beyond length and uniqueness;
  this matches the reference's lack of validation.
- Legacy range passwords are weak verifiers, not encryption.
