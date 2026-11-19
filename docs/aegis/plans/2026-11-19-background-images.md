# Background images

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-sparklines.md`

## Goal

Add the worksheet background-image gap from the rich-objects roadmap:

- `Image::new(path)` and `Image::new_from_buffer(&[u8])`.
- `Worksheet::insert_background_image(&Image)`.

Each worksheet background is stored as one `xl/media/imageN.<ext>` part, related
from that worksheet, and referenced by `BrtBkHim` in the worksheet part. PNG, JPEG,
GIF, and 24-bit BMP inputs are recognized from their magic bytes.

## Aegis Visibility

This adds a new public `Image` owner and the first worksheet media part. It
touches package relationships, content types, and worksheet record order, so
the media owner, relationship ids, and part naming need one durable boundary.

## Baseline / Authority

- MS-XLSB `2.4.310 BrtBkHim`: a `RelID` (`2.5.115`, an
  `XLNullableWideString`) naming the Images-part relationship.
- MS-XLSB `2.1.7.62` worksheet ABNF places `BrtBkHim` after
  `BrtLegacyDrawingHF`, before `OLEOBJECTS`, `LISTPARTS`, and
  `FRTWORKSHEET`.
- MS-XLSB `2.1.7.30` Images part: content type is the image MIME type and the
  worksheet owns an image relationship.
- `BrtBkHim` is already in `KnownID` as id 562.
- `XlNullableWideString` already encodes `RelID`.

## Compatibility Boundary

- `Image::new` and `Image::new_from_buffer` match the reference return shape.
- `insert_background_image(&Image)` matches the reference signature and keeps
  the last image when called repeatedly.
- Image dimensions, insertion anchors, headers/footers, and foreground
  `insert_image` remain absent rather than stubbed.
- Unsupported image data returns `Error::ImageError`; no bytes are written.

## Change Necessity

The work needs a new public image input type and a worksheet media part.
`BrtBkHim` cannot be expressed by existing records, and a caller cannot provide
the image bytes or relationship without a new owner.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: repository convention is test-after regression coverage.
- Test posture: image magic detection, `BrtBkHim` exact bytes/round-trip,
  worksheet media relationship ids, package part/content-type assertions, and a
  LibreOffice open/export check.

## Tasks

1. **Core record.**
   - Add `BrtBkHim { rgb: XlNullableWideString }`, `Unchecked`, `assign_id!`.
   - Re-export it and add a round-trip size case plus an exact-byte assertion.
2. **Image model.**
   - Add `Image` with `Arc<[u8]>` storage so clones and repeat inserts do not
     copy file bytes.
   - Detect PNG, JPEG, GIF, and BMP from magic bytes; expose crate-internal
     extension, MIME type, and data accessors.
   - Add `Error::ImageError`.
3. **Worksheet API and serialization.**
   - Store an optional `Image` on `Worksheet`, add
     `insert_background_image`, and keep the last image.
   - Emit `BrtBkHim` after ignored-error records and before table parts.
   - Compute the image relationship id after table and external-hyperlink ids.
4. **Package wiring.**
   - Emit one `xl/media/imageN.<ext>` part per worksheet background.
   - Add the worksheet image relationship and the image MIME default to
     `[Content_Types].xml`.
   - Keep image part names stable and one-based.
5. **Verification.**
   - Unit and package tests for PNG plus a second extension, relationship id
     after a hyperlink, content type, and exact media bytes.
   - `cargo test --workspace`, `--all-features`, clippy, fmt.
   - LibreOffice open/export check of a generated workbook.

## Review

`BackgroundImageReview` confirmed the `BrtBkHim` id and payload, the worksheet
ABNF order, media part names, content-type defaults, and relationship-id
arithmetic against a workbook with a table, an external hyperlink, and two
images. Three P3 findings were fixed:

1. The merge-range doc block was split by the new method. The background-image
   docs now sit on `insert_background_image`; `merge_range` keeps its complete
   documentation.
2. The package test now includes a table and a hyperlink, so the image
   relationship id is proven to follow both owners (`rId3`), and the released
   part is asserted to contain exactly one `rId3`.
3. The same test asserts `BrtBkHim` appears before `BrtBeginListParts` in the
   worksheet record sequence.

## Verification

- `cargo test --workspace`: 304 core + 414 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 304 + 418 unit tests, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- A generated workbook with `xl/media/image1.png`, `BrtBkHim`, and the image
  relationship opens in LibreOffice; XLSX export preserves `<picture
  r:id="rId1"/>` and the media part.

## Risks

- Excel may ignore a background image when the relationship target or
  `BrtBkHim` id is wrong; the package test asserts both sides agree.
- LibreOffice may drop XLSB backgrounds on import. If so, record-level decode is
  the oracle and the limitation is documented.
