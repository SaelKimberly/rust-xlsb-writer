# Worksheet Collection Access

- Status: Implemented — reviewer findings fixed
- Date: 2026-11-18
- Scope: standalone `Worksheet` construction, workbook lookup, mutable/read-only
  collection access, and `push_worksheet`
- Basis: `docs/gap-analysis-rust-xlsxwriter.md` section 3.1;
  `rust_xlsxwriter` `Worksheet::new`, `Workbook::worksheet_from_name`,
  `Workbook::worksheets`, `Workbook::worksheets_mut`, `Workbook::push_worksheet`

## 1. Goal

Close the worksheet-collection gap with the reference signatures:

```rust
impl Worksheet {
    pub fn new() -> Worksheet;
}

impl Workbook {
    pub fn worksheet_from_name(&mut self, sheetname: &str) -> Result<&mut Worksheet>;
    pub fn worksheets(&mut self) -> &Vec<Worksheet>;
    pub fn worksheets_mut(&mut self) -> &mut Vec<Worksheet>;
    pub fn push_worksheet(&mut self, worksheet: Worksheet);
}
```

A standalone worksheet can be written before it joins a workbook, matching the
reference borrow-checker pattern. Pushing an unnamed worksheet assigns
`SheetN` using the current worksheet count. Named worksheets keep their names.
Existing output is unchanged when these APIs are unused.

## 2. Architecture

`Worksheet` already owns its name, cells, formats, and view state. The only
construction change is making the zero-argument constructor public and moving
the current named constructor to a crate-private helper used by
`Workbook::add_worksheet` and tests. This avoids re-validating the generated
default name on every add.

`Workbook` owns `Vec<Worksheet>`. Lookup scans names and then reuses
`worksheet_from_index`. Collection access returns the vector itself, matching
the reference signatures. `push_worksheet` assigns a default name only when the
standalone worksheet has an empty name.

Name uniqueness remains a save-time invariant in `Workbook::prepare`; no second
validation path is introduced.

## 3. Compatibility Boundary

- `worksheet_from_name` is exact and case-sensitive, like the reference.
- Unknown names return a new `Error::UnknownWorksheetNameOrIndex(String)`.
- `push_worksheet` does not renumber existing sheets or resolve collisions;
  duplicate names fail at save with `SheetNameReused`.
- `worksheets` takes `&mut self` to match the reference, even though it returns
  a shared reference.
- No package layout, record, or default-output change.

## 4. Change Necessity

The gap is public API only; documentation or an internal helper cannot expose
the reference workflow. The minimum code boundary is `workbook.rs`,
`worksheet.rs`, and one error variant.

## 5. TDD Route

Mode: off. Decision: skipped. Authority: no explicit TDD request. Test posture:
one focused unit test per new behavior plus calamine readback where the output
changes.

## 6. Tasks

1. Public construction
   - `worksheet.rs`: rename the private `new(String)` to `new_named`, add
     `pub fn new() -> Self` returning an empty name, and `impl Default`.
   - Update internal test call sites to `new_named`.
   - Replace `Workbook::add_worksheet` construction with `new_named`.
2. Lookup and access
   - `workbook.rs`: add `worksheet_from_name`, `worksheets`, `worksheets_mut`.
   - `error.rs`: add `UnknownWorksheetNameOrIndex(String)`.
3. Push
   - `workbook.rs`: add `push_worksheet`; assign `Sheet{len+1}` only when the
     name is empty.
4. Tests
   - Lookup by name writes to the right sheet and reads back.
   - Unknown name returns the new variant.
   - Standalone, named worksheet pushes without renaming.
   - Unnamed standalone worksheet gets `Sheet1` and reads back.
   - Collection accessors see and mutate the same sheets.
5. Docs
   - Mark the four capabilities supported in the gap analysis.
   - Update `AGENTS.md` API inventory and test counts.
   - Add the plan to `docs/aegis/INDEX.md`.

## 7. Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo fmt --all --check`
- `git diff --check`
- Calamine reads the pushed and looked-up sheets from a saved buffer.

Result: 173 core + 132 writer unit tests and 6 doctests pass; clippy, fmt, and
`git diff --check` are clean. Reviewer found no functional defect; its two
low-severity findings (stale plan status and missing push-state assertions) are
fixed. The push test now covers the initial `Sheet1` default and proves the
pre-push cell survives.

## 8. Risks

- A standalone worksheet has an empty name until pushed; saving it directly is
  impossible because it is not owned by a workbook.
- `push_worksheet` uses `Sheet{len+1}` even when that name is already taken;
  save rejects the duplicate. This matches the reference behavior.
- No compatibility shim for the old crate-private constructor is retained.

## 9. Retirement

Replace the old private constructor; no public API is removed.
