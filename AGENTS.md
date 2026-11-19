# Repository Guidelines

## Project Overview

`rust-xlsb-writer` is a pre-alpha Rust workspace for reading and writing MS-XLSB
(Excel Binary Workbook, BIFF12) files, targeting the [MS-XLSB] specification
revision `22.0` (release `2026-10-16`, superseding `2024-05-21`/revision 17.1).
The long-term goal is cell-by-cell writing plus batched
Apache Arrow ingestion, with styles, low RAM use, and exact IEEE 754 float
fidelity.

Current reality: `rust-xlsb-core` owns the binary record layer, the
value-conversion contract (`ToCellValue`/`CellValue`), the workbook context
(`WbContext`, `NumberPolicy`), the shared-string table (`SSTHolder`), the style
registry (`CellStyle`/`StyleRegistry`), and `chrono` temporal values. The
public XLSB writer lives in `rust-xlsb-writer`: `Workbook`, `Worksheet`,
`Format`, `Color`, `IntoExcelData`/`IntoExcelDateTime`, row/column
widths/formats, approximate autofit, scalar and static array formulas with cached results, multi-sheet package assembly with optional DEFLATE compression, ZIP64 mode, and optional Excel 2023 or custom themes, workbook calculation mode,
read-only recommendation, tab ratio, a workbook or worksheet default cell format, sheet
protection with unprotected ranges, ignored worksheet errors, an `AutoFilter`
range with list/custom filter conditions and automatic row hiding,
data validation with multi-range, list, whole-number, decimal, date, time, and
text-length rules, worksheet tables with header rows, built-in styles, table
AutoFilters, total rows, subtotal/label columns, data/header formats,
calculated-column formulas, table filter conditions, sparkline groups with
line/column/win-loss styles and cross-sheet data ranges, rich strings with
per-run fonts, core/app/custom document properties with explicit creation
timestamps,
user-defined names for literals, A1 references, and expression formulas with
same-workbook sheet and worksheet-range references, and
`save`/`save_to_buffer`/`save_to_writer`. The public write path stores strings in the shared-string
table and emits `BrtCellIsst`; `ToCellValue` remains the low-level core
contract and still converts `&str`/`String` to inline strings. `KnownID` covers
860 of the 878 records in revision 22.0; every id it does carry matches the
spec. Treat the README as a roadmap, not as a description of shipped API.

[MS-XLSB]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xlsb/acc8aa92-1f02-4167-99f5-84f9f676b95a

## Architecture & Data Flow

Two crates. `rust-xlsb-core` is the low-level record, style, shared-string, and
value-conversion layer. `rust-xlsb-writer` owns the public workbook model and
package pipeline.

Record pipeline, in the order bytes are produced:

1. A record struct derives `deku::DekuRead`/`DekuWrite` and stays a plain Rust
   type (no length prefix, no record id in the struct).
2. `Checked::check()` validates field invariants. Types that implement
   `Unchecked` get a blanket `Ok(())` implementation from `check.rs`, so an
   `Unchecked` record has no validation at all.
3. `CheckedBiff::id()` supplies the record identifier, normally through the
   exported `assign_id!(TypeName)` macro, which maps the type name onto the
   `KnownID` variant of the same name.
4. `BiffWrite::biff_push_to(&mut Vec<u8>)` / `biff_push_to_writer` serializes
   the body. A blanket impl covers every type that is
   `DekuContainerRead + DekuContainerWrite + CheckedBiff`.
5. `BiffHead::new(id, size)` encodes the record header (variable-length id plus
   varint size, 1–4 bytes each) and `write_as_biff` / `write_as_empty_biff`
   emit header then body.

`KnownID` is a `#[repr(u16)]` enum of 860 variants whose discriminants
are computed by the `const fn as_biff_id` in `biff_id.rs`. Those numbers are MS-XLSB
*record literals*, not raw spec ids. Never hand-write a discriminant; add the
variant through the same mapping. `KnownID::try_from(raw_u16)` rejects unknown
ids with `num_enum::TryFromPrimitiveError`.

Shared strings go through `SSTHolder` (`sst_sqlite.rs`): a SQLite table keyed by
the `xxh3_64` hash of the string, with a hit counter. `push` returns a stable
index (`ROWID - 1`), `push_batch` wraps the inserts in `BEGIN IMMEDIATE`, and
`write_to_biff_sst` emits `BrtBeginSst` / `BrtSSTItem`* / `BrtEndSst` ordered by
`ROWID`, with UTF-16 code-unit lengths. `len()` / `is_empty()` are a cached
`Arc<AtomicU32>` maintained by `push`/`push_batch`; it is not a query. The
holder is write-session scratch and is not reopened from disk, so ids are
contiguous `0..len`.

Value conversion is separate from records. `ToCellValue::to_cell_value(&dyn
WbContext)` returns a semantic `CellValue`; `WbContext` supplies the
shared-string table (`shared_strings`, `shared_string`) and a `NumberPolicy`
that chooses `Real` or a lossless `Rk` encoding. `&str`/`String` convert to
inline strings at this low-level layer; only an explicit `SharedString` goes
into the SST, and its id is range-checked against the target context at
conversion time.

Rich strings are a writer cell variant, not an SST entry. `write_rich_string`
stores the flattened text plus `ich` offsets and a `BrtFont` per run;
`build_package` registers those fonts after the cell formats and passes the
resulting `BrtFont` index map to the sheet serializer, which emits
`BrtCellRString` with a `RichStr`/`StrRun` payload. Run formats contribute font
properties only; there are no rich fills, borders, or phonetic-generation
paths.

Cell formulas are parsed when written. `formula.rs` compiles same-sheet
A1 references, ranges, literals, operators, and a fixed table of classic Excel
functions directly into a `CellParsedFormula` Ptg stream; unsupported syntax is
rejected with `Error::FormulaError`. The worksheet stores the token stream plus
a cached number, string, boolean, or error result. The sheet writer emits
`BrtFmlaNum`/`BrtFmlaString`/`BrtFmlaBool`/`BrtFmlaError`, and a workbook with
formulas gets `BrtCalcProp` with `fFullCalcOnLoad` unless the caller selected a
calculation mode. Static array formulas store their range on the top-left
cell, emit a `PtgExp` expansion formula followed by `BrtArrFmla`, and fill the
rest of the range with numeric zero. Table calculated-column formulas are
compiled at package save, after table ids and default names are assigned, and
emit `BrtListCCFmla` with `PtgList` structured references. Dynamic-array,
shared, external, cross-sheet, and named-range cell formulas are not
implemented.

Defined-name formulas are encoded at package save. Constants and bare A1
references keep their fast paths; other expressions use the same parser with a
name-only sheet context, emit `PtgRef3d`/`PtgArea3d`, and register their sheets
in `BrtExternSheet`. One `Xti` spans a worksheet range such as
`Sheet1:Sheet3!$A$1`. Unqualified references in a worksheet-scoped name resolve
to that sheet. Other defined names encode as `PtgName` using one ordered
`NameTable`, so forward references work and local names shadow globals.
External-workbook and future/`_xlfn` formulas are rejected.

Conditional formats are stored on the worksheet as one or more ranges, a boxed
`ConditionalFormat` rule, and an optional differential format. Cell-value,
top/bottom, average, duplicate/unique, color-scale, data-bar, icon-set, text,
blank, error, and date-occurring rules share that storage; color scales emit
nested `BrtCFVO`/`BrtColor` records, data bars add the
`BrtBeginDatabar`/`BrtEndDatabar` markers, icon sets add `BrtBeginIconSet` plus
one `BrtCFVO` per threshold, and text/blank/error/date rules emit a fixed
`CFParsedFormula` token stream. `build_package` registers each DXF in the
style registry before writing the styles part and passes the resolved ids to
the sheet serializer, which emits one `BrtBeginConditionalFormatting` block per
rule. DXFs are a separate table; workbooks without conditional formats keep
the zero-DXF styles table.

The public writer has its own `IntoExcelData` / `IntoExcelDateTime` path.
`Worksheet` owns sparse cells, row/column options with outline levels and
autofit widths, merged ranges, worksheet view/display state (including
normal, page-layout, and page-break-preview modes and hidden unused rows), tab
state, print state (page setup, text headers/footers, manual page breaks, print
areas, and repeat titles), hyperlinks with external worksheet relationships,
unprotected ranges, ignored worksheet errors, an `AutoFilter` range and filter
conditions, table filter conditions, a default cell format for detached
worksheets, and a worksheet-local format table; `workbook.rs` owns the
worksheet vector, document
properties, the optional default format and its row/column dimensions, looks
sheets up by index or exact name, and attaches detached `Worksheet::new`
values through `push_worksheet`; `write` stores
context-free `CellData` and `write_with_format` interns a `Format` by value.
Non-finite floats become configurable display strings because XLSB has no
NaN or infinity number cell. At
save, the writer creates an `SSTHolder`, turns high-level strings into shared
strings (`BrtCellIsst`), and converts numbers through `WbContext`.

Formatting is separate again. Core's `StyleRegistry` (`styles.rs`) is seeded
with the default `BrtXF` at index 0; `set_default_style` can replace cell XF 0
and the Normal style XF before worksheet formats are registered. `CellStyle`
pairs a `BrtFont`, `BrtFill`, `BrtBorder`, and the direct `BrtXF` attributes;
`add_style` interns each component, patches
`iFont`/`iFill`/`ixBorder`/`ixfeParent`, and deduplicates the resulting XF. `add_number_format` allocates custom `iFmt` ids from 164. The
public `Format` builder maps to `CellStyle` at save; border-aware range
formatting copies only the requested border sides; `Worksheet` stores row and
per-column width/format options; the writer emits coalesced `BrtColInfo`
records and sets `BrtRowHdr.ixfe` with `fGhostDirty` for rows.
`StyleRegistry::write_to_biff` emits the complete Styles part record sequence.

Formatting is separate again. Core's `StyleRegistry` (`styles.rs`) is seeded
with the default `BrtXF` at index 0; `set_default_style` can replace cell XF 0
and the Normal style XF before worksheet formats are registered. `CellStyle`
pairs a `BrtFont`, `BrtFill`, `BrtBorder`, and the direct `BrtXF` attributes;
`add_style` interns each component, patches
`iFont`/`iFill`/`ixBorder`/`ixfeParent`, and deduplicates the resulting XF. `add_number_format` allocates custom `iFmt` ids from 164. The
public `Format` builder maps to `CellStyle` at save; border-aware range
formatting copies only the requested border sides; `Worksheet` stores row and
per-column width/format options; the writer emits coalesced `BrtColInfo`
records and sets `BrtRowHdr.ixfe` with `fGhostDirty` for rows.
`StyleRegistry::write_to_biff` emits the complete Styles part record sequence.

Formatting is separate again. Core's `StyleRegistry` (`styles.rs`) is seeded
with the default `BrtXF` at index 0; `set_default_style` can replace cell XF 0
and the Normal style XF before worksheet formats are registered. `CellStyle`
pairs a `BrtFont`, `BrtFill`, `BrtBorder`, and the direct `BrtXF` attributes;
`add_style` interns each component, patches
`iFont`/`iFill`/`ixBorder`/`ixfeParent`, and deduplicates the resulting XF. `add_number_format` allocates custom `iFmt` ids from 164. The
public `Format` builder maps to `CellStyle` at save; border-aware range
formatting copies only the requested border sides; `Worksheet` stores row and
per-column width/format options; the writer emits coalesced `BrtColInfo`
records and sets `BrtRowHdr.ixfe` with `fGhostDirty` for rows.
`StyleRegistry::write_to_biff` emits the complete Styles part record sequence.

Temporal conversion lives in `temporal.rs` and reuses the same path.
`NaiveDate` becomes an integer day serial in the 1900 system, so dates are
`BrtCellRk` ints; `NaiveTime`, `NaiveDateTime`, and `TimeDelta` become day
fractions rounded to milliseconds, so they are normally `BrtCellReal`.
`DateTime<Utc>` uses its UTC wall clock. Values outside 1900-01-01..9999-12-31,
chrono leap seconds, and a time that rounds past midnight are rejected. There is
no 1904-system support (`BrtWbProp` exists only for the theme version and an
empty code name) and no timezone other than UTC. Optional borrowed `jiff` civil values map through `chrono` in `data.rs`, so
they use the same serial rules; optional `rust_decimal::Decimal` values convert
to `f64` without allocating. The writer's native `ExcelDateTime` mirrors the
reference serial formula because it also covers Excel's fake 1900-02-29 and
hours beyond 24, which `chrono` rejects.

## Key Directories

| Path | Contents |
|---|---|
| `Cargo.toml` | Virtual workspace manifest; members and shared `[workspace.package]` metadata |
| `rust-xlsb-core/src/` | Low-level layer: record framing, id catalogue, value conversion, temporal serials, context, shared strings, style registry |
| `rust-xlsb-core/src/prelude/` | Public record/structure facade (`record/`, `structure/`, `scope.rs`, `collection.rs`) |
| `rust-xlsb-writer/src/` | Public writer: `Workbook`, `Worksheet`, `Format`/`Color`, data traits, sheet serializer, package assembly |
| `rust-xlsb-writer/examples/` | `showcase.rs`: styled XLSB table through the public API |
| `docs/` | Capability gap/roadmap plus `docs/aegis/` specs, plans, and index |
| `.github/` | `FUNDING.yml` only; no workflows, templates, or CI |

## Development Commands

The workspace builds on stable Rust. `rust-toolchain.toml` pins `1.100.0`
(duplicated in `rust-xlsb-core/`), edition is 2024, and the resolver is `3`.

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
cargo doc -p rust-xlsb-core --no-deps
```

Toolchain selection is the one environment trap here. `rust-toolchain.toml`
pins `1.100.0`, but a wrapper `cargo` earlier in `PATH` (for example the `mbx`
build-cache shim under `~/.local/share/mbx/bin`) dispatches `rustc` itself and
can override that pin, producing misleading errors such as
`rustc 1.88.0-nightly is not supported`. Verify before trusting any result:

```bash
rustc -V            # must print 1.100.0
cargo -v build -p rust-xlsb-core 2>&1 | grep rustc | head -1
```

If the versions disagree, run the toolchain binaries by absolute path, which
also pins the compiler `cargo` spawns:

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace
```

Verified state on this branch:

- `cargo test --workspace` → 711 unit tests and 7 doctests pass, 0 failures, with
  `rustc -V` reporting `1.100.0` for both the cargo and every spawned rustc.
- `cargo test --workspace --all-features` → 715 unit tests and 7 doctests pass,
  including the optional `jiff` and `rust_decimal` adapters.
- `cargo clippy --workspace --all-targets` → no warnings and no code lints.
- `cargo fmt --all --check` → clean.
- `Cargo.lock` is gitignored, so a fresh resolve can pick different minor
  versions; with the current manifests all dependencies resolve and compile on
  `1.100.0`.

No CI workflow or formatter config exists; default `rustfmt` settings apply. The
workspace manifest does configure clippy (`[workspace.lints.clippy]`), and both
crates opt in with `[lints] workspace = true`.

## Code Conventions & Common Patterns

- **Module per record group.** One file per record or small family under
  `prelude/record/` and `prelude/structure/`; re-export from
  `prelude/mod.rs` and register the type there.
- **Spec names stay verbatim.** `#![allow(non_snake_case, clippy::upper_case_acronyms)]`
  is set at the top of `prelude/mod.rs` only. Fields such as `rwSync` and types
  such as `BrtXF`, `BrtWsProp` keep their specification spelling; do not
  "clean up" names. Outside the prelude, use snake_case for functions and
  variables.
- **Errors.** Core has `Error` in `err.rs` with `pub type Result<T>`; the
  writer has its own `Error`/`Result<T, E = Error>` in `error.rs` and wraps core
  errors with `#[from]`. Subsystems that need their own variants define a
  local error enum (`SSTHolderError`) and convert with `#[from]`. Every public
  fallible function documents a `# Errors` section, and doctests accompany
  public constructors.
- **Error fidelity.** The blanket `BiffWrite` impl discards the underlying deku
  error and returns `Error::ValidationError("Invalid BIFF record")`. If you need
  the cause, format it, as `KnownID::try_deku_literal` does.
- **Bit fields are LSB-first.** MS-XLSB packet diagrams place bit 0 in the
  least significant position; deku defaults to MSB. Records with bit fields
  use `#[deku(bit_order = "lsb", ctx = "order: deku::ctx::Order",
  ctx_default = "deku::ctx::Order::Lsb0")]`, and child structs that receive
  that context declare the same `ctx`/`ctx_default` pair. Do not add a
  container-level `bit_order` without the context default: the type stops
  implementing `DekuContainerRead`/`DekuContainerWrite`.
- **Zero-sized markers.** `scope.rs` and `collection.rs` generate `BrtBegin*`/
  `BrtEnd*` structs via `concat-idents`. Scope markers serialize to exactly 0
  bytes; collection markers carry a `u32 count`.
- **Unsafe is deliberate.** `BiffHead::read_from_with_data` /
  `RawBiffLiteral::packed` use `set_len`/`new_uninit_slice`. Keep the same
  style when extending them, keep the uninitialized regions fully written
  before `assume_init`, and add a test. `BiffHead::const_from_data` is now a
  safe slice parser.
- **Adding a record** requires all of: deku derives with the correct bit layout,
  a `Checked` impl (or `Unchecked`), `assign_id!(TypeName)`, a re-export in
  `prelude/mod.rs`, and an `#[case::<name>(|| checked_test::<TypeName>(SIZE))]`
  entry in the `test_biff_export` rstest table in that file's test module.
- **Lints are active.** `rust-xlsb-core` sets `#![deny(dead_code)]`; an unused
  private helper fails the build. Wire it up or delete it. Both crates inherit
  `[workspace.lints]` (`clippy::pedantic` and `clippy::nursery` at `warn`);
  intentional violations carry a scoped `#[expect(..., reason = "…")]` instead
  of a blanket crate-level allow.
- **Allocator.** `mimalloc` is installed as the global allocator in
  `rust-xlsb-writer` only. A second `#[global_allocator]` in core would make the
  workspace fail to link.

`XlString` (`structure/xl_wide_string.rs`) stores `len` as UTF-16 code units
and deku reads `len * 2` bytes; `FromStr` and `from_string` count
`encode_utf16()`, not UTF-8 bytes. `0xFFFF_FFFF` means null, and
`SimpleRichStr` rejects null while `CodeName` limits length to 31.

## Important Files

| File | Role |
|---|---|
| `rust-xlsb-core/src/lib.rs` | Module list, the entire public re-export surface, `assign_id!` |
| `rust-xlsb-core/src/biff_id.rs` | `KnownID` catalogue, `as_biff_id` remap, `RawBiffLiteral` and its literal factories |
| `rust-xlsb-core/src/biff_head.rs` | Record header encode/decode, `read_from_with_data` |
| `rust-xlsb-core/src/biff.rs` | `BiffWrite`, `write_as_biff`, `write_as_empty_biff` |
| `rust-xlsb-core/src/check.rs` | `Checked`, `Unchecked`, `CheckedBiff` traits |
| `rust-xlsb-core/src/err.rs` | `Error`, `Result` |
| `rust-xlsb-core/src/sst_sqlite.rs` | `SSTHolder` shared-string table, cached length, SST emission |
| `rust-xlsb-core/src/styles.rs` | `CellStyle`/`StyleRegistry` deduplication, DXF table, and Styles part emission |
| `rust-xlsb-core/src/context.rs` | `SharedString`, `WbContext`, `NumberPolicy`, `NumberEncoding` |
| `rust-xlsb-core/src/cell_value.rs` | `CellValue`, `ToCellValue` and its impl matrix |
| `rust-xlsb-core/src/temporal.rs` | `chrono` → Excel serial conversion for dates, times, datetimes, durations |
| `rust-xlsb-core/src/test_context.rs` | Test-only `TestContext` for low-level conversion tests |
| `rust-xlsb-core/src/prelude/mod.rs` | Re-exports plus the shared round-trip test harness |
| `rust-xlsb-core/src/prelude/record/brt_merge_cell.rs` | `BrtMergeCell` range record |
| `rust-xlsb-core/src/prelude/record/brt_begin_ws_view.rs` | `WSVIEWS2` view records; `brt_pane.rs`/`brt_sel.rs` carry panes and selection |
| `rust-xlsb-core/src/prelude/record/brt_begin_header_footer.rs` | `BrtBeginHeaderFooter` text payload and flags |
| `rust-xlsb-core/src/prelude/record/brt_page_breaks.rs` | manual row/column break records |
| `rust-xlsb-core/src/prelude/record/brt_calc_prop.rs` | workbook calculation mode and flags |
| `rust-xlsb-core/src/prelude/record/brt_file_sharing.rs` | read-only recommendation record |
| `rust-xlsb-core/src/prelude/record/brt_dval.rs` | `DVALS`, `BrtDVal`, and validation string records |
| `rust-xlsb-core/src/prelude/record/brt_table.rs` | Table, table-column, calculated/total formula, table-part, and table-style records |
| `rust-xlsb-core/src/prelude/record/brt_cell_rstring.rs` | `BrtCellRString` rich-string cell record |
| `rust-xlsb-core/src/prelude/record/brt_conditional.rs` | Conditional-format range block, rule record, value object, color-scale/data-bar/icon-set markers, text/blank/error/date templates, and parsed formula |
| `rust-xlsb-core/src/prelude/record/brt_dxf.rs` | Differential-format records and XF property blobs |
| `rust-xlsb-core/src/prelude/structure/rich_str.rs` | `RichStr`, `StrRun`, `PhRun`, and run validation |
| `rust-xlsb-writer/src/workbook.rs` | `Workbook`, sheet naming/validation, defined names, document properties, theme/default-format selection, save entry points |
| `rust-xlsb-writer/src/theme.rs` | Excel 2023 and custom theme selection, `.thmx`/`.xlsx` theme extraction, and `theme_2023.xml` asset |
| `rust-xlsb-core/src/prelude/record/brt_wb_prop.rs` | `BrtWbProp` workbook flags, theme version, and code name |
| `rust-xlsb-writer/src/defined_name.rs` | User-defined name model, validation, and `NameParsedFormula` token encoding |
| `rust-xlsb-writer/src/formula.rs` | `Formula`, A1 and structured-reference parser, Ptg encoder, function table, cached-result typing, and array expansion helper |
| `rust-xlsb-core/src/prelude/record/brt_fmla.rs` | `CellParsedFormula`, `BrtArrFmla`, and the four formula cell records |
| `rust-xlsb-writer/src/properties.rs` | `DocProperties`, custom values, and chrono creation timestamps |
| `rust-xlsb-writer/src/worksheet.rs` | Sparse cells, row/column options, merge, write, layout, view, and print API |
| `rust-xlsb-writer/src/format.rs` | `Format`, `Color`, enums, `CellStyle` mapping |
| `rust-xlsb-writer/src/data.rs` | `IntoExcelData`/`IntoExcelDateTime` and context-free `CellData` |
| `rust-xlsb-writer/src/datetime.rs` | Native `ExcelDateTime`, validation, parsing, and serial conversion |
| `rust-xlsb-writer/src/filter.rs` | Autofilter condition model and row matching |
| `rust-xlsb-writer/src/data_validation.rs` | Data-validation rules, prompt/error options, and `BrtDVal` encoding |
| `rust-xlsb-writer/src/table.rs` | `Table`/`TableColumn`/`TableFunction`/`TableStyle`, total-row, calculated-formula, and format handling, validation, and Table part serialization |
| `rust-xlsb-writer/src/conditional_format.rs` | Conditional-format trait, cell/top-bottom/average/duplicate/color-scale/data-bar/icon-set/text/blank/error/date rules, values, formula tokens, and range storage |
| `rust-xlsb-writer/src/sheet.rs` | Worksheet BIFF serialization, column coalescing, row spans, rich cells, autofilter, conditional-format/filter/validation/list-part records, print records |
| `rust-xlsb-writer/src/package.rs` | Multi-sheet package, styles, SST, rels, store-only default ZIP with optional DEFLATE and ZIP64 |
| `rust-xlsb-writer/src/error.rs` | Writer `Error`, `Result`, `XlsbError` |
| `rust-xlsb-writer/src/e2e/mod.rs` | Writer package tests read back through `calamine` |
| `rust-xlsb-writer/examples/showcase.rs` | Public-API showcase with row bands, fills, alignment, borders, temporal formats |
| `docs/gap-analysis-rust-xlsxwriter.md` | Current capability gap versus `rust_xlsxwriter` and prioritized P0–P5 writer roadmap |
| `rust-toolchain.toml` | Stable `1.100.0` pin at the root; `rust-xlsb-core/rust-toolchain.toml` duplicates it |

## Runtime/Tooling Preferences

- Stable Rust `1.100.0`, edition 2024, workspace resolver `3`. Both
  `rust-toolchain.toml` files pin the same channel.
- No unstable features are used. `MaybeUninit::write_copy_of_slice`,
  `String::from_utf16le`, and `std::hint::cold_path` are stable on this
  toolchain; `std::hint::unlikely` is not, so branch-hint annotations were
  removed from `biff_head.rs` rather than replaced with a shim.
- `Cargo.lock` is gitignored by design (library workspace). Pin minor versions
  in the manifests when a specific dependency minor matters, and expect
  dependency resolution to drift between checkouts.
- `rusqlite` uses the `bundled` feature, so a C compiler is needed and the first
  build compiles SQLite from source.
- `rust-xlsb-writer` depends on `rust-xlsb-core`, `mimalloc`, `thiserror`,
  `chrono`, and `flate2`; optional `jiff` and `rust_decimal` dependencies are
  enabled by the same-named features. Its dev-dependencies add `calamine`, `deku`, `criterion`,
  and `rstest`. Core no longer depends on `calamine`.
- Dependency set, latest major/minor for this toolchain: `deku 0.20`,
  `rusqlite 0.43`, `strum 0.28`, `criterion 0.8`, `zerocopy 0.8.63`,
  `thiserror 2.0.21`, `num_enum 0.7.6`, `xxhash-rust 0.8.21`,
  `concat-idents 1.1.5`, `chrono 0.4` (no `clock`, `std` only),
  `mimalloc 0.1.52`, `flate2 1.1` (`zlib-rs` backend), `rstest 0.28` (dev),
  `calamine 0.38` (writer dev, `chrono` feature enabled), `jiff 0.2`
  (optional), `rust_decimal 1.40` (optional).
- No release tooling, changelog, or publish workflow. Version is `0.1.0` and the
  README defers `1.0.0-alpha` to community adoption.

## Testing & QA

- Every test lives inside the crate: `#[cfg(test)] mod tests` next to the code
  it covers. Core has no package suite; writer has `#[cfg(test)] mod e2e`
  (`rust-xlsb-writer/src/e2e/`) using the public API and `calamine`. There is
  no `tests/`, `benches/`, or fixture directory.
- 711 unit tests and 7 doctests in default builds: 302 in core, 409 in writer.
  `--all-features` adds 4 writer tests for the optional `jiff` and
  `rust_decimal` adapters, for 715 total. Most core
  unit tests are rstest cases in the `test_biff_export` table in
  `prelude/mod.rs`, each calling the generic `checked_test::<T>`: `Default` →
  `to_bytes` → exact byte-size assert → `from_bytes` → `check()` on both →
  `PartialEq`, `Hash`, and `Debug` equality.
- Case names are `test_biff_export::case_<N>_<name>`; the size passed to
  `checked_test` is the assertion, so change it whenever a record layout
  changes.
- Hand-written core tests: `biff_id::tests::test_known_ids` (literal packing,
  header parse), `biff_head::tests` (varint sizes `0x80`/`0x4000`/`0x200000`
  and two-byte body reads), `sst_sqlite::tests::test_sst_sqlite` (push, batch,
  SST counters, item decode), `prelude::structure::xl_wide_string::tests`,
  `prelude::record::brt_color` LSB/RGB regressions,
  `prelude::record::brt_font::tests::default_font_matches_excel`,
  `prelude::record::brt_row_hdr::ghost_dirty_lands_in_the_lsb_first_bit`,
  and header/footer/page-break record limits and field packing.
- Hand-written writer tests cover `Format` mapping/dedup/validation,
  `IntoExcelData`, worksheet bounds and overrides, sheet record layout
  (`BrtColInfo` coalescing, `BrtRowHdr` ghost-dirty/spans/style precedence),
  package parts, ZIP compression/ZIP64 and readback, multi-sheet save,
  print/header/page-break serialization,
  print-area/repeat-title/user defined names, hyperlink records/relationships, workbook
  calculation/read-only/tab-ratio records, range protection/ignored-error
  records, and calamine round-trips for values and temporal formats.
- `criterion` is a dev-dependency in both crates but no benchmark exists.
- No CI runs any of this. Run `cargo test --workspace` before claiming a change
  works, and update the affected size literal rather than loosening the
  assertion.
- `cargo run -p rust-xlsb-writer --example showcase -- showcase.xlsb` writes a
  styled table through the public API for manual inspection in LibreOffice or
  Excel. LibreOffice rendering is the visual oracle for fills, borders,
  alignment, row/column widths, and temporal formats.

## Commit Style

Git history uses Conventional Commits (`feat:`, `fix:`) with a capitalized
subject. This is observed practice, not a documented rule.
