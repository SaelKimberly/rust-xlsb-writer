# Conditional formatting: data bars

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormatDataBar`, `ConditionalFormatType`, `BrtBeginDatabar`,
  `BrtEndDatabar`, and the existing `BrtCFVO`/`BrtColor` records.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`
  (`ConditionalFormatDataBar`, `set_minimum`, `set_maximum`, `set_fill_color`,
  `set_bar_only`).
- Spec: MS-XLSB §2.4.23, §2.4.43, §2.4.334, §2.4.337, §2.4.399, §2.5.16,
  §2.5.18; ABNF `DATABAR = BrtBeginDatabar 2BrtCFVO BrtColor BrtEndDatabar`.
- Baseline: `721a596 feat: add color scale conditional formats`.

## Goal

Add the basic data-bar rule family:

```rust
let bar = ConditionalFormatDataBar::new()
    .set_minimum(ConditionalFormatType::Number, 3)
    .set_maximum(ConditionalFormatType::Number, 7)
    .set_fill_color(Color::Green)
    .set_bar_only(true);
worksheet.add_conditional_format(1, 0, 20, 0, &bar)?;
```

The rule reuses the existing CF range block, per-sheet priorities, trait
storage, record body hook, `BrtCFVO`, and `BrtColor`.

Out of scope: icon sets, CF14 records and extensions, data-bar borders,
negative bars, axis controls, direction, solid fill, gradients, formula
thresholds, multi-range input, and DXF formats.

## Aegis Visibility

This adds a public rule type and two core records, and extends the CF ABNF path
inside `BrtBeginCFRule`. The nested record order is durable: parsing succeeds
even when the order is wrong, but Excel then drops the bar.

## Architecture

`rust-xlsb-core`:

- `brt_conditional.rs` adds `CF_TYPE_DATABAR=4`, `CF_TEMPLATE_DATABAR=3`, the
  three-byte `BrtBeginDatabar`, and the zero-sized `BrtEndDatabar`.
- `BrtBeginDatabar` is `Checked`: `bLenMin <= bLenMax <= 100`; `fShowValue` is
  `0` or `1`.
- `BrtBeginCFRule::check` accepts `(CF_TYPE_DATABAR, CF_TEMPLATE_DATABAR)`
  through the same no-DXF/no-formula guard used by color scales.
- `BrtCFVO` and `BrtColor` are unchanged; the existing 24-byte MIN/MAX form and
  explicit RGB/theme colors are reused.

`rust-xlsb-writer`:

- `conditional_format.rs` renames the private `ScaleStop`/`ScalePosition` to
  `Threshold`/`ThresholdPosition`, generalizes their error text, and adds
  `ConditionalFormatDataBar`.
- Defaults: minimum `Lowest`, maximum `Highest`, fill `638EC6`, bar-only off.
- `set_minimum`/`set_maximum` accept `impl IntoConditionalFormatValue`;
  minimum rejects `Highest`, maximum rejects `Lowest`; percent/percentile values
  must be finite `0..=100`; number values must be finite.
- `set_fill_color` ignores invalid colors; `set_bar_only` controls
  `fShowValue`.
- `write_body` emits `BrtBeginDatabar`, minimum `BrtCFVO`, maximum `BrtCFVO`,
  one `BrtColor`, then `BrtEndDatabar`.
- `lib.rs` exports the new type.

`package.rs` needs no production change: data bars expose no DXF and therefore
use `u32::MAX`. Tests cover package assembly.

## Wire Contract

- One `BrtBeginConditionalFormatting` block per rule, `ccf=1`, one range.
- `BrtBeginCFRule`: `iType=4` (`CF_TYPE_DATABAR`), `iTemplate=3`
  (`CF_TEMPLATE_DATABAR`), `iParam=0`, `dxfId=0xFFFFFFFF`, all flags zero,
  `cbFmla1..3=0`, null `strParam`.
- `BrtBeginDatabar`: `bLenMin=10`, `bLenMax=90`, `fShowValue=1` by default.
  `set_bar_only(true)` writes `fShowValue=0`.
- Body order: `BrtBeginDatabar`, minimum `BrtCFVO`, maximum `BrtCFVO`,
  `BrtColor`, `BrtEndDatabar`.
- CFVO mapping: `Lowest` -> `CFVO_MIN`; `Highest` -> `CFVO_MAX`; `Number` ->
  `CFVO_NUM`; `Percent` -> `CFVO_PERCENT`; `Percentile` ->
  `CFVO_PERCENTILE`.
- `BrtColor` is the explicit RGB/theme color produced by
  `Color::to_brt_color`; `Default` and `Automatic` are rejected.
- No DXF and no formula payload.

## Compatibility Boundary

- Existing cell, top/bottom, average, duplicate/unique, and color-scale rules
  keep their bytes and call shapes.
- Public names match the reference subset: `ConditionalFormatDataBar::new`,
  `set_minimum`, `set_maximum`, `set_fill_color`, and `set_bar_only`.
- `ConditionalFormatType::Automatic` and `Formula` remain absent; data-bar
  defaults use `Lowest`/`Highest`, which produce the same MIN/MAX records.
- Workbooks without data bars keep their existing bytes.

## TDD Route

- Mode: `off` — no explicit user/project TDD authority.
- Decision: `skipped`.
- Test posture: exact-byte core records, writer mapping/validation tests,
  package record-order/decode tests, and known-good fixture comparison.
- Verification: focused tests, then workspace/all-feature tests, clippy, fmt,
  and `git diff --check`.

## Tasks

1. **Core records and validation**
   - Add the data-bar constants, `BrtBeginDatabar`, `BrtEndDatabar`, and
     data-bar rule validation.
   - Assert the 3-byte begin payload, 0-byte end marker, length ordering, and
     flag range.
   - Verify: core conditional-record tests and the `test_biff_export` table.

2. **Shared threshold cleanup**
   - Rename the private scale-only threshold vocabulary to a neutral name and
     generalize error text without changing scale bytes.
   - Verify: existing color-scale tests.

3. **Data bar rule**
   - Add `ConditionalFormatDataBar` with defaults, threshold setters, fill
     color, and bar-only.
   - Validate positions, numeric values, percentages, and explicit colors.
   - Verify: default/custom thresholds, bar-only, invalid values, no DXF.

4. **Package and external verification**
   - Assert the block/rule/body/end order, rule fields, CFVO types/values,
     color, and absent DXF.
   - Compare record order and `BrtBeginDatabar` bytes with the known-good
     `github.com/mbleron/ExcelGen` `cond-formatting.xlsb` fixture.
   - Update gap analysis, AGENTS counts/architecture, plan status, and index.
   - Run the reviewer, fix valid findings, and commit via closing-sessions.

## Verification Matrix

| Seam | Evidence |
|---|---|
| Begin record | 3-byte body; default 10/90/1; bar-only 10/90/0 |
| Rule | `iType=4`, `iTemplate=3`, `iParam=0`, zero flags/formulas, `dxfId=MAX` |
| Body | Begin, MIN, MAX, one color, end; no formulas or DXF |
| Thresholds | All five `ConditionalFormatType` mappings; bounds rejected |
| API | Reference call shape compiles; invalid values rejected |
| Package | Record order; no DXF; existing rule bytes unchanged |
| External | Known-good XLSB has the same begin payload and record order |
| Regression | Workspace/all-feature tests, clippy, fmt, diff check |

## Risks And Limits

- Excel writes MIN/MAX threshold records at 32 bytes with an ignored empty
  formula; this writer emits the spec-permitted 24-byte form. The known-good
  fixture shows the 32-byte form. Exact-byte tests cover our chosen form.
- The basic XLSB record exposes only bar lengths, show-value, two thresholds,
  and one color. Border, negative, axis, direction, and gradient controls need
  CF14 and stay absent.
- LibreOffice may drop data bars on XLSB import, as it does for color scales;
  package decode and fixture comparison are the oracle.

## Evidence

- Default workspace: 564 tests pass (264 core + 293 writer unit tests and 7
  doctests).
- All features: 568 tests pass.
- `cargo clippy --workspace --all-targets --all-features`: no warnings.
- `cargo fmt --all --check` and `git diff --check`: clean.
- The known-good fixture has `BrtBeginDatabar` payload `0a5a00`
  (`bLenMin=10`, `bLenMax=90`, `fShowValue=0`) between `BrtBeginCFRule` with
  `iType=4`, `iTemplate=3`, `dxfId=0xFFFFFFFF`, zero flags, and two 32-byte
  MIN/MAX `BrtCFVO` records, one `BrtColor`, and `BrtEndDatabar`.
- A generated workbook with a default bar and a bar-only fixed-range green bar
  decodes to the expected `BrtBeginDatabar` payloads `0a5a01` and `0a5a00`.
- LibreOffice drops the data bars on XLSB→XLSX conversion, matching its
  behavior for color scales. Package decode and fixture comparison are the
  oracle.

## Review

`DataBarReview` found no blocking defects. It verified the rule header, begin
payload, body order, CFVO mapping, color handling, DXF avoidance, priorities,
exports, and the color-scale validation refactor against the known-good
fixture.

The four non-blocking coverage gaps were fixed:

1. The invalid-fill-color branch is pinned; an out-of-range RGB leaves the
   default blue.
2. Data-bar and threshold boundary-accept cases cover percent/percentile 0 and
   100, negative numbers, and `Lowest`/`Highest`.
3. The full 46-byte `BrtBeginCFRule` payload is asserted.
4. A mixed formatted rule plus data bar asserts the DXF id stays aligned and the
   `None` entry does not consume a style id.
