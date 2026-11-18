# Worksheet autofit and limits

- Status: Implemented — reviewed clean
- Date: 2026-11-18
- Scope: `Worksheet::autofit`, `set_autofit_max_row`,
  `set_autofit_max_width`, and deprecated `autofit_to_max_width`.
- Gap source: `docs/gap-analysis-rust-xlsxwriter.md` §3.3.
- Reference: vendored `rust_xlsxwriter` v0.102.0,
  `Worksheet::autofit_worksheet`, `utility::pixel_width`, and
  `store_column_width`.

## Goal

Let a worksheet size its columns from the written cell data, with the same
approximate Calibri 11 pixel metrics and the same opt-in row/width limits as
the reference writer.

## Architecture

Autofit reads the existing sparse `cells` map and writes the existing
`ColumnOptions.width` / `best_fit` fields. No new BIFF records and no package
format changes are needed: autofit only changes `BrtColInfo` widths that an
explicit `set_column_width` already produces.

Width calculation is a pure private helper over `CellData`. It must not
allocate per cell:

- strings are measured in place, line by line;
- integers use a digit count;
- floats use a counting `fmt::Write` sink instead of `to_string`;
- booleans, dates, times, datetimes, and durations use fixed reference widths;
- blanks and error cells contribute nothing.

Autofit applies the 7-pixel cell padding, the 16-pixel autofilter dropdown
padding, and the configured maximum width. Manual column widths are preserved
unless the computed width is wider, matching `store_column_width`.

## Tasks

### Task 1: Autofit state and limits

**Files:**
- Modify: `rust-xlsb-writer/src/worksheet.rs`

**Interfaces:**
- Produces:
  - `Worksheet::autofit(&mut self) -> &mut Self`
  - `Worksheet::set_autofit_max_row(&mut self, max_row: RowNum) -> &mut Self`
  - `Worksheet::set_autofit_max_width(&mut self, max_width: u32) -> &mut Self`
  - deprecated `Worksheet::autofit_to_max_width(&mut self, max_width: u32) -> &mut Self`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn autofit_sizes_columns_from_cell_data() {
    let mut worksheet = Worksheet::new();
    worksheet.write(0, 0, "Hello").expect("string");
    worksheet.write(0, 1, 123456).expect("integer");
    worksheet.write(0, 2, true).expect("boolean");
    worksheet.write(1, 0, "Hello World").expect("longer string");
    worksheet.write(2, 0, "short").expect("shorter string");

    worksheet.autofit();

    assert_eq!(worksheet.column_options()[&0].width, Some(76.0 / 7.0));
    assert_eq!(worksheet.column_options()[&1].width, Some(44.0 / 7.0));
    assert_eq!(worksheet.column_options()[&2].width, Some(33.0 / 7.0));
    assert!(worksheet.column_options()[&0].best_fit);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rust-xlsb-writer autofit_sizes_columns_from_cell_data`
Expected: FAIL because `autofit` does not exist.

- [ ] **Step 3: Write minimal implementation**

Add `MAX_AUTOFIT_WIDTH_PIXELS: u32 = 1790`, fields
`max_autofit_row: RowNum` and `max_autofit_width: u32`, and initialize them to
`MAX_ROW` and `MAX_AUTOFIT_WIDTH_PIXELS`.

```rust
pub fn autofit(&mut self) -> &mut Self {
    let mut max_widths: BTreeMap<ColNum, u32> = BTreeMap::new();
    for (&row, cells) in &self.cells {
        if row >= self.max_autofit_row {
            break;
        }
        for (&col, cell) in cells {
            let mut width = autofit_cell_width(&cell.data);
            if width == 0 {
                continue;
            }
            if self.autofilter.is_some_and(|range| {
                row == range.rwFirst && col >= range.colFirst && col <= range.colLast
            }) {
                width += 16;
            }
            width = (width + 7).min(self.max_autofit_width);
            max_widths
                .entry(col)
                .and_modify(|max| *max = (*max).max(width))
                .or_insert(width);
        }
    }
    for (col, pixels) in max_widths {
        self.store_autofit_width(col, pixels);
    }
    self
}
```

`store_autofit_width` converts pixels with the existing `pixels_to_width` and:

```rust
let width = pixels_to_width(pixels);
let options = self.columns.entry(col).or_default();
if options.width.is_none()
    || options.best_fit
    || width > options.width.expect("checked")
{
    options.width = Some(width);
    options.best_fit = true;
}
```

Width helpers:

```rust
fn autofit_cell_width(data: &CellData) -> u32 {
    match data {
        CellData::String(text) => text.lines().map(pixel_width).max().unwrap_or(0),
        CellData::Integer(value) => {
            7 * integer_digits(*value) + if *value < 0 { 7 } else { 0 }
        }
        CellData::Number(value) => 7 * display_len(*value),
        CellData::Bool(true) => 31,
        CellData::Bool(false) => 36,
        CellData::Date(_)
        | CellData::Time(_)
        | CellData::DateTime(_)
        | CellData::Duration(_) => 68,
        CellData::Blank | CellData::Error(_) => 0,
    }
}
```

`pixel_width` is the reference Calibri 11 character table, capped at 1790
pixels after 233 characters. `integer_digits` repeatedly divides by 10, and
negative integers add the 7-pixel sign width.
`display_len` implements `fmt::Write` on a `usize` counter and uses
`write!(&mut sink, "{value}")`, so no `String` is allocated.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p rust-xlsb-writer autofit_sizes_columns_from_cell_data`
Expected: PASS.

- [ ] **Step 5: Add limit and preservation tests**

```rust
#[test]
fn autofit_preserves_wider_manual_widths_and_widens_narrow_ones() {
    let mut worksheet = Worksheet::new();
    worksheet.write(0, 0, "Hello").expect("cell");
    worksheet.set_column_width(0, 20).expect("manual");
    worksheet.set_column_width(1, 2).expect("narrow");
    worksheet.write(0, 1, "Hello").expect("cell");

    worksheet.autofit();

    assert_eq!(worksheet.column_options()[&0].width, Some(20.0));
    assert_eq!(worksheet.column_options()[&1].width, Some(5.0));
    assert!(!worksheet.column_options()[&0].best_fit);
    assert!(worksheet.column_options()[&1].best_fit);
}

#[test]
fn autofit_honors_row_and_width_limits() {
    let mut worksheet = Worksheet::new();
    worksheet.write(0, 0, "short").expect("cell");
    worksheet.write(10, 0, "Much longer text").expect("cell");
    worksheet.set_autofit_max_row(10);
    worksheet.set_autofit_max_width(100);
    worksheet.autofit();
    assert_eq!(worksheet.column_options()[&0].width, Some(34.0 / 7.0));

    let mut worksheet = Worksheet::new();
    worksheet.write(0, 0, "MMMMMMMMMMMMMMMMMMMM").expect("cell");
    worksheet.set_autofit_max_width(100);
    worksheet.autofit();
    assert_eq!(worksheet.column_options()[&0].width, Some(95.0 / 7.0));
}
```

Also cover multiline strings, autofilter header padding, booleans, dates, and
the deprecated wrapper:

```rust
#[test]
#[allow(deprecated)]
fn autofit_to_max_width_delegates_to_limit_and_autofit() {
    let mut worksheet = Worksheet::new();
    worksheet.write(0, 0, "MMMMMMMMMMMMMMMMMMMM").expect("cell");
    worksheet.autofit_to_max_width(100);
    assert_eq!(worksheet.column_options()[&0].width, Some(95.0 / 7.0));
}
```

- [ ] **Step 6: Run the autofit tests**

Run: `cargo test -p rust-xlsb-writer autofit`
Expected: PASS.

### Task 2: Public surface and documentation

**Files:**
- Modify: `rust-xlsb-writer/src/lib.rs`
- Modify: `docs/gap-analysis-rust-xlsxwriter.md`
- Modify: `AGENTS.md`
- Modify: `docs/aegis/INDEX.md`
- Modify: this plan's status

**Interfaces:**
- No new exports: `Worksheet` already re-exports inherent methods.

- [ ] **Step 1: Make the methods public and documented**

`autofit`, `set_autofit_max_row`, and `set_autofit_max_width` carry doc comments
that state the Calibri 11 approximation, the 1790-pixel/255-column limits, and
that explicit widths survive unless autofit is wider. `autofit_to_max_width`
is `#[deprecated]` and delegates to the two calls.

- [ ] **Step 2: Update capability matrix**

Change the four autofit rows in §3.3 from Missing to Supported, naming the
7-pixel/16-pixel padding and approximate metrics.

- [ ] **Step 3: Update docs and test counts**

Update `AGENTS.md` counts after the final run. Add the plan to
`docs/aegis/INDEX.md`.

- [ ] **Step 4: Run the full checks**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
git diff --check
```

Expected: all tests pass, no clippy warnings, formatting and diff checks clean.

## Verification

Result: 193 core + 194 writer unit tests and 6 doctests pass. Clippy, fmt, and
diff checks pass. Reviewer `AutofitReview` found two P2 issues and one P3 doc
issue; all fixed:

- a zero maximum width no longer creates zero-width column entries;
- negative integers now include the sign in their pixel width;
- the default row-limit documentation now says the last row is not measured.

## Review Focus

- A cell whose text ends with `\r` or contains `\r\n`: `lines()` must measure
  the visible line, not the terminator.
- A negative integer at `i64::MIN`: digit count must use `unsigned_abs` and
  must not overflow.
- A float with a long `Display` form: counting output must match
  `to_string().len()` without allocating.
- A manual column wider than the computed width: width and `best_fit` must not
  change.
- A zero maximum width: autofit must not create an entry or hide populated
  columns.
- A negative integer: its sign must count toward the pixel width.
- A column with only blanks or errors: autofit must not create a
  `ColumnOptions` entry.
