# Worksheet headers, footers, and manual page breaks

- Status: Implemented — reviewer found no defects
- Date: 2026-11-18
- Parent analysis: `docs/gap-analysis-rust-xlsxwriter.md` §3.3, §6 P2
- Baseline: `7403bd6` (`feat: add worksheet page setup and print options`)
- Target: `rust-xlsb-core`, `rust-xlsb-writer`

## 1. Goal

Complete the text header/footer and manual page-break printing surface:

- `set_header(text)`
- `set_footer(text)`
- `set_header_footer_scale_with_doc(enable)`
- `set_header_footer_align_with_page(enable)`
- `set_page_breaks(&[row])`
- `set_vertical_page_breaks(&[column])`

Headers use Excel control codes (`&L`, `&C`, `&R`, `&P`, `&N`, `&D`, `&T`,
`&F`, `&A`, `&Z`, `&B`, `&I`, `&U`, `&&`) and expand the reference aliases
(`&[Page]`, `&[Pages]`, `&[Date]`, `&[Time]`, `&[File]`, `&[Tab]`, `&[Path]`,
`&[Picture]`). A default workbook stays byte-identical: no header/footer record
and no page-break records.

## 2. Scope And Acceptance

Accepted when:

- `BrtBeginHeaderFooter` carries the signed `grbit` flags and six nullable
  header/footer strings, followed by the empty `BrtEndHeaderFooter` scope;
- header/footer strings are UTF-16 limited to 255 code units; longer input is
  ignored without changing prior text;
- `fHFScaleWithDoc` and `fHFAlignMargins` default to true and are configurable;
- `BrtBeginRwBrk`/`BrtBeginColBrk` carry `ibrkMac` and `ibrkManMac`, followed by
  one `BrtBrk` per break and the matching end marker;
- horizontal breaks span all columns (`0..=16383`), vertical breaks span all
  rows (`0..=1048575`), `fMan = 1`, and `fPivot = 0`;
- break inputs ignore `0`, sort, and deduplicate; more than 1023 valid breaks
  is rejected with `Error::TooManyPageBreaks`; rows above `1_048_575` or
  columns above `16_383` are rejected with `Error::RowColumnLimit`;
- an empty break slice is a no-op; a non-empty slice with only zeros clears the
  stored breaks;
- record order follows the worksheet ABNF: page setup, header/footer, row
  breaks, column breaks, then `BrtEndSheet`;
- calamine still reads cells from a workbook using every new option;
- existing tests and default output remain unchanged.

Out of scope: header/footer images, even/first-page variants, print areas,
repeat rows/columns, automatic page breaks, and printer settings.

## 3. Change Necessity

The gap analysis lists `set_header`, `set_footer`, and `set_page_breaks` as
missing. No configuration can emit these BIFF records; the record structs and
worksheet print state are the minimum boundary.

## 4. Aegis Visibility

The public `Worksheet` contract, error enum, and binary record catalogue
change. A durable plan records the record layouts, ABNF order, alias expansion,
validation, and default-compatibility boundary.

## 5. Owners And Files

- `rust-xlsb-core/src/prelude/structure/xl_wide_string.rs` — add
  `HeaderFooterString`, a nullable length-limited wide string.
- `rust-xlsb-core/src/prelude/structure/mod.rs`, `prelude/mod.rs` — export.
- `rust-xlsb-core/src/prelude/record/brt_begin_header_footer.rs` — new
  `BrtBeginHeaderFooter` and `BrtEndHeaderFooter`.
- `rust-xlsb-core/src/prelude/record/brt_page_breaks.rs` — new
  `BrtBeginRwBrk`, `BrtBeginColBrk`, `BrtBrk`, `BrtEndRwBrk`, `BrtEndColBrk`.
- `rust-xlsb-core/src/prelude/record/mod.rs` — register records.
- `rust-xlsb-writer/src/worksheet.rs` — header/footer and break state, setters,
  alias expansion, validation.
- `rust-xlsb-writer/src/error.rs` — `TooManyPageBreaks`.
- `rust-xlsb-writer/src/sheet.rs` — emit header/footer and break records.
- `rust-xlsb-writer/src/e2e/mod.rs` — calamine readback with every option.
- `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`, `docs/aegis/INDEX.md`.

## 6. Record Layouts

`BrtBeginHeaderFooter` (`2.4.313`):

- `grbit: u16` — bit 0 `fHFDiffOddEven`, bit 1 `fHFDiffFirst`, bit 2
  `fHFScaleWithDoc`, bit 3 `fHFAlignMargins`, bits 4..15 reserved zero.
- `stHeader`, `stFooter`, `stHeaderEven`, `stFooterEven`, `stHeaderFirst`,
  `stFooterFirst`: `HeaderFooterString` (nullable `XLNullableWideString`,
  maximum 255 UTF-16 code units).

`BrtBeginRwBrk` / `BrtBeginColBrk`:

- `ibrkMac: u32`, `ibrkManMac: u32`; both equal the number of `BrtBrk`
  records.

`BrtBrk`:

- `unRwCol: u32`, `unColRwStrt: u32`, `unColRwEnd: u32`, `fMan: u32`,
  `fPivot: u32` (20 bytes).

End markers are zero-byte scope records. Break-list records appear after
header/footer and before `BrtEndSheet`.

## 7. Compatibility Boundary

- `BrtBeginHeaderFooter` is emitted only when a header/footer string is set or
  either flag is disabled.
- A header/footer present but empty is distinct from absent; absent strings
  serialize as null.
- `BrtBeginRwBrk` and `BrtBeginColBrk` are emitted only for non-empty break
  lists.
- Existing sheet data, view, merge, style, and page-setup output is unchanged.

## 8. TDD Route

- Mode: auto
- Decision: strict
- Authority: new public API and seven binary records.
- Test posture: add failing record and writer tests first, then implement the
  minimum code to pass.
- Verification: unit tests, calamine readback, LibreOffice PDF page count and
  header/footer text, clippy, fmt.

## 9. Tasks

1. Add failing tests for `HeaderFooterString`: 255 code units accepted, 256
   rejected, null accepted.
2. Add failing tests for `BrtBeginHeaderFooter` (26 bytes with six null
   strings; `0x000C` default flags; text payload) and zero-byte end marker.
3. Add failing tests for the break records: begin records 8 bytes, `BrtBrk` 20
   bytes, end markers zero bytes.
4. Implement the records with spec field names; register ids, exports, and
   round-trip size cases.
5. Add failing worksheet tests for default output, header/footer aliases,
   255/256 length behavior, flag toggles, and preserved previous text on
   invalid input.
6. Add `HeaderFooter` state and setters; expand aliases in one pass.
7. Add failing break tests for sorting, deduplication, zero removal, limits,
   count cap, empty no-op, and all-zero clearing.
8. Add `Error::TooManyPageBreaks` and break setters that validate before
   replacing stored state.
9. Emit the header/footer record followed by row and column break lists after
   page setup.
10. Add calamine readback and LibreOffice PDF evidence.
11. Update the gap matrix, AGENTS counts, and plan status.

## 10. Risks And Verification

- Risk: `BrtBeginHeaderFooter` mixes a flag word with six variable strings;
  keep the flags as a `u16` and parse the strings directly to avoid deku order
  state leaking into `XlString`.
- Risk: Excel aliases expand to shorter control codes; length is checked after
  expansion, as the reference does.
- Risk: a zero row/column break is invalid; ignore zeros before the count
  check.
- Risk: all-zero input would leave an empty but present break list; clear the
  stored list explicitly.
- Verification: `cargo test --workspace`, clippy, fmt; decode package records
  and inspect fields; calamine readback; PDF page count and extracted
  header/footer text.

## 11. Execution Handoff

- Route: inline.
- User confirmation required: no.
- Status: implemented; reviewer found no defects.
- Verification: 158 core + 93 writer unit tests and 6 doctests; clippy and fmt
  clean; calamine readback. Real Excel fixtures decode through
  `BrtBeginHeaderFooter`: `Simple.xlsb` has six null strings and default flags
  `0x000c`; `testVarious.xlsb` has header/footer text and flags `0x000f`.
  LibreOffice renders a 60-row sheet with breaks at rows 20 and 40 as three
  PDF pages and prints `Report 1 of 3` through `Report 3 of 3` in the header.
  UTF-16 boundaries cover 255/256 code units, surrogate pairs, and a
  negative decode of a 256-unit string. Reviewer added no defects; only
  residual notes about escaped `&&[alias]` behavior matching the reference
  and the external Excel fixture not living in the tree.
