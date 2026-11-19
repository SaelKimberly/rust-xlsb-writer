# Dynamic array formulas

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-xlsxwriter-compatible-api.md`

## Goal

Add the dynamic-array gap from the P0 roadmap:

- `Worksheet::write_dynamic_array_formula`
- `Worksheet::write_dynamic_array_formula_with_format`
- `Worksheet::write_dynamic_formula`
- `Worksheet::write_dynamic_formula_with_format`

Dynamic formulas reuse the existing array formula parser and records. The
top-left cell gains a `BrtCellMeta` record, and the workbook gains an
`xl/metadata.bin` part describing the `XLDAPR` cell metadata type.

## Aegis Visibility

This adds public formula APIs and the first Metadata part. It touches worksheet
record order, workbook relationships, content types, and the formula cell data
model, so the metadata owner and one-based index boundary need a durable plan.

## Baseline / Authority

- MS-XLSB `2.1.7.34` Metadata part:
  `BrtBeginMetadata`, `BrtBeginEsmdtinfo`/`BrtMdtinfo`/`BrtEndEsmdtinfo`,
  `BrtBeginEsfmd`/`BrtBeginFmd`/`DYNAMICARRAYMETADATA`/`BrtEndFmd`/`BrtEndEsfmd`,
  `BrtBeginEsmdb`/`BrtMdb`/`BrtEndEsmdb`, `BrtEndMetadata`.
- MS-XLSB `2.4.60` / `2.4.416`: the begin marker is empty; the end marker carries
  `fDynamic` and `fCollapsed`.
- MS-XLSB `2.4.325 BrtCellMeta.icmb`: one-based index of a `BrtMdb` in the
  cell-metadata collection.
- MS-XLSB `2.4.710 BrtMdb` and `2.5.94 Mdir`: one `iMdt`/`mdd` pair.
- Oracle: an Aspose.Cells XLSB with two dynamic formulas emits a 117-byte
  `xl/metadata.bin`; `XLDAPR`, `BrtCellMeta(1)`, and `BrtArrFmla` reproduce the
  same shape.
- Existing `ArrayFormulaCell`, `BrtArrFmla`, `BrtFmla*`, and `BrtFRTBegin` are
  reused.

## Compatibility Boundary

- New methods match the reference signatures and return `Result<&mut Self>`.
- Static `write_array_formula*` output stays byte-for-byte unchanged.
- Dynamic formulas accept the same formula syntax as static array formulas:
  same-sheet A1 references, ranges, operators, and supported classic functions.
  Future `_xlfn.` functions, cross-sheet references, and defined names remain
  rejected.
- The spill range is not calculated; callers provide the range and the cached
  result is stored on the top-left cell. Remaining cells are zero-filled, as for
  static arrays.
- No `BrtValueMeta` is emitted; it is not required for dynamic metadata.
- Unknown metadata fields are not stubbed.

## Change Necessity

`BrtArrFmla` alone cannot mark a formula as dynamic. The caller needs an API that
sets the dynamic flag, the worksheet needs to emit `BrtCellMeta`, and package
assembly needs the Metadata part. No existing owner represents any of those.

## TDD Route

- Mode: `auto`
- Decision: `strict`
- Authority: public API plus package schema change.
- Test posture: add failing record, sheet, and package tests before wiring the
  writer path.
- Reason: metadata bytes and the one-based `icmb` index are externally consumed.

## Tasks

1. **Core metadata records.**
   - Add `record/brt_metadata.rs` with `BrtBeginMetadata`, `BrtEndMetadata`,
     `BrtBeginEsmdtinfo`, `BrtMdtinfo`, `BrtEndEsmdtinfo`, `BrtBeginEsfmd`,
     `BrtEndEsfmd`, `BrtBeginFmd`, `BrtEndFmd`, `BrtBeginEsmdb`, `BrtMdb`,
     `Mdir`, `BrtEndEsmdb`, `BrtCellMeta`, `BrtBeginDynamicArrayPr`, and
     `BrtEndDynamicArrayPr`.
   - Re-export through `prelude/mod.rs` and register round-trip size cases.
   - Assert the fixed dynamic metadata header bytes and `icmb` exactly.

2. **Worksheet model and API.**
   - Add `is_dynamic` to `ArrayFormulaCell` and `has_dynamic_arrays` to
     `Worksheet`.
   - Add the four public methods; route them through `store_array_formula`.
   - Keep static callers passing `false`.

3. **Worksheet serialization.**
   - For dynamic top-left cells, emit `BrtCellMeta { icmb: 1 }` immediately
     before the formula record.
   - Use `fAlwaysCalc = false` for dynamic array formulas to match the oracle.

4. **Metadata part.**
   - Write the record sequence above with one `XLDAPR` type and one `Mdir`.
   - Emit `xl/metadata.bin`, the `sheetMetadata` workbook relationship, and the
     `application/vnd.ms-excel.sheetMetadata` content type only when a dynamic
     formula exists.

5. **Verification.**
   - Record sizes and exact bytes; package relationship id; no metadata part for
     static arrays; cell metadata precedes the formula record.
   - `cargo test --workspace`, `--all-features`, clippy, fmt.
   - Read the generated file back with Aspose.Cells and LibreOffice.

## Review

`DynamicFormulaReview` found no code defects. It independently compared the
generated `xl/metadata.bin` with the Aspose and POI oracles and confirmed the
117-byte, 16-record layout, `XLDAPR`, `BrtCellMeta(1)`, the `BrtArrFmla` range,
and the package content type and relationship. Two P3 documentation errors were
fixed:

1. `AGENTS.md` reported the all-features total as 726; the correct total is 742.
2. `ArrayFormulaCell` still described itself as static; it now says array.

The reviewer also confirmed that keeping `has_dynamic_arrays` sticky after an
overwrite matches the reference implementation; the metadata part alone is
valid.

## Verification

- `cargo test --workspace`: 322 core + 416 writer unit tests, 7 doctests.
- `cargo test --workspace --all-features`: 322 + 420 unit tests, 7 doctests.
- `cargo clippy --workspace --all-targets --all-features`: clean.
- `cargo fmt --all --check`: clean.
- Aspose.Cells reads both generated formulas with
  `isDynamicArrayFormula = true` and the expected `BrtArrFmla` ranges.
- LibreOffice opens the XLSB, calculates the spilled values, and preserves
  array formulas on XLSX export, but drops the metadata part; package decode
  remains the metadata oracle.

## Risks And Mitigations

- **Metadata record mismatch:** use the Aspose oracle and byte assertions for
  every record body.
- **Index drift:** `BrtCellMeta.icmb` is fixed at `1`, matching the single
  `BrtMdb` in the cell-metadata collection.
- **Spill semantics:** the writer does not evaluate formulas. Cached results and
  zero fill remain caller-visible behavior and are documented.
- **Future functions:** unsupported names fail with the existing
  `FormulaError`; they are not silently accepted.
- **Calamine:** may ignore dynamic metadata. Package decode and Aspose are the
  oracles.

## Retirement

No second owner. `ArrayFormulaCell::is_dynamic` owns the flag; `sheet.rs` owns
`BrtCellMeta` emission; `package.rs` owns the Metadata part. Static array
serialization remains the same path.
