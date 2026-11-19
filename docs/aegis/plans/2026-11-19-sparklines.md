# Sparklines

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-defined-names.md`

## Goal

Add the common sparkline path from the gap analysis: a single sparkline in one
cell and a grouped sparkline across a row or column.

Public API:

- `Sparkline`, `SparklineType`, `SparklineRange`, `ChartEmptyCells`.
- `Worksheet::add_sparkline(row, col, &sparkline)`.
- `Worksheet::add_sparkline_group(first_row, first_col, last_row, last_col, &sparkline)`.

The data range may be on the same or another worksheet. Style, markers,
high/low/first/last/negative points, axis, hidden data, empty-cell handling,
right-to-left, line weight, and custom/group axis bounds are supported.

## Aegis Visibility

This is the first feature that emits future records into the worksheet part.
It adds shared FRT structures and two record families, but no package part,
relationship, or content type. The worksheet writer already owns record order;
the package owns Xti allocation.

`XtiCollector` remains the only `BrtExternSheet` allocator. Sparkline area
formulas allocate their sheet indexes through it before the workbook part is
serialized. No second extern-sheet owner is added.

## Baseline / Authority

- MS-XLSB `2.4.228 BrtBeginSparklineGroup`: FRTHeader, 16 property bits, eight
  `BrtColor` values, manual max/min, line weight, `isltype`.
- MS-XLSB `2.4.806 BrtSparkline`: FRTHeader carrying one `FRTSqref` and one
  `FRTFormula`.
- MS-XLSB `2.5.61 FRTHeader`, `2.5.60 FRTFormulas`, `2.5.59 FRTFormula`,
  `2.5.98.9 FRTParsedFormula`, `2.5.66 FRTSqref`.
- `FRTParsedFormula` orders `cce`, `cb`, `rgce`, `rgcb`, unlike
  `CellParsedFormula` (`cce`, `rgce`, `cb`, `rgcb`).
- Sparkline `PtgArea3d` in the oracle uses opcode `0x5B` (VALUE type).
- `BrtFRTBegin`/`BrtFRTEnd` wrap the complete `BrtBeginSparklineGroups` block.
- Aspose.Cells 24.10 XLSB output is the byte oracle for record order, group
  size 98, and sparkline size 63.

## Compatibility Boundary

- Source-compatible subset: `set_range` accepts the same 5-tuple shape
  `("Sheet1", first_row, first_col, last_row, last_col)`, plus a same-sheet
  4-tuple extension.
- `SparklineType::WinLose` maps to `isltype = 2` (100% stacked column), matching
  the existing XLSX writer's `stacked` mapping.
- Unsupported reference setters (`set_style`, `set_date_range`,
  `set_column_order`) stay absent rather than becoming stubs.
- Unknown sheet names, invalid ranges, and a group whose data range does not
  match the target row/column are rejected before output.
- A grouped target must be one row or one column.

## Change Necessity

The records already have stable ids in `KnownID`; only payload layouts are
missing. Reusing the existing worksheet writer and `XtiCollector` keeps
sparkline state in one place and avoids a parallel package path.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: FRT record round-trips and exact sizes, aggregate/group range
  expansion, worksheet record order, package Xti reuse, and an Aspose read-back
  of a generated workbook.

## Tasks

1. **Core FRT structures.**
   - Add `FrtHeader`, `FrtSqref`, `FrtSqrefs`, `FrtFormula`, `FrtFormulas`,
     `FrtParsedFormula`.
   - Enforce the `fRef`/`fRelID` unsupported bits, reserved bits, counts, and
     formula lengths in `Checked`.
2. **Core sparkline records.**
   - Add `BrtBeginSparklineGroup` and `BrtSparkline`.
   - Register empty scope markers for `SparklineGroups` and `Sparklines`.
   - Add exact-size round-trip cases to the prelude table.
3. **Writer API.**
   - Add `SparklineRange`, `Sparkline`, `SparklineType`, `ChartEmptyCells`,
     and `Error::SparklineError`.
   - Add worksheet storage, range validation, and grouped cell expansion.
4. **Worksheet serialization.**
   - Emit one `BrtFRTBegin`/`BrtFRTEnd` block, then groups, then sparklines.
   - Encode each data range as one `PtgArea3d` value token.
5. **Package wiring.**
   - Resolve sparkline sheet names and allocate Xti indexes before the workbook
     part is written.
   - Reuse existing entries for repeated sheets and for names/tables.
6. **Verification.**
   - `cargo test --workspace`, `--all-features`, clippy, fmt.
   - Generate a workbook and read sparkline groups back with Aspose.Cells.

## Risks And Mitigations

- FRT record order: the oracle shows `BrtFRTBegin`, groups, `BrtFRTEnd` at the
  end of the worksheet part.
- Empty groups markers serialize to zero bytes, matching the oracle.
- Group flags are exactly 16 bits; the oracle's default `0x1800` confirms the
  individual-auto-max/min bits.
- Aspose may normalize colors; tests assert dimensions and order rather than
  its exact tint choices.

## Review

`SparklineReview` confirmed the record layouts, FRT wrapping, grouped
expansion, and Xti wiring. Findings fixed:

1. Sheet lookup and `Xti` allocation were hoisted from every cell to once per
   group.
2. `Sparkline::set_line_weight` now clamps finite values to `0..=1584` and
   ignores non-finite values, matching the record invariant.
3. Group expansion no longer clones the whole `SparklineRange` per cell;
   `sheet` is shared as `Arc<str>` and the remaining fields are listed.
4. The save path reuses one `BrtSparkline` record template per group, mutating
   the sqref and 15-byte formula in place.
5. Oracle byte assertions now cover the group flag word (`0x180A` for gaps +
   markers + individual auto bounds) and the `FrtSqref` `fDoAdjust` bit.
6. Package tests cover cross-sheet ranges, grouped rows, per-cell formula
   indexes, repeated Xti entries, and an unknown-sheet save error.

## Verification

- `cargo test --workspace`: 302 core + 409 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 302 + 413 unit tests, 7 doctests.
- `cargo clippy --workspace --all-targets`: clean.
- `cargo fmt --all --check`: clean.
- Generated workbook with three single sparklines and a three-cell group reads
  back through Aspose.Cells with all six cell locations and data ranges.
