# Worksheet Protection: Options and Password Verifier

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: `Worksheet::protect`, `Worksheet::protect_with_password`,
  `Worksheet::protect_with_options`, `ProtectionOptions`, and the
  `BrtSheetProtection` record
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` section 3.4 and P1;
  `rust_xlsxwriter` `worksheet::protect*` and `protection::ProtectionOptions`

## 1. Goal

Close the basic sheet-protection gap with reference signatures:

```rust
#[derive(Clone, Copy)]
pub struct ProtectionOptions {
    pub select_locked_cells: bool,
    pub select_unlocked_cells: bool,
    pub format_cells: bool,
    pub format_columns: bool,
    pub format_rows: bool,
    pub insert_columns: bool,
    pub insert_rows: bool,
    pub insert_links: bool,
    pub delete_columns: bool,
    pub delete_rows: bool,
    pub sort: bool,
    pub use_autofilter: bool,
    pub use_pivot_tables: bool,
    pub edit_scenarios: bool,
    pub edit_objects: bool,
    pub contents: bool,
}

impl Worksheet {
    pub fn protect(&mut self) -> &mut Worksheet;
    pub fn protect_with_password(&mut self, password: &str) -> &mut Worksheet;
    pub fn protect_with_options(&mut self, options: &ProtectionOptions) -> &mut Worksheet;
}
```

Only a called method adds a protection record. Unprotected worksheets keep
their current byte stream.

## 2. Architecture

`ProtectionOptions` is `Copy` (16 booleans); passing it by shared reference and
storing it by value adds no allocation. `Default` matches `rust_xlsxwriter`:
selection enabled, editing operations disabled, `contents` true for chartsheet
parity even though this writer has no chartsheets.

`Worksheet` stores:

```rust
struct SheetProtection {
    options: ProtectionOptions,
    password_hash: u16,
}
```

as `Option<SheetProtection>`. The three methods preserve existing values, as
the reference does:

- `protect` creates default options only when protection is first enabled.
- `protect_with_password` sets the verifier and keeps existing options.
- `protect_with_options` stores options and keeps an existing verifier.

The password verifier is the 16-bit legacy Excel algorithm over the UTF-8
bytes, with the byte length mixed in. Empty passwords hash to zero, which the
record treats as no password.

`BrtSheetProtection` (MS-XLSB 2.4.794) is 66 bytes:

- `protpwd: u16`
- `fLocked: u32`
- fifteen `u32` permission booleans: objects, scenarios, format cells,
  columns, rows, insert columns, rows, hyperlinks, delete columns, rows,
  selection of locked cells, sort, autofilter, pivot tables, selection of
  unlocked cells.

The XLSB fields are "editing is allowed" flags, so they map directly from the
options booleans. `fLocked` is 1. The optional `BrtSheetProtectionIso` pair is
omitted; the worksheet ABNF permits that.

The record is emitted after `BrtEndSheetData` and before merged cells,
hyperlinks, page setup, and the other post-cell records, matching
`WORKSHEET = ... CELLTABLE [BrtSheetCalcProp] [[BrtSheetProtectionIso]
BrtSheetProtection] ...`.

## 3. Compatibility Boundary

- Signatures and return types match `rust_xlsxwriter`.
- All `ProtectionOptions` field names and defaults match.
- Passwords are the weak legacy sheet verifier, not encryption; only the
  reference ASCII use case is claimed.
- `unprotect_range` and `unprotect_range_with_options` stay missing; they need
  protected-range records and are a separate gap.
- Default output is byte-identical when no protection method is called.

## 4. Change Necessity

The workbook model has no sheet-protection state, and core has no
`BrtSheetProtection` record. Both are required before any API can emit the
record.

## 5. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
record size/round-trip and exact default bytes, verifier vectors, package record
decode and ABNF position, default-output absence, and LibreOffice XLSX import
inspection for the generated `sheetProtection` element.

## 6. Tasks

1. Core
   - Add `BrtSheetProtection` with the 17 fields above, `Unchecked`,
     `assign_id!`, a 66-byte export case, and an exact-byte test.
   - Re-export it from `prelude::record` and `prelude`.
2. Writer API
   - Add `protection.rs` with `ProtectionOptions` and the verifier.
   - Add `Option<SheetProtection>` to `Worksheet`, with the three methods.
   - Re-export `ProtectionOptions`.
3. Serialization
   - Emit `BrtSheetProtection` once after `BrtEndSheetData` when enabled.
4. Tests
   - Verify verifier vectors `"" -> 0`, `"password" -> 0x83AF`,
     `"abc123" -> 0xC58F`.
   - Decode the record from a generated sheet and assert every flag mapping.
   - Assert `protect` preserves a previously set password and options.
   - Assert default worksheets emit no protection record.
   - Assert record order relative to `BrtEndSheetData` and `BrtBeginMergeCells`.
5. Default lock state
   - Lock `Format::new`, `CellStyle::default`, cell XF 0, and the Normal style
     XF; keep `set_unlocked` as the opt-out.
6. Docs
   - Mark the three protect methods supported in the gap analysis; leave
     unprotect ranges missing.
   - Update `AGENTS.md` API inventory and test counts.
   - Add this plan to `docs/aegis/INDEX.md`.

## 7. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- LibreOffice conversion to XLSX exposes the expected `sheetProtection`
  attributes and separate locked/unlocked XF protection states.

Result: 179 core + 145 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. Reviewer found one P1 defect: default cell XFs
were unlocked, so protection did not stop cell edits. `Format::new`,
`CellStyle::default`, cell XF 0, and the Normal style XF now lock cells, with
tests covering both locked and unlocked styles. LibreOffice conversion showed
locked style 1 and unlocked style 2 on the two written cells.

## 8. Risks

- The record layout and flag polarity are proven by decode tests and spec
  review; no Excel fixture is available.
- Legacy password protection is intentionally weak and not data encryption.
- LibreOffice may normalize option attributes on XLSB import; the record decode
  remains the primary oracle.
- Default cell XFs are now locked, matching Excel and `rust_xlsxwriter`. This
  changes the style bit for every workbook but fixes protection enforcement for
  unformatted cells; `Format::set_unlocked` remains the opt-out.

## 9. Retirement

No existing API is removed. `SheetProtection` is private and can later carry
protected-range state without changing the public options type.
