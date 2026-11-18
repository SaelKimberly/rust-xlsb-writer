# XLSB writer API implementation plan

- Status: Ready for execution
- Date: 2026-11-18
- Design spec: `docs/aegis/specs/2026-11-18-xlsxwriter-compatible-api-design.md`
- Superseded-in-part spec: `docs/aegis/specs/2026-11-18-cell-values-and-workbook-context-design.md`
- Execution context: current branch `deku-biff`; reuse the existing workspace
- Public API owner: `rust-xlsb-writer`
- Compatibility: source-compatible subset of `rust_xlsxwriter`, plus additive date/time helpers

Aegis Visibility: this plan is durable because it changes public API ownership,
the XLSB package writer, and the retirement boundary of core `Workbook` /
`Worksheet`.

## 1. Plan Basis And Requirement Readiness

### 1.1 Requirement Ready Check

```text
Requirement Ready Check:
- Requirement source refs: docs/aegis/specs/2026-11-18-xlsxwriter-compatible-api-design.md (§1–§8, §11); user instruction "Proceed with plan"
- Goals and scope refs: design spec §2, §5.2–§5.7
- User / scenario refs: design spec §1, §6
- Requirement item refs: design spec §5.2–§5.8, §8.1–§8.5
- Acceptance / verification criteria refs: design spec §8; plan §7
- Open blocker questions: none
- Decision: ready
```

### 1.2 Baseline And Authority Refs

- `AGENTS.md`: workspace rules, Rust 1.100.0 pin, record conventions, test
  layout, no stubs.
- `docs/aegis/specs/2026-11-18-xlsxwriter-compatible-api-design.md`: approved
  API shape and compatibility boundary.
- `docs/aegis/specs/2026-11-18-cell-values-and-workbook-context-design.md`:
  low-level `CellValue` / `WbContext` / temporal contract; §3, §5.7, and §11
  are superseded in part.
- `thirdparty/rust_xlsxwriter` v0.102.0 (`d850973`): reference signatures and
  behavior for `Workbook`, `Worksheet`, `Format`, `Color`, and `IntoExcelData`.
- Current dirty baseline: Calibri 11 default-font change and the new design
  spec are uncommitted. Do not revert them. Implementation commits must not
  absorb unrelated user edits.

### 1.3 User-Visible Outcome

A caller can write:

```rust
use rust_xlsb_writer::{Format, Workbook};

fn main() -> Result<(), rust_xlsb_writer::Error> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_column_width(0, 22)?;
    worksheet.write(0, 0, "Hello")?;

    let bold = Format::new().set_bold();
    worksheet.write_with_format(1, 0, 42, &bold)?;

    workbook.save("out.xlsb")?;
    Ok(())
}
```

The file opens in `calamine` and LibreOffice, contains the written values and
formats, and uses the expected XLSB package parts.

## 2. Goal, Architecture, And Compatibility

### 2.1 Goal

Implement the approved writer-owned API and XLSB package pipeline. The result
must support multiple worksheets, cell writes, `Format` builders, row/column
widths and formats, style deduplication, shared strings, and the three save
entry points.

### 2.2 Architecture

```text
rust-xlsb-core                 rust-xlsb-writer
--------------                 ----------------
records / BiffHead             Workbook (public)
StyleRegistry / CellStyle      Worksheet (public)
SSTHolder                      Format / Color (public)
CellValue / ToCellValue        IntoExcelData / CellData
WbContext / NumberPolicy       sheet.rs (private)
temporal.rs                    package.rs (private)
                               error.rs (public)
```

- Core remains the low-level record and conversion owner.
- Writer owns the public facade and package assembly.
- `Workbook` owns `Vec<Worksheet>`.
- `Worksheet` owns sparse cells, row/column options, and a worksheet-local
  `Format` table.
- `Format` is independent of `Workbook`; conversion to `CellStyle` occurs at
  save.
- `ToCellValue` remains a low-level save-time conversion contract. Public
  writes use `IntoExcelData`.

### 2.3 Tech Stack

- Stable Rust `1.100.0`, edition 2024, resolver 3.
- `deku` for record layout through core.
- `rusqlite` (bundled) through core `SSTHolder`.
- `chrono` in writer for `IntoExcelData` / `IntoExcelDateTime`.
- `calamine` as writer dev-dependency for package round-trips.
- `mimalloc` remains the writer global allocator.
- No new ZIP dependency; use the existing raw store-only writer.

### 2.4 Compatibility Boundary

- Source-compatible for the implemented subset: `Workbook`, `Worksheet`,
  `Format`, `Color`, `IntoExcelData`, shared `NaiveDate` / `NaiveTime` /
  `NaiveDateTime` values, `write*`, and `save*`.
- Additive extensions, not replacements for missing `rust_xlsxwriter` methods:
  unformatted `write_date` and `write_time`; `TimeDelta` and `DateTime<Utc>`
  conversion. `rust_xlsxwriter` exposes only unformatted `write_datetime` plus
  `write_date_with_format` / `write_time_with_format`, and its
  `IntoExcelDateTime` covers `ExcelDateTime` plus the `Naive*` chrono types.
- `write_datetime` takes `impl IntoExcelDateTime` in both crates, but the trait
  method differs: this writer uses `into_cell_data(self) -> Result<CellData>`;
  `rust_xlsxwriter` uses `to_excel_serial_date(&self) -> f64`. Shared `Naive*`
  values are source-compatible; custom trait implementations are not.
- Output is XLSB, regardless of path extension.
- Error type is `Error` / `XlsbError`; no `XlsxError` alias.
- Omitted features are absent at compile time: formulas, URLs, rich strings,
  merges, charts, images, tables, conditional formats, data validation, notes,
  textboxes, sparklines, autofilters, defined names, page setup, panes.
- Exact integer policy: reject values outside `f64` exact range; do not round.
- Default font is Calibri 11.
- ZIP is store-only.

## 3. Change Necessity, Ripple, And Existence

### 3.1 Change Necessity

```text
Change Necessity:
- User-visible need: write XLSB files through a rust_xlsxwriter-shaped API
- No-change / non-code option: docs cannot write packages; current core API cannot support add_worksheet + save
- Why code change is necessary: public ownership and package writer are missing
- Minimum change boundary: rust-xlsb-writer facade plus core low-level reuse; remove core high-level types only after writer is complete
- Decision: code-change
```

### 3.2 Ripple Signal Triage

Public API, package output, compatibility, shared core, and retirement all fire.

- Canonical owner: `rust-xlsb-writer` for the facade; `rust-xlsb-core` for
  records, styles, SST, and value conversion.
- Downstream consumers: core unit tests, `e2e` package tests, `showcase`,
  workspace users, and future Arrow ingestion.
- Source-of-truth risk: two `Workbook` owners exist temporarily while the
  writer is built. Core `Workbook` / `Worksheet` are retired in Task 10;
  no fallback or alias remains.
- Verification carries: `calamine` round-trip, record-level assertions,
  LibreOffice render, full workspace tests, clippy, fmt.

### 3.3 Existence Check

```text
Existence Check:
- Proposed new surface: rust-xlsb-writer public facade and package writer
- Existing owner / reuse candidate: rust-xlsb-core currently owns Workbook / Worksheet; rust-xlsb-writer is a stub
- Why existing surface is insufficient: borrowed Worksheet cannot support add_worksheet + save; core has no public save
- Creation proof: approved design spec; user selected writer ownership
- Entropy / retirement impact: core high-level owner, core e2e harness, and core showcase are removed; no compatibility shim
- Decision: add-with-proof
```

### 3.4 Plan Pressure Test

```text
Plan Pressure Test:
- Owner / contract / retirement: writer owns facade; core keeps low-level; retirement is explicit in Task 10
- Architecture integrity / higher-level path: no second package writer; package.rs is the only ZIP/rels owner
- Verification scope: unit, record-level, calamine, LibreOffice, full workspace
- Task executability: each task compiles; writer tasks are ordered by type dependencies
- Pressure result: proceed
```

### 3.5 Complexity Budget

```text
Complexity Budget:
- Artifact class: public API + binary package writer
- Target files / artifacts: writer lib.rs, error.rs, format.rs, data.rs, worksheet.rs, sheet.rs, package.rs, workbook.rs; core lib.rs/workbook.rs/e2e removal
- Current pressure: core workbook.rs (236 lines), e2e harness (511 lines), writer lib.rs (536 lines, mostly commented prototype)
- Projected post-change pressure: writer split across seven focused modules; core loses high-level model and e2e
- Budget result: at-risk without splitting; within-budget with sheet.rs separated from package.rs
- Planned governance: one task per module boundary; no monolithic writer file; verification after each task
```

```text
Plan-Time Complexity Check:
- Target files: rust-xlsb-writer/src/{format,data,worksheet,sheet,package,workbook,error}.rs
- Existing size / shape signals: writer is a commented prototype; core Workbook and e2e are single-purpose but large
- Owner fit: writer for facade/package; core for records/styles/SST
- Add-in-place risk: dumping all API and package code into lib.rs would be unmaintainable
- Better file boundary: split format/data/worksheet/sheet/package/workbook/error
- Recommendation: add owner files; split sheet serialization from package assembly
```

## 4. TDD Route

```text
TDD Route:
- Mode: auto
- Decision: strict
- Authority: Aegis writing-plans route guard; this is shared/core public API and binary output
- Test posture: failing behavior test first for each new behavior; scaffolding and refactor tasks preserve existing tests
- Reason: public contract, style deduplication, record layout, and multi-sheet package output are high-risk regression surfaces
- Verification: cargo test --workspace, clippy, fmt, calamine round-trip, LibreOffice render
```

For each behavior task: add the smallest failing test, implement the minimum,
then run the task command. Refactor/migration tasks use existing tests as the
regression harness and add coverage only for a newly exposed edge.

## 5. Task Dependency Order

| Task | Depends on |
|---|---|
| 1. Writer foundation | — |
| 2. Format / Color | 1 |
| 3. Data conversion | 1 |
| 4. Worksheet model | 2, 3 |
| 5. Sheet serialization | 2, 3, 4 |
| 6. Package assembly | 5 |
| 7. Workbook and save | 6 |
| 8. Writer e2e tests | 7 |
| 9. Showcase migration | 7 |
| 10. Core retirement | 8, 9 |
| 11. Docs and final proof | 10 |

Tasks are execution units, not Git history units. After a task is reviewed and
freshly verified, the coordinator may create one scoped commit for that task.

## 6. Implementation Tasks

### Task 1 — Writer crate foundation

**Files**

- `rust-xlsb-writer/Cargo.toml`
- `rust-xlsb-writer/src/lib.rs`
- `rust-xlsb-writer/src/error.rs` (new)
- `rust-xlsb-core/src/lib.rs`

**Purpose**

Replace the commented prototype with a clean module root, the shared error
type, and the core record helper the writer needs.

**Change**

- Remove the commented `BiffPointer` / prototype block from `lib.rs`; keep the
  license header and `MiMalloc` global allocator.
- Declare only the modules that exist at this stage; extend `lib.rs` in later
  tasks.
- Add `chrono` dependency and `calamine` dev-dependency. Add `thiserror`.
- Add `error.rs` with:
  - `Core(#[from] rust_xlsb_core::Error)`
  - `Sst(#[from] rust_xlsb_core::SSTHolderError)`
  - `Io(#[from] std::io::Error)`
  - `InvalidSheetName(String)`
  - `SheetNameReused(String)`
  - `RowColumnLimit`
  - `RowColumnOrder`
  - `WorksheetIndex(usize)`
  - `StringTooLong`
  - `ValueNotRepresentable(&'static str)`
- Export `Error`, `Result<T, E = Error>`, and `XlsbError`.
- Re-export `push_record` from `rust-xlsb-core/src/lib.rs` so the writer does
  not duplicate the record framing helper.

**Compatibility impact**

No public writer API yet. Core adds only the `push_record` re-export; records
and conversion are unchanged.

**Verification**

```bash
cargo check -p rust-xlsb-writer
```

### Task 2 — Format and Color

**Files**

- `rust-xlsb-writer/src/format.rs` (new)
- `rust-xlsb-writer/src/lib.rs`

**Purpose**

Provide the xlsxwriter-shaped `Format` builder and map it to core `CellStyle`.

**Change**

- Add public `Color`, `FormatAlign`, `FormatBorder`, `FormatPattern`, and
  `Format`.
- Add private `NumFormat { Code(String), Index(u16) }`.
- `Format` fields: `font: BrtFont`, `fill: BrtFill`, `border: BrtBorder`,
  `xf: BrtXF`, `num_format: Option<NumFormat>`, `pattern_set: bool`.
- Implement v1 setters:
  - `set_bold`, `set_italic`
  - `set_font_color`, `set_font_name`, `set_font_size`
  - `set_num_format`, `set_num_format_index(u8)`
  - `set_align`, `set_text_wrap`
  - `set_pattern`, `set_background_color`, `set_foreground_color`
  - border methods and per-side colors
  - `set_locked`, `set_hidden`, `unset_hidden`
- `set_background_color` auto-selects `Solid` only when `!pattern_set` and the
  color is not `Default` / `Automatic`; `set_pattern` sets `pattern_set`.
- Add `pub(crate) fn to_cell_style(&self, registry: &mut StyleRegistry) -> Result<CellStyle>`.
  It resolves custom number formats, patches `xf.iFmt`, validates font name
  length, font size, and color indexes, and maps solid-fill colors so
  the visible color lands in `brtColorFore`.
- Map `FormatBorder::Hair` to `BorderType::Hairline`; map `FormatAlign`
  horizontal/vertical variants; map `FormatPattern` by ordinal.
- Export `Color`, `Format`, `FormatAlign`, `FormatBorder`, `FormatPattern`.

**Tests first**

- `Format::new()` converts to the default `CellStyle` and deduplicates to XF 0.
- Bold, italic, font size, font name, font color.
- Custom number format registers at 164 and deduplicates; index 14 is direct.
- Border style/color mapping for all four sides.
- Background color without a pattern becomes solid.
- Explicit `FormatPattern::None` is not overridden by a later background color.
- Font name above 31 UTF-16 units, font size outside `1.0..=409.0`, invalid
  theme index/shade, and unregistered custom index fail at conversion.
- `set_hidden` / `unset_hidden` toggle `BrtXF.fHidden`.

**Compatibility impact**

Adds the public `Format` / `Color` API from the design spec. Core `CellStyle`
remains low-level and unchanged.

**Verification**

```bash
cargo test -p rust-xlsb-writer format
```

### Task 3 — Data conversion

**Files**

- `rust-xlsb-writer/src/data.rs` (new)
- `rust-xlsb-writer/src/lib.rs`

**Purpose**

Add the public write trait and context-free cell data model.

**Change**

- Add `CellData`:
  `Blank`, `Bool`, `Error`, `Integer(i64)`, `Number(f64)`, `String`,
  `Date`, `Time`, `DateTime`, `Duration`.
- Add public `IntoExcelData` with `write` and `write_with_format`.
- Add public `IntoExcelDateTime` with `into_cell_data(self) -> Result<CellData>`.
- Implement `IntoExcelData` for:
  - `()`, `Option<T>`, `bool`, `BErr`
  - `u8`, `u16`, `u32`, `i8`, `i16`, `i32`, `i64`
  - `u64` with `i64` range check
  - `f32`, `f64`
  - `&str`, `String`, `Cow<'_, str>`
  - `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `TimeDelta`,
    `DateTime<Utc>`, and references to each
- Implement `IntoExcelDateTime` for the same chrono owned/reference set.
- `Option::None` writes a blank; with a format it writes a formatted blank.
- `u64` above `i64::MAX` fails immediately with
  `ValueNotRepresentable`; `i64` / `u64` above `f64` exact range fail later
  through `WbContext::store_integer`.

**Tests first**

- Every impl produces the expected `CellData`.
- `Option::Some` / `None`, owned and borrowed chrono values.
- `u64::MAX` errors; `i64::MAX` stores then fails at conversion.
- No workbook context is needed to construct `CellData`.

**Compatibility impact**

Adds public `IntoExcelData` / `IntoExcelDateTime`. `ToCellValue` remains
low-level. Shared `Naive*` chrono values are source-compatible; `TimeDelta` and
`DateTime<Utc>` are additive extensions, and a custom `IntoExcelDateTime`
implementation is not, because the trait method is `into_cell_data`, not
`to_excel_serial_date`.

**Verification**

```bash
cargo test -p rust-xlsb-writer data
```

### Task 4 — Worksheet model

**Files**

- `rust-xlsb-writer/src/worksheet.rs` (new)
- `rust-xlsb-writer/src/lib.rs`

**Purpose**

Own cell, row, column, and format data; expose the v1 write and layout API.

**Change**

- Add `Worksheet` with:
  - `name: String`
  - `cells: BTreeMap<u32, BTreeMap<u16, Cell>>`
  - `rows: BTreeMap<u32, RowOptions>`
  - `columns: BTreeMap<u16, ColumnOptions>`
  - `formats: Vec<Format>`
  - `format_ids: HashMap<Format, u32>`
- Add `Cell { data: CellData, format: Option<u32> }`,
  `RowOptions { height: Option<f64>, format: Option<u32> }`, and
  `ColumnOptions { width: Option<f64>, format: Option<u32> }`.
- Add `pub type RowNum = u32`, `pub type ColNum = u16`.
- Implement `set_name`, `name`, `write`, `write_with_format`, `write_row`,
  `write_column`, typed write helpers, date/time helpers, column width/format
  setters, and row height/format setters.
- Validate row/column bounds, widths `0.0..=255.0`, heights
  `0.0..=409.5`, and string length in UTF-16 code units.
- Intern `Format` by value; cells/rows/columns store local indexes.
- Add `pub(crate)` accessors for cells, rows, columns, formats, name.

**Tests first**

- Write stores the expected `CellData` and local format index.
- Row/column bounds and string length errors.
- `set_column_width` and range width override; later calls win.
- Column/row format interning deduplicates equal `Format` values.
- `set_name` rejects empty, >31 chars, and invalid characters.
- `write_row` / `write_column` stop at the failing item and keep earlier cells.

**Compatibility impact**

Adds the public `Worksheet` API. No package output yet.

**Verification**

```bash
cargo test -p rust-xlsb-writer worksheet
```

### Task 5 — Sheet serialization

**Files**

- `rust-xlsb-writer/src/sheet.rs` (new)
- `rust-xlsb-writer/src/lib.rs`

**Purpose**

Serialize one worksheet to BIFF records using the writer model.

**Change**

- Add `pub(crate) fn write_sheet(
  sheet: &Worksheet,
  style_indexes: &[u32],
  sst: &SSTHolder,
  out: &mut Vec<u8>,
) -> Result<()>`.
- Add a private `SaveContext<'a> { sst: &'a SSTHolder }` implementing
  `WbContext`.
- Emit `BrtBeginSheet`, `BrtWsDim`, column infos, sheet data, end markers.
- Column infos: coalesce sorted columns with equal width/format; use default
  `coldx = 2_158`, `fUserSet = true` for explicit options.
- Row headers: default `miyRw = 300`; set row `ixfe` and `fGhostDirty` when a
  row format exists; group spans by 1024 columns; never split one row into
  multiple `BrtRowHdr` records.
- Effective style: cell > row > column > 0.
- Convert `CellData`:
  - `String` -> `ctx.shared_string` -> `BrtCellIsst`
  - `Integer` -> `ctx.store_integer`
  - `Number` -> `ctx.store_f64`
  - temporal -> core `ToCellValue`
  - `Blank` / `Bool` / `Error` directly
- Use `push_record` from core for every record.

**Tests first**

- `BrtWsDim` min/max covers cells, row options, and column options.
- Column width emits `coldx = 5632` for 22.0 and coalesces adjacent equal
  columns; a sub-range override splits the range.
- Row height emits `miyRw = 600` for 30.0.
- Row format sets `BrtRowHdr.ixfe` and `fGhostDirty`; column format sets
  `BrtColInfo.ixfe`.
- Effective style precedence cell > row > column > default.
- Strings emit `BrtCellIsst`; numbers use Rk/Real per policy; blank/bool/error
  records are correct.
- A row with cells at columns 0 and 16,383 emits two spans; a full-width row
  emits 16 spans with `ccolspan <= 16`.

**Compatibility impact**

Private output path only. Record behavior matches the design spec.

**Verification**

```bash
cargo test -p rust-xlsb-writer sheet
```

### Task 6 — Package assembly

**Files**

- `rust-xlsb-writer/src/package.rs` (new)
- `rust-xlsb-writer/src/lib.rs`

**Purpose**

Build the complete XLSB ZIP from a slice of worksheets.

**Change**

- Add `pub(crate) fn build_package(sheets: &[Worksheet]) -> Result<Vec<u8>>`.
- Create `SSTHolder::create_in_memory()` and `StyleRegistry::new()`.
- For each sheet, convert every unique `Format` through `to_cell_style`, add it
  to the registry, and build the local-to-workbook XF map.
- Call `sheet::write_sheet` for each sheet.
- Emit:
  - `[Content_Types].xml`
  - `_rels/.rels`
  - `xl/_rels/workbook.bin.rels`
  - `xl/workbook.bin` with one `BrtBundleSh` per sheet
  - `xl/worksheets/sheetN.bin`
  - `xl/styles.bin`
  - `xl/sharedStrings.bin` only when the SST is non-empty
- Use the existing raw store-only ZIP writer; move it into `package.rs` with
  fixed DOS timestamp and stable part order.
- Add `#[cfg(test)]` helpers to expose part names and worksheet bytes for
  record-level assertions.

**Tests first**

- One-sheet and two-sheet packages open in `calamine`; values and sheet names
  match.
- ZIP contains the expected parts; shared strings part is omitted when empty.
- Styles part is present and has one default XF.
- Workbook rels contain one sheet relationship per sheet plus styles/SST.
- Multi-sheet `BrtBundleSh` names and rel IDs match the sheet parts.

**Compatibility impact**

Private package builder. Public save arrives in Task 7.

**Verification**

```bash
cargo test -p rust-xlsb-writer package
```

### Task 7 — Workbook and save APIs

**Files**

- `rust-xlsb-writer/src/workbook.rs` (new)
- `rust-xlsb-writer/src/lib.rs`
- `rust-xlsb-writer/src/package.rs` only if the test helpers need adjustment

**Purpose**

Expose the public `Workbook` and the three save entry points.

**Change**

- Add `Workbook { worksheets: Vec<Worksheet> }`.
- `new()` is infallible.
- `add_worksheet()` pushes a default `SheetN` and returns `&mut Worksheet`.
- `worksheet_from_index(index)` returns `Result<&mut Worksheet>`.
- `save`, `save_to_buffer`, and `save_to_writer` share `build_package`.
- `save`/`save_to_buffer` add one default sheet when empty.
- Validate sheet names before serialization; duplicates are case-insensitive.
- `save` builds bytes first, then creates/truncates the path and writes.
- `save_to_writer<W: Write + Send>` writes the built bytes.

**Tests first**

- `Workbook::new()` + `add_worksheet()` + writes + `save_to_buffer()` round-trip
  through `calamine`.
- Two sheets have the expected names and values.
- Empty workbook saves one default sheet.
- Duplicate and invalid sheet names fail before output.
- `save` to a path and `save_to_writer` produce the same bytes as
  `save_to_buffer`.
- Validation failures do not create the target file.

**Compatibility impact**

Adds the public entry point. Core still has its old `Workbook` until Task 10;
the two are in different crates and no public release happens between tasks.

**Verification**

```bash
cargo test -p rust-xlsb-writer workbook
```

### Task 8 — Writer e2e tests

**Files**

- `rust-xlsb-writer/src/e2e/mod.rs` (moved from core)
- `rust-xlsb-writer/src/e2e/harness.rs` (moved and adapted)
- `rust-xlsb-writer/src/lib.rs`
- `rust-xlsb-writer/Cargo.toml` (dev-dependency check)

**Purpose**

Preserve the package-level `calamine` oracle and record-level assertions on the
new public API.

**Change**

- Move the core `e2e` module into writer under `#[cfg(test)]`.
- Replace the old `CellSpec` / raw `CellStyle` model with public
  `Worksheet::write*`, `Format`, and `Workbook::save_to_buffer`.
- Keep record-level checks by using the `#[cfg(test)]` package helpers to read
  worksheet bytes and decode `BrtColInfo`, `BrtRowHdr`, and cell records.
- Preserve coverage for values, temporal values, row/column styles, shared
  strings, and style-only rows.
- Remove the old core package builder from the new harness; do not recreate a
  second package writer.

**Tests first**

- Existing `e2e` assertions adapted one-for-one.
- Add multi-sheet value round-trip and a default-style Calibri check.
- Add a format deduplication end-to-end assertion.

**Compatibility impact**

Tests only. Core `e2e` still exists until Task 10.

**Verification**

```bash
cargo test -p rust-xlsb-writer e2e
```

### Task 9 — Showcase migration

**Files**

- `rust-xlsb-writer/examples/showcase.rs` (new)

**Purpose**

Demonstrate the public API with distinct row/column styles, alignment, fills,
borders, and temporal formats.

**Change**

- Rewrite the showcase using `Workbook`, `Worksheet`, and `Format`.
- Keep the existing styled table layout: header row, alternating bands,
  italic label column, 10 pt value column, temporal number formats.
- Use row/column formats plus per-cell formats for visible parity in
  LibreOffice.
- Replace the `harness::build_with_sheet` call with `workbook.save(path)`.
- Keep the border cycle and outer frame from the existing demo.
- Do not include the core `e2e` harness by path.

**Compatibility impact**

Example only. The core copy remains until Task 10 removes it.

**Verification**

```bash
cargo run -p rust-xlsb-writer --example showcase -- /tmp/showcase.xlsb
```

### Task 10 — Core retirement

**Files**

- `rust-xlsb-core/src/workbook.rs` (delete)
- `rust-xlsb-core/src/e2e/` (delete)
- `rust-xlsb-core/examples/showcase.rs` (delete)
- `rust-xlsb-core/src/lib.rs`
- `rust-xlsb-core/src/cell_value.rs`
- `rust-xlsb-core/src/temporal.rs`
- `rust-xlsb-core/src/test_context.rs` (new, test-only)
- `rust-xlsb-core/Cargo.toml`

**Purpose**

Remove the old high-level owner and make core low-level only.

**Change**

- Delete `workbook.rs`; remove `mod workbook` and `pub use workbook::{...}`.
- Remove `#[cfg(test)] mod e2e;` and the `src/e2e/` directory.
- Add test-only `TestContext` with an in-memory `SSTHolder` and `WbContext`
  impl; update `cell_value.rs` and `temporal.rs` tests to use it.
- Remove `calamine` from core dev-dependencies after the e2e module is gone.
- Confirm no public `rust_xlsb_core::Workbook` / `Worksheet` re-export remains.

**Tests first**

This is a retirement/refactor task. Run the existing core unit tests after
migrating the test context; add no duplicate behavior tests.

**Compatibility impact**

Breaking core API: `Workbook` / `Worksheet` move to writer. This is approved by
the design spec. No compatibility alias or fallback.

**Retirement track**

Old core high-level types, core e2e harness, and core showcase source are removed.
The only package writer is `rust-xlsb-writer/src/package.rs`.

**Verification**

```bash
cargo test -p rust-xlsb-core
```

### Task 11 — Docs and final proof

**Files**

- `AGENTS.md`
- `README.md` only if the API status section needs correction
- `docs/aegis/specs/2026-11-18-xlsxwriter-compatible-api-design.md` status
- `docs/aegis/plans/2026-11-18-xlsxwriter-compatible-api.md`

**Purpose**

Align repository authority with the new crate ownership and record final
evidence.

**Change**

- Update `AGENTS.md`:
  - core = records/styles/SST/value conversion
  - writer = public `Workbook` / `Worksheet` / `Format` / package
  - directory and important-file tables
  - test counts and verification commands
  - default font Calibri 11
  - no `Workbook` / `Worksheet` in core
- Update the design spec status to `Implemented` only after all tasks pass.
- Do not invent a changelog or release workflow; none exists.
- Remove generated `showcase.xlsb` from the repository; keep verification
  artifacts under `/tmp`.

**Verification**

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
cargo run -p rust-xlsb-writer --example showcase -- /tmp/showcase.xlsb
soffice --headless --convert-to 'pdf:calc_pdf_Export' --outdir /tmp/render /tmp/showcase.xlsb
pdftoppm -png -r 130 -singlefile /tmp/render/showcase.pdf /tmp/render/showcase
```

Inspect the rendered PNG for fills, borders, alignment, row/column widths, and
temporal formats. Record actual test counts in `AGENTS.md`.

## 7. Verification And Evidence

### 7.1 Task-Level Gates

- Each task compiles independently.
- Behavior tasks have a failing test before implementation.
- `cargo test -p rust-xlsb-writer <module>` passes for the touched module.
- Record-level tests assert the exact record type and fields, not only
  `calamine` values.

### 7.2 Final Gates

- `cargo test --workspace` → all unit tests and doctests pass.
- `cargo clippy --workspace --all-targets` → zero warnings.
- `cargo fmt --all --check` → clean.
- `calamine` reads values from one-sheet and multi-sheet packages.
- LibreOffice renders the showcase with visible styles.
- `git status` contains no generated `.xlsb`, PDF, or PNG artifacts.

### 7.3 Acceptance Evidence

- Public compatibility example compiles and saves a readable XLSB.
- Multi-sheet package opens in `calamine`.
- `BrtColInfo` coalescing and `BrtRowHdr` span boundary tests pass.
- `Format` deduplication and default Calibri 11 tests pass.
- Full-width row `ccolspan <= 16` test passes.
- LibreOffice PNG shows header fills, row bands, borders, and alignment.

## 8. Risks And Mitigations

| Risk | Mitigation |
|---|---|
| Temporary two `Workbook` owners while writer is built | Task 10 removes core owner; no release between tasks; no alias |
| Theme tint scaling inferred from spec | Unit test the scale; render `Color::Theme` and inspect; adjust before release |
| Solid fill foreground/background semantics | Test `set_background_color` default-solid and both-color cases; render in LibreOffice |
| Pattern ordinal mapping | Unit test every `FormatPattern`; render a pattern sample |
| Multi-sheet rels/content types wrong | `calamine` + ZIP part assertions; test two sheets |
| `BrtRowHdr` span limit | Boundary test at columns 0 and 16,383; keep 1024 grouping; never split row headers |
| Core API break | Task 10 removes core types; update all tests/showcase; document migration |
| Store-only ZIP size | Accepted non-goal; no correctness impact |
| Default font change | Calibri 11 already applied; test `BrtFont::default` and expected 39-byte record |
| Failed final write leaves partial file | Build bytes first; document I/O-only partial risk |

## 9. Open Questions

None. The design spec resolves compatibility level, owner, error naming, string
storage, column override semantics, effective style precedence, and output
extension behavior.

## 10. Plan Self-Review

- Approved-scope coverage: all design-spec goals and v1 acceptance items map to
  tasks.
- Placeholders: none.
- Owner/type consistency: writer owns facade and package; core owns records,
  styles, SST, and conversion.
- Minimum change boundary: no formulas, charts, images, or unsupported stubs.
- Compatibility: source-compatible subset, `XlsbError`, `.xlsb`, additive
  date/time helpers, explicit non-goals.
- Verification: task commands, calamine round-trip, record-level assertions,
  LibreOffice render, full workspace gates.
- Retirement: core `Workbook` / `Worksheet`, core e2e harness, and core showcase
  are removed; no fallback remains.
- Conditional triggers: high-complexity/public-contract/complexity/new-owner
  guidance applied; no unhandled trigger.

## 11. Execution Handoff

- Route: `inline` — tasks share `lib.rs`, core types, and package state;
  subagent parallelism would create integration conflicts.
- Availability: no subagent dependency.
- Fallback: none needed.
- User confirmation required: no.
- Next skill: `aegis:executing-plans`.
