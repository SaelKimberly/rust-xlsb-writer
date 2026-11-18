# Hyperlinks

- Status: Implemented — reviewer found and fixed internal relId
- Date: 2026-11-18
- Scope: `Url`, `Worksheet::write_url` and variants, `BrtHLink`, worksheet
  relationships
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` sections 3.2, 3.7, 5, and 8;
  `rust_xlsxwriter` `write_url`, `write_url_with_text`,
  `write_url_with_format`, `write_url_with_options`, and `Url`

## 1. Goal

Add the smallest complete hyperlink path:

- `Url::new`, `set_text`, `set_tip`
- `Worksheet::write_url`
- `Worksheet::write_url_with_text`
- `Worksheet::write_url_with_format`
- `Worksheet::write_url_with_options`
- `Format::set_hyperlink`

Links must survive as clicable targets in Excel/LibreOffice and the cell must
still read as a string through calamine. A workbook with no links stays
byte-identical.

## 2. Architecture

A hyperlink has two outputs:

1. The visible cell string, stored through the existing shared-string path.
2. A `BrtHLink` record in the worksheet part.

External links also need one relationship per hyperlink in
`xl/worksheets/_rels/sheetN.bin.rels`, with
`Type=http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink`.
Internal links use an empty `relId` and put the sheet/cell fragment in
`location`, exactly as MS-XLSB section 2.4.693 requires.

`BrtHLink` sits after `BrtEndSheetData`/`BrtEndMergeCells` and before
`BrtPrintOptions`, matching the worksheet ABNF. There is no begin/end wrapper.

## 3. Core Record

Add `prelude/record/brt_hlink.rs`:

```rust
pub struct BrtHLink {
    pub rfx: UncheckedRfX,
    pub relId: XlNullableWideString,
    pub location: XlWideString,
    pub tooltip: XlWideString,
    pub display: XlWideString,
}
```

The record is `Unchecked`; worksheet validation owns the range. Default size is
32 bytes: 16-byte range plus four empty strings. A round-trip test pins it.

## 4. URL Model

Add `rust-xlsb-writer/src/url.rs`:

```rust
pub struct Url {
    link: String,          // original input, used for errors and default text
    text: String,
    tooltip: String,
    target: String,        // relationship target, empty for internal links
    location: String,      // fragment or sheet location
    link_type: HyperlinkType,
}
```

`HyperlinkType` is `Url`, `File`, `Internal`. Parsing follows the reference:

- `internal:` selects `Internal`; location is the remainder; default text is
  the location.
- `mailto:` selects `Url`; default text drops the `mailto:` prefix.
- `file://` selects `File`; a relative path drops the scheme from the target;
  default text is the path.
- Any other link containing `://` selects `Url`.
- Anything else is `Error::UnknownUrlType`.

Split `#fragment` for URL and file links. Escape the relationship target with
the reference URL escape set (`%`, space, quotes, angle brackets, brackets,
caret, backtick, braces). Do not escape internal locations.

Limits: link and location 2080 UTF-16 units, tooltip 255. Text uses the
existing 32,767-unit string limit. Invalid input fails before any cell or
hyperlink state changes.

`Url` owns strings because it is stored per hyperlink. Parsing borrows from the
stored original where possible and allocates only escaped/derived fields.

## 5. Worksheet State

Add one sparse field:

```rust
hyperlinks: BTreeMap<(RowNum, ColNum), Url>,
```

`write_url` writes the cell text first, then stores the link, so a failed write
leaves neither. With no explicit format, use `Format::new().set_hyperlink()`
(blue `#0563C1`, single underline, no theme scheme). `write_url_with_options`
accepts optional text, tooltip, and format like the reference.

## 6. Serialization

Worksheet part:

- For each hyperlink in `(row, col)` order, emit a single-cell `BrtHLink`.
- External links get `relId = "rIdN"` with `N` assigned from 1 in the same
  order; internal links get an empty `relId`, not a null one (2.4.693 forbids
  null).
- `location` carries the fragment; `tooltip` and `display` carry the tooltip
  and visible text.

Package:

- Emit `xl/worksheets/_rels/sheetN.bin.rels` only when a sheet has external
  links.
- Relationship IDs match the `BrtHLink` order.
- XML-escape relationship targets, including `&`, `"`, `<`, and `>`.

No content-type override is needed because the package already declares the
`rels` default extension.

## 7. Verification

Unit tests:

- `Url` parsing for https with fragment, mailto, internal, file, unknown, and
  tooltip/length rejection.
- `BrtHLink` round trip and default size.
- `write_url` stores the default text and format; text/format/tooltip variants
  override it.
- invalid row/column leaves no cell and no hyperlink.
- internal links omit relationships; external links get one per link.
- `BrtHLink` order and rel IDs match; record bytes carry the expected range,
  rel ID, location, and tooltip.
- no-link output stays unchanged.

E2E:

- Calamine reads the display strings.
- LibreOffice opens the XLSB and converts to XLSX with hyperlink
  relationships intact; inspect the converted package for targets.

Commands: `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
`cargo fmt --all --check`, `git diff --check`.

Result: 173 core + 129 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. LibreOffice converts the showcase XLSB to XLSX
with the external relationship and the internal `location` intact; it emits
`<hyperlink ref="B12" r:id="rId1" display="Rust language"/>` and
`<hyperlink ref="B13" location="Showcase!B2" display="Jump to B2"/>`.

## 8. Compatibility And Risks

- `write` does not accept `Url`; `write_url` is the supported path. Adding a
  `CellData` link variant would duplicate ownership of the cell.
- Overwriting a linked cell with another value leaves the hyperlink record.
  The reference also retains the stored hyperlink map.
- Complex URL normalization (Windows shares, percent-escaped anchors) is
  limited to the reference's common cases.
- Hyperlink cell style uses RGB blue, not the theme-10 hyperlink color, because
  this writer's theme color range stops at 9.

## 9. Retirement

None; additive API and records.
