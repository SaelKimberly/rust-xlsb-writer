# Cell note appearance and accessibility

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-notes.md`

## Goal

Close the remaining basic note presentation gap:

- `Note::set_background_color(color)` with RGB and named colors.
- `Note::set_alt_text(text)` with the reference 255 UTF-16 limit.
- `Note::reset_text(text)` so one note can be reused.
- Note VML emits the chosen fill color and escaped `alt` text.

Threaded comments, author-prefix formatting, note text fonts, and object movement
stay out of scope.

## Aegis Visibility

`Note` gains three builder methods, two fields, and one default RGB value. Note
VML changes its `fillcolor`, `<v:fill color2>`, and optional `alt` attribute.
No new records, parts, relationships, or content types.

## Baseline / Authority

- The default note fill is `#FFFFE1`, matching the reference note default.
- Excel notes accept RGB colors; theme, default, and automatic colors are
  ignored, matching the reference restriction.
- The reference caps note alt text at 255 UTF-16 code units; longer input is
  ignored.
- VML puts accessibility text in the shape `alt` attribute.
- `reset_text` clears and reuses the existing `String` allocation.

## Compatibility Boundary

- `Note` keeps its existing author, visibility, width, and height semantics.
- `set_background_color` accepts `Color` and RGB/named inputs. Invalid RGB and
  non-RGB colors leave the previous fill unchanged.
- Alt text is escaped as an XML attribute (`&`, `"`, `<`, `>` and control
  characters), not copied into the VML verbatim.
- Empty alt text omits the attribute.

## Verification

- `cargo test --workspace`: 332 core + 457 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 332 + 461, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Note tests cover default and custom fills, named and invalid RGB colors,
  escaped alt text, the 255-code-unit cap, empty-alt omission, and text reset.

## Review

`NoteAppearanceReview` found three P3 test-coverage holes, all fixed:

- The overlong-alt test started from an empty value, so it could not prove the
  previous text survives. It now seeds `"keep"`.
- Invalid RGB and named-color paths were untested. The test now asserts
  `0x0100_0000` is rejected and `Color::Red` is accepted.
- Default fill and empty-alt omission were untested. A default note now asserts
  `fillcolor="#FFFFE1"` and no `alt` attribute.

Reviewer verdict: correct; no residual P0-P3 findings.

## Change Necessity

Note VML hard-coded `#FFFFE1` and had no alt text, so callers could not change
the note fill or provide an accessible label despite the reference API.
