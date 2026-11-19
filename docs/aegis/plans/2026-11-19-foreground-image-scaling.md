# Foreground image scaling and fit-to-cell

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-foreground-images.md`

## Goal

Close the image scaling part of the rich-objects gap:

- `Image::width`, `height`, `width_dpi`, and `height_dpi` getters.
- `Image::set_width`, `set_height`.
- `Image::set_scale_width`, `set_scale_height`.
- `Image::set_scale_to_size(width, height, keep_aspect_ratio)`.
- `Worksheet::insert_image_fit_to_cell(row, col, &Image, keep_aspect_ratio)`.
- `Worksheet::insert_image_fit_to_cell_centered(row, col, &Image)`.

Scaling is logical: the media bytes stay unchanged, and only the DrawingML
anchor extent changes. The image position, relationship, media numbering, and
part layout stay owned by the foreground-image code.

## Aegis Visibility

This extends the existing `Image` value and `PlacedImage` geometry. It changes
the image constructor boundary (dimension and DPI parsing moves from insertion
to construction), the drawing anchor size from integer pixels to scaled pixels,
and the worksheet insertion API. The owner stays `image.rs` for image metadata,
`drawing.rs` for anchors and DrawingML, and `worksheet.rs` for fit-to-cell
placement.

## Baseline / Authority

- ECMA-376 `xdr:twoCellAnchor`, `xdr:from`, `xdr:to`, and `a:xfrm/a:ext`
  determine the displayed size. The media part is not resampled.
- Excel uses 9,525 EMU per pixel at 96 DPI.
- `rust_xlsxwriter` v0.102.0 `image.rs` defines the public sizing contract and
  the DPI-relative formulas:
  `width * scale_width * 96 / width_dpi`, and
  `scale = target / native * native_dpi / 96`.
- `rust_xlsxwriter` `insert_image_fit_to_cell` scales to the target cell's
  pixel dimensions and leaves the top-left at the cell origin.
  `insert_image_fit_to_cell_centered` keeps the aspect ratio and centers the
  result in the cell.
- PNG DPI comes from the `pHYs` chunk in pixels per metre with unit 1. JPEG DPI
  comes from the JFIF APP0 segment in dots per inch or dots per centimetre.
  GIF and BMP have no DPI path in the reference and use 96.

## Compatibility Boundary

- New sizing methods mirror the reference names and builder-chain shape.
- Native image bytes and the `Arc<[u8]>` storage are unchanged; cloning a scaled
  image still shares media bytes.
- Dimension and DPI parsing happens once in `Image::new` and
  `Image::new_from_buffer`. A malformed foreground image still has valid magic
  bytes and fails at insertion with `Error::ImageError`; background images that
  never need dimensions remain accepted.
- Unscaled output stays byte-identical except for images with non-96 DPI
  metadata. Those now use Excel's DPI-relative logical size, matching the
  reference.
- Scaling does not add `embed_image`, object movement, alt text, hyperlinks, or
  anchor-to-cell resizing. Those remain absent rather than stubbed.
- `insert_image_fit_to_cell` uses default and custom column/row geometry,
  including hidden and filter-hidden cells. A zero-sized target leaves the
  image unscaled, matching the reference guard.
- `Image` drops `Eq` because scale and DPI are floating-point. `PartialEq`
  remains for tests and caller comparisons.

## Change Necessity

`PlacedImage` currently derives its extent from the native pixel size, and the
image type has no logical size. There is no existing API that can express a
scaled extent or a cell-relative fit target. The drawing extent and worksheet
geometry helpers must learn scaled dimensions; adding a parallel unscaled path
would create a second owner.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: image metadata and DPI parsing for PNG/JPEG; scale math for
  width, height, independent axes, aspect ratio, and zero guards; drawing
  anchor and `a:ext` assertions for scaled, fitted, and centered images; package
  assertions for media bytes remaining unchanged; Aspose/LibreOffice read-back.

## Tasks

1. **Image metadata and scaling.**
   - Replace `pixel_size()` re-parsing with an `ImageInfo` value parsed once:
     native width, height, width DPI, and height DPI.
   - Add allocation-free `pHYs` and JFIF parsing.
   - Add getters and the five builder methods.
   - Keep background-image construction valid when dimensions are unavailable.
   - Test metadata, DPI, scale math, and zero guards.
2. **Scaled geometry.**
   - Change `PlacedImage` width and height to scaled `f64` pixels.
   - Round the DrawingML `a:ext` EMUs from the scaled extent.
   - Keep anchor start/end offsets integer-EMU and preserve existing unscaled
     output.
   - Expose the worksheet column/row pixel helpers to fit-to-cell.
3. **Worksheet fit APIs.**
   - Add `insert_image_fit_to_cell` and
     `insert_image_fit_to_cell_centered`.
   - Scale to the target cell's actual pixel width/height, using the same
     default metrics and hidden-row/column rules as anchors.
   - Center with rounded pixel offsets without exceeding `u32`.
   - Test target extent, aspect ratio, and centered offsets.
4. **Package tests.**
   - Assert a scaled image's `a:ext` and anchor positions.
   - Assert fit-to-cell on custom column/row pixels.
   - Assert centered offsets.
   - Assert the media part bytes are untouched by scaling.
5. **Docs and verification.**
   - Update the gap analysis, `AGENTS.md`, and `docs/aegis/INDEX.md`.
   - Run fmt, clippy, default and all-feature tests.
   - Generate a fitted image XLSB, read it with Aspose, and export with
     LibreOffice to inspect the DrawingML extent.

## Review Focus

- Scaling must not copy or resample image bytes.
- DPI must affect unscaled and scaled logical sizes exactly as the reference
  formula does.
- `set_scale_to_size` with `keep_aspect_ratio` must choose the lesser scale.
- Fit-to-cell must use the cell's current pixel dimensions after default
  formats, not the default 64x20 assumption.
- Centered offsets must stay inside the target cell and not overflow.
- A truncated background image must remain accepted.
- An unscaled image on a default worksheet must keep the existing anchor bytes.

## Review

`ImageScalingReview` found one P3 defect in the first implementation: the JFIF
DPI guard required both axes to be greater than 1, so an axis with a bogus 0/1
density discarded the valid other axis. `jpeg_info` now applies the guard per
axis and keeps the 96 default only for the bogus axis. A regression covers
`Xdensity = 1` with `Ydensity = 300`.

The reviewer confirmed that scale math, DPI normalization, EMU rounding, anchor
extent/`to` geometry, fit and centered offsets, package ordering, and unscaled
anchor bytes match the reference.

## Verification

- `cargo test --workspace`: 324 core + 438 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 324 + 442 unit tests, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells reads a 100x50 image at 300 DPI as 32x16 logical pixels, and a
  fit-to-cell image at the requested 80x40 pixels.
- LibreOffice exports scaled, fitted, and centered extents with the same
  proportions and offsets.
- Package tests assert unchanged media bytes for scaled images, fit-to-cell
  extent, centered offsets, and hidden zero-sized targets.

## Risks And Mitigations

- **Anchor drift:** compute one scaled extent and use it for both `a:ext` and
  the `to` anchor.
- **DPI floating point:** store f64 only in `Image` and round once at the EMU
  boundary.
- **Background regression:** parse metadata as `Option`, not as a construction
  error.
- **Fit-to-cell rounding:** use the same pixel helper as anchors and assert the
  exact EMU output.
- **API drift:** mirror the vendored reference names and builder shapes; do not
  invent option structs.

## Retirement

`image.rs` stays the only owner of image metadata and scale. `drawing.rs` stays
the only owner of anchor geometry. `worksheet.rs` only chooses a scale from its
cell metrics and stores a `PlacedImage`; it does not duplicate DrawingML.
