# Conditional formatting: text, blank, and error rules

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormatText`, `ConditionalFormatTextRule`,
  `ConditionalFormatBlank`, `ConditionalFormatError`, and the existing
  `BrtBeginCFRule`/`CFParsedFormula` records.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`
  (`ConditionalFormatText`, `ConditionalFormatBlank`,
  `ConditionalFormatError`).
- Spec: MS-XLSB §2.4.23, §2.5.16, §2.5.17, §2.5.18, §2.5.98.45,
  §2.5.98.46, §2.5.98.49, §2.5.98.72, §2.5.98.74.
- Baseline: `868180d feat: add icon set conditional formats`.

## Goal

Add the three standard expression-rule families that use fixed formulas:

```rust
let text = ConditionalFormatText::new()
    .set_rule(ConditionalFormatTextRule::Contains("rust".to_owned()))
    .set_format(Format::new().set_background_color(Color::Yellow));
worksheet.add_conditional_format(1, 0, 20, 0, &text)?;

let blanks = ConditionalFormatBlank::new()
    .set_format(Format::new().set_background_color(Color::Red));
worksheet.add_conditional_format(1, 1, 20, 1, &blanks)?;

let errors = ConditionalFormatError::new()
    .invert()
    .set_format(Format::new().set_font_color(Color::Green));
worksheet.add_conditional_format(1, 2, 20, 2, &errors)?;
```

Out of scope: date periods, arbitrary formula rules, multi-range input,
CF14/`BrtBeginCFRule14`, icon, data-bar, and color-scale changes.

## Aegis Visibility

This adds three public rule types and a new formula-token boundary. The CF
template/parameter mapping, text `strParam`, and exact formula bytes are durable
wire contracts that package tests must pin.

## Architecture

`rust-xlsb-core`:

- Add `CF_TEMPLATE_CONTAINS_TEXT=0x08`,
  `CF_TEMPLATE_CONTAINS_BLANKS=0x09`,
  `CF_TEMPLATE_CONTAINS_NO_BLANKS=0x0A`,
  `CF_TEMPLATE_CONTAINS_ERRORS=0x0B`, and
  `CF_TEMPLATE_CONTAINS_NO_ERRORS=0x0C`.
- Add `CF_TEXTOPER_CONTAINS=0`, `CF_TEXTOPER_NOT_CONTAINS=1`,
  `CF_TEXTOPER_BEGINS_WITH=2`, and `CF_TEXTOPER_ENDS_WITH=3`.
- Extend `BrtBeginCFRule::check`:
  - Text: `iType=CF_TYPE_EXPRESSION`, `iTemplate=CONTAINS_TEXT`,
    `iParam` in `0..=3`, non-null `strParam` of 1..=255 UTF-16 code units.
  - Blank/error: `iType=CF_TYPE_EXPRESSION`, template in `0x09..=0x0C`,
    `iParam=0`, null `strParam`.
  - Both require a non-empty `formula1` whose `cce` matches `cbFmla1`,
    reject `formula2`/`formula3`, and allow only the stop flag.
- Existing rule families must still reject non-null `strParam`.

`rust-xlsb-writer`:

- Add private formula-token helpers:
  - `PtgRefN` relative self-reference: `0x4C`, row offset 0, column
    `0xC000` (relative row and column).
  - `PtgStr`: `0x17`, UTF-16 length, UTF-16LE bytes.
  - `PtgInt`: `0x1E`, little-endian `u16`.
  - `PtgFunc`: `0x41`, function id `u16`.
  - `PtgFuncVar`: `0x42`, parameter count `u8`, function id `u16`.
- Text formulas:
  - `Contains`: `NOT(ISERROR(SEARCH(text,ref)))`
  - `DoesNotContain`: `ISERROR(SEARCH(text,ref))`
  - `BeginsWith`: `LEFT(ref,len)=text`
  - `EndsWith`: `RIGHT(ref,len)=text`
- Blank formulas: `LEN(TRIM(ref))=0` and `LEN(TRIM(ref))>0` when inverted.
- Error formulas: `ISERROR(ref)` and `NOT(ISERROR(ref))` when inverted.
- `ConditionalFormatText` stores an optional rule, optional format, and stop
  flag. `validate` rejects a missing or empty rule, or text over 255 UTF-16
  units. `begin_rule` sets `iType=2`, `iTemplate=8`, `iParam` to the text
  operator, `strParam` to the search text, and `formula1` to the generated
  token stream.
- `ConditionalFormatBlank` and `ConditionalFormatError` store `inverted`,
  format, and stop flag; `invert()` flips the template.
- Export the new types from `lib.rs`.

`package.rs` needs no production change: these rules use the normal DXF table
and `dxf_format()`.

## Wire Contract

All three families use `CF_TYPE_EXPRESSION` (`iType=2`), one
`CFParsedFormula` in `formula1`, no `formula2`/`formula3`, and `iParam` as
follows:

| Rule | `iTemplate` | `iParam` | `strParam` |
|---|---:|---:|---|
| text contains | `0x08` | 0 | search text |
| text does not contain | `0x08` | 1 | search text |
| text begins with | `0x08` | 2 | search text |
| text ends with | `0x08` | 3 | search text |
| blanks | `0x09` | 0 | null |
| non-blanks | `0x0A` | 0 | null |
| errors | `0x0B` | 0 | null |
| non-errors | `0x0C` | 0 | null |

`cbFmla1` is set to `formula1.cce`; `cbFmla2` and `cbFmla3` are zero.
`flags` contains only `F_STOP_TRUE` when enabled. `dxfId` is the registered
DXF id or `0xFFFFFFFF`.

## TDD Route

- Mode: `auto`; decision: `light`.
- Authority: user request is implementation plus reviewer; no explicit strict
  TDD.
- Posture: core validation tests, writer exact-token tests, and a package
  decode test pin the wire layout. The formula builder is small and bounded.
- Verification: `cargo test --workspace`, all-features tests, clippy, fmt,
  `git diff --check`.

## Tasks

1. Core constants and validation.
   - Add template/text-operator constants.
   - Move the non-null `strParam` rejection into rule-specific checks.
   - Add text and formula-rule guards.
   - Export constants through `prelude`.
2. Core tests.
   - Accept each template with a formula, text parameters, and null/non-null
     string parameters as required.
   - Reject missing formulas, second formulas, wrong parameters, empty or
     oversized text, and null text.
3. Writer types and formulas.
   - Add formula-token helpers and text/blank/error formula builders.
   - Add `ConditionalFormatTextRule`, `ConditionalFormatText`,
     `ConditionalFormatBlank`, and `ConditionalFormatError`.
   - Implement the `ConditionalFormat` trait and exports.
4. Writer tests.
   - Pin exact token streams for all four text rules, both blank rules, and
     both error rules.
   - Pin UTF-16 text lengths, text `strParam`, stop flags, DXF ids, and
     validation boundaries.
5. Package test.
   - Add text, blank, and error rules with DXFs; decode the rules and assert
     templates, parameters, formulas, DXFs, and no extra CF wrapper.
6. Documentation and review.
   - Update `AGENTS.md`, gap analysis, plan status, and index.
   - Run the reviewer, fix findings, then commit.

## Verification

- `cargo test --workspace`: 580 tests pass (573 unit + 7 doctests; 269 core,
  304 writer).
- `cargo test --workspace --all-features`: 584 tests pass (577 unit + 7
  doctests).
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- Package test proves text/blank/error templates, `iParam`, `strParam`, exact
  error formula bytes, DXFs, priorities, and the absence of CF14/wrapper
  records.
- A throwaway pyxlsb parse recognized every generated token:
  `RefNPtg(row=0, col=0, row_rel=True, col_rel=True)`, `StringPtg`,
  `IntegerPtg`, and the expected `FuncPtg`/`FuncVarPtg` indices.

## Review

`TextBlankErrorReview` found no blocking defects. It independently checked the
template and text-operator values, `PtgRefN`/`PtgStr`/`PtgInt`/`PtgFunc`
layouts, function ids, RPN order, DXF handling, priorities, and record order
against MS-XLSB and the vendored reference.

Four coverage findings and two non-blocking observations were fixed:

1. The non-ASCII test now pins the full `PtgStr` UTF-16 stream for `😀`.
2. A text rule without a format pins `dxfId = 0xFFFFFFFF`.
3. The package test pins the exact 17-byte blank formula stream.
4. Core tests reject mixed template/type combinations, extra flags, and
   missing formulas.
5. `value_formula` now shares `push_formula_string` instead of duplicating the
   `PtgStr` encoder.
6. `push_formula_string` rejects text over 255 UTF-16 units, and text
   `begin_rule` calls `validate` so an empty rule cannot bypass the bound.

## Risks

- The formula uses `PtgRefN` with zero relative offsets, assuming Excel treats
  the reference as the current cell in CF formulas. No local Excel-generated
  XLSB file contains text/blank/error rules; package decoding and pyxlsb
  token parsing are the available oracles.
- MS-XLSB requires `strParam` for text rules and null otherwise; the core check
  enforces both so malformed records cannot be written silently.
- Excel's `cbFmla1` is an ignored nonzero marker, but this writer uses `cce` to
  stay consistent with its existing cell-rule records.
