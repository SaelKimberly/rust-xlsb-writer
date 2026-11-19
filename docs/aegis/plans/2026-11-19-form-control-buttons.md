# Form-control buttons

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-notes.md`

## Goal

Close the basic form-control button gap:

- `Button::new`, `set_caption`, `set_macro`, `set_alt_text`, `set_width`,
  `set_height`.
- `Worksheet::insert_button(row, col, &Button)`.
- `Worksheet::insert_button_with_offset(row, col, &Button, x, y)`.
- One per-sheet VML drawing containing button shapes, linked with
  `BrtLegacyDrawing`.
- Buttons and notes share that VML part when both are present.

Shapes, checkboxes, chartsheets, and richer control properties stay out of
scope.

## Aegis Visibility

Adds one public builder, two worksheet methods, a button map, one VML shape type
and shape sequence, one worksheet relationship, and no new binary record types
or content types. The existing note VML path is widened to emit buttons first,
then notes.

## Baseline / Authority

- A form-control button is a VML shape with `_x0000_t201` and
  `x:ClientData ObjectType="Button"`.
- The shape uses `o:button="t"`, a `buttonFace [67]` fill, centered Calibri
  220 text, and an anchor from the insertion cell to the button extent.
- `x:FmlaMacro` stores `[0]!macro`. An empty macro becomes
  `[0]!ButtonN_Click`; an empty caption becomes `Button N`.
- Buttons and notes share one VML part per worksheet. Button shapes are written
  first; note shapes continue the same 1024-id block sequence.
- VML parts use the existing default content type
  `application/vnd.openxmlformats-officedocument.vmlDrawing` and worksheet
  relationship type
  `http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing`.

## Compatibility Boundary

- `Button` mirrors the reference caption, macro, alt-text, width, and height
  builder names. Object movement remains out of scope.
- Captions, macros, and alt text are capped at 255 UTF-16 code units; overlong
  input leaves the previous value unchanged. Zero width/height is ignored.
- `insert_button_with_offset` keys buttons by cell and offsets, so two buttons
  can share a cell; a second insertion at the same key replaces the first.
- Button text, macro text, and alt text are XML-escaped.
- A sheet with notes still emits `commentsN.bin`; a button-only sheet does not.
- Header/footer images keep their separate VML part and `BrtLegacyDrawingHF`
  record.

## Change Necessity

The writer can emit notes and header images through VML but has no button model,
button VML shape, or button insertion API. Boolean cells are not form controls,
so no existing cell path can represent a button.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: builder limits; VML shapetype, shape ids, anchors, offsets,
  captions, macros, and alt text; row/column validation; package part,
  relationship, and `BrtLegacyDrawing`; combined notes and buttons; Aspose
  shape read-back.

## Tasks

1. **Button model.**
   - Add `Button` with the reference defaults, builder methods, and limits.
   - Re-export it from the crate root.
2. **Worksheet API.**
   - Store buttons in a deterministic map keyed by cell and offsets.
   - Add the two insertion methods and row/column validation.
3. **VML.**
   - Emit the button shapetype and button shapes before notes.
   - Continue shape ids and z-index across buttons and notes.
   - Reuse the existing anchor and DPI helpers.
4. **Package.**
   - Create a VML part when either buttons or notes exist.
   - Add the worksheet relationship and `BrtLegacyDrawing` for either.
   - Keep the comments part notes-only.
5. **Tests and docs.**
   - Add builder, VML, worksheet, and package tests.
   - Verify with Aspose shape read-back and LibreOffice conversion.
   - Update `AGENTS.md`, the gap analysis, and `docs/aegis/INDEX.md`.

## Verification

- `cargo test --workspace`: 332 core + 463 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 332 + 467, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- A generated workbook with two buttons and a note loads in Aspose.Cells as
  three shapes: one note and two form-control buttons with the expected
  captions, macros, alt text, and cells.
- LibreOffice opens the generated XLSB and exports a PDF without error. Its PDF
  text export does not include form-control captions, so Aspose is the
  shape-level oracle for this slice.

## Review

`ButtonReview` compared the implementation with
`thirdparty/rust_xlsxwriter/src/{button,vml}.rs` and found no P0-P3 findings.
It confirmed the builder limits, VML shapetype and child order, anchors,
shape/idmap progression, package relationships, and `BrtLegacyDrawing` wiring.
The only observations were non-blocking: integer-pixel anchor truncation already
present in notes, an approximate Worksheet method count, and a stale
`note_vml` parameter name. The parameter was renamed to `legacy_vml`; the rest
needs no change.
