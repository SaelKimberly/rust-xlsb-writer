# Foreground images

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-background-images.md`

## Goal

Add the foreground-image gap from the rich-objects roadmap:

- `Worksheet::insert_image(row, col, &Image)`.
- `Worksheet::insert_image_with_offset(row, col, &Image, x_offset, y_offset)`.
- Multiple images per worksheet, in insertion order.

Each worksheet with images owns one
`xl/drawings/drawingN.xml` part and a drawing relationship. The DrawingML
references one `xl/media/imageN.<ext>` part per inserted image. PNG, JPEG, GIF,
and 24-bit BMP are supported by the existing `Image` type.

## Aegis Visibility

This adds the first worksheet DrawingML part and the foreground-media use of
`Image`. It touches worksheet record order, sheet relationships, content types,
media numbering, image size parsing, and a new `BrtDrawing` record. Those
boundaries need one owner: `drawing.rs` for placement and XML, `package.rs` for
part and relationship numbering, and `sheet.rs` for `BrtDrawing` placement.

## Baseline / Authority

- MS-XLSB `2.4.354 BrtDrawing`: a `RelID` (`2.5.115`) naming the Drawings-part
  relationship.
- MS-XLSB `2.1.7.62` worksheet ABNF places `BrtDrawing` after `IGNOREECS` and
  before `BrtLegacyDrawing`, `BrtBkHim`, and `LISTPARTS`.
- ECMA-376 `xdr:wsDr`, `xdr:twoCellAnchor`, `xdr:pic`, and
  `a:blip r:embed` define the DrawingML picture shape used by Excel.
- `KnownID::BrtDrawing` already exists as record id 550; the payload matches
  `BrtBkHim`'s `XlNullableWideString` relationship id.
- Excel positions drawing pixels at 9,525 EMU per pixel.

## Compatibility Boundary

- `insert_image` and `insert_image_with_offset` match the reference signatures
  and return `Result<&mut Worksheet>`.
- `Image::new` and `Image::new_from_buffer` remain unchanged; dimension parsing
  happens only when an image is inserted.
- Only native image dimensions and caller offsets are supported. Scaling,
  `embed_image`, fit-to-cell, hyperlinks, alt text, and object movement remain
  absent rather than stubbed.
- A malformed image that has valid magic bytes but no readable dimensions
  returns `Error::ImageError` from insertion; no part is written.
- Out-of-range rows or columns return `Error::RowColumnLimit`.
- Background images stay independent. A worksheet can have both a foreground
  drawing and a background image.
- Existing `BrtDrawing`-free worksheets keep byte-identical output.

## Change Necessity

`BrtDrawing`, the Drawings part, its relationships, and the media parts cannot
be expressed by existing records or worksheet state. A caller cannot place a
foreground image without new placement state and package parts.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: image dimension parsing for all four formats, insertion
  validation, exact `BrtDrawing` bytes and round-trip, drawing XML anchors and
  relationship ids, content types, part absence without images, coexistence
  with background images, and an Aspose/LibreOffice read-back check.

## Tasks

1. **Core record.**
   - Add `BrtDrawing { rgb: XlNullableWideString }`, `Unchecked`, `assign_id!`.
   - Re-export it and add the `checked_test::<BrtDrawing>(4)` case plus an exact
     `rId1` byte assertion.
2. **Image dimensions.**
   - Add `Image::pixel_size(&self) -> Result<(u32, u32)>` with allocation-free
     PNG, JPEG, GIF, and BMP header parsing.
   - Reject truncated headers, zero dimensions, and unsupported JPEG frames.
   - Test each format and malformed inputs.
3. **Drawing model and XML.**
   - Add `drawing.rs` with `PlacedImage { image, row, col, x_offset, y_offset,
     width, height }`.
   - Build a `twoCellAnchor` with `from`/`to` cell positions and offsets
     computed from the start cell, image pixels, and worksheet row/column
     sizes.
   - Write `xdr:pic` with `cNvPr`, `picLocks`, `blipFill`, `spPr`, and
     `clientData`.
   - Test anchors, offsets, and multiple pictures.
4. **Worksheet API.**
   - Store `Vec<PlacedImage>` on `Worksheet`; add the two insertion methods.
   - Validate row/column limits up front and parse dimensions once.
   - Pass a drawing relationship id to `sheet::write_sheet_with_fonts` and emit
     `BrtDrawing` after ignored errors, before `BrtBkHim`.
5. **Package wiring.**
   - Emit one `xl/drawings/drawingN.xml` and
     `xl/drawings/_rels/drawingN.xml.rels` per image-bearing worksheet.
   - Emit one `xl/media/imageN.<ext>` per placed image, deduplicating image MIME
     defaults but not bytes.
   - Add sheet drawing and background relationships in stable order: tables,
     external hyperlinks, drawing, background.
   - Add drawing content-type overrides.
   - Tests: no-image output unchanged, image package parts, relationship ids,
     background coexistence, BrtDrawing order, and multiple sheets.
6. **Docs and verification.**
   - Update the gap analysis, `AGENTS.md`, and `docs/aegis/INDEX.md`.
   - Run fmt, clippy, default and all-feature tests.
   - Read the generated XLSB back with Aspose and open it in LibreOffice.

## Review Focus

- A valid image with truncated headers must fail at insertion, not produce a
  zero-sized drawing.
- Insertion at a non-zero row/column with offsets must not place the picture at
  the sheet origin.
- A worksheet with both a drawing and a background image must use distinct
  relationship ids and emit `BrtDrawing` before `BrtBkHim`.
- Multiple pictures on one sheet must use one drawing part and one
  relationship per media part.
- Workbooks without foreground images must not gain drawing parts, content-type
  overrides, or relationship fragments.

## Review

`ForegroundImageReview` found no P0/P1 defects, but did find real geometry
defects in the first implementation:

1. The `to` anchor ignored the start offset, so an offset image was clipped by
   its own offset. `advance_columns`/`advance_rows` now start from
   `image_size + offset`.
2. Narrow columns used the 7 px digit rule even below one character.
   `width_pixels` now uses the stored `(digit_width, padding)` metrics and the
   sub-one-character `width * (digit_width + padding)` branch.
3. Anchors ignored non-default default-column metrics. `Worksheet` now stores
   the metrics chosen by both workbook and detached-worksheet
   `set_default_format`, and drawing geometry uses them.
4. Anchors counted filter-hidden rows with a `RowOptions` entry at full height.
   `absolute_row_pixels` now applies filter visibility before the row height.
   It also treats unused-hidden rows as zero height.

Regression tests cover offsets, 6 px narrow columns, 80 px default metrics,
custom/default row heights, hidden columns, filter-hidden rows with row
options, and unused-row hiding.

## Verification

- `cargo test --workspace`: 324 core + 432 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 324 + 436 unit tests, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells reads two generated pictures at the requested rows/columns with
  the expected native 40x20 px size, including a 10x5 px offset.
- LibreOffice converts the XLSB to XLSX and preserves both media parts and
  drawing anchors.
- A JPEG input generated by ImageMagick is parsed and inserted at row 100,
  column 100.

## Risks And Mitigations

- **Anchor drift:** compute `to` using the actual row and column sizes, then
  assert the XML in a package test.
- **Media numbering:** allocate media indices from the same global counter as
  background images.
- **Schema validity:** keep the DrawingML minimal and verify with Aspose and
  LibreOffice, not only string assertions.
- **Large images:** use `u64` EMU math to avoid overflow; reject dimensions
  beyond `u32`.

## Retirement

No second owner. `PlacedImage` owns one foreground placement, `drawing.rs` owns
DrawingML, `package.rs` owns part/relationship numbering, and `sheet.rs` owns
the `BrtDrawing` position.
