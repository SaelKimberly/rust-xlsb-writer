# ZIP64 mode and extended property datetime inputs

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: `Workbook::use_zip_large_file` and `ExcelDateTime`/jiff document
  property dates.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.1, §3.7, §3.8, and
  §4/§5 notes for ZIP64 and `set_properties`.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Workbook::use_zip_large_file`, `properties.rs`, and the `zip` crate's
  `FileOptions::large_file`.

## Goal

Close two bounded gaps without changing default output:

1. `Workbook::use_zip_large_file(bool)` enables ZIP64 entry extensions so an
   archive can exceed the 4 GiB per-entry or directory limits. When disabled,
   package bytes stay byte-for-byte identical.
2. `DocProperties` accepts `ExcelDateTime` and jiff civil dates/datetimes for
   creation time and custom properties, making `set_properties` fully
   supported for the documented input set.

## Architecture

`WorkbookSettings` gains `zip_large_file: bool`. `build_package` passes it to the
single ZIP owner, `zip_store`.

When enabled, every entry gets a ZIP64 extra field carrying its uncompressed and
compressed sizes, local header size fields use the `0xffff_ffff` sentinel, and
the version-needed field is 45. Central-directory entries carry the same size
data plus a 64-bit local-header offset only when that offset needs it. If the
archive crosses an entry-count, central-directory size, or central-directory
offset limit, `zip_store` writes the ZIP64 end-of-central-directory record and
locator before the standard EOCD. Small archives still use the standard EOCD,
matching the reference `zip` crate behavior.

The compressed payload remains streamed directly into the archive buffer. No
temporary compressed `Vec` is introduced. Sizes and offsets are widened to
`u64` only inside the ZIP writer; normal mode continues to reject values beyond
`u32`.

For properties, `ExcelDateTime` gains a crate-private RFC 3339 formatter used by
`IntoCustomDateTime` and `IntoCustomProperty`. The optional `jiff` feature adds
the same two traits for `&jiff::civil::Date` and `&jiff::civil::DateTime`,
mirroring the existing chrono adapters and the reference strftime formats.

## Tech Stack

Rust 1.100.0, edition 2024, existing `flate2` DEFLATE path, optional
`jiff 0.2` feature, `ExcelDateTime`, `DocProperties`, and `zip_store`.

## Baseline / Authority Refs

- `rust-xlsb-writer/src/workbook.rs`
- `rust-xlsb-writer/src/package.rs`
- `rust-xlsb-writer/src/properties.rs`
- `rust-xlsb-writer/src/datetime.rs`
- `rust-xlsb-writer/src/e2e/mod.rs`
- `thirdparty/rust_xlsxwriter/src/workbook.rs::use_zip_large_file`
- `thirdparty/rust_xlsxwriter/src/properties.rs`
- `zip 2.6.1` `types.rs::Zip64ExtraFieldBlock` and `write.rs`

## Compatibility Boundary

- Public API: `Workbook::use_zip_large_file(&mut self, enable: bool) -> &mut
  Workbook`, matching the reference name and return type.
- ZIP64 is off by default; disabling it after enabling restores the original
  archive path.
- Compression level and ZIP64 remain independent.
- ZIP64 emission adds 20 bytes per entry when enabled, even for small files, as
  documented by the reference.
- `set_creation_datetime(&ExcelDateTime)` and `set_custom_property(name,
  &ExcelDateTime)` now compile. `jiff` adapters are behind the existing
  `jiff` feature, so default dependencies and tests are unchanged.
- No XLSX packaging, BIFF records, worksheet parts, or relationships change.

## Change Necessity

The ZIP writer currently rejects any archive beyond 32-bit fields and has no
public large-file switch. `DocProperties` currently accepts only chrono dates;
the reference contract also accepts `ExcelDateTime` and optional jiff civil
dates. Existing APIs cannot express either request.

## TDD Route

- Mode: auto
- Decision: strict
- Authority: repository test convention; both changes alter externally visible
  package/property contracts.
- Test posture: add a failing package test for ZIP64 headers and a failing
  property test for native/jiff dates, implement until green, then add calamine
  readback and default-output regressions.
- Reason: ZIP header fields and XML property timestamps are external contracts,
  not thin adapters.

## Tasks

### Task 1: RED — property datetime inputs

**Files:**
- Modify: `rust-xlsb-writer/src/properties.rs`
- Modify: `rust-xlsb-writer/src/datetime.rs`

Add tests that require `set_creation_datetime(&ExcelDateTime)` and
`set_custom_property("date", &ExcelDateTime)`. Add a feature-gated jiff test for
civil date and datetime. They fail to compile until the impls exist.

Verification: `cargo test -p rust-xlsb-writer properties` fails for missing
trait implementations.

### Task 2: GREEN — property adapters

**Files:**
- Modify: `rust-xlsb-writer/src/properties.rs`
- Modify: `rust-xlsb-writer/src/datetime.rs`

Add the crate-private RFC 3339 formatter and the `ExcelDateTime` trait impls.
Add jiff `IntoCustomDateTime` and `IntoCustomProperty` impls under
`#[cfg(feature = "jiff")]`. Update trait and method docs.

Verification: property tests pass in default and all-features builds.

### Task 3: RED — ZIP64 package output

**Files:**
- Modify: `rust-xlsb-writer/src/package.rs`

Add a package test that calls `use_zip_large_file(true)`, saves, and asserts
version 45, sentinel local sizes, and a 16-byte ZIP64 size extra field. Add a
default-output test that enabling then disabling reproduces the baseline bytes.

Verification: the test fails until the setting and writer branch exist.

### Task 4: GREEN — ZIP64 writer

**Files:**
- Modify: `rust-xlsb-writer/src/workbook.rs`
- Modify: `rust-xlsb-writer/src/package.rs`

Add the settings flag and public method. Teach `zip_store` to emit local and
central ZIP64 extras, overflow-sentinel fields, conditional 64-bit offsets, and
ZIP64 EOCD records when the archive actually crosses a limit. Preserve the
method-0/8 branches and streamed compression.

Verification: ZIP64 tests pass; existing package tests remain unchanged in
normal mode.

### Task 5: Readback and documentation

**Files:**
- Modify: `rust-xlsb-writer/src/e2e/mod.rs`
- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `docs/aegis/INDEX.md`
- Modify: `AGENTS.md`

- Add calamine readback for a ZIP64-enabled workbook.
- Mark `use_zip_large_file`, ZIP64, and `set_properties` supported.
- Register this plan and refresh test counts.
- Record the remaining untested limit: no >4 GiB package is built in tests.

Verification: `cargo test --workspace`,
`cargo test --workspace --all-features`,
`cargo clippy --workspace --all-targets --all-features`,
`cargo fmt --all --check`, and `git diff --check`.
Also validate a generated ZIP64 archive with Python `zipfile` or `unzip -t`.

## Review

`Zip64PropertiesReview` found one boundary defect: the end-record branch used
strict `>` comparisons while the standard EOCD writes `0xffff`/`0xffff_ffff`
sentinels. The branch now uses `>=` at all three thresholds, and non-ZIP64
entries reject values at or above `0xffff_ffff` instead of writing an
unbacked sentinel. `zip64_end_records_cover_threshold_values` pins the exact
boundary. The reviewer found no other defects.

## Risks

- ZIP64 offset/size fields are easy to transpose; tests assert the exact extra
  field order (`uncompressed`, then `compressed`, then offset).
- calamine's `zip` reader is the external oracle for the small forced-ZIP64
  case. A real >4 GiB archive is not built in tests because it would require
  gigabytes of memory; the EOCD branch is covered by synthetic footer tests.
- Enabling ZIP64 on tiny archives makes readers require ZIP64 support, matching
  the reference. Default output is unchanged.
- `ExcelDateTime` can represent times without dates; property formatting then
  mirrors the reference's `0000-00-00` behavior rather than inventing a date.

## Retirement

No parallel ZIP path is kept. Normal mode remains the existing store path with
the same bytes. ZIP64 mode is a branch in the same writer, not a second package
builder.
