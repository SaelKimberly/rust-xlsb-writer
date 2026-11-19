# Custom theme support

- Status: Implemented — reviewed
- Date: 2026-11-19
- Scope: `Workbook::use_custom_theme`, theme XML extraction from raw XML,
  `.thmx`, and `.xlsx` files, and the existing theme package wiring.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.1 and §5.
- Reference: vendored `rust_xlsxwriter` v0.102.0 `workbook.rs`
  (`use_custom_theme`, `read_theme_from_path`).
- Spec: MS-XLSB §2.1.7.52 and §2.4.868 (`dwThemeVersion = 0` delegates the
  version to the Theme part).
- Baseline: `e4c218b docs: record array formula ABNF evidence`.

## Goal

Add the reference workbook entry point for user-supplied themes:

```rust
let mut workbook = Workbook::new();
workbook.use_custom_theme("corporate.thmx")?;
let worksheet = workbook.add_worksheet();
worksheet.write(0, 0, "Hello")?;
workbook.save("workbook.xlsb")?;
```

The call must:

- accept a raw theme XML file;
- accept a `.thmx` archive using `theme/theme/theme1.xml`;
- accept an `.xlsx`-style archive using `xl/theme/theme1.xml`;
- reject non-XML input, archives without a theme part, and theme XML that
  contains `<a:blipFill>`;
- emit the supplied bytes as `xl/theme/theme1.xml` with the existing theme
  content type and workbook relationship;
- emit `BrtWbProp` with `dwThemeVersion = 0` so Excel resolves the version from
  the theme part;
- leave default workbooks unchanged.

Out of scope: parsing theme colors into `Format`, theme color re-indexing,
importing a theme from an XLSB workbook, and `.thmx` archives that store the
theme under a non-standard path.

## Aegis Visibility

This is a public workbook API and a package producer contract. The most
visible failures are an invalid theme silently written, an XML file treated as
a ZIP, a `.thmx` read from the wrong path, or a custom theme carrying the
Excel 2023 version. Each is pinned by a test.

## Architecture

`rust-xlsb-writer`:

- Extend `src/theme.rs`:
  - `Theme::Custom(Box<str>)` alongside `Theme::Excel2023`.
  - `Theme::version()` returns `202_300` for Excel 2023 and `0` for custom.
  - `read_theme_from_path` reads the file once into bytes, tries
    `zip::ZipArchive` over a `Cursor`, then falls back to UTF-8 text.
  - ZIP lookup order is `theme/theme/theme1.xml` then
    `xl/theme/theme1.xml`.
- Add `zip = { version = "9.0", default-features = false,
  features = ["deflate-flate2-zlib-rs"] }`.
- `Workbook::use_custom_theme` validates the XML and stores the boxed string.
- `WorkbookSettings` becomes `Clone` instead of `Copy`.
- `package::build_package` and `workbook_part` borrow `&WorkbookSettings`;
  `content_types` takes `has_theme: bool` because it only branches on
  presence.

`package.rs` already owns the theme part, content type, relationship, and
`BrtWbProp`; only the theme XML source and the zero version are new.

## Wire Contract

| Piece | Value |
|---|---|
| Theme part | `xl/theme/theme1.xml` |
| Theme content type | `application/vnd.openxmlformats-officedocument.theme+xml` |
| Theme relationship | `http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme` |
| Relationship target | `theme/theme1.xml` |
| `BrtWbProp.dwThemeVersion` | `0` for custom themes |
| `.thmx` member | `theme/theme/theme1.xml` |
| `.xlsx` member | `xl/theme/theme1.xml` |

## Compatibility Boundary

- `Workbook::new()` and every workbook that never selects a theme keep the
  current parts and relationship ids.
- `use_custom_theme` does not change the default format or dimensions. It may
  be called before or after adding worksheets, matching the reference.
- Custom theme XML is stored verbatim; no normalization or reflow.
- `use_custom_theme` replaces an earlier custom or Excel 2023 theme.

## Change Necessity

The API is absent, and the existing `Theme` enum only stores a `'static`
asset. Custom themes need one owned XML payload and one path reader. ZIP
reading is new because the writer previously only produced ZIP files; the
`zip` crate is the reference implementation's reader and avoids a
hand-rolled central-directory parser.

## TDD Route

- Mode: auto.
- Decision: strict.
- Authority: new public API, file-format reader, and package producer.
- Test posture: add failing API/reader/package tests first, then implement.
- Reason: a wrong archive path, stale theme version, or missing blip-fill
  rejection is consumer-visible.

## Tasks

1. **Reader and API**
   - Files: `Cargo.toml`, `src/theme.rs`, `src/workbook.rs`, `src/error.rs`.
   - Add `Error::ThemeError`, `Theme::Custom`, `read_theme_from_path`, and
     `Workbook::use_custom_theme`.
   - Tests: raw XML, `.xlsx` ZIP, `.thmx` ZIP precedence, missing theme part,
     invalid XML, and `<a:blipFill>` rejection.
   - Verify: `cargo test -p rust-xlsb-writer theme`.

2. **Package wiring**
   - Files: `src/package.rs`, `src/workbook.rs`.
   - Borrow settings through `build_package`/`workbook_part`, pass theme
     presence to `content_types`, and emit `dwThemeVersion = 0` for custom.
   - Tests: custom part bytes equal the input, theme relationship and content
     type present, `BrtWbProp` present with version 0; default output remains
     theme-free.
   - Verify: targeted package tests.

3. **Regression sweep and docs**
   - Update `README.md` if it claims only the 2023 theme, `AGENTS.md` counts,
     the gap analysis status, and `docs/aegis/INDEX.md`.
   - Run the full verification set.

## Verification

- `cargo test --workspace`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `git diff --check`
- LibreOffice import of a workbook generated with a custom theme, if the
  theme survives an XLSX round-trip; otherwise report the package-level proof
  and the visual limit.

## Risks

- A raw theme XML file and a ZIP file can both be valid inputs; the reader
  tries ZIP first and falls back to text, matching the reference.
- `zip` 9.x adds a runtime dependency; it is limited to deflate/store reads and
  uses the same `zlib-rs` backend as the existing writer.
- Excel may reject structurally invalid theme XML that still starts with
  `<?xml`. The writer preserves bytes and does not act as a schema validator.
- LibreOffice may normalize or drop custom theme parts on XLSB import; the
  package contract remains the authoritative check.

## Review

`CustomThemeReview` found no blocking defects. It independently verified:

- reader parity with the vendored reference: ZIP first, then UTF-8 text, and
  the same `.thmx`/`.xlsx` lookup order;
- real `Technic.thmx`, `technic.xlsx`, and `technic.xml` fixtures produce a
  byte-identical emitted theme part (7,699 bytes, SHA-256 match);
- `civic.xml` is rejected for image fill and `empty.xml` for non-XML;
- theme part, content type, `rId4` relationship, and
  `BrtWbProp.dwThemeVersion = 0` in a generated XLSB; default workbooks still
  omit all three;
- MS-XLSB §2.4.868 defines zero as “version specified by the Theme part”;
- LibreOffice imports a custom-theme XLSB without repair, then drops the theme
  on XLSX export, as predicted.

Remaining uncertainty: Excel-specific acceptance of `dwThemeVersion = 0` with
a verbatim custom theme cannot be checked locally. The spec and reference
behavior support it.
