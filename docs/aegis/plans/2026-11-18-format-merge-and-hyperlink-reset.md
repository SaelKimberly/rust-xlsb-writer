# Format merge and hyperlink-style reset

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: `Format::merge`, `Format::unset_hyperlink_style`
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` section 3.6 and the vendored
  `rust_xlsxwriter` v0.102.0 `format.rs` merge and unset methods

## 1. Goal

Close two formatting gaps with source-compatible methods:

```rust
impl Format {
    pub fn merge(&self, other: &Format) -> Format;
    pub fn unset_hyperlink_style(mut self) -> Format;
}
```

`merge` returns a new format. Properties already set on `self` win over
`other`; boolean flags union, while `locked` intersects because it defaults to
true. `unset_hyperlink_style` reverses `set_hyperlink` by clearing the font
color and restoring underline and font scheme to the default font.

Neither method changes serialization or package output until the resulting
`Format` is used.

## 2. Architecture

`Format` is a writer-side value object; no core record changes are needed.
`merge` clones `self` once and calls a private `merge_with` helper:

- Number format: keep `self` when set, otherwise clone `other`.
- Font: union italic, strikeout, outline, shadow, condense, and extend; bold
  wins if either side is bold. Copy name, size, family, charset, underline,
  script, scheme, and raw color only when `self` still has the default.
- Fill: keep an explicit pattern; copy the pattern and raw colors only when
  `self` is unset. Copy the foreground/background color options when absent.
- Border: copy each side style and color only when `self` has no style/color;
  union diagonal direction flags.
- Alignment/protection: union wrap, shrink, hidden formula, and quote prefix;
  intersect locked; copy horizontal/vertical alignment, indent, rotation, and
  reading order only when `self` is at its default.

The existing `to_cell_style` path validates and maps the merged format exactly
like any other format, so no new XML or BIFF records are introduced.

## 3. Tests

- Merging fills unset font, fill, border, alignment, number-format, and
  protection fields from the second format.
- Existing values on the first format win; boolean flags union and locked
  intersects.
- `set_hyperlink().unset_hyperlink_style()` equals `Format::new()`.
- Merge identity in both directions for every format property group, explicit
  top-alignment precedence, conflicting diagonal directions, and explicit
  `FormatPattern::None`.
- Full workspace tests, clippy, fmt, and diff check pass.

Result: 192 core + 179 writer unit tests and 6 doctests pass. Reviewer findings
fixed: an explicit vertical `Top` alignment is tracked so it survives a merge
against another vertical alignment, and conflicting diagonal directions keep
the first format's direction instead of combining both.

## 4. Risks

- `Format` stores font/fill/border values in both typed fields and pending
  color options; merge must handle both without duplicating output.
- Explicit `FormatPattern::None` is preserved through `pattern_set`; the
  reference does not distinguish it from the default, so this is a stricter
  merge rather than a package-format difference.
