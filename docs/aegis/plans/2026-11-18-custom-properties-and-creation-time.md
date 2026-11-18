# Custom document properties and creation time

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: `DocProperties::set_custom_property`,
  `DocProperties::set_creation_datetime`, `IntoCustomProperty`,
  `IntoCustomDateTime`, `docProps/custom.xml`, and core creation timestamps
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` sections 3.1, 3.8, P4, and the
  vendored `rust_xlsxwriter` v0.102.0 `properties.rs`, `custom.rs`, and
  `core.rs`

## 1. Goal

Finish the document-properties gap left by the previous batch:

```rust
impl DocProperties {
    pub fn set_creation_datetime(self, create_time: impl IntoCustomDateTime) -> DocProperties;
    pub fn set_custom_property(
        self,
        name: impl Into<String>,
        value: impl IntoCustomProperty,
    ) -> DocProperties;
}
```

Supported custom values are `&str`, `String`, `&String`, `i32`, `f64`, `bool`,
`&chrono::DateTime<Utc>`, `&chrono::NaiveDateTime`, and `&chrono::NaiveDate`.
`set_creation_datetime` accepts the same chrono date types. Both traits are
public for source compatibility with the reference API.

Uncalled methods add no XML elements or package parts, so existing output stays
unchanged.

## 2. Architecture

`DocProperties` gains:

- `creation_time: Option<String>` — already formatted as Excel's UTC W3CDTF
  string.
- `custom_properties: Vec<CustomProperty>` — insertion order, one typed value.

`CustomProperty` is a public opaque struct with private fields; a private enum
holds `Text(String)`, `Int(i32)`, `Real(f64)`, `Bool(bool)`, or
`DateTime(String)`. The value is converted exactly once when the setter runs.
No extra allocation is added to string values: `IntoCustomProperty` consumes the
supplied `String` where possible.

`package.rs` gains one `custom_xml` writer and one boolean indicating whether
custom properties exist. `build_package` appends `docProps/custom.xml` only
when the vector is non-empty. Content types and `_rels/.rels` gain the custom
part and relationship only in that case.

`core_xml` emits `dcterms:created` and `dcterms:modified` with
`xsi:type="dcterms:W3CDTF"` only when `creation_time` is set. Both use the same
timestamp, matching the reference. Omitting the setter keeps the existing
deterministic core XML.

## 3. XML and parts

`docProps/custom.xml` uses the reference shape:

```xml
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/custom-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <property fmtid="{D5CDD505-2E9C-101B-9397-08002B2CF9AE}" pid="2" name="...">
    <vt:lpwstr>...</vt:lpwstr>
  </property>
</Properties>
```

PIDs start at 2 and follow insertion order. Value element mapping:

| Value | Element |
|---|---|
| text | `vt:lpwstr` |
| i32 | `vt:i4` |
| f64 | `vt:r8` |
| bool | `vt:bool` |
| datetime | `vt:filetime` |

Property names are XML attributes and use the attribute escaper. Values use
`escape_xml_data`, so control characters and non-characters follow the existing
`_xHHHH_` rule.

Content type:
`application/vnd.openxmlformats-officedocument.custom-properties+xml`.
Root relationship type:
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties`,
target `docProps/custom.xml`.

## 4. Compatibility Boundary

- API signatures match the reference for the listed types.
- No ExcelDateTime, jiff, or owned-value chrono implementations are added.
- The reference always writes core timestamps and custom parts; this writer
  writes timestamps only when set and `custom.xml` only when non-empty.
- No custom-property reader is added; properties remain write-session state.

## 5. Change Necessity

The existing `DocProperties` has no custom-value storage and `core_xml` has no
timestamp elements. Packaging has no custom part, content type, or root
relationship. The change reuses the existing XML escaping and store-only ZIP
assembly; no new dependency or owner is needed.

## 6. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
unit tests for value conversion, package-level XML assertions for every custom
type and PID order, attribute/data escaping, custom part presence only when
used, creation timestamps present/absent, content type and relationship checks,
plus an independent reader check.

## 7. Tasks

1. Properties API
   - Add `CustomProperty`, `CustomPropertyValue`, `IntoCustomProperty`, and
     `IntoCustomDateTime` in `properties.rs`.
   - Add the two setters and update derives on `DocProperties`.
   - Export both traits and `CustomProperty` from `lib.rs`.
2. Package plumbing
   - Add `has_custom_properties` to content types and root relationships.
   - Emit `docProps/custom.xml` only for non-empty custom properties.
   - Emit creation/modified elements only when `creation_time` is set.
3. XML escaping
   - Extend the attribute escaper for newline and control characters so
     user-provided property names cannot corrupt XML.
4. Tests
   - String, i32, f64, bool, and datetime custom values; PID order; name and
     text escaping; content type and relationship.
   - No custom part by default and when only core properties are set.
   - Creation time in both core elements; no timestamps by default.
5. Docs
   - Mark custom properties and creation time supported or partially in the gap
     analysis; update counts and `AGENTS.md`.
   - Add this plan to `docs/aegis/INDEX.md`.

## 8. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- Independent readback of `docProps/custom.xml` and `docProps/core.xml`.

Result: 192 core + 172 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. Python `zipfile` and ElementTree parsed every
generated property part, and LibreOffice converted the XLSB to XLSX with the
core, app, and custom properties intact. Reviewer findings fixed: non-finite
`f64` values now use `NaN`/`INF`/`-INF`, and custom property names use a
dedicated attribute escaper so hyperlink target escaping is unchanged.

## 9. Risks

- No Excel fixture is available; the XML schema and relationship types are
  asserted directly and checked with an independent XML reader.
- Reference defaults (current creation time, unconditional custom part) are
  intentionally not copied to preserve deterministic default output.
- f64 custom values use Rust's `to_string`, matching the reference; non-finite
  values are not rejected.
