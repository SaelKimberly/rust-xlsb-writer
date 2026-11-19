# Autofilter matching for temporal and boolean cells

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `FilterCondition::matches` for `CellData::Date`, `Time`, `DateTime`,
  `ExcelDateTime`, `Duration`, and `Bool`.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.4.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `filter.rs` and
  `Worksheet::row_matches_*`; both leave temporal and boolean cells
  unmatched.
- Spec: Microsoft Excel stores dates and times as 1900-system serial numbers;
  Excel's boolean column list shows `TRUE`/`FALSE`.
- Baseline: `05b8df7 fix: correct multi-range formula bases`.

## Goal

Autofilter conditions must evaluate every cell type the writer can store:

- numeric list and custom filters match date, time, datetime, `ExcelDateTime`,
  and duration cells through their Excel serial numbers;
- string list and custom filters match boolean cells through `TRUE`/`FALSE`;
- numeric custom filters also compare booleans as `1`/`0`;
- blanks, strings, numbers, rich strings, and the existing non-blank custom
  filter keep their current behaviour.

Example:

```rust
worksheet.write_datetime(1, 0, &date_2024_01_01, &format)?;
worksheet.autofilter(0, 0, 2, 0)?;
worksheet.filter_column(
    0,
    &FilterCondition::new().add_custom_filter(
        FilterCriteria::GreaterThanOrEqualTo,
        45292_i32,
    ),
)?;
```

## Aegis Visibility

The change is behavioural, not bytes. The autofilter records already carry the
conditions; only automatic row hiding is wrong. A durable plan is useful
because the matcher is shared by every worksheet and because temporal
conversion must reuse the core serial contract instead of growing a second
date implementation.

## Requirement Ready Check

- Approved source: gap analysis §3.4 states temporal and boolean cells stay
  unmatched.
- Scenarios: numeric list, numeric custom, string list, string custom, AND/OR
  pairs, non-blank custom filter, each temporal variant, both booleans, and
  invalid temporal values.
- Acceptance: `FilterCondition::matches` returns the Excel result for each
  scenario, and `apply_filters` hides the expected rows without changing
  filter records.
- No user decision is required; the gap names the missing behaviour.

## Change Necessity

The code change is confined to `rust-xlsb-writer/src/filter.rs`. Reusing
`ToCellValue` for temporal values avoids copying the core 1900-system epoch,
leap-day, millisecond-rounding, and duration rules. A local conversion would
drift from `BrtCellReal`/`BrtCellRk` output.

## Ripple Signal Triage

- Canonical owner: `FilterCondition` in `rust-xlsb-writer/src/filter.rs`.
- Downstream consumer: `Worksheet::apply_filters` in
  `rust-xlsb-writer/src/worksheet.rs`; it needs no signature change.
- No public type, method, record field, or package part changes.
- `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`, and test
  counts in `AGENTS.md` change.

## Architecture

Add one private helper in `filter.rs`:

```rust
fn numeric_value(cell: &CellData) -> Option<f64>
```

It returns stored integers/numbers directly and temporal values through
`ToCellValue` with a private `WbContext` whose `shared_strings` is
unreachable: temporal conversions never intern strings. `CellValue::Number`
is unwrapped from either `Real` or `Rk`.

`FilterCondition::matches_list`:

- string and rich-string handling stays;
- boolean cells match list strings `true`/`false`;
- numeric list entries match `numeric_value` exactly;
- blank handling stays.

`FilterCondition::matches_custom`:

- numeric filters compare `numeric_value`; booleans compare as `1`/`0`;
- string filters compare strings as today; booleans compare against
  `true`/`false`;
- every other cell type keeps the non-blank custom-filter rule.

No allocations are added for booleans or temporal values. Existing string
normalization remains the only allocating path.

## Compatibility Boundary

- Public `FilterCondition`, `FilterCriteria`, `FilterData`, and
  `IntoFilterData` are unchanged.
- Existing filter record serialization is unchanged.
- Strings, numbers, rich strings, blanks, and non-blank custom filters keep
  their current results.
- Invalid temporal values (outside Excel's date range or leap seconds) match
  no numeric filter.

## TDD Route

- Mode: `auto`.
- Decision: strict for the matcher behaviour.
- Authority: bugfix in shared row-hiding logic with regression signals.
- Test posture: add failing unit cases for each temporal variant and both
  booleans, then implement, then run the workspace suite.

## Tasks

1. **Regression tests**
   - Files: `rust-xlsb-writer/src/filter.rs`.
   - Add cases for date, time, datetime, `ExcelDateTime`, duration, boolean
     list/custom, boolean numeric custom, invalid temporal values, and
     unchanged string/number/blank behaviour.
   - Verify: the new cases fail before the helper is added.

2. **Temporal and boolean matching**
   - Files: `rust-xlsb-writer/src/filter.rs`.
   - Add `numeric_value`, the private numeric context, boolean list matching,
     and boolean/numeric custom matching.
   - Keep `matches_list` and `FilterData::matches` allocation-free for the new
     branches.
   - Verify: `cargo test -p rust-xlsb-writer filter`.

3. **Documentation and workspace verification**
   - Update the gap table row, this plan status, `docs/aegis/INDEX.md`, and
     test counts in `AGENTS.md`.
   - Verify: workspace tests with and without all features, clippy, fmt, and
     `git diff --check`.

## Verification

- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo fmt --all -- --check`
- `git diff --check`

## Risks

- Excel may coerce booleans differently in unusual custom comparisons. The
  plan follows the visible list values (`TRUE`/`FALSE`) and numeric `1`/`0`
  coercion, which covers ordinary autofilter use.
- Temporal filters compare stored serials exactly, as Excel does. Callers must
  use the serial value, not a formatted date string.
- The private `WbContext::shared_strings` path is unreachable for the temporal
  implementations used here; keep the conversion helper private to avoid
  exposing a context that panics for strings.

## Review

`AutofilterTemporalBoolReview` found no blocking defect. It confirmed:

- temporal serials use the same core conversion path as cell writing;
- `RkNumber::value()` is lossless and integer filter inputs are exact in
  `f64`;
- `NumericFilterContext::shared_strings` cannot be reached for the four
  temporal types, and string cells never enter the helper;
- boolean list matching correctly requires a string filter, so a numeric list
  value `1` does not match `TRUE`;
- existing string, number, blank, and non-blank custom-filter branches are
  unchanged.

The reviewer also ran the full workspace suite, clippy with all features, and
the format check.

## Retirement

No old matcher branch is retained. The `_ => false` list and
`_ => is_non_blank_filter` custom fallbacks remain only for cell types with no
defined filter semantics.
