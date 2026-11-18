# Worksheet views: freeze panes, selection, top-left cell

- Status: Implemented — reviewer finding fixed
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.3, §6 P0
- Target: `rust-xlsb-core`, `rust-xlsb-writer`

## 1. Goal

Add the P0c view-control subset:

- `Worksheet::set_freeze_panes(row, col)`
- `Worksheet::set_selection(first_row, first_col, last_row, last_col)`
- `Worksheet::set_top_left_cell(row, col)`

The worksheet part emits `WSVIEWS2` only when one of these is set. Sheets
without view state keep byte-identical output.

## 2. Spec Basis

Worksheet ABNF (`2.1.7.62`):

```text
WORKSHEET = BrtBeginSheet [BrtWsProp] [BrtWsDim] [WSVIEWS2] ...
WSVIEWS2  = BrtBeginWsViews 1*WSVIEW2 BrtEndWsViews
WSVIEW2   = [ACUID] BrtBeginWsView [BrtPane] *4[BrtSel] ... BrtEndWsView
```

Records:

| Record | Payload | Source |
|---|---:|---|
| `BrtBeginWsViews` / `BrtEndWsViews` | 0 / 0 | scope markers |
| `BrtBeginWsView` | 30 | `2.4.313` |
| `BrtPane` | 29 | `2.4.189` |
| `BrtSel` | 20 + 16 per range | `2.4.720` |

`BrtBeginWsView` fields: 11 flag bits + 5 reserved bits, `xlView`, `rwTop`,
`colLeft`, `icvHdr`, 3 reserved bytes, four zoom `u16`s, `iWbkView`.
Writer defaults: gridlines/headings/zeros/guts/default gridline color on,
Normal view, zoom 100, workbook view 0.

`BrtPane`: two `Xnum` splits, `rwTop`, `colLeft`, `pnnAct` (bottom-right for a
two-axis freeze), then `fFrozenNoSplit` for ordinary frozen panes.

`BrtSel`: `pnn`, active row/column, range index 0, one `UncheckedRfX`.
Use the top-left pane (`0x0000_0003`) for the writer's single selection.

## 3. Design

Core gets five record modules plus a public `UncheckedSqRfX` structure. Existing
`KnownID` variants (151, 152) and scope names are reused. No new id mapping.

`Worksheet` stores a compact `SheetView`:

```rust
struct SheetView {
    freeze: Option<(RowNum, ColNum)>,
    selection: Option<UncheckedRfX>,
    top_left: Option<(RowNum, ColNum)>,
}
```

`merge_range`-style validation runs before mutation:

- bounds: existing `check_row` / `check_col`;
- selection order: `Error::RowColumnOrder`;
- no partial state on error.

Serialization writes `WSVIEWS2` immediately after `BrtWsDim`, before column
infos and sheet data. `BrtPane` is omitted when no freeze is set. `BrtSel` is
omitted when no selection is set.

Allocations: one `Vec` with one `UncheckedRfX` per emitted `BrtSel`; no per-cell
or per-row allocation. Format paths are untouched.

## 4. TDD Route

- Mode: auto
- Decision: strict for record layout and public view state
- Authority: public API plus binary contract change
- Test posture: record round-trip/size tests, worksheet validation tests,
  serialized-record assertions, calamine readback, LibreOffice visual check.

## 5. Tasks

1. Core records:
   - Add `BrtBeginWsView`/`BrtEndWsView`, `BrtPane`, `BrtSel`,
     `BrtBeginWsViews`/`BrtEndWsViews`, `UncheckedSqRfX`.
   - Register ids and exports.
   - Add `checked_test` sizes and hand tests for pane flags and selection range.
2. Worksheet state/API:
   - Add `SheetView`, accessor, three setters, validation.
   - Unit tests for bounds/order and replacement semantics.
3. Serialization:
   - Emit `WSVIEWS2` after `BrtWsDim`.
   - Assert record ids, pane split values, selection range, top-left.
4. Package proof:
   - Calamine opens a sheet with freeze/selection.
   - LibreOffice renders a frozen header row.
5. Docs/closeout:
   - Update gap analysis, AGENTS test counts, plan status.
   - Reviewer, fixes, commit.

## 6. Verification

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

Plus `calamine` readback and a LibreOffice PDF/PNG render.

## 7. Out Of Scope

Default row height, tab color, sheet visibility, zoom controls, gridline toggles,
view modes, and `BrtWsProp` changes. No split panes or multiple selections.

## 8. Execution Handoff

- Status: complete.
- Verification: 132 core + 62 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine reads view-bearing sheets.
- Reviewer: one P2 finding; `BrtSel.pnn` now follows the active pane computed
  from the freeze boundary and active cell, and `BrtPane.pnnAct` uses the same
  value. Covering tests added.
- Renderer limit: LibreOffice opens the fixture but does not retain XLSB pane
  or selection records on XLSX conversion, so view state is proven at record
  level rather than by a rendered freeze.
- Next: P0b formulas; hyperlinks, default row height, sheet visibility, tab
  color, and freeze top-cell control remain.

- Route: inline; core records + writer model + serializer.
- User confirmation required: no.
