# Conditional formatting: 2- and 3-color scales

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormat2ColorScale`, `ConditionalFormat3ColorScale`,
  `ConditionalFormatType`, `BrtCFVO`, and the color-scale markers.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`
  (`ConditionalFormat2ColorScale`, `ConditionalFormat3ColorScale`).
- Spec: MS-XLSB §2.4.23, §2.4.28, §2.4.334, §2.4.385, §2.5.16, §2.5.18,
  §2.5.19; older grammar `COLORSCALE = BrtBeginColorScale
  ((2CFVO 2BrtColor) / (3CFVO 3BrtColor)) BrtEndColorScale`.
- Baseline: `b102179 feat: add conditional format rule families`.

## Goal

Add the two reference color-scale rule families without CF14 records:

```rust
let scale = ConditionalFormat2ColorScale::new()
    .set_minimum(ConditionalFormatType::Number, 3)
    .set_maximum(ConditionalFormatType::Number, 7)
    .set_minimum_color(Color::Yellow)
    .set_maximum_color(Color::Green);
worksheet.add_conditional_format(1, 0, 20, 0, &scale)?;
```

Both reuse the existing CF range block, per-sheet priorities, worksheet-owned
trait storage, and package assembly. They need no DXF or shared-string state.

Out of scope: data bars, icon sets, text/date/formula rules, multi-range input,
CF14 records, and formula-valued CFVO thresholds.

## Aegis Visibility

This adds public types and three core records, and extends the CF ABNF path
inside `BrtBeginCFRule`. The exact record order and `dxfId` sentinel need a
durable contract because an incorrect sequence still parses as records but
renders no scale.

## Architecture

`rust-xlsb-core`:

- `brt_conditional.rs` adds constants `CF_TYPE_GRADIENT`, `CF_TEMPLATE_GRADIENT`,
  and `CFVO_NUM/MIN/MAX/PERCENT/PERCENTILE`.
- `BrtBeginCFRule::check` accepts `(CF_TYPE_GRADIENT, CF_TEMPLATE_GRADIENT)`,
  requiring `iParam=0`, no flags, no formulas, and `dxfId=0xFFFFFFFF`.
- `BrtCFVO` is a checked 24-byte record with `iType`, `numParam`,
  `fSaveGTE`, `fGTE`, `cbFmla`, and an optional formula slot reserved for a
  later threshold-formula feature. This implementation requires `cbFmla=0`.
- Zero-sized `BrtBeginColorScale` and `BrtEndColorScale` markers.

`rust-xlsb-writer`:

- Extend the hidden `ConditionalFormat` trait with a default no-op
  `write_body(&mut Vec<u8>)` so color scales can emit their nested records
  without allocating a temporary record list.
- `sheet.rs` writes `begin_rule`, then `write_body`, then `BrtEndCFRule`.
  Existing rule types keep an empty body.
- `ConditionalFormatRange` delegates the body call.
- `conditional_format.rs` defines the scale types and `ConditionalFormatType`.
- `package.rs` needs no change: color scales expose no DXF and therefore use
  `u32::MAX`.

## Wire Contract

Shared:

- One `BrtBeginConditionalFormatting` block per rule, `ccf=1`, one range.
- `BrtBeginCFRule`: `iType=3` (`CF_TYPE_GRADIENT`), `iTemplate=2`
  (`CF_TEMPLATE_GRADIENT`), `iParam=0`, `dxfId=0xFFFFFFFF`, all flags zero,
  `cbFmla1..3=0`, null `strParam`.
- Body records appear before `BrtEndCFRule`:
  `BrtBeginColorScale`, all `BrtCFVO` thresholds in scale order, all
  `BrtColor` entries in the same order, then `BrtEndColorScale`.
- 2-color scale: 2 CFVO + 2 colors. 3-color scale: 3 CFVO + 3 colors.
- `BrtCFVO`: `fSaveGTE=0`, `fGTE=0`, `cbFmla=0`; no threshold formula.
- `BrtColor`: the RGB/theme `BrtColor` produced by `Color::to_brt_color`.

Scale mapping:

- Lowest -> `CFVO_MIN`; Highest -> `CFVO_MAX`; Number -> `CFVO_NUM`;
  Percent -> `CFVO_PERCENT`; Percentile -> `CFVO_PERCENTILE`.
- Defaults: 2-color min `Lowest`, max `Highest`, colors `FFEF9C` and `63BE7B`;
  3-color min `Lowest`, midpoint `Percentile(50)`, max `Highest`, colors
  `F8696B`, `FFEB84`, `63BE7B`.
- Minimum cannot be `Highest`; maximum cannot be `Lowest`; midpoint cannot be
  either. Percent and percentile values are finite `0..=100`; Number values
  are finite.
- `set_stop_if_true(true)` is rejected at add because §2.4.23 requires the
  gradient flag to be zero.

## Compatibility Boundary

- Existing cell, top/bottom, average, and duplicate rules keep their bytes and
  call shapes.
- New public names match the reference: `ConditionalFormat2ColorScale`,
  `ConditionalFormat3ColorScale`, `ConditionalFormatType`, `set_minimum`,
  `set_maximum`, `set_minimum_color`, `set_maximum_color`, midpoint setters,
  and `set_stop_if_true`.
- `set_minimum`/`set_maximum`/`set_midpoint` accept
  `impl IntoConditionalFormatValue`; string values are rejected at add.
- The `Formula` and `Automatic` type variants, data bars, icon sets,
  multi-range input, and CF14 are absent.
- Workbooks without color scales keep their existing bytes.

## TDD Route

- Mode: `off` — no explicit user/project TDD authority.
- Decision: `skipped`.
- Test posture: exact-byte core tests, writer mapping/validation tests,
  package record-order/decode tests, and a LibreOffice XLSB→XLSX round-trip.
- Verification: focused tests, then workspace/all-feature tests, clippy, fmt,
  `git diff --check`, and the external round-trip.

## Tasks

1. **Core records and validation**
   - Add CFVO constants, `BrtCFVO`, color-scale markers, and gradient rule
     validation.
   - Assert the 24-byte CFVO payload, min/max/percent flags, and reserved-field
     rejection.
   - Verify: core conditional-record tests.

2. **Scale trait plumbing**
   - Add the hidden body hook with a default no-op.
   - Call it between `BrtBeginCFRule` and `BrtEndCFRule`.
   - Verify: existing rule tests still pass.

3. **2-color scale**
   - Add `ConditionalFormatType` and `ConditionalFormat2ColorScale`.
   - Validate position/type/value combinations and map to two CFVOs and two
     colors.
   - Verify: defaults, each type, color overrides, and rejection cases.

4. **3-color scale**
   - Add `ConditionalFormat3ColorScale` with midpoint defaults and setters.
   - Encode three CFVOs and colors in order.
   - Verify: default midpoint, all midpoint types, and position validation.

5. **Package and external verification**
   - Assert `BrtBeginColorScale`, CFVO/color order, `BrtEndColorScale`, null
     DXF, and unchanged cell-rule record order.
   - Convert a generated XLSB to XLSX with LibreOffice and inspect the
     `colorScale`, `cfvo`, and `color` elements.
   - Update gap analysis, AGENTS counts/architecture, plan status, and index.
   - Run the reviewer, fix valid findings, and commit via closing-sessions.

## Verification Matrix

| Seam | Evidence |
|---|---|
| CFVO | Exact 24-byte payload; min/max/num/percent/percentile mapping |
| Rule | `iType=3`, `iTemplate=2`, `iParam=0`, zero flags/formulas, `dxfId=MAX` |
| 2-color | Two CFVO then two colors; defaults and overrides |
| 3-color | Three CFVO then three colors; midpoint default and overrides |
| API | Builders compile with reference call shape; invalid values rejected |
| Package | Record order; no DXF; existing cell-rule bytes unchanged |
| External | LibreOffice XLSB→XLSX preserves both color scales |
| Regression | Workspace/all-feature tests, clippy, fmt, diff check |

## Risks And Limits

- The older grammar confirms the all-CFVOs-then-all-colors order; the current
  revision still needs the package test because the worksheet ABNF does not
  expand the nested production.
- LibreOffice may normalize colors or thresholds on XLSX export. Exact-byte
  tests remain the oracle; the round-trip is a visual/structural check.
- Formula-valued thresholds require the future formula encoder and are not
  part of this batch.

## Evidence

- Default workspace: 555 tests pass (260 core + 288 writer unit tests and 7
  doctests).
- All features: 559 tests pass.
- `cargo clippy --workspace --all-targets --all-features`: no warnings.
- `cargo fmt --all --check` and `git diff --check`: clean.
- A known-good `cond-formatting.xlsb` fixture from
  `github.com/mbleron/ExcelGen` has the same record order
  (`BrtBeginColorScale`, all `BrtCFVO`, all `BrtColor`, `BrtEndColorScale`) and
  the same 24-byte CFVO payload for NUM/PERCENT/PERCENTILE records and the
  same 8-byte color payloads. Its first two-color rule has `iType=3`,
  `iTemplate=2`, `iParam=0`, `dxfId=0xFFFFFFFF`, zero flags, and no formulas;
  only `iPri` differs because the fixture has earlier rules.
- Excel writes MIN/MAX thresholds with an ignored empty formula and one
  ignored GTE flag, producing a 32-byte CFVO; this writer omits the ignored
  bytes, which the spec permits. Package tests cover the 24-byte form.
- LibreOffice drops color scales on XLSB import for both this writer and the
  known-good fixture, including PDF output, so it is not an external oracle for
  this feature. Package decode and exact-byte tests remain the oracle.

## Review

`ColorScaleReview` found no blocking defects. It verified the rule header, body
order, record ids, CFVO/color layouts, defaults, enum mapping, validation,
existing-rule regression, and DXF absence.

Non-blocking observations, disposition:

1. Excel writes MIN/MAX CFVO records at 32 bytes with an ignored empty formula
   and ignored GTE flag. This writer emits the spec-permitted 24-byte form; the
   limitation is documented above rather than adding dead bytes.
2. `ConditionalFormatType::Automatic` and `Formula` are absent by the stated
   scope. Formula thresholds need the future formula encoder.
3. LibreOffice drops color scales on XLSB import even for a known-good fixture;
   exact-byte/package tests are the oracle.
