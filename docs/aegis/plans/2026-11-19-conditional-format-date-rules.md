# Conditional formatting: date-occurring rules

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Worksheet::add_conditional_format`, the `ConditionalFormat` trait,
  `ConditionalFormatDate`, `ConditionalFormatDateRule`, the date CFTemp
  constants, and `BrtBeginCFRule` date validation.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4 and §5.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `conditional_format.rs`
  (`ConditionalFormatDate`, `ConditionalFormatDateRule`).
- Spec: MS-XLSB §2.4.23, §2.5.12, §2.5.16, §2.5.98.6, §2.5.98.45.
- Baseline: `ee4b811 fix: treat conditional format formula sizes as markers`.

## Goal

Add Excel's "Dates Occurring" conditional-format family:

```rust
let today = ConditionalFormatDate::new()
    .set_rule(ConditionalFormatDateRule::Today)
    .set_format(Format::new().set_background_color(Color::Yellow));
worksheet.add_conditional_format(1, 0, 20, 0, &today)?;

let last_week = ConditionalFormatDate::new()
    .set_rule(ConditionalFormatDateRule::LastWeek)
    .set_stop_if_true(true);
worksheet.add_conditional_format(1, 1, 20, 1, &last_week)?;
```

All ten reference variants are in scope: `Yesterday`, `Today`, `Tomorrow`,
`Last7Days`, `LastWeek`, `ThisWeek`, `NextWeek`, `LastMonth`, `ThisMonth`,
and `NextMonth`. The default is `Yesterday`, matching the reference.

Out of scope: arbitrary formula conditional formats, x14/CF14 rules,
multi-range input, and date helper methods on `Worksheet`.

## Aegis Visibility

This adds a public rule type and ten template/parameter pairs. The mapping is a
durable wire contract: an incorrect `iTemplate`/`iParam` pair changes the Excel
dialog selection even if the formula bytes are valid. Core validation and
package tests must pin both.

## Architecture

`rust-xlsb-core` (`prelude/record/brt_conditional.rs`):

- Add `CF_TEMPLATE_TIME_PERIOD_*` constants for `0x0F..=0x18`.
- Extend `BrtBeginCFRule::check` so those templates use `check_date_rule`.
- `check_date_rule`:
  - requires the exact CFDateOper in `iParam` for its template,
  - requires a null `strParam`,
  - requires one non-empty formula with a nonzero `cbFmla1`,
  - rejects `formula2`/`formula3` and every flag except `fStopTrue`.
- Re-export the constants through `prelude/mod.rs`.

`rust-xlsb-writer`:

- Add `ConditionalFormatDateRule` and `ConditionalFormatDate`.
- Add token helpers for the fixed date formulas:
  - `PtgAdd=0x03`, `PtgSub=0x04`, `PtgLT=0x09`, `PtgLE=0x0A`,
    `PtgEQ=0x0B`, `PtgGE=0x0C`, `PtgGT=0x0D`.
  - Fixed functions: `AND=36`, `OR=37`, `MONTH=68`, `YEAR=69`,
    `WEEKDAY=70`, `ROUNDDOWN=213`, `TODAY=221`, `FLOOR=285`.
  - `PtgFunc` for fixed-arity functions; `PtgFuncVar(2, AND|OR)` for the
    two-condition wrappers.
- Emit the reference formulas in RPN against a relative self-reference
  (`PtgRefN`, zero offsets), matching the existing text/blank/error rules.
- `begin_rule` returns `CF_TYPE_EXPRESSION`, the date template, the matching
  CFDateOper, `dxfId`, `iPri`, only the stop flag, `cbFmla1=1`, a null
  `strParam`, and one formula.
- Export the two new public types from `lib.rs`.

`package.rs` and `sheet.rs` need no production change: date rules use the
normal DXF table and one `BrtBeginConditionalFormatting` block.

## Wire Contract

| Rule | `iTemplate` | `iParam` | Formula |
|---|---:|---:|---|
| Today | `0x0F` | 0 | `FLOOR(ref,1)=TODAY()` |
| Tomorrow | `0x10` | 6 | `FLOOR(ref,1)=TODAY()+1` |
| Yesterday | `0x11` | 1 | `FLOOR(ref,1)=TODAY()-1` |
| Last7Days | `0x12` | 2 | `AND(TODAY()-FLOOR(ref,1)<=6,FLOOR(ref,1)<=TODAY())` |
| LastMonth | `0x13` | 5 | `AND(MONTH(ref)=MONTH(TODAY())-1,OR(YEAR(ref)=YEAR(TODAY()),AND(MONTH(ref)=1,YEAR(ref)=YEAR(TODAY())-1)))` |
| NextMonth | `0x14` | 8 | `AND(MONTH(ref)=MONTH(TODAY())+1,OR(YEAR(ref)=YEAR(TODAY()),AND(MONTH(ref)=12,YEAR(ref)=YEAR(TODAY())+1)))` |
| ThisWeek | `0x15` | 3 | `AND(TODAY()-ROUNDDOWN(ref,0)<=WEEKDAY(TODAY())-1,ROUNDDOWN(ref,0)-TODAY()<=7-WEEKDAY(TODAY()))` |
| NextWeek | `0x16` | 7 | `AND(ROUNDDOWN(ref,0)-TODAY()>(7-WEEKDAY(TODAY())),ROUNDDOWN(ref,0)-TODAY()<(15-WEEKDAY(TODAY())))` |
| LastWeek | `0x17` | 4 | `AND(TODAY()-ROUNDDOWN(ref,0)>=WEEKDAY(TODAY()),TODAY()-ROUNDDOWN(ref,0)<WEEKDAY(TODAY())+7)` |
| ThisMonth | `0x18` | 9 | `AND(MONTH(ref)=MONTH(TODAY()),YEAR(ref)=YEAR(TODAY()))` |

`ref` is `PtgRefN` with zero row/column offsets and relative row/column flags.
`cbFmla1` is `1`; `cbFmla2` and `cbFmla3` are zero. `flags` is `0` or
`F_STOP_TRUE`. `dxfId` is the registered DXF id or `0xFFFFFFFF`.

## TDD Route

- Mode: `auto`; decision: `light`.
- Authority: user request is implementation plus reviewer; no explicit strict
  TDD.
- Posture: core validation tests, writer exact-token tests, and a package
  decode test pin the template/parameter pairs and formula streams.
- Verification: `cargo test --workspace`, all-features tests, clippy, fmt,
  `git diff --check`.

## Tasks

1. Core constants and validation.
   - Add the ten date CFTemp constants.
   - Add exact template-to-CFDateOper validation.
   - Export constants through `prelude`.
2. Core tests.
   - Accept all ten pairs with one formula and a null `strParam`.
   - Reject mismatched template/parameter pairs, missing formulas, second
     formulas, non-null `strParam`, and non-stop flags.
3. Writer types and formulas.
   - Add the enum, struct, setters, `Default`, and `ConditionalFormat` impl.
   - Add operator/function token helpers and the ten RPN streams.
   - Export the new types.
4. Writer tests.
   - Pin exact formula bytes for all ten rules.
   - Pin template, parameter, stop flag, DXF id, marker, and null `strParam`.
5. Package test.
   - Add formatted and unformatted date rules; decode the worksheet and assert
     templates, parameters, formulas, DXF registration, priorities, and the
     absence of CF14/wrapper records.
6. Documentation and review.
   - Update `AGENTS.md`, gap analysis, plan status, and index.
   - Run the reviewer, fix findings, then commit.

## Verification

- `cargo test --workspace`: 585 tests pass (578 unit + 7 doctests; 271 core,
  307 writer).
- `cargo test --workspace --all-features`: 589 tests pass (582 unit + 7
  doctests).
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- Writer tests pin all ten formula streams, including the variable-arity
  `WEEKDAY` token, template/parameter pairs, stop flag, null `strParam`,
  `cbFmla1=1`, DXF ids, and `formula2` absence.
- Package tests decode today and last-month rules with one DXF, assert
  priorities and exact today bytes, and reject CF14/wrapper records.
- A generated `/tmp/date_cf.xlsb` parsed successfully with pyxlsb's token
  reader: `RefNPtg(row=0, col=0, row_rel=True, col_rel=True)`,
  `IntegerPtg`, `FuncPtg` for TODAY/FLOOR/ROUNDDOWN/MONTH/YEAR,
  `FuncVarPtg(argc=1)` for WEEKDAY, and `FuncVarPtg(argc=2)` for AND/OR.

## Review

`DateCFReview` found one wire-format defect and two coverage gaps. All were
fixed:

1. `WEEKDAY` is variable-arity in MS-XLSB (`weekday-params = val [val]`), so
   it must use `PtgFuncVar` with one parameter, not `PtgFunc`. LastWeek,
   ThisWeek, and NextWeek now emit `0x42 01 46 00`.
2. The package test now rejects `BrtBeginCFRule14` as well as the CF wrapper.
3. Core date tests now reject a missing `formula1` and a zero `cbFmla1`.

The reviewer independently confirmed the ten CFTemp values, CFDateOper pairs,
operator bytes, other function ids, operand order, DXF/no-DXF paths,
priorities, and `cbFmla1=1`. Non-blocking observations were the plan's stale
verification text, extra `PartialEq`/`Eq`/`Hash` derives, and the additive
`set_stop_if_true` method; all are intentional or corrected here.

## Risks

- The formulas depend on Excel evaluating `PtgRefN` relative to each cell in
  the conditional-format range, the same assumption already used by the text
  and blank/error rules.
- LibreOffice drops date conditional formats on XLSB import, like the other CF
  families. Package decode plus exact token tests remain the oracle.
- No Excel-generated XLSB with date rules was available, so the reference
  formula strings and MS-XLSB token grammar are the only external checks.
