# merge_range: MS-XLSB merged cells

- Status: Implemented — reviewer clean
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.5, §6 P0
- Target: `rust-xlsb-core`, `rust-xlsb-writer`
- Method pack: Aegis implementation plan

## 1. Goal

Add `Worksheet::merge_range(first_row, first_col, last_row, last_col, text,
format)` with source-compatible signatures and metadata behavior, backed by the
three MS-XLSB merge records:

- `BrtBeginMergeCells { cmcs }`
- `BrtMergeCell` (`rwFirst`, `rwLast`, `colFirst`, `colLast`)
- `BrtEndMergeCells`

The top-left cell holds the string and format; the rest of the rectangle is
formatted blanks. Overlapping and single-cell merges are rejected.

## 2. Requirement Ready Check

```text
Requirement Ready Check:
- Requirement source refs: docs/gap-analysis-rust-xlsxwriter.md §3.5, §6 P0; user request
- Goals and scope refs: this plan §1, §3
- User / scenario refs: rust_xlsxwriter merge_range callers
- Requirement item refs: merge records, writer API, overlap/single-cell validation, serial serializer
- Acceptance / verification criteria refs: this plan §7
- Open blocker questions: none
- Decision: ready
```

## 3. Scope

### In scope

- Core `BrtMergeCell` structure, `BrtBegin/EndMergeCells` markers, prelude
  exports, and round-trip test.
- `Worksheet::merge_range` with `&str` text and `&Format`.
- Overlap detection and single-cell rejection.
- Merge record emission after sheet data.
- `calamine` and LibreOffice checks.

### Out of scope

- Formulas, hyperlinks, rich strings.
- Merge-aware row/column deletion or clearing.
- New package parts.

## 4. Owners

| Owner | Change |
|---|---|
| `rust-xlsb-core/src/prelude/record/` | `BrtMergeCell` and merge markers |
| `rust-xlsb-core/src/prelude/collection.rs` | `MergeCells` begin/end markers |
| `rust-xlsb-core/src/prelude/mod.rs` | exports and round-trip case |
| `rust-xlsb-writer/src/worksheet.rs` | merged-range model and API |
| `rust-xlsb-writer/src/sheet.rs` | MERGECELLS emission |
| `rust-xlsb-writer/src/error.rs` | single-cell/overlap errors |

## 5. TDD Route

```text
TDD Route:
- Mode: auto
- Decision: strict
- Authority: Aegis writing-plans route guard; public API + binary records
- Test posture: failing record/API tests first
- Verification: cargo test --workspace, clippy, fmt, calamine, LibreOffice
```

## 6. Implementation Tasks

### Task 1 — Core merge records

- Add `BrtMergeCell { rfx: UncheckedRfX }` in `prelude/record/brt_merge_cell.rs`.
  `UncheckedRfX::check` already validates row/column order and bounds.
- Add `MergeCells` to `declare_simple_collections!` so
  `BrtBeginMergeCells { count }` and `BrtEndMergeCells` exist.
- Re-export all three from `prelude/mod.rs`; add a `test_biff_export` case.
- Add a hand-written test that a merge cell round-trips and preserves the range.

### Task 2 — Worksheet merge model and API

- Add `MergeRange` (first/last row/column) and `Worksheet.merged_ranges:
  Vec<MergeRange>`.
- Add `merge_range`:
  - validate bounds/order;
  - reject a single cell;
  - reject any overlap with an existing range (iterate ranges; no per-cell
    overlap map);
  - write the string with format at the top-left;
  - write formatted blanks over the rest of the inclusive rectangle;
  - store the range.
- Add `Error::MergeRangeSingleCell` and `Error::MergeRangeOverlaps`.
- Reuse one interned format index for all cells in the call; do not clone
  `Format` per cell.

### Task 3 — Serialization

- Emit `BrtBeginMergeCells { count }`, each `BrtMergeCell`, and
  `BrtEndMergeCells` after `BrtEndSheetData` and before `BrtEndSheet`.
- Do not emit the collection when there are no merged ranges.
- Counts use `u32`; ranges use `UncheckedRfX`.

### Task 4 — Tests

- Core record round-trip and merge-cell field layout.
- `merge_range` writes top-left text and formatted blanks.
- Single-cell and overlap errors.
- Worksheet record stream contains one merge collection and one merge cell.
- `calamine` reads the top-left value.
- LibreOffice renders the merged rectangle.

### Task 5 — Docs, review, closing commit

- Update gap analysis: mark merge as supported; roadmap P0 item.
- Update AGENTS test counts.
- Launch reviewer; fix findings.
- Use closing-sessions for memory/docs/commit.

## 7. Acceptance Criteria

- `merge_range(0, 0, 1, 2, "Merged", &format)` writes `Merged` at `(0,0)` and
  formatted blanks at the other five cells.
- The worksheet part contains exactly one `BrtBeginMergeCells`, one
  `BrtMergeCell` with `(0,0,1,2)`, and one `BrtEndMergeCells`.
- `calamine` reports `"Merged"` at `(0,0)` and empty at `(0,1)`.
- Single-cell merges fail with `MergeRangeSingleCell`.
- Overlapping merges fail with `MergeRangeOverlaps` before any new cells are
  written.
- No merge collection is emitted for a sheet without merges.
- Full workspace tests, clippy, and fmt pass.

## 8. Risks

- Record order in the worksheet part: emit after sheet data; validate with
  calamine and LibreOffice.
- Overlap checking is O(number of merged ranges), acceptable for typical
  sheets; no per-cell map allocation.
- Merge formatting must be applied to every cell, not just the top-left.
- No merge-aware clear/delete; document as out of scope.

## 9. Execution Handoff

- Status: complete.
- Verification: 123 core + 56 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine and LibreOffice render confirm the merged rectangle.
- Reviewer: NO FINDINGS; row/column order error text was generalized after
  review.
- Next: P0b formulas; hyperlinks and worksheet state remain P0c.

- Route: inline; core record + writer model + serializer.
- User confirmation required: no.
