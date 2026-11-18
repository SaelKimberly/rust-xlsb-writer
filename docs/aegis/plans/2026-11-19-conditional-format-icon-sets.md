# Conditional formatting: icon sets

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormatIconSet`, `ConditionalFormatCustomIcon`,
  `ConditionalFormatIconType`, `BrtBeginIconSet`, `BrtEndIconSet`, and the
  existing `BrtCFVO` record.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`
  (`ConditionalFormatIconSet`, `ConditionalFormatCustomIcon`,
  `ConditionalFormatIconType`).
- Spec: MS-XLSB §2.4.23, §2.4.91, §2.4.334, §2.4.445, §2.5.16, §2.5.18,
  §2.5.86.
- Baseline: `12eaec9 feat: add data bar conditional formats`.

## Goal

Add Excel's standard icon-set rules:

```rust
let rule = ConditionalFormatIconSet::new()
    .set_icon_type(ConditionalFormatIconType::ThreeTrafficLights)
    .reverse_icons(true)
    .show_icons_only(true);
worksheet.add_conditional_format(1, 0, 20, 0, &rule)?;
```

Custom thresholds use the same reference API shape:

```rust
let icons = [
    ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percent, 0),
    ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percentile, 50),
    ConditionalFormatCustomIcon::new().set_rule(ConditionalFormatType::Percentile, 90),
];
let rule = ConditionalFormatIconSet::new()
    .set_icon_type(ConditionalFormatIconType::ThreeTrafficLights)
    .set_icons(&icons);
```

Out of scope: CF14/`BrtBeginIconSet14`, custom icon pictures, `ThreeStars`,
`ThreeTriangles`, and `FiveBoxes` (x14-only sets), multi-range rules, formula
thresholds, and stop-if-true. MS-XLSB requires `fStopTrue=0` for multi-state
rules, so the API omits it rather than silently dropping it.

## Aegis Visibility

This adds a public rule family and two records, and extends the CF type/template
dispatch. The nested record order and the 6-byte begin payload are durable wire
contracts that package tests must pin.

## Architecture

`rust-xlsb-core`:

- `brt_conditional.rs` adds `CF_TYPE_MULTISTATE=6`,
  `CF_TEMPLATE_MULTISTATE=4`, `BrtBeginIconSet`, and `BrtEndIconSet`.
- `BrtBeginIconSet` stores `iSet: u32` and raw `flags: u16`. `F_ICON=1<<1`,
  `F_REVERSE=1<<2`; `Checked` accepts KPI sets `0..=0x10` and only those two
  bits.
- `BrtBeginCFRule::check` accepts `(CF_TYPE_MULTISTATE,
  CF_TEMPLATE_MULTISTATE)` through the no-DXF/no-parameter/no-formula guard.
- `BrtCFVO` is unchanged; icon thresholds set `fSaveGTE=1` and `fGTE=1`.

`rust-xlsb-writer`:

- `conditional_format.rs` adds `ConditionalFormatIconType` with all standard
  sets, `ConditionalFormatCustomIcon`, and `ConditionalFormatIconSet`.
- Default thresholds match the reference: 3 icons `Percent(0,33,67)`, 4 icons
  `Percent(0,25,50,75)`, 5 icons `Percent(0,20,40,60,80)`.
- `set_icon_type` resets thresholds to that set's defaults.
- `set_icons` copies the threshold slice, forces the first to `Percent(0)`,
  and stores it exactly; validation rejects a count that does not match the
  icon type.
- Thresholds accept `Number`, `Percent`, or `Percentile`; values must be finite,
  and percent/percentile values must be `0..=100`.
- `reverse_icons` sets `F_REVERSE`; `show_icons_only` sets `F_ICON`.
- `write_body` emits `BrtBeginIconSet`, one `BrtCFVO` per threshold, then
  `BrtEndIconSet`.
- `lib.rs` exports the three new types.

`package.rs` needs no production change: icon sets expose no DXF and use
`u32::MAX`.

## Wire Contract

- `BrtBeginCFRule`: `iType=6` (`CF_TYPE_MULTISTATE`), `iTemplate=4`
  (`CF_TEMPLATE_MULTISTATE`), `dxfId=0xFFFFFFFF`, `iParam=0`, all flags zero,
  `cbFmla1..3=0`, null `strParam`.
- `BrtBeginIconSet`: `iSet` = `KPISets` value, `flags` = `fIcon<<1 |
  fReverse<<2`. Payload is 6 bytes.
- `BrtCFVO`: threshold type `CFVO_NUM`/`CFVO_PERCENT`/`CFVO_PERCENTILE`,
  `numParam` = numeric value, `fSaveGTE=1`, `fGTE=1`, no formula.
- The first `BrtCFVO` is ignored by Excel but still required; it is always
  `Percent(0)`.
- `BrtEndIconSet` is zero-sized.

KPI mapping: `ThreeArrows=0`, `ThreeArrowsGray=1`, `ThreeFlags=2`,
`ThreeTrafficLights=3`, `ThreeTrafficLightsWithRim=4`, `ThreeSigns=5`,
`ThreeSymbolsCircled=6`, `ThreeSymbols=7`, `FourArrows=8`,
`FourArrowsGray=9`, `FourRedToBlack=10`, `FourHistograms=11`,
`FourTrafficLights=12`, `FiveArrows=13`, `FiveArrowsGray=14`,
`FiveHistograms=15`, `FiveQuadrants=16`.

## TDD Route

- Mode: `auto`; decision: `light`.
- Authority: user request is implementation plus reviewer, no explicit strict
  TDD.
- Posture: core exact-byte/layout tests and writer body/mapping/validation
  tests; package test decodes the nested records.
- Verification: `cargo test --workspace`, all-features tests, clippy, fmt,
  `git diff --check`.

## Tasks

1. Core records and constants.
   - Add `BrtBeginIconSet`/`BrtEndIconSet`, constants, checks, exports.
   - Extend the rule dispatch.
   - Add exact-byte, boundary, and rule-body tests.
2. Writer API.
   - Add the icon type enum, custom threshold type, and icon-set rule.
   - Add default generation, threshold validation, and body serialization.
   - Export from `lib.rs`.
   - Add unit tests for every icon set count, flags, body order, and invalid
     thresholds.
3. Package oracle.
   - Add a test that decodes the rule block, begin/end records, CFVO types and
     values, and asserts no DXF.
4. Docs and verification.
   - Update `AGENTS.md`, gap analysis, plan evidence, and index.
   - Run the clean checks above.

## Evidence

- Default workspace: 573 tests pass (268 core + 298 writer unit tests and 7
  doctests).
- All features: 577 tests pass.
- `cargo clippy --workspace --all-targets --all-features`: no warnings.
- `cargo fmt --all --check` and `git diff --check`: clean.
- Package decode covers default and five-icon rules, begin/end markers, CFVO
  order and values, flags, and the zero-DXF styles table.
- No XLSB fixture with icon sets was available; the spec layout and decoded
  package are the oracle.

## Verification

- Core: `BrtBeginIconSet` payload `[iSet LE, flags LE]`; rejects invalid KPI
  and reserved bits; accepts icon-only and reverse flags.
- Rule: exact 46-byte `BrtBeginCFRule` body with `iType=6`, `iTemplate=4`,
  `dxfId=0xFFFFFFFF`.
- Writer: body begins `BrtBeginIconSet`, contains 3–5 CFVO records, ends
  `BrtEndIconSet`; all CFVO use `fSaveGTE=1`, `fGTE=1`; first is
  `Percent(0)`.
- Package: nested records decode in order; no DXF entry.
- Existing conditional-format bytes are unchanged.

## Review

`IconSetReview` found no blocking defects. It confirmed the record ids, 6-byte
begin payload, flag bit positions, rule type/template, CFVO layout and ordering,
no-DXF/no-formula rule, KPI mapping, and test counts against MS-XLSB.

Two non-blocking findings were fixed:

1. The writer flag test now checks reverse-only, icon-only, and both flags
   separately, so swapped or duplicated bits cannot pass.
2. The plan no longer claims `set_icons` copies at most five thresholds; it
   copies the slice and validation rejects a mismatched count.

The remaining residual risk is the contradictory `fReverse` value table in
§2.4.91. The writer uses bit 2 = reversed, matching the field name and XLSX
`reverse="1"`. No local Excel-produced XLSB fixture with icon sets exists;
package decoding is the oracle.

## Risks

- `BrtBeginIconSet` flag bit positions follow §2.4.91. The spec table's
  `fReverse` value wording is contradictory; the writer uses bit 2 = reversed,
  matching the field name and the XLSX `reverse` attribute.
- LibreOffice cannot import XLSB icon sets reliably; package decoding is the
  oracle.
- Excel may write 32-byte CFVO records with an ignored empty formula; this
  writer emits the spec-permitted 24-byte no-formula form, as for color scales
  and data bars.
