# Formula-driven validation and conditional formatting

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: formula-valued data-validation rules, custom data validation, and
  formula conditional formatting.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4, §5, and P1.
- Reference: vendored `rust_xlsxwriter` v0.102.0
  `DataValidation::{allow_*_formula, allow_list_formula, allow_custom}` and
  `ConditionalFormatFormula`.
- Spec: MS-XLSB §2.4.23 (`BrtBeginCFRule`), §2.4.356 (`BrtDVal`),
  §2.5.98.12 (`NameParsedFormula`), and §2.5.98.4 (`CellParsedFormula`).
- Baseline: `2b7535a feat: add custom theme support`.

## Goal

Close the formula-backed analysis-rule gaps with the reference API:

```rust
let mut workbook = Workbook::new();
let worksheet = workbook.add_worksheet();

let limit = DataValidation::new()
    .allow_whole_number_formula(DataValidationRule::LessThanOrEqualTo(Formula::new("=D1")));
worksheet.add_data_validation(1, 0, 10, 0, &limit)?;

let list = DataValidation::new().allow_list_formula(Formula::new("=F1:F9"));
worksheet.add_data_validation(1, 1, 10, 1, &list)?;

let custom = DataValidation::new()
    .allow_custom(Formula::new("=AND(ISTEXT(D2),EXACT(D2,UPPER(D2)))"));
worksheet.add_data_validation(1, 2, 10, 2, &custom)?;

let format = Format::new().set_background_color(Color::Yellow);
let rule = ConditionalFormatFormula::new()
    .set_rule("=ISODD(B3)")
    .set_format(format);
worksheet.add_conditional_format(2, 1, 11, 1, &rule)?;
```

The formulas use the existing formula tokenizer. Unsupported syntax keeps the
current boundary: cross-sheet references, defined names, dynamic-array
functions, and array constants return `Error::FormulaError` when the rule is
serialized.

## Aegis Visibility

`BrtDVal.formula1`/`formula2` and `BrtBeginCFRule.formula1` are durable wire
fields. A literal number written where Excel expects a formula, or a formula
without the rule marker, produces a workbook that imports with the wrong rule
or repairs. The public methods also extend the source-compatible API, so the
method names, argument types, and defaults are pinned.

## Requirement Ready Check

- Approved source: gap analysis §3.4 and the reference API.
- Scenarios: formula comparison bounds, formula list source, custom formula,
  formula conditional format, malformed formula, unsupported cross-sheet
  reference, formatted and unformatted rules.
- Acceptance: generated records decode with the expected validation type,
  operator, formula tokens, DXF id, priority, and stop flag; LibreOffice
  imports and preserves the rules on XLSX export where it exposes them.
- No user decision is required; the reference defines the API and defaults.

## Change Necessity

Literal rules currently store their values as strings and encode a single
`PtgNum`/`PtgStr`. Formula rules need the same comparison model but must keep
the incoming `Formula` until serialization, then compile it to
`NameParsedFormula` or `CFParsedFormula`. Reusing the literal path would parse
`=D1` as a number and fail, so the operand must carry its source kind.

The formula conditional format is absent entirely. It belongs in
`conditional_format.rs`, alongside the other `CF_TYPE_EXPRESSION` rules, and
reuses the same tokenizer as cell formulas.

## Ripple Signal Triage

- Canonical formula tokenizer remains `rust-xlsb-writer/src/formula.rs`.
- Canonical validation owner remains `rust-xlsb-writer/src/data_validation.rs`.
- Canonical conditional-format owner remains
  `rust-xlsb-writer/src/conditional_format.rs`.
- `Formula` gains one crate-private accessor; no second formula owner.
- `rust-xlsb-core` records are unchanged; no new record types are needed.
- `docs/aegis/INDEX.md`, the gap analysis, and `AGENTS.md` counts change.

## Architecture

`rust-xlsb-writer/src/formula.rs`:

- Add `pub(crate) fn as_str(&self) -> &str` so rule builders can compile the
  stored formula without cloning it.
- Add `compile_relative(formula, base_row, base_col)`, which emits
  `PtgRefN`/`PtgAreaN` for relative references and keeps `PtgRef`/`PtgArea`
  for fully absolute references. Relative row and column fields are offsets
  from the rule range's top-left cell, per MS-XLSB `RgceLocRel`/`RgceAreaRel`.
- Add `ISODD` (`0x01A5`) and `ISEVEN` (`0x01A4`) to the function table; the
  reference conditional-format examples use them.

`rust-xlsb-writer/src/data_validation.rs`:

- Replace the `String` operands inside `DataValidationRuleInternal` with
  `DataValidationOperand::{Literal(String), Formula(Formula)}`.
- `DataValidationRule<T>::into_internal_rule` wraps literal values.
- Add `DataValidationRule<Formula>::into_formula_rule`, mapping every
  comparison variant to formula operands.
- Add `DataValidationType::Custom` and a `CustomFormula(Formula)` rule.
- Add public methods:
  - `allow_whole_number_formula(DataValidationRule<Formula>)`
  - `allow_decimal_number_formula(DataValidationRule<Formula>)`
  - `allow_date_formula(DataValidationRule<Formula>)`
  - `allow_time_formula(DataValidationRule<Formula>)`
  - `allow_text_length_formula(DataValidationRule<Formula>)`
  - `allow_list_formula(Formula)`
  - `allow_custom(Formula)`
- Extend `formulas()` to emit validation type 7 for custom rules and to
  compile formula operands. Literal operands keep `number_formula` and
  `string_formula`.

`rust-xlsb-writer/src/conditional_format.rs`:

- Add `ConditionalFormatFormula` with `new`, `set_rule(impl Into<Formula>)`,
  `set_format`, and `set_stop_if_true`.
- `validate` requires a non-empty formula.
- `begin_rule_in_range` compiles the formula into `CFParsedFormula` with the
  target range's top-left offset and uses the existing
  `formula_rule(CF_TEMPLATE_FMLA, 0, ...)` helper. `CF_TEMPLATE_FMLA` is 1;
  `CF_TEMPLATE_EXPR` (0) remains the cell-value template.
- Re-export the type from `lib.rs`.

`rust-xlsb-writer/src/worksheet.rs` and `package.rs` need no new paths; both
rule stores already route through `record()`/`begin_rule()`.

## Wire Contract

| Rule | Record fields |
|---|---|
| Numeric/date/time/text-length formula | `BrtDVal.val_type` 1/2/4/5/6, comparison operator, compiled `formula1`/`formula2` |
| List formula | `BrtDVal.val_type` 3, operator 0, compiled `formula1` |
| Custom formula | `BrtDVal.val_type` 7, operator 0, compiled `formula1` |
| Formula conditional format | `BrtBeginCFRule.iType` expression, `iTemplate` 1 (`CF_TEMPLATE_FMLA`), `iParam` 0, `cbFmla1` 1, compiled `formula1`, caller `dxfId`/priority/stop |

Formula tokens are little-endian MS-XLSB `rgce`; no string wrapping is added.

## Compatibility Boundary

- Existing literal validation and conditional-format bytes are unchanged.
- `DataValidation::new()` remains an ignored “Any” rule.
- Formula rules use the same defaults for blank handling, dropdowns, prompts,
  errors, and stop-if-true as their literal counterparts.
- `Formula::new` continues to accept optional `=` and array braces.
- Cross-sheet and named formulas remain rejected; this plan does not widen the
  formula grammar.
- `ConditionalFormatFormula` is source-compatible with the reference for the
  supported formula subset.

## TDD Route

- Mode: `auto`.
- Decision: strict for public API and wire fields.
- Authority: new public methods plus durable formula bytes.
- Test posture: add rule-level and package-level tests first, observe the
  expected failure, then implement.

## Tasks

1. **Formula accessor and conditional formula rule**
   - Files: `src/formula.rs`, `src/conditional_format.rs`, `src/lib.rs`.
   - Add module tests for empty-formula rejection, expression template,
     priority/DXF/stop fields, and compiled tokens; add a package test that
     decodes `BrtBeginCFRule` from a saved workbook.
   - Verify: `cargo test -p rust-xlsb-writer conditional`.

2. **Formula validation operands and public methods**
   - Files: `src/data_validation.rs`.
   - Add operand kind, formula rule conversion, custom type, and the seven
     public methods.
   - Tests: every formula comparison variant, list formula, custom formula,
     empty/cross-sheet rejection, record `Checked`, and unchanged literal
     bytes.
   - Verify: `cargo test -p rust-xlsb-writer data_validation`.

3. **Package integration**
   - Files: `src/package.rs`.
   - Add a saved workbook with formula comparison, list, and custom
     validations plus a formula conditional format; decode `BrtDVal` and
     `BrtBeginCFRule` fields and assert record order.
   - Verify: `cargo test -p rust-xlsb-writer package`.

4. **Documentation and dependency pin**
   - Update `docs/gap-analysis-rust-xlsxwriter.md` §3.4 and §5, this plan,
     `docs/aegis/INDEX.md`, and the test counts in `AGENTS.md`.
   - Pin `zip` to the resolved minor (`9.7`) so a fresh lockfile cannot drift
     across the theme reader's tested behavior.
   - Verify: full workspace checks.

## Verification

- `cargo test --workspace`: 628 unit tests and 7 doctests pass (285 core,
  343 writer).
- `cargo test --workspace --all-features`: 632 unit tests and 7 doctests pass
  (285 core, 347 writer).
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- LibreOffice imports the generated XLSB and preserves the formula
  conditional format as `type="expression"` with formula `ISODD(B3)`.
- LibreOffice drops relative data-validation references on XLSB import,
  including ExcelGen's own `test-dataval.xlsb`. The writer's `PtgRefN`
  encoding matches ExcelGen's record: `E2:E15` with a formula one column left
  stores row offset 0 and column offset -1.
- Package decode tests assert validation types 1/3/7, operators, relative
  formula tokens, `CF_TEMPLATE_FMLA`, DXF/priority/stop fields, and record
  order.

## Review

`FormulaRulesReview` found no blocking correctness defect. It independently
verified against ExcelGen fixtures:

- expression conditional formats use `iType=2`, `iTemplate=1`, `iParam=0`,
  `cbFmla1=1`, and `PtgRefN` for relative references;
- `test-dataval.xlsb` stores a relative column as 14-bit two's complement;
- a generated `=ISODD(A1)` rule on `B2:B3` survived LibreOffice export with
  the expected formula, proving the 20-bit row offset decodes as `-1` rather
  than wrapping;
- focused formula, validation, conditional-format, package, and core tests
  passed.

One minor test weakness was fixed: the package test now asserts the full
`ISODD(B3)` token stream instead of only its first byte, so a broken
range-base handoff cannot pass.

## Risks

- LibreOffice drops relative data-validation references on XLSB import even
  for Excel-generated files; record-level decode plus the ExcelGen fixture is
  the authoritative proof.
- Relative references must use offsets from the rule range's top-left cell.
  Writing absolute coordinates as `PtgRefN` would shift validation formulas.
- Excel evaluates relative references in a conditional-format formula against
  each cell in the range. `compile_relative` emits `PtgRefN`/`PtgAreaN`
  offsets from the range top-left, matching ExcelGen's formula rules.
- The formula tokenizer is narrower than Excel's. A formula that Excel accepts
  but this parser rejects is reported as `FormulaError` rather than written
  silently.
- Validation formulas are compiled when the workbook is serialized, so a
  malformed formula is reported by `save*`, not by the builder method, matching
  the existing deferred-record path.
