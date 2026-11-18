# Workbook default format

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: `Workbook::set_default_format` and the style/XLSB plumbing it needs.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.1 and §3.3.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Workbook::set_default_format` and `StyleRegistry`.

## Goal

Add `Workbook::set_default_format(&Format, row_height, col_width)` so a
workbook can change the style used by cells that have no explicit format. Keep
the existing default output byte-identical when the method is unused.

## Architecture

`StyleRegistry` owns the default cell XF at index 0. Today that XF and the
Normal style XF are hard-coded defaults. The change makes both replaceable
before worksheet formats are registered:

1. `StyleRegistry::set_default_style(CellStyle)` registers the style's font,
   fill, border and number format, then replaces cell XF 0 and the Normal style
   XF. The registry remains the single style owner.
2. `Workbook::set_default_format` must be called before any worksheet is added,
   matching the reference contract. It stores one cloned `Format` and the
   requested pixel dimensions.
3. Worksheets added later receive the default row height, default column
   width, and one cloned `Format` for default hyperlink styling. The package
   passes the workbook copy to the style registry.
4. `build_package` registers the workbook default as style 0 before worksheet
   formats. Cells without a format already resolve to style index 0.

## Tech Stack

Rust 1.100.0, edition 2024, existing `deku` records, `StyleRegistry`,
`Workbook`, `Worksheet`, `package.rs`, `sheet.rs`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/workbook.rs`
- `rust-xlsb-writer/src/worksheet.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-writer/src/sheet.rs`
- `rust-xlsb-core/src/styles.rs`
- `thirdparty/rust_xlsxwriter/src/workbook.rs::set_default_format`
- `thirdparty/rust_xlsxwriter/src/worksheet.rs::initialize_default_format`

## Compatibility Boundary

- Public signature:
  `pub fn set_default_format(&mut self, format: &Format, row_height: u32, col_width: u32) -> Result<&mut Self>`.
- Allowed column pixel widths match the reference: 56, 64, 72, 80, 96, 104,
  120. Other widths return `Error::Parameter`.
- Calling after adding a worksheet returns `Error::Parameter`.
- No new public worksheet method; the worksheet default format is workbook
  state, so per-sheet divergence cannot occur.
- `Worksheet::set_default_format` remains missing and the gap analysis stays
  explicit about that.

## Change Necessity

A public workbook setter alone cannot satisfy the feature: the package must
emit a custom style at XF 0, and the sheet must emit the requested default row
height and column width. The minimum code boundary is one core registry method
plus workbook/package/sheet plumbing.

## TDD Route

- Mode: auto
- Decision: light
- Authority: repository test convention; no explicit strict TDD request.
- Test posture: add focused regression tests for the registry, workbook guard,
  and package records. The behavior is bounded and observable through existing
  record-level helpers.
- Reason: a new public API plus a core style mutation is shared enough to need
  tests, but the implementation is a single owner handoff, not a new subsystem.

## Tasks

### 1. Core default style

Files: `rust-xlsb-core/src/styles.rs`.

- Add `style_xf: BrtXF` to `StyleRegistry`.
- Initialize it to the current default Normal XF.
- Add `set_default_style(&mut self, style: CellStyle) -> Result<()>`:
  register number format/font/fill/border, build the cell XF with
  `ixfeParent = 0`, replace XF 0 and its dedup entry, and build the Normal
  style XF with `ixfeParent = 0xFFFF`.
- Emit `style_xf` from `write_to_biff` instead of constructing the default.
- Test custom font/bold propagation to both XF 0 and the Normal XF, and that
  adding the same style returns index 0.

Verification: `cargo test -p rust-xlsb-core styles::tests`.

### 2. Workbook API and dimensions

Files: `rust-xlsb-writer/src/workbook.rs`,
`rust-xlsb-writer/src/worksheet.rs`.

- Add `default_format: Option<Format>`, `default_row_height: Option<u32>`,
  `default_col_width: Option<u32>` to `Workbook`.
- Add `set_default_format` with the reference guard, supported widths, and row
  height validation.
- Apply the numeric defaults when adding or pushing worksheets.
- Store `default_col_width: Option<f64>` in `Worksheet` and add a crate-private
  setter/getter using the existing pixel conversion.
- Test rejection after a worksheet exists, rejection of unsupported widths, and
  default row/column storage.

Verification: targeted writer tests.

### 3. Package and sheet emission

Files: `rust-xlsb-writer/src/package.rs`, `rust-xlsb-writer/src/sheet.rs`.

- Pass `Option<&Format>` to `build_package`; register it as style 0 before
  sheet formats.
- `write_sheet_format` emits when a default column width exists and uses it for
  `dxGCol`.
- Row headers without explicit height fall back to the worksheet default.
- Column infos without an explicit width fall back to the worksheet default.
- Test the styles part has the custom font in the Normal XF and cell XF 0, and
  the sheet part has the expected `BrtWsFmtInfo` height/width.

Verification: `cargo test -p rust-xlsb-writer`.

### 4. Documentation and verification

Files: `AGENTS.md`, `docs/gap-analysis-rust-xlsxwriter.md`,
`docs/aegis/INDEX.md`.

- Mark `Workbook::set_default_format` supported; keep
  `Worksheet::set_default_format` missing.
- Update test counts.
- Run `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  `cargo fmt --all --check`, and `git diff --check`.

## Verification

Result: 193 core + 184 writer unit tests and 6 doctests pass. Clippy, fmt, and
diff checks pass.

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- Package-level decode of `xl/styles.bin` and
  `xl/worksheets/sheet1.bin`.
- Existing default-output tests remain green.

## Risks

- XF 0 is workbook-global. The API deliberately prevents per-sheet defaults.
- Default column width converts the requested pixel width with the reference's
  digit-width and padding metrics, so the emitted `dxGCol` matches the supported
  font metrics.
- Default row height is within Excel's 409.5-point limit; larger pixel values
  are rejected.

## Retirement

No legacy path is retained. If no default format is set, the registry keeps the
existing default style and the package output path is unchanged.
