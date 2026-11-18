# Rich strings

- Status: Implemented — reviewed clean
- Date: 2026-11-19
- Scope: `Worksheet::write_rich_string`,
  `Worksheet::write_rich_string_with_format`, `BrtCellRString`, `RichStr`,
  `StrRun`, and rich-run font registration.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.2 and §4.
- Reference: vendored `rust_xlsxwriter` v0.102.0
  `Worksheet::write_rich_string`, `store_rich_string`, and `get_rich_string`.

## Goal

Add a source-compatible subset of rich strings: one cell containing a
concatenated text with per-run font formatting. The public signature and
validation match the reference:

```rust
worksheet.write_rich_string(row, col, &[(&Format, &str)])?;
worksheet.write_rich_string_with_format(row, col, &[(&Format, &str)], &format)?;
```

The first release supports font-run formatting only: bold, italic,
strikethrough, underline, superscript/subscript, font name, size, color, family,
charset, and scheme. It does not add phonetic runs, rich shared-string table
entries, or run-level fill/border/number formats. Rich strings are written as
`BrtCellRString`, not SST entries, so the shared-string contract is unchanged.

## Aegis Visibility

The change adds a public writer API, a cell variant, two MS-XLSB structures, one
record, and a new style-registry seam for font-only runs. The plan keeps the
record layout, run offsets, font-index ownership, and validation boundary
explicit.

## Architecture

`rust-xlsb-core` owns the wire format:

- `prelude/structure/rich_str.rs`: `RichStr`, `StrRun`, `PhRun`.
- `prelude/record/brt_cell_rstring.rs`: `BrtCellRString`.
- `StyleRegistry::register_font(BrtFont) -> Result<u16>` registers a font-only
  entry and returns its `BrtFont` index. Existing format registration continues
  to use the same private deduplication table.

`rust-xlsb-writer` owns the user model:

- `data::RichString { text, runs }` and `data::RichRun { ich, font }`.
- `CellData::RichString(RichString)`.
- `Worksheet::write_rich_string` and
  `Worksheet::write_rich_string_with_format`.
- `Format::to_rich_font()` applies font color and validates font name and size.
- `package::build_package` registers every run font in sheet order after the
  existing cell formats, then passes a `HashMap<BrtFont, u16>` to
  `sheet::write_sheet`.
- `sheet::push_cell` emits `BrtCellRString` with run offsets converted from
  UTF-16 code-unit lengths to `StrRun.ich` and looked-up `StrRun.ifnt` values.

The cell format remains the worksheet style index. Rich runs do not allocate a
`BrtXF`, fill, or border entry.

## RichStr Contract

`RichStr` layout follows [MS-XLSB] §2.5.122:

1. `flags: u8`; bit 0 `fRichStr`, bit 1 `fExtStr`, bits 2..7 unused zero.
2. `str: XlWideString`.
3. When `fRichStr`: `dwSizeStrRun: u32` then `rgsStrRun: Vec<StrRun>`.
4. When `fExtStr`: `phoneticStr: XlWideString`, `dwPhoneticRun: u32`, then
   `rgsPhRun: Vec<PhRun>`.

`StrRun` is `ich: u16`, `ifnt: u16`. `PhRun` is `ichFirst`, `ichMom`, `cchMom`,
`ifnt`, then a 16-bit flags word. The writer emits `fExtStr = 0`; the decoder
keeps the extension for real-file completeness.

Validation:

- At least one `(&Format, &str)` tuple.
- No empty segment.
- Concatenated text at most 32,767 UTF-16 code units.
- Every run offset is strictly increasing and less than the text length.
- Font name at most 31 UTF-16 code units; font size in 20..=8,180 twips.
- Every emitted `StrRun.ifnt` resolves in the workbook font table.

The first segment starts at offset 0 even when its format is default, so the
record always carries an explicit font index for every run.

## Compatibility Boundary

- Public API matches reference names and argument shape.
- Single-segment rich strings are accepted, as in the reference.
- `write_rich_string_with_format` adds a cell-wide format; run formats only
  affect font properties.
- Shared strings, tables, data validation, filters, and existing cell records
  are unchanged.
- No formula, phonetic, run-level fill/border, or rich SST support.

## TDD Route

- Mode: `off` — no explicit user/project TDD authority.
- Decision: `skipped`.
- Test posture: record round-trip tests, writer validation tests, package
  decode tests, and a LibreOffice XLSX shared-string round-trip.
- Verification: focused writer/core tests first, then workspace test, clippy,
  fmt, and LibreOffice round-trip where it preserves rich runs.

## Tasks

1. **Core structures and record**
   - Add `RichStr`, `StrRun`, `PhRun` with deku derives and `Checked`.
   - Add `BrtCellRString` with cell-range and rich-string validation.
   - Add `assign_id!(BrtCellRString)`, prelude exports, and `test_biff_export`
     cases.
   - Add a record test with two runs at distinct UTF-16 offsets and a font id.
   - Verify: core tests for default byte sizes and run round-trip.

2. **Font registration seam**
   - Add `StyleRegistry::register_font`; keep `add_font` private.
   - Factor `Format` font construction into `to_rich_font` without duplicating
     validation.
   - Test default-font deduplication and bold-font registration.
   - Verify: core style test plus writer format test.

3. **Worksheet API and data model**
   - Add `RichString`/`RichRun` and `CellData::RichString`.
   - Implement both public methods, including tuple, empty-string, length,
     row/column, and font validation.
   - Include rich strings in autofit width measurement.
   - Verify: validation and storage tests; no extra allocations beyond the
     flattened text and bounded run vector.

4. **Package and sheet emission**
   - Register all rich-run fonts once in `build_package`.
   - Pass the font-index map into `write_sheet` and `push_cell`.
   - Emit `BrtCellRString`; resolve every `ifnt`; preserve cell format and
     dimension behavior.
   - Verify: package decode asserts text, flags, run offsets, font ids, and
     font records; LibreOffice exports the runs to XLSX shared strings.

5. **Docs and closure**
   - Update gap analysis counts and status; mark rich strings supported.
   - Update `AGENTS.md` test counts and public API description.
   - Run default/all-feature tests, clippy, fmt, `git diff --check`.
   - LibreOffice round-trip: inspect the exported XLSX shared string runs when
     LibreOffice preserves them; otherwise record the drop as an evidence limit.
   - Close with the Aegis plan and commit.

## Verification Matrix

| Seam | Evidence |
|---|---|
| `RichStr`/`StrRun`/`PhRun` | Exact-byte round-trip and run-offset test |
| `BrtCellRString` | `test_biff_export` size plus custom run decode |
| Font registration | Registry dedup test and package font-index assertions |
| API validation | Empty tuple, empty segment, overlong text, bad font, bad cell |
| Flattened text | Package decode reads the exact concatenated string; calamine has no `BrtCellRString` reader |
| Formatting | Package decode proves distinct run font ids; LibreOffice check if available |
| Regression | Full workspace tests and all-feature tests |

## Risks And Limits

- Calamine does not expose `BrtCellRString`. Rich text is proven by package
  decode plus the LibreOffice XLSX shared-string export, which preserved run
  offsets, bold/italic, and colors.
- `CellData` is public but `#[doc(hidden)]`; adding a variant is a pre-alpha
  internal contract change, not a source-compatible downstream guarantee.
- The writer does not intern rich strings in the SST. This is intentional: XLSB
  cell records carry `RichStr` directly.
- Full phonetic runs are decoded but not generated.

## Retirement

No old path is retained. The new record and cell variant own rich strings;
plain strings continue through `BrtCellIsst`. No fallback writes rich text as
plain text.

## Evidence

- Default workspace: 503 unit tests and 7 doctests pass.
- All features: 507 unit tests and 7 doctests pass.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check` and `git diff --check`: clean.
- LibreOffice converted a generated XLSB to XLSX and preserved the rich-string
  runs in `xl/sharedStrings.xml`: UTF-16 offsets, bold/italic, and red/blue run
  colors matched the input.
- Calamine does not read `BrtCellRString`; record and package decode tests
  remain the structural oracle.

## Review

`RichStringReview` found and the implementation fixed:

1. Rich-string cells were invisible to autofilter matching, hiding matching
   rows. `filter.rs` now handles `CellData::RichString` in list and custom
   matching, with unit and package-level hidden-row coverage.
2. `RichStr::check` omitted phonetic-run ordering and total parent-length
   checks. It now enforces strictly increasing `ichFirst`/`ichMom`,
   `cchMom <= str.len` per run, and `sum(cchMom) <= str.len`; the rejection
   test covers both the ordering and total-overrun branches.
3. Record tests were self-round-trips only. Both `BrtCellRString` and the
   rich+phonetic `RichStr` now assert their complete wire bytes.

Re-review confirmed the filter union arms, phonetic bounds, and 29/43-byte
golden payloads are correct, with no remaining blocking findings.

Follow-up review hardening:

- Phonetic runs also require `ichMom + cchMom <= str.len`, not only separate
  bounds.
- Table header inference reads existing rich-string headers as their flattened
  text.
- Package font registration skips the cell scan when a worksheet never wrote a
  rich string; when it does scan, current cells remain authoritative, so
  overwrites cannot leave stale fonts.
