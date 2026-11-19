# Defined names: references to other names

- Date: 2026-11-19
- Status: Implemented — reviewed
- Branch: `deku-biff`
- Depends on: `2026-11-19-defined-name-sheet-ranges.md`

## Goal

Accept same-workbook defined-name references inside defined-name formulas:
`=Base*2`, `=SUM(Rate, Tax)`, `=Sheet1!LocalName`, and
`=Sheet1!LocalName+Global`.

Each reference encodes as one `PtgName` operand whose `nameindex` is the
one-based position of the target `BrtName` record. The target may be defined
after the referencing name, and worksheet-scoped names resolve to their own
sheet before the global fallback.

Cell formulas, table calculated-column formulas, validation, and conditional
formatting keep rejecting defined-name references.

## Aegis Visibility

This changes the defined-name encoder's contract: the parser now needs the
complete ordered name table, not just worksheets and Xti indexes. The table is
built once before any formula is encoded so forward references and local
shadowing resolve deterministically.

`XtiCollector` remains the only `BrtExternSheet` index allocator. No second
name-index owner is added.

## Baseline / Authority

- MS-XLSB `2.5.98.88 PtgName`: first byte `0x23`, then a 4-byte one-based
  `nameindex` into the workbook's `BrtName` collection.
- MS-XLSB `2.4.718 BrtName`: file order defines that collection.
- MS-XLSB `2.5.98.12 NameParsedFormula`: token stream container.
- Existing owners:
  - `rust-xlsb-writer/src/defined_name.rs`: `DefinedName`, `XtiCollector`,
    name encoding.
  - `rust-xlsb-writer/src/formula.rs`: `NameContext` and Ptg emission.
  - `rust-xlsb-writer/src/package.rs`: built-in name order, user-name sort
    order, `BrtName` serialization.

## Compatibility Boundary

- No public API changes. `Workbook::define_name` accepts more strings.
- Existing constant, bare A1, and sheet-reference fast paths keep their exact
  bytes.
- Name lookup is case-insensitive, matching Excel.
- An explicit sheet qualifier resolves that sheet's local name first, then the
  global name. An unqualified reference in a local name resolves the local
  scope first, then global.
- An unqualified reference in a global name only resolves global names.
- Unknown names, sheet-qualified names on a sheet range, external workbook
  names, relative references, and future/`_xlfn` functions remain rejected.
- Built-in print-area, filter-database, and print-title names stay first in the
  collection and shift user-name indexes; they are not user-referenceable.

## Change Necessity

`BrtName` order is already fixed by `write_defined_names`. Reusing that order
in one `NameTable` is smaller and safer than trying to resolve names while
streaming records. The parser already owns identifier parsing and Ptg output;
adding a name branch is the only minimum extension.

## TDD Route

- Mode: `off`
- Decision: `skipped`
- Authority: no explicit TDD request; repository convention is test-after
  regression coverage.
- Test posture: collector/name-index unit tests plus package decoding of
  `BrtName` token streams.

## Tasks

1. **Build one ordered name table.**
   - Add `NameTable` and `NameEntry` to `defined_name.rs`.
   - Compute the built-in `BrtName` count from print areas, autofilters, and
     repeat-title names.
   - Sort user names by `sort_name` with the same stable order as the old
     encoder, assign one-based indexes after the built-ins, and resolve each
     local scope once.
   - Expose the entries for serialization and a case-insensitive lookup that
     applies local-before-global resolution.

2. **Emit `PtgName`.**
   - Add `PTG_NAME = 0x23` and a 4-byte index writer to `formula.rs`.
   - Extend `NameContext` with an optional `NameTable`; keep cell formulas
     name-free by leaving it `None`.
   - In name context, resolve an identifier after function/table/cell checks.
   - After a sheet prefix, fall back from a cell reference to a defined name;
     reject sheet ranges and external-workbook syntax.
   - Resolve explicit sheet-qualified names against that sheet before global.

3. **Wire package serialization.**
   - Build `NameTable` after the built-in records and before user names.
   - Serialize user names by iterating table entries, preserving exact record
     order and scope.
   - Keep `XtiCollector` ordering unchanged.

4. **Prove the feature.**
   - Unit: `Base = Sheet1!$A$1`, `Double = Base*2` emits
     `[0x23, 1, 0, 0, 0, 0x1E, 2, 0, 0x05]`.
   - Unit: forward reference gets the later target index.
   - Unit: local name shadows global for sheet-qualified and local-scope
     lookups; unqualified global names reject local-only targets.
   - Unit: built-in print area shifts the user name index.
   - Unit: Unicode names resolve and encode.
   - Unit: unknown, sheet-range-qualified, and external names are rejected.
   - Package: print-area built-in plus two user names encodes the expected
     `BrtName` token indexes and record order.
   - Existing formula, table, validation, and conditional-format tests remain
     unchanged.

5. **Update boundaries.**
   - Remove defined-name references from the missing list in the gap matrix,
     hidden-gaps table, and P4 row.
   - Update `Workbook::define_name` docs and `AGENTS.md`.
   - Update plan status and `docs/aegis/INDEX.md`.

## Verification

```bash
cargo test -p rust-xlsb-writer defined_name
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets
cargo fmt --all --check
```

Package decode is the byte oracle. LibreOffice drops defined-name formulas on
XLSB import, so it can only prove the package opens, not the token index.

Result: 691 unit tests and 7 doctests in default builds (290 core, 401
writer); 695 unit tests and 7 doctests with all features. Clippy and fmt are
clean. A generated workbook with a print area, a global reference, and a
sheet-qualified local reference opens in LibreOffice and exports CSV without
error. A real Excel `goal.xlsb` fixture confirms the `0x23` + one-based
`BrtName` index encoding for forward LAMBDA references.

## Review

`NameReferenceReview` confirmed the `PtgName` index math, built-in shifts,
local/global shadowing, forward references, and name-context isolation. It
found two P2 parser gaps, both fixed:

1. Names starting with `_` or `\` were accepted by `Workbook::define_name` but
   not routed to name parsing. `parse_primary` now dispatches those starts.
2. Names that begin with a cell-reference prefix, such as `Q1Sales`, were
   parsed as a cell plus trailing text. Cell-reference matches are now rejected
   when the next character is a name character, for both unqualified and
   sheet-qualified names.

Regression tests cover both shapes, including a sheet-qualified lookalike.
Additional coverage handles Unicode names and Unicode case folding.

## Risks And Mitigations

- **Name index drift:** built-in names and user names share the collection.
  Mitigation: one `NameTable` owns the order, and a package test includes a
  print area before user names.
- **Forward references:** a target may be defined after the referrer.
  Mitigation: build the full table before encoding any formula.
- **Local/global shadowing:** the wrong record changes recalculation.
  Mitigation: lookup is local-first only for matching explicit or local scope;
  global formulas reject unqualified local names.
- **Parser ambiguity:** cell names, functions, table references, and sheet
  prefixes already have precedence. Mitigation: insert the name branch after
  those checks and keep existing tests.
- **Sheet ranges:** `Sheet1:Sheet3!Name` has no single local scope.
  Mitigation: reject before lookup.

## Retirement

No second owner. `NameTable` owns `BrtName` order and lookup; `formula.rs`
owns Ptg emission; `XtiCollector` remains the only 3D index allocator.

## Self-Review

- Scope covers one public behavior: more accepted `define_name` formulas.
- Files are existing owners; no new module.
- Verification names the byte-level seam (`BrtName` order and `PtgName`).
- Unsupported boundaries stay explicit.
