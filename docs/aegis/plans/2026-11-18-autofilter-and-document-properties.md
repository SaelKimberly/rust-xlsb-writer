# Autofilter ranges and workbook document properties

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: `Worksheet::autofilter`, `Workbook::set_properties`,
  `DocProperties`, `BrtBeginAFilter`, `BrtEndAFilter`, `docProps/core.xml`, and
  `docProps/app.xml`
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` sections 3.1, 3.4, 3.8, P1, and
  P4; vendored `rust_xlsxwriter` v0.102.0 `Worksheet::autofilter`,
  `Workbook::set_properties`, `DocProperties`, `Core`, and `App`.

## 1. Goal

Close two independent gaps with reference signatures:

```rust
impl Worksheet {
    pub fn autofilter(
        &mut self,
        first_row: RowNum,
        first_col: ColNum,
        last_row: RowNum,
        last_col: ColNum,
    ) -> Result<&mut Worksheet>;
}

impl Workbook {
    pub fn set_properties(&mut self, properties: &DocProperties) -> &mut Workbook;
}

impl DocProperties {
    pub fn new() -> DocProperties;
    pub fn set_title(self, title: impl Into<String>) -> DocProperties;
    pub fn set_subject(self, subject: impl Into<String>) -> DocProperties;
    pub fn set_manager(self, manager: impl Into<String>) -> DocProperties;
    pub fn set_company(self, company: impl Into<String>) -> DocProperties;
    pub fn set_category(self, category: impl Into<String>) -> DocProperties;
    pub fn set_author(self, author: impl Into<String>) -> DocProperties;
    pub fn set_keywords(self, keywords: impl Into<String>) -> DocProperties;
    pub fn set_comment(self, comment: impl Into<String>) -> DocProperties;
    pub fn set_status(self, status: impl Into<String>) -> DocProperties;
    pub fn set_hyperlink_base(self, hyperlink_base: impl Into<String>) -> DocProperties;
}
```

Uncalled methods add no records or package parts, so existing output is
unchanged.

## 2. Architecture

`Worksheet` stores `autofilter: Option<UncheckedRfX>`. Calling `autofilter`
checks both corners against the row/column limits, rejects reversed ranges with
`Error::RowColumnOrder`, and replaces any previous range. Excel permits one
worksheet autofilter, matching the reference overwrite behavior. No filter
criteria are stored, so the record is a pure range declaration.

`Workbook` stores `doc_properties: Option<DocProperties>`. `set_properties`
clones the value once; building a package only borrows it. `build_package`
takes `Option<&DocProperties>` and adds `docProps/core.xml`,
`docProps/app.xml`, content-type overrides, and root relationships only when a
value is present.

`DocProperties` stores ten optional strings. It does not allocate when read;
the one clone in `set_properties` matches the reference's borrowed parameter.
XML escaping writes through `Cow` from `escape_xml`, so clean values do not
allocate an extra string.

## 3. Records and parts

### `BrtBeginAFilter` (MS-XLSB 2.4.8)

- `rfx: UncheckedRfX`, exactly 16 bytes.
- Marks the start of the worksheet autofilter collection.
- `write_defined_names` adds the hidden, sheet-scoped
  `_xlnm._FilterDatabase` name with the same 3D area formula as the range, so
  Excel recognizes the filter range.
- Emitted after sheet protection and unprotected ranges, before merge records,
  followed immediately by `BrtEndAFilter` because filter-column records are not
  implemented yet. This matches the Worksheet ABNF and SheetJS record order.

### `BrtEndAFilter` (MS-XLSB 2.4.365)

- Zero-byte scope marker.
- Added as its own record module rather than through the generic scope macro,
  because `BrtBeginAFilter` carries a range and cannot be the empty marker.

### `docProps/core.xml`

Uses the reference element order: `dc:title`, `dc:subject`, `dc:creator`,
`cp:keywords`, `dc:description`, `cp:lastModifiedBy`, `cp:category`, and
`cp:contentStatus`. Empty optional fields are omitted except creator and
last-modified-by, which the reference always emits.

### `docProps/app.xml`

Uses the reference element order: `Application`, `DocSecurity`, `ScaleCrop`,
`HeadingPairs`, `TitlesOfParts`, optional `Manager`, `Company`,
`LinksUpToDate`, `SharedDoc`, optional `HyperlinkBase`, `HyperlinksChanged`, and
`AppVersion`. The worksheet heading pair and part titles come from the same
sheet slice that is already being packaged; very-hidden sheets are excluded.

Content types add:

- `/docProps/core.xml` — `application/vnd.openxmlformats-package.core-properties+xml`
- `/docProps/app.xml` — `application/vnd.openxmlformats-officedocument.extended-properties+xml`

Root relationships add `metadata/core-properties` and
`extended-properties`.

## 4. Compatibility Boundary

- `autofilter` matches the reference signature, bounds checks, order check, and
  overwrite behavior. `filter_column` remains the next P1 item.
- `DocProperties` supports the ten string properties. Custom properties and
  explicit creation timestamps are not exposed yet; omitting
  `set_properties` keeps `docProps` absent.
- The reference always emits docProps on save. This writer emits them only when
  requested, preserving existing packages.
- No docProps reader is added; the holder is write-session state.

## 5. Change Necessity

`Worksheet` has no autofilter state, and core has neither autofilter record.
`Workbook` has no metadata state, and packaging has no docProps parts or root
relationships for them. Both additions reuse existing range, XML-escaping, ZIP,
and package helpers.

## 6. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
record round-trip and exact range bytes, worksheet API bounds/order/overwrite,
package-level record decode, absent parts by default, content-type and
relationship checks, XML escaping, sheet-name/count metadata, and LibreOffice
XLSX round-trip for the autofilter and document properties.

## 7. Tasks

1. Core records
   - Add `BrtBeginAFilter` with `rfx: UncheckedRfX` and `BrtEndAFilter`.
   - Register both in `prelude` and the record-table tests.
2. Worksheet API
   - Add autofilter state, accessor, validation, overwrite behavior, and tests.
   - Serialize the autofilter after protection and before merges.
   - Emit the hidden, scoped `_xlnm._FilterDatabase` defined name.
3. Workbook metadata
   - Add `DocProperties` with the ten string setters.
   - Add `Workbook::set_properties` and package plumbing.
   - Add core/app XML, content types, and root relationships.
4. Tests
   - Exact autofilter record bytes, package decode, and defined-name formula.
   - Bounds, reversed range, and overwrite tests.
   - Absent autofilter/docProps by default.
   - DocProps part presence, XML escaping, sheet counts/names, and root rels.
5. Docs
   - Mark autofilter and core document properties supported in the gap analysis.
   - Update `AGENTS.md` API inventory and test counts.
   - Add this plan to `docs/aegis/INDEX.md`.

## 8. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- LibreOffice XLSB → XLSX conversion exposes `<autoFilter>` and the expected
  core/app property values.

Result: 192 core + 165 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. LibreOffice preserves the core/app document
properties but drops the worksheet `AutoFilter` on XLSB import, so the
autofilter range and scoped `_xlnm._FilterDatabase` name are proven by
package-level record decode and an independent SheetJS read, which reports
`!autofilter.ref == "A1:B4"` and the filter name `Sheet1!$A$1:$B$4`; the
defined-name formula is asserted against the 3D area encoder. SheetJS also
reads the title, author, company, comment, status, and subject properties.
Document-property and sheet-name control characters use Excel's `_xHHHH_`
escaping and are covered by package tests.

## 9. Risks

- No Excel fixture is available; autofilter placement is proven by the ABNF,
  package decode, and LibreOffice round-trip.
- The reference emits docProps unconditionally and always includes a current
  creation timestamp. This writer omits both until requested so output stays
  deterministic.
- Custom document properties and explicit creation time remain unsupported.
- Filter criteria (`filter_column`) are not implemented; the autofilter range
  alone provides the dropdowns but does not hide rows.
