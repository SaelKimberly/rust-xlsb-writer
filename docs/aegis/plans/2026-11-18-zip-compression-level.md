# ZIP compression level

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: `Workbook::set_zip_compression_level` and DEFLATE package entries.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.1 and §3.8.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Workbook::set_zip_compression_level` and `Packager` compression handling.

## Goal

Let a caller request DEFLATE compression for the XLSB ZIP package with Excel's
1–9 level range, without changing default output. A workbook that never calls
the new method keeps the current store-only archive byte layout.

## Architecture

`WorkbookSettings` gains `zip_compression_level: Option<u8>`. `build_package`
passes that option to `zip_store`, which is already the single ZIP owner.

`zip_store` selects method `0` (store) when the option is absent and method `8`
(DEFLATE) when present. It writes each local header with reserved space, streams
the compressed payload directly into the archive buffer, then backfills CRC,
compressed size, and name. No per-part compressed `Vec` is allocated.

`flate2` supplies raw DEFLATE. Its `zlib-rs` backend is already in the
dependency graph through the dev-only ZIP reader, so adding it does not bring a
second compression backend into the build.

The test-only ZIP reader learns method 8, so existing part assertions keep
working for both modes.

## Tech Stack

Rust 1.100.0, edition 2024, `flate2 1.1` with `default-features = false` and the
`zlib-rs` backend, existing `Workbook`, `WorkbookSettings`, and `zip_store`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/workbook.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-writer/src/e2e/mod.rs`
- `thirdparty/rust_xlsxwriter/src/workbook.rs::set_zip_compression_level`
- `thirdparty/rust_xlsxwriter/src/packager.rs` compression setup
- `docs/aegis/plans/2026-11-18-xlsxwriter-compatible-api.md`

## Compatibility Boundary

- Public API: `Workbook::set_zip_compression_level(&mut self, level: u8) -> &mut
  Workbook`. It matches the reference name and return type.
- Valid levels are 1 through 9. Values outside that range print the reference's
  range warning and leave the previous setting unchanged.
- Absence of a setting keeps every entry stored, preserving current package
  bytes and tests.
- ZIP64/large-file mode and per-entry method choices are out of scope; the gap
  entry remains partial for `use_zip_large_file`.
- No BIFF record, worksheet part, content type, or relationship changes.

## Change Necessity

The current ZIP writer hardcodes method 0 and cannot compress any package. A
caller-facing option requires a new settings field, one public method, and one
method-selection branch in the existing ZIP owner. No existing API can express
the request.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: repository test convention; compression changes the package
  producer and its readback contract.
- Test posture: add a failing package test that requires DEFLATE entries, then
  implement until it passes; add a calamine readback test and a default-output
  regression test.
- Reason: the ZIP local/central headers and compressed bytes are an external
  contract, not a thin adapter.

## Tasks

### Task 1: RED — prove the package is stored

**Files:**
- Modify: `rust-xlsb-writer/src/package.rs`

Add a package test that creates a workbook, calls
`set_zip_compression_level(6)`, saves, and asserts the first local ZIP header
uses method 8. The test compiles only after the API exists; with a temporary
no-op setter it fails on method 0.

Verification: `cargo test -p rust-xlsb-writer zip_compression` fails for the
missing method/no-op behavior.

### Task 2: GREEN — settings and compressed writer

**Files:**
- Modify: `rust-xlsb-writer/Cargo.toml`
- Modify: `rust-xlsb-writer/src/workbook.rs`
- Modify: `rust-xlsb-writer/src/package.rs`

Add the optional `flate2` dependency, the settings field, the public setter,
method selection, streamed DEFLATE, and method-8 decode in the test-only
`part_bytes` helper. Keep local and central headers consistent: method, CRC,
compressed size, uncompressed size, and version fields.

Verification: the RED test passes; existing `part_bytes`-based package tests
still pass in both modes.

### Task 3: Regression and readback

**Files:**
- Modify: `rust-xlsb-writer/src/package.rs`
- Modify: `rust-xlsb-writer/src/e2e/mod.rs`
- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `docs/aegis/INDEX.md`

- Assert default output still uses method 0.
- Assert invalid levels outside 1–9 leave the previous setting intact.
- Add a calamine round-trip over a compressed workbook.
- Mark compression supported and leave large-file mode explicitly missing.
- Register this plan in the workspace index.

Verification: `cargo test --workspace`,
`cargo test --workspace --all-features`,
`cargo clippy --workspace --all-targets --all-features`,
`cargo fmt --all --check`, and `git diff --check`.

## Review

`ZipCompressionReview` found no defects. Its fresh-workbook invalid-level
suggestion was applied as `invalid_zip_compression_level_is_ignored_by_default`.
The stderr warning is retained for reference compatibility, and `flate2` stays a
normal dependency because the public setter must work without extra features.

## Risks

- ZIP method 8 support in calamine is the external oracle; a passing round-trip
  proves header and stream consistency.
- `flate2`'s `zlib-rs` feature must stay aligned with the dev ZIP reader so the
  workspace does not compile two DEFLATE backends.
- Compression can enlarge tiny entries; this is valid ZIP and not an error.

## Retirement

No old path is retained: method 0 remains the documented default, not a
compatibility shim. The test-only method-8 decoder exists solely to inspect both
archive modes and has no runtime role.
