# Repository Guidelines

## Project Overview

`rust-xlsb-writer` is a pre-alpha Rust workspace for reading and writing MS-XLSB
(Excel Binary Workbook, BIFF12) files, targeting the [MS-XLSB] specification
revision `22.0` (release `2026-10-16`, superseding `2024-05-21`/revision 17.1).
The long-term goal is cell-by-cell writing plus batched
Apache Arrow ingestion, with styles, low RAM use, and exact IEEE 754 float
fidelity.

Current reality: only the binary record layer exists. There is no workbook type,
no worksheet model, no cell API, no ZIP/OLE package assembly, and no file
writer. `KnownID` covers 860 of the 878 records in revision 22.0; every id it
does carry matches the spec. Treat the README as a roadmap, not as a
description of shipped API.

[MS-XLSB]: https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xlsb/acc8aa92-1f02-4167-99f5-84f9f676b95a

## Architecture & Data Flow

Two crates. `rust-xlsb-core` holds all live code; `rust-xlsb-writer` is a stub
whose entire body is commented out.

Record pipeline, in the order bytes are produced:

1. A record struct derives `deku::DekuRead`/`DekuWrite` and stays a plain Rust
   type (no length prefix, no record id in the struct).
2. `Checked::check()` validates field invariants. Types that implement
   `Unchecked` get a blanket `Ok(())` implementation from `check.rs`, so an
   `Unchecked` record has no validation at all.
3. `CheckedBiff::id()` supplies the record identifier, normally through the
   exported `assign_id!(TypeName)` macro, which maps the type name onto the
   `KnownID` variant of the same name.
4. `BiffWrite::biff_push_to(&mut Vec<u8>)` / `biff_push_to_writer` serializes
   the body. A blanket impl covers every type that is
   `DekuContainerRead + DekuContainerWrite + CheckedBiff`.
5. `BiffHead::new(id, size)` encodes the record header (variable-length id plus
   varint size, 1–4 bytes each) and `write_as_biff` / `write_as_empty_biff`
   emit header then body.

`KnownID` is a `#[repr(u16)]` enum of 860 variants whose discriminants
are computed by the `const fn as_biff_id` in `biff_id.rs`. Those numbers are MS-XLSB
*record literals*, not raw spec ids. Never hand-write a discriminant; add the
variant through the same mapping. `KnownID::try_from(raw_u16)` rejects unknown
ids with `num_enum::TryFromPrimitiveError`.

Shared strings go through `SSTHolder` (`sst_sqlite.rs`): a SQLite table keyed by
the `xxh3_64` hash of the string, with a hit counter. `push` returns a stable
index (`ROWID - 1`), `push_batch` wraps the inserts in `BEGIN IMMEDIATE`, and
`write_to_biff_sst` emits `BrtBeginSst` / `BrtSSTItem`* / `BrtEndSst`. Note the
doc comment claims strings are ordered by descending frequency, but the read
query has no `ORDER BY`; do not rely on that claim.

## Key Directories

| Path | Contents |
|---|---|
| `Cargo.toml` | Virtual workspace manifest; members and shared `[workspace.package]` metadata |
| `rust-xlsb-core/src/` | All working code: record framing, id catalogue, traits, shared strings |
| `rust-xlsb-core/src/prelude/` | Public record/structure facade (`record/`, `structure/`, `scope.rs`, `collection.rs`) |
| `rust-xlsb-writer/src/lib.rs` | Stub crate; only a `MiMalloc` global allocator is live |
| `.github/` | `FUNDING.yml` only; no workflows, templates, or CI |

## Development Commands

The workspace builds on stable Rust. `rust-toolchain.toml` pins `1.100.0`
(duplicated in `rust-xlsb-core/`), edition is 2024, and the resolver is `3`.

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
cargo doc -p rust-xlsb-core --no-deps
```

Toolchain selection is the one environment trap here. `rust-toolchain.toml`
pins `1.100.0`, but a wrapper `cargo` earlier in `PATH` (for example the `mbx`
build-cache shim under `~/.local/share/mbx/bin`) dispatches `rustc` itself and
can override that pin, producing misleading errors such as
`rustc 1.88.0-nightly is not supported`. Verify before trusting any result:

```bash
rustc -V            # must print 1.100.0
cargo -v build -p rust-xlsb-core 2>&1 | grep rustc | head -1
```

If the versions disagree, run the toolchain binaries by absolute path, which
also pins the compiler `cargo` spawns:

```bash
PATH="$(dirname "$(rustup which --toolchain 1.100.0 cargo)"):$PATH" cargo test --workspace
```

Verified state on this branch:

- `cargo test --workspace` → 36 unit tests and 6 doctests pass, 0 failures, with
  `rustc -V` reporting `1.100.0` for both the cargo and every spawned rustc.
- `cargo clippy --workspace --all-targets` → no warnings and no code lints.
- `cargo fmt --all --check` → clean.
- `Cargo.lock` is gitignored, so a fresh resolve can pick different minor
  versions; with the current manifests all dependencies resolve and compile on
  `1.100.0`.

No CI workflow or formatter config exists; default `rustfmt` settings apply. The
workspace manifest does configure clippy (`[workspace.lints.clippy]`), and both
crates opt in with `[lints] workspace = true`.

## Code Conventions & Common Patterns

- **Module per record group.** One file per record or small family under
  `prelude/record/` and `prelude/structure/`; re-export from
  `prelude/mod.rs` and register the type there.
- **Spec names stay verbatim.** `#![allow(non_snake_case, clippy::upper_case_acronyms)]`
  is set at the top of `prelude/mod.rs` only. Fields such as `rwSync` and types
  such as `BrtXF`, `BrtWsProp` keep their specification spelling; do not
  "clean up" names. Outside the prelude, use snake_case for functions and
  variables.
- **Errors.** One crate-level enum, `Error` in `err.rs`, with a
  `pub type Result<T>` alias; it is built on `thiserror`. Subsystems that need
  their own variants define a local error enum (`SSTHolderError`) and convert
  with `#[from]`. Every public fallible function documents a `# Errors`
  section, and doctests accompany public constructors.
- **Error fidelity.** The blanket `BiffWrite` impl discards the underlying deku
  error and returns `Error::ValidationError("Invalid BIFF record")`. If you need
  the cause, format it, as `KnownID::try_deku_literal` does.
- **Zero-sized markers.** `scope.rs` and `collection.rs` generate `BrtBegin*`/
  `BrtEnd*` structs via `concat-idents`. Scope markers serialize to exactly 0
  bytes; collection markers carry a `u32 count`.
- **Unsafe is deliberate.** `BiffHead::const_from_data` parses with raw pointer
  offsets, and `read_from_with_data` / `RawBiffLiteral::packed` use
  `set_len`/`new_uninit_slice`. Keep the same style when extending them, keep
  the uninitialized regions fully written before `assume_init`, and add a test.
- **Adding a record** requires all of: deku derives with the correct bit layout,
  a `Checked` impl (or `Unchecked`), `assign_id!(TypeName)`, a re-export in
  `prelude/mod.rs`, and an `#[case::<name>(|| checked_test::<TypeName>(SIZE))]`
  entry in the `test_biff_export` rstest table in that file's test module.
- **Lints are active.** `rust-xlsb-core` sets `#![deny(dead_code)]`; an unused
  private helper fails the build. Wire it up or delete it. Both crates inherit
  `[workspace.lints]` (`clippy::pedantic` and `clippy::nursery` at `warn`);
  intentional violations carry a scoped `#[expect(..., reason = "…")]` instead
  of a blanket crate-level allow.
- **Allocator.** `mimalloc` is installed as the global allocator in
  `rust-xlsb-writer` only. A second `#[global_allocator]` in core would make the
  workspace fail to link.

`XlString` (`structure/xl_wide_string.rs`) has a known bug to be aware of: the
`len` field is documented as UTF-16 code units and deku reads `len * 2` bytes,
but `FromStr` and `from_string` assign `str.len()`, which is the UTF-8 byte
length. Non-ASCII strings therefore encode a wrong length. `0xFFFF_FFFF` means
null, and `SimpleRichStr` rejects null while `CodeName` limits length to 31.

## Important Files

| File | Role |
|---|---|
| `rust-xlsb-core/src/lib.rs` | Module list, the entire public re-export surface, `assign_id!` |
| `rust-xlsb-core/src/biff_id.rs` | `KnownID` catalogue, `as_biff_id` remap, `RawBiffLiteral` and its literal factories |
| `rust-xlsb-core/src/biff_head.rs` | Record header encode/decode, `read_from_with_data` |
| `rust-xlsb-core/src/biff.rs` | `BiffWrite`, `write_as_biff`, `write_as_empty_biff` |
| `rust-xlsb-core/src/check.rs` | `Checked`, `Unchecked`, `CheckedBiff` traits |
| `rust-xlsb-core/src/err.rs` | `Error`, `Result` |
| `rust-xlsb-core/src/sst_sqlite.rs` | `SSTHolder` shared-string table and SST emission |
| `rust-xlsb-core/src/prelude/mod.rs` | Re-exports plus the shared round-trip test harness |
| `rust-xlsb-writer/src/lib.rs` | Stub; allocator plus ~500 lines of commented prototype |
| `rust-toolchain.toml` | Stable `1.100.0` pin at the root; `rust-xlsb-core/rust-toolchain.toml` duplicates it |

## Runtime/Tooling Preferences

- Stable Rust `1.100.0`, edition 2024, workspace resolver `3`. Both
  `rust-toolchain.toml` files pin the same channel.
- No unstable features are used. `MaybeUninit::write_copy_of_slice`,
  `String::from_utf16le`, and `std::hint::cold_path` are stable on this
  toolchain; `std::hint::unlikely` is not, so branch-hint annotations were
  removed from `biff_head.rs` rather than replaced with a shim.
- `Cargo.lock` is gitignored by design (library workspace). Pin minor versions
  in the manifests when a specific dependency minor matters, and expect
  dependency resolution to drift between checkouts.
- `rusqlite` uses the `bundled` feature, so a C compiler is needed and the first
  build compiles SQLite from source.
- `rust-xlsb-writer` depends only on `rust-xlsb-core` and `mimalloc`. Other
  crates were removed because no code referenced them; add one back together
  with the code that uses it, otherwise cargo's `unused_dependencies` lint
  fires.
- Dependency set, latest major/minor for this toolchain: `deku 0.20`,
  `rusqlite 0.43`, `strum 0.28`, `criterion 0.8`, `zerocopy 0.8.63`,
  `thiserror 2.0.21`, `num_enum 0.7.6`, `xxhash-rust 0.8.21`,
  `concat-idents 1.1.5`, `mimalloc 0.1.52`, `rstest 0.28` (dev).
- No release tooling, changelog, or publish workflow. Version is `0.1.0` and the
  README defers `1.0.0-alpha` to community adoption.

## Testing & QA

- Tests are inline `#[cfg(test)] mod tests` modules inside `rust-xlsb-core`;
  there is no `tests/`, `benches/`, `examples/`, or fixture directory.
- 36 unit tests and 6 doctests total. Most unit tests are rstest cases in the
  `test_biff_export` table in `prelude/mod.rs`, each calling the generic
  `checked_test::<T>`: `Default` → `to_bytes` → exact byte-size assert →
  `from_bytes` → `check()` on both → `PartialEq`, `Hash`, and `Debug` equality.
- Case names are `test_biff_export::case_<N>_<name>`; the size passed to
  `checked_test` is the assertion, so change it whenever a record layout
  changes.
- Hand-written tests: `biff_id::tests::test_known_ids` (literal packing, header
  parse), `sst_sqlite::tests::test_sst_sqlite` (push, batch, SST counters, item
  decode), `prelude::structure::xl_wide_string::tests::test_xl_wide_string`,
  and `prelude::record::brt_color::test_brt_color`.
- Round-trip tests are the only oracle. Nothing is validated against Excel,
  LibreOffice, or an independent reader, and header boundary cases
  (`0x80`, `0x4000`, `0x200000` size thresholds, `Incomplete`, `ConsumeFailed`)
  are untested.
- `criterion` is a dev-dependency in both crates but no benchmark exists.
- No CI runs any of this. Run `cargo test --workspace` before claiming a change
  works, and update the affected size literal rather than loosening the
  assertion.

## Commit Style

Git history uses Conventional Commits (`feat:`, `fix:`) with a capitalized
subject. This is observed practice, not a documented rule.
