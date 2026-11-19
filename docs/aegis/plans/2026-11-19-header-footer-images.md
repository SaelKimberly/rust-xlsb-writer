# Header and footer images

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-foreground-image-scaling.md`

## Goal

Close the header/footer image gap:

- `HeaderImagePosition` (`Left`, `Center`, `Right`).
- `Worksheet::set_header_image(&Image, position)`.
- `Worksheet::set_footer_image(&Image, position)`.
- A `&G` or `&[Picture]` placeholder is required in the matching header/footer section.
- Header and footer images are stored in a per-sheet VML drawing part and linked from the worksheet with `BrtLegacyDrawingHF`.

Images are not resampled. `Image` scale and DPI settings determine the VML shape size.

## Aegis Visibility

This adds a public position enum and two worksheet methods, two worksheet image
slots, one core record (`BrtLegacyDrawingHF`), one VML part and relationship
part per image-bearing sheet, one worksheet relationship, and one content-type
default. The package already owns media parts, image types, and worksheet
relationship IDs.

## Baseline / Authority

- [MS-XLSB] header/footer image VML is linked with `BrtLegacyDrawingHF`
  (record 552), whose `stRelID` is an `XLNullableWideString` relationship id.
- [MS-XLSB] `RelID` is an `XLNullableWideString` of at most 255 characters.
- The worksheet relationship type is
  `http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing`
  and targets `../drawings/vmlDrawingN.vml`.
- The VML drawing has one relationship per unique image to
  `../media/imageN.<ext>`.
- `[Content_Types].xml` uses a `Default Extension="vml"` entry with
  `application/vnd.openxmlformats-officedocument.vmlDrawing`.
- The Excel header/footer syntax uses `&L`, `&C`, `&R` for sections and `&G`
  (also written `&[Picture]`) for the picture placeholder.
- `rust_xlsxwriter` v0.102.0 requires the placeholder in the matching section
  and writes VML image shapes with 72-DPI point dimensions and
  `(dimension + 0.25).floor()` rounding.

## Compatibility Boundary

- New API mirrors the reference names and builder shape.
- `set_header`/`set_footer` already normalize `&[Picture]` to `&G`; validation
  accepts both spellings.
- Header/footer images use the existing `Image` scale setters and DPI parsing.
  A malformed image is rejected when it is inserted, not at save time.
- The VML part is emitted only for sheets with a header or footer image.
- Both header and footer images on one sheet share one VML part.
- No notes, buttons, comments, or VML text boxes are added.
- Header/footer images share one media part per distinct byte sequence, across
  sections and sheets. VML shape ids advance by 1024 per drawing part, matching
  the reference.
- `BrtLegacyDrawingHF` is emitted after `BrtDrawing` and before `BrtBkHim`,
  matching the worksheet ABNF.

## Change Necessity

The writer has no worksheet image relationship for VML, no `BrtLegacyDrawingHF`
record, and no VML assembly. Adding only a media part would produce an orphaned
image without a header/footer shape.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: placeholder validation; VML shape names, sizes, and image
  relationships; sheet relationship; `BrtLegacyDrawingHF` payload; media bytes;
  absence when no header/footer image exists; Aspose read-back and XLSX export.

## Tasks

1. **Core record.**
   - Add `BrtLegacyDrawingHF` with an `XlNullableWideString` relationship id.
   - Register it in the prelude test table.
2. **Worksheet API.**
   - Add `HeaderImagePosition` and header/footer image slots.
   - Validate the matching `&G` section before storing a cloned image.
   - Validate image dimensions through `Image::scaled_size`.
3. **Sheet records.**
   - Emit `BrtLegacyDrawingHF` with the VML relationship id.
4. **Package.**
   - Build VML and VML relationship parts for each image-bearing sheet.
   - Add media parts and image content types.
   - Add the worksheet `vmlDrawing` relationship and the VML content-type
     default.
5. **Tests.**
   - Assert placeholder rejection.
   - Assert VML shape ids, point sizes, image rel ids, sheet relationship,
     `BrtLegacyDrawingHF`, content type, and media bytes.
6. **Docs and verification.**
   - Update the gap analysis, `AGENTS.md`, and `docs/aegis/INDEX.md`.
   - Run fmt, clippy, default and all-feature tests.
   - Generate a header/footer image XLSB, load it with Aspose.Cells, and inspect
     the XLSX VML export.

## Review

`HeaderImageReview` found one P1 and one P2.

- P1: `BrtLegacyDrawingHF` was emitted before page breaks, ignored errors, and
  `BrtDrawing`. The worksheet ABNF puts it after `BrtDrawing` and before
  `BrtBkHim`. The call moved, and the drawing/background test asserts the
  order.
- P2: the reviewer saw shape ids restart at 1025 in every VML part. The current
  implementation bases them on the part id, `1025 + (data_id - 1) * 1024 +
  index`; a two-sheet test asserts data 1/s1025 and data 2/s2049. Header media
  also dedupe by bytes across sections and sheets.

The reviewer confirmed the VML payload, media relationships, content types,
worksheet relation numbering, DPI-to-point math, and placeholder validation.

## Verification

- `cargo test --workspace`: 326 core + 448 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 326 + 452, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells loads the generated XLSB, reports the header/footer picture
  codes, and its XLSX export preserves both VML shapes with shared media.
- Package tests cover placeholder rejection, `&[Picture]`, VML shape ids and
  point sizes, media deduplication, per-part shape blocks, worksheet
  relationship ids after drawings/backgrounds, `BrtLegacyDrawingHF` payload and
  order, content type, and media bytes.

## Review Focus

- `BrtLegacyDrawingHF` placement and payload.
- VML part/relationship names and content type.
- Worksheet relationship ID collision with tables, hyperlinks, drawings, and
  background images.
- DPI/scale math in VML points.
- Placeholder validation and image dimensions.
- Media and string allocations.
