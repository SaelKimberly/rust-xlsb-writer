# Multi-range data validation and conditional formatting

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `set_multi_range()` for data validation and every conditional-format
  rule type.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4 and §5.
- Reference: vendored `rust_xlsxwriter` v0.102.0
  `DataValidation::set_multi_range` and every
  `ConditionalFormat*::set_multi_range`.
- Spec: MS-XLSB §2.4.34 (`BrtBeginConditionalFormatting`), §2.4.356
  (`BrtDVal`), and §2.5.156 (`UncheckedSqRfX`).
- Baseline: `c40077a feat: add formula-driven validation and conditional
  formats`.

## Goal

Apply one rule to several non-contiguous ranges, matching the reference API:

```rust
let validation = DataValidation::new()
    .allow_whole_number(DataValidationRule::GreaterThan(0))
    .set_multi_range("B3:D6 I3:K6 B9:D12 I9:K12");
worksheet.add_data_validation(0, 0, 0, 0, &validation)?;

let rule = ConditionalFormatCell::new()
    .set_rule(ConditionalFormatCellRule::GreaterThan(50))
    .set_format(format)
    .set_multi_range("$B$3:$D$6,$I$3:$K$6,$B$9:$D$12,$I$9:$K$12");
worksheet.add_conditional_format(0, 0, 0, 0, &rule)?;
```

The range passed to `add_data_validation` or `add_conditional_format` is
validated as before and used when `set_multi_range` was not called. A
non-empty multi-range replaces that primary range.

## Aegis Visibility

`BrtDVal.sqrfx` and `BrtBeginConditionalFormatting.sqrfx` are durable wire
fields. Writing one range where Excel expects a union silently applies the
rule to too few cells; writing ranges in the wrong order or with swapped
rows/columns applies it to the wrong area. The public `set_multi_range` method
is source-compatible API and must exist on every rule type.

## Requirement Ready Check

- Approved source: gap analysis §3.4 and the vendored reference.
- Scenarios: space-separated ranges, comma-separated ranges, `$` anchors,
  single cells, multiple areas, replacement of the primary range, malformed
  input, range-count limits, and one rule with several DFX/formula records.
- Acceptance: the decoded `crfx`/`rgRefx` matches the requested areas; formulas
  in the rule use the first area's top-left cell as their relative base;
  worksheets without multi-ranges keep byte-identical output.
- No user decision is required; the reference defines the method names and
  argument types.

## Change Necessity

Both record types already carry `UncheckedSqRfX`, but the writer always builds
it with `crfx: 1`. The missing piece is a parser and storage for several
`UncheckedRfX` areas. The existing single range remains the default, so no
wire field changes are needed.

A shared parser is required because data validation and conditional formatting
must accept the same syntax. A second parser in either owner would drift on
anchors, separators, and limits.

## Ripple Signal Triage

- Canonical range parser becomes `rust-xlsb-writer/src/range_list.rs`.
- Canonical data-validation owner remains
  `rust-xlsb-writer/src/data_validation.rs`.
- Canonical conditional-format owner remains
  `rust-xlsb-writer/src/conditional_format.rs`.
- `rust-xlsb-core` keeps one conditional-format block helper and gains a
  multi-range variant; no record field changes.
- `Worksheet` stores the effective range list instead of one range.
- `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`, and
  `AGENTS.md` counts change.

## Architecture

`rust-xlsb-writer/src/range_list.rs`:

```rust
pub(crate) enum RangeList {
    Single(UncheckedRfX),
    Multiple(Box<[UncheckedRfX]>),
}
```

- `parse(input, max)` accepts A1 cells and `A1:B2` areas separated by spaces,
  commas, or whitespace. `$` anchors are accepted and ignored.
- Reject empty input, sheet/workbook qualifiers, more than three column
  letters, rows or columns outside `MAX_ROW`/`MAX_COL`, inverted areas, and
  counts over the caller limit.
- `from_primary(primary, multi_range, max)` returns `Single(primary)` when no
  override exists, otherwise the parsed list.
- `as_slice()` borrows the areas without allocation; `first()` supplies the
  formula base.

`rust-xlsb-writer/src/data_validation.rs`:

- Add `multi_range: Option<Box<str>>` and
  `set_multi_range(impl Into<String>) -> Self`.
- `record()` takes `&[UncheckedRfX]` and writes `crfx: len` with every area.
  Formula compilation uses `ranges[0]`.
- `DataValidationRange` stores `RangeList`.

`rust-xlsb-writer/src/conditional_format.rs`:

- Add the same `multi_range` field and setter to `ConditionalFormatText`,
  `Blank`, `Error`, `Formula`, `Date`, `Cell`, `Top`, `Average`, `Duplicate`,
  `2ColorScale`, `3ColorScale`, `DataBar`, `IconSet`, and `CustomIcon`.
- Add `ConditionalFormat::multi_range(&self) -> Option<&str>` with a `None`
  default; every rule returns its stored value.
- `ConditionalFormatRange` stores `RangeList`; `begin_rule` uses the first
  area, and `write_conditional_formats` emits one block containing every area.

`rust-xlsb-core/src/prelude/record/brt_conditional.rs`:

- Relax `BrtBeginConditionalFormatting::check` from exactly one range to
  `1..=8192`.
- Add `conditional_format_block_ranges(&[UncheckedRfX])`; keep
  `conditional_format_block` as the one-range wrapper used by existing
  callers.

`rust-xlsb-writer/src/worksheet.rs` and `sheet.rs`:

- `add_data_validation` and `add_conditional_format` parse the override after
  validating the primary range. Invalid input returns `Error::Parameter`.
- Duplicate validation detection compares the full effective range list.
- Emission passes the stored list to `record()` or the block helper.

## Wire Contract

| Rule | Record fields |
|---|---|
| Data validation, one range | `BrtDVal.sqrfx.crfx = 1`, one `UncheckedRfX` |
| Data validation, multi-range | `BrtDVal.sqrfx.crfx = n`, `rgRefx` in input order |
| Conditional format, one range | `BrtBeginConditionalFormatting.sqrfx.crfx = 1` |
| Conditional format, multi-range | `BrtBeginConditionalFormatting.sqrfx.crfx = n`, `rgRefx` in input order, one `BrtBeginCFRule` |

`UncheckedRfX` fields stay zero-based and inclusive. `crfx` is serialized by
deku from the vector length.

## Compatibility Boundary

- Existing calls without `set_multi_range` keep their current bytes.
- `set_multi_range` replaces the primary range, matching the reference.
- Empty or whitespace-only overrides clear the override.
- Malformed overrides are rejected by `add_*`, not by `set_multi_range`; the
  builder remains infallible.
- Limits: data validation accepts 1..=8191 areas (spec `< 8192`); conditional
  formatting accepts 1..=8192 areas (`BrtBeginConditionalFormatting14` is for
  larger unions).
- Multi-range strings are same-sheet only; sheet-qualified areas are rejected.
- The conditional-format rule formula compiles against the first area's
  top-left cell, matching Excel's multi-selection behavior.

## TDD Route

- Mode: `auto`.
- Decision: strict for public API and wire fields.
- Authority: new public methods plus durable range bytes.
- Test posture: add failing parser, worksheet, core, and package tests first;
  then implement.

## Tasks

1. **Shared range-list parser**
   - Files: `src/range_list.rs`, `src/lib.rs`.
   - Tests: spaces, commas, anchors, single cells, inverted and malformed
     areas, qualifiers, limits, empty input.
   - Verify: `cargo test -p rust-xlsb-writer range_list`.

2. **Data validation multi-range**
   - Files: `src/data_validation.rs`, `src/worksheet.rs`, `src/sheet.rs`.
   - Tests: setter, stored list, duplicate replacement, formula base, invalid
     input leaves state unchanged, record bytes with two areas.
   - Verify: `cargo test -p rust-xlsb-writer data_validation`.

3. **Conditional-format multi-range**
   - Files: `src/conditional_format.rs`, `src/worksheet.rs`, `src/sheet.rs`,
     `rust-xlsb-core/src/prelude/record/brt_conditional.rs`,
     `rust-xlsb-core/src/prelude/mod.rs`.
   - Tests: every rule type exposes the setter; worksheet stores all areas;
     formula base uses the first area; core block check accepts several ranges
     and rejects zero or too many; package emission.
   - Verify: `cargo test -p rust-xlsb-writer conditional_format` and
     `cargo test -p rust-xlsb-core brt_conditional`.

4. **Package integration**
   - Files: `src/package.rs`.
   - Save a workbook with multi-range validation and conditional formatting;
     decode `BrtDVal` and `BrtBeginConditionalFormatting` and assert `crfx`,
     `rgRefx`, and record order.
   - Verify: `cargo test -p rust-xlsb-writer package`.

5. **Documentation**
   - Update `docs/gap-analysis-rust-xlsxwriter.md` §3.4 and §5, this plan,
     `docs/aegis/INDEX.md`, and `AGENTS.md` test counts.
   - Verify: full workspace checks.

## Verification

- `cargo test --workspace`: 640 unit tests and 7 doctests pass (286 core,
  354 writer).
- `cargo test --workspace --all-features`: 644 unit tests and 7 doctests pass
  (286 core, 358 writer).
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- LibreOffice imports the generated XLSB and exports
  `sqref="B3:D6 I3:K6"` for both the data validation and the conditional
  format.
- Package decode tests assert the exact range order and `crfx`.

## Review

`MultiRangeReview` found no blocking or minor defects. It verified the
protocol limits, zero-based/clusive range encoding, one-record union emission,
formula base, worksheet state on malformed input, single-range byte identity,
allocation behavior, and source-compatible setters on all 13 rule types.
Residual test and documentation gaps were closed afterward:

- core and parser boundary tests now exercise 8192 accepted and 8193 rejected;
- the conditional-format setter test now covers whitespace-only clearing;
- the public `add_*` docs describe the multi-range override and its
  `Error::Parameter` failure path.

## Risks

- `UncheckedSqRfX` stores a count plus an owned vector, so emission allocates
  one vector per rule. The one-range case does not allocate until record
  construction, and the parser stores `Single` without a box.
- LibreOffice and Excel may order multiple areas differently on import/export;
  the writer preserves caller order.
- A malformed multi-range string is reported by `add_*`, unlike the reference,
  which stores the raw string for later serialization. This is the safer
  failure point and keeps worksheet state unchanged.
- Conditional-format formulas use the first area's top-left cell as their
  relative base. If Excel instead uses the active cell, a multi-selection with
  a different active cell could evaluate differently; the first area is the
  only stable base the writer can store.
