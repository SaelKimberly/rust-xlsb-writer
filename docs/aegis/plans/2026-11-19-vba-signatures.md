# VBA project signatures

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-vba-projects.md`

## Goal

- Add `Workbook::add_vba_project_with_signature(project, signature)`.
- Emit `xl/vbaProjectSignature.bin` with content type
  `application/vnd.ms-office.vbaProjectSignature`.
- Emit `xl/_rels/vbaProject.bin.rels` with one relationship:
  `rId1` of type
  `http://schemas.microsoft.com/office/2006/relationships/vbaProjectSignature`
  targeting `vbaProjectSignature.bin`.

Signing or validating the signature is out of scope; the bytes are packaged
verbatim.

## Aegis Visibility

This adds one workbook field, one package part, one relationship part, and one
content-type override. It reuses the project part and code-name preparation
from the previous slice. No worksheet or BIFF records change.

## Baseline / Authority

- `rust_xlsxwriter` v0.102.0 `add_vba_project_with_signature(project,
  signature)` adds the project first, then reads the signature bytes.
- The signature part is an override, not a default extension:
  `application/vnd.ms-office.vbaProjectSignature`.
- `vbaProject.bin.rels` owns the relationship from the project to its
  signature. The project is not a relationship to itself. The vendored
  packager writes only the signature relationship.
- The signature is written only when a VBA project is present.

## Compatibility Boundary

- The method mirrors the reference name, argument order, and builder shape.
- Signature bytes are stored once in a boxed slice and copied once into the
  owned ZIP part list.
- Calling `add_vba_project_with_signature` twice replaces both binaries.
- Calling `add_vba_project` alone leaves any previously supplied signature in
  place, matching the reference's independent fields.
- No signature verification, OLE parsing, or `.xlsm` workbook content type is
  added.

## Change Necessity

The existing package cannot attach the detached signature that Excel expects
for a signed VBA project. Without the relationship part, the signature file
would be orphaned.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: package assertions for both binary parts, the signature
  content type, the project relationship part, and the absence of signature
  parts for unsigned workbooks; Aspose read-back.

## Tasks

1. **Workbook state and API.**
   - Add `vba_signature: Option<Box<[u8]>>`.
   - Add `add_vba_project_with_signature`, delegating the project to
     `add_vba_project`.
2. **Package.**
   - Extend `build_package` with a borrowed signature slice.
   - Emit the signature part and override.
   - Emit `xl/_rels/vbaProject.bin.rels` only when a signature is present.
3. **Tests.**
   - Assert project and signature bytes, override, and relationship XML.
   - Assert an unsigned project has neither signature part nor relationship
     part.
4. **Docs and verification.**
   - Update the gap analysis, `AGENTS.md`, and `docs/aegis/INDEX.md`.
   - Run fmt, clippy, default and all-feature tests.
   - Load a signed generated XLSB with Aspose.Cells and confirm it opens and
     preserves the macro project.

## Review

`VbaSignatureReview` found one P2 and one P3.

- P2: a zero-byte signature file emitted a zero-byte signature part, its
  relationship, and its content-type override. The reference gates on
  non-empty bytes. The package now emits all three only when the signature is
  non-empty; a test covers an empty signature file.
- P3: the gap analysis said Aspose preserved both binaries on re-save. Aspose
  preserves the signature bytes but re-serializes the project. The evidence
  text now says that.

The reviewer confirmed the relationship part name, `rId1`, type, target, XML
shape, content type, API argument order, and package ordering match the
vendored reference.

## Verification

- `cargo test --workspace`: 324 core + 444 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 324 + 448, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells loads the signed generated XLSB, reports the VBA project and
  `Sheet1` code name, keeps the signature bytes on re-save, and re-serializes
  the project.
- Package tests assert both binaries, the signature content type, the
  one-relationship project relationship part, and the absence of signature
  parts and override for an unsigned or empty-signature project.

## Review Focus

- Relationship part name, type, and target.
- Content-type override only for a present signature.
- No signature without a project.
- No extra copies or broad OLE/signature parsing.
