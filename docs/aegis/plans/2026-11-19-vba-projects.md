# VBA projects and code names

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: none

## Goal

Close the basic macro-enabled-workbook gap:

- `Workbook::add_vba_project(path)` reads a `vbaProject.bin` and writes it as
  `xl/vbaProject.bin`.
- The workbook relationship uses the VBA project type and the project part
  gets the matching content type.
- `Workbook::set_vba_name(name)` sets `BrtWbProp.strName`.
- `Worksheet::set_vba_name(name)` sets `BrtWsProp.strName`.
- `add_vba_project` defaults the workbook code name to `ThisWorkbook` when no
  name was set, matching the reference.

Digitally signed VBA projects remain out of scope rather than stubbed.

## Aegis Visibility

This adds one binary package part plus two workbook relationships and changes
two existing BIFF property owners: `BrtWbProp` (workbook part) and `BrtWsProp`
(worksheet part). The package builder gains a borrowed project slice; the
worksheet gains one optional code name.

## Baseline / Authority

- [MS-XLSB] Comments are unrelated. The VBA project is a binary part referenced
  from `xl/_rels/workbook.bin.rels` with
  `http://schemas.microsoft.com/office/2006/relationships/vbaProject`.
- The [Content_Types] override for `/xl/vbaProject.bin` is
  `application/vnd.ms-office.vbaProject`.
- `BrtWbProp` carries `dwThemeVersion` and `strName` (`CodeName`, maximum 31
  UTF-16 code units).
- `BrtWsProp` carries `strName` as the worksheet VBA code name.
- `rust_xlsxwriter` v0.102.0 `add_vba_project` accepts a path, defaults the
  workbook code name to `ThisWorkbook`, and `set_vba_name` enforces:
  non-empty, at most 31 characters, word characters only, and a leading letter.

## Compatibility Boundary

- `add_vba_project` and both `set_vba_name` methods mirror the reference names
  and builder shape.
- The project bytes are stored once in a boxed slice and copied only into the
  ZIP part list, which already owns every part as `Vec<u8>`.
- No `.xlsm`-style workbook content type changes are needed: the XLSB workbook
  content type is already `macroEnabled.main`.
- `add_vba_project_with_signature` and `vbaProjectSignature.bin` remain absent.
- VBA code names do not affect sheet lookup, defined names, or formula parsing.
- Calling `add_vba_project` twice replaces the project bytes but keeps an
  explicitly set workbook code name.

## Change Necessity

The package builder cannot emit a VBA project, and neither the workbook nor the
worksheet can carry a code name. Adding only the binary part would produce a
macro-enabled workbook whose VBA object names are empty, so both property
records must learn to emit the name.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: package assertions for the project bytes, content type,
  workbook relationship, `BrtWbProp.strName`, and `BrtWsProp.strName`; name
  validation tests; external package comparison against an Aspose-generated
  macro XLSB and an Aspose load/save round trip.

## Tasks

1. **Workbook state and API.**
   - Add `vba_project: Option<Box<[u8]>>` and `vba_codename` to
     `WorkbookSettings`.
   - Add `add_vba_project` and `set_vba_name` with the reference validation.
   - Default the code name in `add_vba_project`.
2. **Worksheet code name.**
   - Add `vba_codename` to `Worksheet` and `set_vba_name`.
   - Fill unnamed code names as `SheetN` during save when a project exists.
   - Emit `BrtWsProp` when only a code name is present, and set `strName`.
3. **Package.**
   - Extend `build_package` with a borrowed project slice.
   - Emit `xl/vbaProject.bin`, the content-type override, and the workbook
     relationship.
   - Emit `BrtWbProp` when either a theme or a code name is present.
4. **Tests.**
   - Assert project bytes, override, relationship, and both code names.
   - Assert invalid VBA names are rejected.
   - Assert a workbook without a project has no VBA part or relationship.
5. **Docs and verification.**
   - Update the gap analysis, `AGENTS.md`, and `docs/aegis/INDEX.md`.
   - Run fmt, clippy, default and all-feature tests.
   - Compare the package shape with an Aspose-generated macro XLSB, then load
     and re-save with Aspose to confirm the project survives.

## Review

`VbaReview` found one P2 and one P3.

- P2: `add_vba_project` defaulted only the workbook code name, so unnamed
  worksheets lacked the `SheetN` code names the reference assigns to VBA
  document modules. `Workbook::prepare` now fills every unnamed worksheet code
  name after adding a project, independent of the visible sheet name. A test
  covers two sheets, one renamed `Data`, with code names `Sheet1` and `Sheet2`.
- P3: the no-VBA test checked the content type and ZIP part but not the
  workbook relationship. It now asserts the relationship is absent too.

The reviewer found the package bytes, content type, relationship type and ID
allocation, theme-only `BrtWbProp` serialization, and name validation correct.

## Verification

- `cargo test --workspace`: 324 core + 442 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 324 + 446, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells loads the generated macro XLSB, reports the VBA project and
  `Sheet1` code name for a sheet named `Data`, and preserves the project on
  re-save.
- Package tests assert the project bytes, content-type override, workbook
  relationship, explicit and default code names, invalid names, and the absence
  of VBA parts and relationships without a project.

## Review Focus

- Content-type override and relationship type/order.
- `BrtWbProp` still byte-identical for theme-only workbooks.
- `BrtWsProp` emission for code-name-only worksheets.
- Code-name validation and UTF-16 length limit.
- No extra project-byte copies at construction or save.
