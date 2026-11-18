# Conditional formatting: numeric and string cell rules

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`,
  `ConditionalFormatCell`, `ConditionalFormatCellRule`,
  `ConditionalFormatValue`, `BrtDXF`, `XFProps`, `BrtBeginCFRule`, and
  `BrtBeginConditionalFormatting`.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0
  `conditional_format.rs` cell-rule API and MS-XLSB §2.4.23, §2.4.34,
  §2.4.355–§2.4.359, §2.5.15, §2.5.18, §2.5.98.6, §2.5.159–§2.5.164.

## Goal

Add the first conditional-formatting slice: the reference `ConditionalFormatCell`
API for equal, not-equal, greater/less, greater/less-or-equal, between, and
not-between rules over numbers and strings, with an optional differential
format.

```rust
let rule = ConditionalFormatCell::new()
    .set_rule(ConditionalFormatCellRule::GreaterThanOrEqualTo(50))
    .set_format(Format::new().set_background_color(Color::Green));
worksheet.add_conditional_format(1, 0, 10, 0, &rule)?;
```

The rule writes `BrtBeginConditionalFormatting` / `BrtBeginCFRule` /
`BrtEndCFRule` / `BrtEndConditionalFormatting`. The optional format writes a
`BrtDXF` into the styles part and is referenced by `dxfId`.

Explicitly out of scope: colour scales, data bars, icon sets, top/bottom,
average, duplicate/unique, blank/error, date, text-specific, and formula
rules; multi-range input; dates/times as rule values; `stop_if_true`
is supported because it is one packed bit. `BrtBeginCFRule14` and extension
records are not emitted.

## Aegis Visibility

The change adds a public API, a public rule/value vocabulary, a worksheet-owned
rule list, two core record families, a new style-registry table, and a new
styles/worksheet save seam. The plan pins the wire layouts, DXF subset, and
save order before code.

## Architecture

`rust-xlsb-core`:

- `prelude/record/brt_dxf.rs`: `BrtDXF`, `XFProps`, `XFProp`.
- `prelude/record/brt_conditional.rs`: `BrtBeginConditionalFormatting`,
  `BrtEndConditionalFormatting`, `CFParsedFormula`, `BrtBeginCFRule`.
- `StyleRegistry` owns a deduplicated `Vec<BrtDXF>` and
  `HashMap<BrtDXF, u32>`; `register_dxf` returns the zero-based index.
- `BrtColor::xf_prop_bytes` exposes the exact 8-byte `XFPropColor` payload
  without a heap allocation.

`rust-xlsb-writer`:

- `conditional_format.rs`: public rule/value types and the internal
  `ConditionalFormatRange` stored by a worksheet.
- `Worksheet::conditional_formats` list plus `add_conditional_format`,
  range/order checks, and rule validation at save.
- `Format::to_dxf` maps the supported font and fill properties to `BrtDXF`;
  unsupported differential properties are rejected instead of silently
  dropped.
- `package::build_package` registers each conditional format's DXF after cell
  formats and rich fonts, before `registry.write_to_biff`, and passes one
  `Option<u32>` per worksheet rule to the sheet writer.
- `sheet.rs` emits each `BrtBeginConditionalFormatting` block after merged
  cells and before data validation, matching the worksheet ABNF order. The
  non-14 rules are not wrapped in the CF14 `BrtBeginConditionalFormattings`
  collection.

## Wire Contract

`BrtBeginConditionalFormatting` is `ccf: u32`, `fPivot: u32`, then
`UncheckedSqRfX` with one `UncheckedRfX`. `ccf = 1`; every range is emitted as
its own block.

`BrtBeginCFRule`:

1. `iType = 1` (`CF_TYPE_CELLIS`), `iTemplate = 0`.
2. `dxfId` is the registered DXF index or `0xFFFFFFFF` when no format was set.
3. `iPri` starts at 1 per worksheet and increases by one per rule.
4. `iParam` is the `CFOper` value: between `1`, not-between `2`, equal `3`,
   not-equal `4`, greater `5`, less `6`, greater-or-equal `7`,
   less-or-equal `8`.
5. `reserved1`, `reserved2`, and reserved flag bits are zero; only
   `fStopTrue` may be set.
6. `cbFmla1` and `cbFmla2` equal the corresponding `CFParsedFormula.cce`;
   `cbFmla3 = 0`.
7. `strParam` is null.
8. `rgce1` and, for between/not-between, `rgce2` are `CFParsedFormula`
   records with a `PtgNum` or `PtgStr` token and `cb = 0`.

`BrtDXF` is a 15-bit zero reserved field, `fNewBorder = 0`, then `XFProps`.
`XFProps` is `reserved = 0`, `cprops`, and `XFProp` entries. Each `XFProp`
stores its 2-byte type, a 2-byte byte length, and opaque property bytes.

Supported DXF properties:

- `0x0002` background color from `BrtFill.brtColorBack`/foreground fallback.
- `0x0005` font color.
- `0x0018` font name as a 2-byte-length `LPWideString`.
- `0x0019` bold weight (2 bytes).
- `0x001A` underline (2 bytes).
- `0x001B` script (2 bytes).
- `0x001C` italic, `0x001D` strike, `0x001E` outline, `0x001F` shadow,
  `0x0020` condense, `0x0021` extend (1 byte each).
- `0x0022` charset, `0x0023` family, `0x0025` font scheme (1 byte each).
- `0x0024` font size in twips as a 4-byte unsigned integer.

Border, number-format, alignment, wrap, protection, and rotation properties
are rejected with `Error::ValueNotRepresentable` because this slice does not
encode them. A format with no differences from `BrtFont::default()` and
`BrtFill::default()` produces an empty `XFProps`.

## Compatibility Boundary

- Public names and call shape match the reference cell-rule subset.
- `set_rule` accepts the same eight enum variants through a generic numeric or
  string value trait.
- `set_format` accepts `impl Into<Format>`; `set_stop_if_true` is additive.
- `add_conditional_format` takes `&ConditionalFormatCell` and returns
  `Result<&mut Worksheet>`.
- Existing worksheets, styles, and parts are byte-identical when no
  conditional format is added.
- The `add_conditional_format` reference is generic over `ConditionalFormat`;
  other conditional-format types remain absent.

## TDD Route

- Mode: `off` — no explicit user/project TDD authority.
- Decision: `skipped`.
- Test posture: exact-byte record tests, DXF/format mapping tests, worksheet
  validation tests, package decode tests, and a LibreOffice XLSX round-trip.
- Verification: focused tests first, then workspace and all-feature tests,
  clippy, fmt, `git diff --check`, and the external round-trip.

## Tasks

1. **Core DXF records**
   - Add `XFProp`, `XFProps`, and `BrtDXF` with deku derives and `Checked`.
   - Add `BrtColor::xf_prop_bytes`.
   - Add prelude exports, `assign_id!`, and `test_biff_export` cases.
   - Test exact `BrtDXF` bytes for a background-color property and an empty DXF.
   - Verify: core record tests.

2. **Core conditional records**
   - Add `CFParsedFormula` and `BrtBeginCFRule`; validate `iPri`, `iType`,
     `iTemplate`, `iParam`, formula presence, and between/not-between operand
     counts.
   - Add `BrtBeginConditionalFormatting`, zero-sized end marker, and worksheet
     scope markers.
   - Add exact-byte tests for a between rule with two `PtgNum` formulas and for a
     no-format equal rule.
   - Verify: core record tests.

3. **Style registry and Format mapping**
   - Add `StyleRegistry::register_dxf` with value deduplication and count.
   - Emit the DXF table instead of a fixed count of zero.
   - Add `Format::to_dxf` and a private `build_fill` shared with
     `to_cell_style`.
   - Test background/font color, bold, font name/size, empty DXF, and rejection
     of unsupported border/number-format/alignment properties.
   - Verify: core styles test plus writer format tests.

4. **Worksheet API**
   - Add `ConditionalFormatCell`, `ConditionalFormatCellRule<T>`,
     `ConditionalFormatValue`, and `IntoConditionalFormatValue`.
   - Add `Worksheet::add_conditional_format` with row/column, order, missing
     rule, and invalid rule-value checks.
   - Store one rule per range with an optional format and `stop_if_true`.
   - Verify: validation and storage tests; no rule is written without a range.

5. **Package and sheet emission**
   - Register DXF entries in sheet/rule order before writing styles.
   - Pass resolved `Option<u32>` DXF ids into the sheet writer.
   - Emit conditional-formatting blocks in the ABNF slot before data
     validation.
   - Verify: package decode asserts the rule, formula token, range, priority,
     DXF id, and styles DXF count; LibreOffice imports the rule and differential
     fill/font into an XLSX.
   - Regression: a workbook without conditional formats keeps the existing part
     bytes and zero-DXF styles table.

6. **Docs and closure**
   - Mark cell conditional formatting partial/supported in the gap matrix.
   - Update AGENTS.md test counts and architecture notes.
   - Run default/all-feature tests, clippy, fmt, and `git diff --check`.
   - Run the reviewer, fix valid findings, and commit via closing-sessions.

## Verification Matrix

| Seam | Evidence |
|---|---|
| DXF bytes | Exact-byte test for background property and empty DXF |
| CF rule bytes | Exact-byte test for two `PtgNum` formulas and null string |
| Style registry | DXF dedup, count, and styles-part decode |
| Format mapping | Font/fill properties, empty DXF, unsupported-property rejection |
| Worksheet API | Missing rule, bad range, bad value, stop flag, storage |
| Package | Decode sheet and styles parts; assert range, priority, formula, DXF id |
| External | LibreOffice XLSB→XLSX preserves conditional formatting and dxf fill/font |
| Regression | Full workspace tests, all-feature tests, clippy, fmt |

## Risks And Limits

- XLSB conditional formatting is sparsely documented outside MS-XLSB; the spec
  sample and the LibreOffice round-trip are the primary oracles.
- Calamine does not expose conditional formatting, so package decode remains the
  structural oracle.
- DXF carries font and fill properties only. Border and number-format
  conditional formats are rejected rather than silently degraded.
- String rule values use `PtgStr`; dates/times remain unsupported in this slice.
- `BrtBeginCFRule14`, extension GUIDs, and multi-range input are not emitted.

## Retirement

No existing owner is replaced. Plain styles continue through `BrtXF`; DXFs are a
separate table used only by conditional rules. Workbooks without conditional
formats keep the zero-DXF table and unchanged worksheet bytes.

## Evidence

- Default workspace: 527 unit tests and 7 doctests pass.
- All features: 531 unit tests and 7 doctests pass.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- LibreOffice XLSB→XLSX preserved greater-than, between, string-equal, and
  no-format rules; DXF fill, font color, bold, and font size also survived.
- Package tests decode the range, priority, operator, formula token, DXF id,
  styles DXF table, and record order.

## Review

`ConditionalFormatReview` found and the implementation fixed:

1. The non-14 sheet block was incorrectly wrapped in the CF14
   `BrtBeginConditionalFormattings` collection. The wrapper was removed.
2. `BrtBeginCFRule` packed flags relied on ambiguous deku bit order. Flags are
   now a raw `u16` with only `fStopTrue` (bit 1) allowed.
3. `0x0018` font name used a 4-byte length; `LPWideString` requires a 2-byte
   length.
4. `0x0024` font size wrote 2 bytes; the property requires a 4-byte unsigned
   twips value.

The review also identified API and coverage gaps; the implementation now
accepts `ConditionalFormatValue`, `&String`, and `Cow<'_, str>` values, uses the
reference `new_value` trait method, validates `CFParsedFormula` through
`Checked`, requires null `strParam`, and tests DXF deduplication and sheet record
order. Re-review confirmed the fixes and found no remaining blocking findings.
