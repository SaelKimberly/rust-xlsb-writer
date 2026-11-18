# Conditional formatting: top/bottom, average, and duplicate rules

- Status: Implemented — reviewed; test gaps fixed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormatTop`, `ConditionalFormatAverage`,
  `ConditionalFormatDuplicate`, and `BrtBeginCFRule`.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`.
- Spec: MS-XLSB §2.4.23, §2.5.15–§2.5.18.
- Baseline: `c54f393 feat: add cell conditional formats`.

## Goal

Extend the shipped cell-value conditional formats with the three reference
rule families that need no CF14 extension records:

- `ConditionalFormatTop` — top/bottom count or percentage.
- `ConditionalFormatAverage` — above/below/equal average and 1–3 standard
  deviations.
- `ConditionalFormatDuplicate` — duplicate or inverted unique values.

All three reuse `BrtBeginConditionalFormatting`, `BrtBeginCFRule`, the existing
DXF table, per-sheet priorities, `stop_if_true`, and optional formats.

```rust
let top = ConditionalFormatTop::new()
    .set_rule(ConditionalFormatTopRule::BottomPercent(20))
    .set_format(Format::new().set_background_color(Color::Red));
worksheet.add_conditional_format(1, 0, 20, 0, &top)?;
```

Out of scope: text/date/blank/error/formula rules, colour scales, data bars,
icon sets, multi-range input, `BrtBeginCFRule14`, and extension GUIDs.

## Aegis Visibility

This adds public types and a public trait, changes the worksheet-owned rule
storage to a trait object, and widens a validated core record. The wire values
and compatibility boundary need a durable reference before code.

## Architecture

`rust-xlsb-core`:

- `BrtBeginCFRule` keeps its layout. The packed `flags` word gains constants
  for `fStopTrue` (bit 1), `fAbove` (bit 2), `fBottom` (bit 3), and `fPercent`
  (bit 4); reserved bits remain rejected.
- `Checked` accepts the supported `(iType, iTemplate)` combinations and
  validates `iParam`, flags, and formula presence for each.

`rust-xlsb-writer`:

- `conditional_format.rs` defines a public `ConditionalFormat` trait with
  doc-hidden implementation methods: validate, DXF lookup, build
  `BrtBeginCFRule`, and clone into `Box<dyn ConditionalFormat + Send + Sync>`.
- `ConditionalFormatRange` stores the range and that boxed rule.
- `Worksheet::add_conditional_format` becomes generic over
  `T: ConditionalFormat + Send + Sync`, preserving all existing cell-rule
  calls.
- Each rule type owns its wire mapping; `sheet.rs` only writes the range block,
  the rule, and the end markers.
- `package.rs` continues to register each rule's optional format through
  `Format::to_dxf` and `StyleRegistry::register_dxf`.

## Wire Contract

Shared:

- `dxfId` is the registered DXF index or `0xFFFFFFFF`.
- `iPri` starts at 1 per worksheet and is unique per sheet.
- `reserved1`, `reserved2`, `strParam`, and all unused flag bits are zero.
- `cbFmla1..3` and formulas are zero/omitted unless the rule is cell-value.
- A rule is emitted as one `BrtBeginConditionalFormatting` block containing
  `BrtBeginCFRule` / `BrtEndCFRule`, then `BrtEndConditionalFormatting`.

Top/bottom:

- `iType = 5` (`CF_TYPE_FILTER`), `iTemplate = 5`
  (`CF_TEMPLATE_FILTER`), `iParam = rank`.
- Flags: `fBottom` for bottom rules, `fPercent` for percentage rules,
  `fStopTrue` when requested. `fAbove = 0`.
- Count ranks are `1..=1000`; percentage ranks are `1..=100`.
- No formulas.

Average:

- `iType = 2` (`CF_TYPE_EXPRIS`).
- `iTemplate`: above `0x19`, below `0x1A`, equal-or-above `0x1D`,
  equal-or-below `0x1E`.
- `iParam` is the standard-deviation count `0..=3`; equal variants use `0`.
- `fAbove` is 1 for above/equal-or-above, 0 for below/equal-or-below.
  `fBottom` and `fPercent` are 0.
- No formulas, per the §2.4.23 formula table.

Duplicate/unique:

- `iType = 2`; `iTemplate = 0x1B` for duplicates or `0x07` for unique values.
- `iParam = 0`, `fAbove = fBottom = fPercent = 0`.
- No formulas.

## Compatibility Boundary

- Existing `ConditionalFormatCell` calls and bytes are unchanged.
- `add_conditional_format` accepts every implemented rule type by reference,
  matching the reference generic call shape.
- New rule types expose the reference names, defaults, `set_rule`, `set_format`,
  and `set_stop_if_true`; duplicate additionally exposes `invert`.
- Multi-range and custom-format APIs remain absent.
- Workbooks without conditional formats keep their existing bytes.

## TDD Route

- Mode: `off` — no explicit user/project TDD authority.
- Decision: `skipped`.
- Test posture: exact-byte core tests, writer rule-mapping tests, package decode
  tests, and a LibreOffice XLSB→XLSX round-trip.
- Verification: focused tests, then workspace/all-feature tests, clippy, fmt,
  `git diff --check`, and the external round-trip.

## Tasks

1. **Core rule validation**
   - Add flag constants/accessors and widen `BrtBeginCFRule::check`.
   - Validate filter rank/percent, average template/flags/`iParam`, duplicate
     template, and formula omission.
   - Add exact-byte tests for filter flags and average/duplicate records.
   - Verify: core conditional-record tests.

2. **Common worksheet rule interface**
   - Add the public `ConditionalFormat` trait and boxed storage.
   - Make `add_conditional_format` generic without changing call syntax.
   - Keep validation at add and sheet-order priority assignment at save.
   - Verify: existing cell tests plus a mixed-rule storage test.

3. **Top/bottom rules**
   - Add `ConditionalFormatTopRule` and `ConditionalFormatTop`.
   - Validate count/percentage ranges and encode `CF_TYPE_FILTER`.
   - Verify: builder/default/validation tests and package field assertions.

4. **Average rules**
   - Add `ConditionalFormatAverageRule` and `ConditionalFormatAverage`.
   - Map all ten variants to template, `iParam`, and `fAbove`.
   - Verify: every variant's encoded fields plus flag validation.

5. **Duplicate/unique rules**
   - Add `ConditionalFormatDuplicate` with `invert`.
   - Encode the duplicate or unique template with no formulas.
   - Verify: both templates and `stop_if_true`.

6. **Package, external, and docs**
   - Assert mixed-rule priorities, DXF ids/dedup, and no formula payloads for
     non-cell rules.
   - Convert a generated XLSB to XLSX with LibreOffice and inspect the `cfRule`
     attributes and DXFs.
   - Update gap analysis, AGENTS counts/architecture, plan status, and index.
   - Run the reviewer, fix valid findings, and commit via closing-sessions.

## Verification Matrix

| Seam | Evidence |
|---|---|
| Flags | Exact bytes at `reserved2` for stop/bottom/percent; reserved rejection |
| Top | Rank and percentage boundaries; `iType=5`, `iTemplate=5`, flags |
| Average | All ten variants map to the four templates, `iParam`, `fAbove` |
| Duplicate | Duplicate and unique templates, `iParam=0`, no formulas |
| API | Generic `add_conditional_format`; defaults, setters, validation |
| Package | Mixed rules, unique priorities, DXF ids, record order |
| External | LibreOffice XLSB→XLSX preserves rule types and formats |
| Regression | Existing cell-rule bytes and zero-DXF output |

## Risks And Limits

- Average and duplicate rules carry no formula in this wire mapping. The spec's
  formula table and the LibreOffice round-trip are the oracles.
- Calamine does not expose conditional formats; package decode is structural.
- Excel is unavailable; LibreOffice is the visual/import oracle.
- `BrtBeginCFRule14` and multi-range conditional formats remain unsupported.

## Retirement

No existing owner is replaced. `BrtBeginCFRule` stays the single non-14 rule
record; the existing DXF table and sheet write slot are reused.

## Evidence

- Default workspace: 543 tests pass (253 core + 283 writer unit tests and 7
  doctests).
- All features: 547 tests pass.
- `cargo clippy --workspace --all-targets --all-features`: no issues.
- `cargo fmt --all --check` and `git diff --check`: clean.
- LibreOffice XLSB→XLSX preserved top/bottom percent, equal-or-below average,
  duplicate values, and unique values on separate sheets, including DXF fills.
- LibreOffice renumbers XLSX priorities from the XLSB `iPri` values and
  collapses duplicate plus unique rules that share one range; package decode
  tests remain the priority oracle.

## Review

`ConditionalRulesReview` found no blocking defects. It verified flag bits,
all four average templates, filter rank/percent bounds, duplicate/unique
templates, formula omission, DXF dedup, per-sheet priorities, and reference
call shapes. It identified three test-coverage gaps, all fixed:

1. Top-rule mapping did not assert `iParam`.
2. Average/duplicate exact-byte tests did not assert `iParam` and flags.
3. Top-rule validation did not accept-check legal rank and percentage limits.
