# rust-xlsb-writer ↔ rust_xlsxwriter capability gap and roadmap

- Status: Current-state analysis
- Date: 2026-11-18
- Current writer: `rust-xlsb-writer` at HEAD
- Reference: vendored `rust_xlsxwriter` v0.102.0, commit `d850973`
- Scope: XLSB writer features. Reading and Arrow ingestion are separate
  workstreams and are not scored here.
- Related: `docs/aegis/specs/2026-11-18-xlsxwriter-compatible-api-design.md`,
  `docs/aegis/plans/2026-11-18-xlsxwriter-compatible-api.md`

## 1. Summary

`rust-xlsb-writer` now has a usable, source-compatible subset of the
`rust_xlsxwriter` API: owned workbooks and worksheets, cell values, basic
`Format` styling, row/column layout, shared strings, multi-sheet XLSB packaging,
and the three save entry points, plus an `AutoFilter` range and core/app/custom
document properties with explicit creation timestamps.

It is not a feature-equivalent replacement. The largest gaps are:

1. **Formulas** — no scalar, array, dynamic-array, or formula-result support.
2. **Worksheet layout and view** — user-defined names support literals and
   absolute A1 references only; arbitrary formulas are missing.
3. **Analysis features** — conditional formats cover cell-value comparisons,
   top/bottom, average, duplicate/unique, 2-/3-color scales, basic data bars,
   standard icon sets, text, blank, and error rules; date, formula, x14 icon
   sets, and multi-range rules are missing.
   Basic tables, `AutoFilter` ranges, list/custom filter conditions, data
   validation for non-formula rules, and structural sheet protection are
   present.
4. **Rich objects** — no images, charts, notes, shapes, buttons, checkboxes, or
   sparklines.
5. **Workbook metadata and packaging** — DEFLATE ZIP compression, ZIP64, and
   user-defined names are available, but there are no themes, chartsheets, or
   VBA; core/app/custom document properties are present.
6. **Data ecosystem** — no `Formula`, serde, or Polars adapters; optional
   `jiff` and `rust_decimal` features and the native `ExcelDateTime` type are
   present.
7. **Memory modes** — no constant-memory or low-memory worksheet writers.

The current API surface is deliberately narrow: roughly 13 `Workbook`, 110
`Worksheet`, and 48 `Format` public methods, versus 31, 165, and 52 methods in
the vendored `rust_xlsxwriter`. Method count is not feature count, but it shows
the breadth gap.

## 2. What Already Works

| Area | Current capability |
|---|---|
| Workbook | `new`, `add_worksheet`, standalone `Worksheet::new` with `push_worksheet`, lookup by index or exact name, collection access, calculation mode, read-only recommendation, tab ratio, default cell format, `save`, `save_to_buffer`, `save_to_writer` |
| Worksheet identity | `set_name`, `name`, name validation, default `SheetN` names |
| Values | strings, numbers, booleans, `BErr`, blank, `Option<T>`, `Result<T, E>`, `ExcelDateTime`, chrono dates/times/datetimes, `TimeDelta`, `DateTime<Utc>`, optional `jiff` civil values and `rust_decimal::Decimal`, non-finite display strings |
| Writes | generic `write`, `write_with_format`, formatted/unformatted row and column writes, row/column matrices, typed string/number/boolean/blank/date/time/datetime helpers |
| Cell layout | column width and range width, approximate autofit with row/width limits, column format and range format, border-aware range format, row height, row format, hidden rows/columns, hidden unused rows, cell/range format edits, clear value/format, merged ranges |
| View | freeze panes and top-cell, selection, top-left cell, zoom, normal/page-layout/page-break-preview modes, gridline/zero/right-to-left flags, hidden/very-hidden/active/selected/first tab, tab color |
| Analysis | worksheet `AutoFilter` ranges with list/custom filter conditions and automatic row hiding; basic tables with header rows, built-in styles, and table AutoFilters |
| Metadata | core, app, and custom document properties; explicit creation timestamp |
| Hyperlinks | `write_url` and text/format/tooltip variants, internal and external targets, default hyperlink style, worksheet relationships |
| Print | page orientation, paper size, order, margins, scale/fit, centering, gridlines/headings, black-and-white, draft, first page number, text headers/footers, manual row/column breaks, print areas, repeat rows/columns |
| Protection | sheet protection with optional legacy password verifier, selectable permissions, and unprotected ranges |
| Layout defaults | default row height and pixel variant |
| Format | bold, italic, font color/name/size, number format code/index, alignment, wrap, fill pattern/colors, four border sides/colors, underline, strikethrough, script, indent, rotation, reading direction, shrink, font scheme/family/charset, diagonal borders, locked/unlocked/hidden, quote prefix |
| Styles | value-based `Format` interning, font/fill/border/XF deduplication, Calibri 11 default, custom `iFmt` ids from 164 |
| Strings | high-level strings intern to the SST and emit `BrtCellIsst`; low-level `ToCellValue` keeps inline strings; rich strings emit `BrtCellRString` with font-run indexes |
| Package | multi-sheet workbook, styles part, shared strings part, rels/content types, store-only default ZIP with optional DEFLATE and ZIP64 |
| Errors | `Error` / `XlsbError`, sheet-name, unknown-name, row/column, string-length, value-representation, date/time range, and date/time parse errors |
| Verification | 269 core + 304 writer unit tests, 7 doctests; 4 optional-feature tests with all features; calamine round-trips, LibreOffice visual proof |

## 3. Capability Matrix

Status meanings:

- **Supported** — public API exists and is covered by tests.
- **Partial** — some subset exists, but important modes are missing.
- **Missing** — no public API.
- **Not applicable / later** — XLSX-specific or outside the current XLSB writer
  scope.

### 3.1 Workbook

| `rust_xlsxwriter` capability | Current | Gap |
|---|---|---|
| `new`, `add_worksheet`, `worksheet_from_index`, `worksheet_from_name`, `save*` | Supported | — |
| `Worksheet::new`, `push_worksheet` | Supported | Standalone worksheets attach later |
| `worksheets`, `worksheets_mut` | Supported | Worksheet collection access |
| `add_worksheet_with_constant_memory` | Missing | Streaming writer |
| `add_worksheet_with_low_memory` | Missing | Low-memory writer |
| `new_worksheet_with_constant_memory`, `new_worksheet_with_low_memory` | Missing | Detached worksheet constructors |
| `set_tempdir` | Missing | Temp-file policy for streaming |
| `add_chartsheet` | Missing | Chartsheet packaging |
| `define_name` | Partial | Global/local records; literals and absolute A1 cell/range references |
| `set_properties` | Supported | Core/app/custom strings, typed values, and chrono, jiff, or `ExcelDateTime` creation times |
| `set_default_format` | Supported | Workbook default format; set before worksheets |
| `use_excel_2023_theme`, `use_custom_theme` | Missing | Theme part and theme indexes |
| `add_vba_project`, `add_vba_project_with_signature`, `set_vba_name` | Missing | Macro-enabled XLSB |
| `set_zip_compression_level` | Supported | Deflate levels 1–9; default store-only |
| `use_zip_large_file` | Supported | ZIP64 entry extensions and oversized-directory records |
| `read_only_recommended` | Supported | `BrtFileSharing` flag |
| `set_calculation_mode` | Supported | `BrtCalcProp` mode and flags |
| `set_tab_ratio` | Supported | `BrtBookView.iTabRatio` |
| `register_format`, `populate_string_table` | Missing | Advanced/low-level format and SST hooks |

### 3.2 Worksheet Values And Formulas

| Capability | Current | Gap |
|---|---|---|
| `write`, `write_with_format` | Supported | — |
| `write_row`, `write_column` | Supported | — |
| `write_row_with_format`, `write_column_with_format` | Supported | — |
| `write_row_matrix`, `write_column_matrix` | Supported | — |
| `write_string`, `write_number`, `write_boolean`, `write_blank` | Supported | — |
| `write_datetime`, `write_date_with_format`, `write_time_with_format` | Supported | Unformatted `write_date`/`write_time` are additive extensions |
| `write_date`, `write_time` (unformatted) | Supported extension | Not present in `rust_xlsxwriter` |
| `write_formula`, `write_formula_with_format` | Missing | Formula records and result caching |
| `write_array_formula`, `write_dynamic_array_formula` | Missing | Array/dynamic formula records |
| `write_dynamic_formula` | Missing | Dynamic formula records |
| `write_rich_string` | Supported | `BrtCellRString` plus font-run indexes |
| `write_url`, `write_url_with_text`, `write_url_with_options` | Supported | `BrtHLink` plus worksheet relationships |
| `write_rich_string_with_format` | Supported | Cell-wide format plus run fonts |
| `write_url_with_format` | Supported | Caller-supplied link format |
| `set_formula_result`, `set_formula_result_default` | Missing | Stored formula results |
| `set_nan_value`, `set_infinity_value`, `set_neg_infinity_value` | Supported | Display text for non-finite numbers |
| `set_hidden_zeroes` | Supported | — |

### 3.3 Worksheet Layout, View, And Print

| Capability | Current | Gap |
|---|---|---|
| `set_row_height`, `set_row_format` | Supported | — |
| `set_row_height_pixels` | Supported | — |
| `set_row_hidden`, `set_row_unhidden` | Supported | — |
| `set_default_row_height`, `set_default_row_height_pixels` | Supported | — |
| `group_rows`, `group_rows_collapsed` | Supported | Outline groups |
| `set_column_width`, `set_column_range_width` | Supported | — |
| `set_column_width_pixels`, `set_column_range_width_pixels` | Supported | — |
| `set_column_autofit_width` | Supported | Autofit width hint |
| `set_column_format`, `set_column_range_format` | Supported | — |
| `set_column_hidden`, `set_column_range_hidden` | Supported | — |
| `group_columns`, `group_columns_collapsed` | Supported | Column outline groups |
| `group_symbols_above`, `group_symbols_to_left` | Supported | Outline display |
| `hide_unused_rows` | Supported | Row hiding |
| `autofit`, `autofit_to_max_width` | Supported | Approximate Calibri 11 width; deprecated wrapper |
| `set_autofit_max_row`, `set_autofit_max_width` | Supported | Calculation limits |
| `set_default_format` | Supported | Worksheet or detached-worksheet default format and dimensions |
| `set_cell_format`, `set_range_format` | Supported | — |
| `set_range_format_with_border` | Supported | Border-aware range edit |
| `clear_cell`, `clear_cell_format` | Supported | — |
| `set_selection`, `set_top_left_cell` | Supported | — |
| `set_freeze_panes` | Supported | — |
| `set_freeze_panes_top_cell` | Supported | — |
| `set_zoom` | Supported | `set_zoom_to_fit` remains chartsheet-only |
| `set_screen_gridlines` | Supported | — |
| `set_right_to_left` | Supported | — |
| `set_active`, `set_selected`, `set_hidden`, `set_very_hidden` | Supported | — |
| `set_first_tab`, `set_tab_color` | Supported | — |
| `set_landscape`, `set_portrait`, `set_paper_size`, `set_page_order` | Supported | — |
| `set_margins` | Supported | — |
| `set_header`, `set_footer` | Supported | Text headers/footers |
| `set_header_footer_scale_with_doc`, `set_header_footer_align_with_page` | Supported | Header/footer scaling/alignment |
| `set_header_image`, `set_footer_image` | Missing | Header/footer images |
| `set_print_area`, `set_repeat_rows`, `set_repeat_columns` | Supported | Sheet-scoped defined names |
| `set_print_scale`, `set_print_fit_to_pages` | Supported | — |
| `set_print_center_horizontally`, `set_print_center_vertically` | Supported | — |
| `set_print_gridlines`, `set_print_headings`, `set_print_black_and_white`, `set_print_draft` | Supported | — |
| `set_page_breaks`, `set_vertical_page_breaks` | Supported | Manual page breaks |
| `set_view_normal`, `set_view_page_layout`, `set_view_page_break_preview` | Supported | View modes |
| `set_print_first_page_number` | Supported | — |

### 3.4 Tables, Filters, Validation, And Protection

| Capability | Current | Gap |
|---|---|---|
| `autofilter` | Supported | Range record, filter columns, and automatic row hiding |
| `filter_column`, `filter_automatic_off` | Partial | List/custom string and number filters, blanks, AND/OR; temporal and boolean cells stay unmatched |
| `add_table` | Partial | Header rows, custom columns, built-in styles, and table AutoFilters; totals, formulas, and per-column formats missing |
| `add_conditional_format` | Partial | Cell-value, top/bottom, average, duplicate/unique, 2-/3-color scale, basic data-bar, standard icon-set, text, blank, and error rules; date, formula, x14 icons, data-bar extensions, and multi-range rules missing |
| `add_data_validation` | Partial | List, whole/decimal, date/time, and text-length rules plus prompt/error options; formula-valued rules and multi-range input missing |
| `add_sparkline`, `add_sparkline_group` | Missing | Sparkline extension parts |
| `protect`, `protect_with_password`, `protect_with_options` | Supported | `BrtSheetProtection`; legacy verifier, no encryption |
| `unprotect_range`, `unprotect_range_with_options` | Supported | `BrtRangeProtection` records |
| `ignore_error`, `ignore_error_range` | Supported | `BrtCellIgnoreEC` collection |
| `set_serialize_headers`, serde serialization/deserialization | Missing | Serde integration |

### 3.5 Rich Objects

| Capability | Current | Gap |
|---|---|---|
| `merge_range` | Supported | — |
| `insert_image`, `embed_image`, image offsets/fit-to-cell | Missing | Drawing/image parts |
| `insert_background_image` | Missing | Worksheet background |
| `insert_chart`, `insert_chart_with_offset` | Missing | Chart parts |
| `insert_note` | Missing | Comments/notes |
| `insert_shape`, `insert_shape_with_offset` | Missing | Drawing shapes |
| `insert_button`, `insert_button_with_offset` | Missing | Form controls |
| `insert_checkbox`, `insert_checkbox_with_format` | Missing | Checkbox cells |
| `show_all_notes`, `set_default_note_author` | Missing | Notes display |

### 3.6 Formatting

| `Format` capability | Current | Gap |
|---|---|---|
| `new`, bold, italic, font color/name/size | Supported | — |
| `set_num_format`, `set_num_format_index` | Supported | — |
| `set_align`, `set_text_wrap` | Supported | — |
| `set_pattern`, background/foreground color | Supported | — |
| `set_border`, four sides, four side colors | Supported | — |
| `set_locked`, `set_hidden`, `unset_hidden` | Supported | — |
| `merge` | Supported | `Format::merge` precedence merge |
| `set_font_scheme`, `set_font_family`, `set_font_charset` | Supported | — |
| `set_underline`, `set_font_strikethrough`, `set_font_script` | Supported | — |
| `set_indent`, `set_rotation`, `set_reading_direction`, `set_shrink`, `unset_shrink` | Supported | — |
| `set_border_diagonal`, diagonal color/type | Supported | — |
| `set_hyperlink` | Supported | Default blue underline style |
| `unset_hyperlink_style` | Supported | Restores default font color and underline |
| `set_unlocked` | Supported | — |
| `set_quote_prefix`, `unset_quote_prefix` | Supported | — |
| `set_checkbox`, `unset_checkbox` | Missing | Checkbox format |
| `unset_bold`, `unset_italic`, `unset_font_strikethrough`, `unset_text_wrap`, `unset_shrink` | Supported | — |

### 3.7 Data Types And Features

| Capability | Current | Gap |
|---|---|---|
| Strings, numbers, booleans, errors, blank | Supported | — |
| `Option<T>` | Supported | — |
| `Result<T, E>` | Supported | — |
| chrono `NaiveDate`/`NaiveTime`/`NaiveDateTime` | Supported | — |
| `TimeDelta`, `DateTime<Utc>` | Supported | Additive extensions |
| `ExcelDateTime` | Supported | Native constructors, parser, serial conversion, and cell writes |
| jiff civil types | Supported | Optional `jiff` feature; borrowed dates, times, datetimes |
| `rust_decimal::Decimal` | Supported | Optional `rust_decimal` feature; converted to `f64` |
| `Formula` | Missing | Formula value type |
| `Url` | Supported | Text, tooltip, internal/file/URL targets |
| Serde serialization | Missing | Optional feature |
| Polars integration | Missing | Optional feature |

### 3.8 Package And Runtime

| Capability | Current | Gap |
|---|---|---|
| Multi-sheet XLSB package | Supported | — |
| Styles part | Supported | — |
| Shared strings part | Supported | — |
| Content types and relationships | Supported | — |
| Store-only ZIP | Supported | Default; no compression |
| Deflate compression | Supported | `set_zip_compression_level` |
| ZIP compression level / large-file mode | Supported | Levels 1–9 and optional ZIP64; no >4 GiB test fixture |
| Constant-memory writer | Missing | Streaming worksheet |
| Low-memory writer | Missing | Reduced cell retention |
| Temp-dir control | Missing | Streaming scratch space |
| Document properties/themes/macros/chartsheets | Partial | Core/app/custom document properties; themes, macros, and chartsheets missing |
| Deterministic output timestamps | Supported | Fixed DOS timestamp |

## 4. Behavioral And Contract Differences

These are not necessarily missing features, but they matter for ports.

| Area | `rust_xlsxwriter` | `rust-xlsb-writer` |
|---|---|---|
| Output format | XLSX | XLSB |
| Error name | `XlsxError` | `Error` / `XlsbError`; no alias |
| Save extension | `.xlsx` | `.xlsb`; extension is not validated |
| Default font | Calibri 11 | Calibri 11 |
| Integer precision | `i64`/`u64` cast to `f64` with possible loss | exact or `ValueNotRepresentable` |
| Unformatted date/time | `write_datetime` only | `write_date`/`write_time` are additive extensions |
| `IntoExcelDateTime` | `to_excel_serial_date(&self) -> f64` | `into_cell_data(self) -> Result<CellData>` |
| `CellData` | internal | public but `#[doc(hidden)]` because it appears in a public trait signature |
| Rich strings | supported | supported; run fonts only |
| Formulas | supported | missing |
| Row/column writes | formatted and unformatted variants | both variants |
| Worksheet collection | full mutable/immutable access | index, exact-name, and collection access |
| Memory modes | constant/low-memory | cell-retaining in-memory model |
| ZIP | deflate, compression controls | deflate levels 1–9, optional ZIP64; store-only default |

## 5. Hidden Implementation Gaps

The public API gap is only part of the problem. Each feature also needs XLSB
records or package parts that do not exist yet.

| Feature area | Missing implementation |
|---|---|
| Formulas | Formula token encoder, `BrtFmlaString`/`BrtFmlaNum`/`BrtFmlaBool`/`BrtFmlaError`-class records, cached results, shared/array formula state |
| Hyperlinks | Shipped: `BrtHLink`, URL parsing, worksheet relationships, tooltip/display options |
| Rich strings | Shipped `BrtCellRString`/`RichStr`/`StrRun` with font-run indexes; no phonetic generation or rich SST interning |
| Tables | Shipped: Table parts, `LISTPARTS`, built-in styles, and table AutoFilter ranges; totals, calculated columns, and per-column formats remain |
| Filters | Shipped: `BrtBeginFilterColumn`, list/custom filter records, and automatic row hiding; table filters and icon/top-10/dynamic filters remain |
| Conditional formats | Shipped: cell-value, top/bottom, average, duplicate/unique, 2-/3-color scale, basic data-bar, standard icon-set, text, blank, and error rules; date, formula, x14 icons, data-bar extensions, and multi-range rules remain |
| Data validation | Shipped `DVALS` records for non-formula rules and prompt/error options; formula-valued rules and multi-range input remain |
| Images/charts/shapes | Drawing parts, relationships, anchors, media/chart parts |
| Chartsheets | Chartsheet parts and workbook relationships |
| Notes | Comment/note parts and VML/drawing linkage |
| Defined names | User literals and absolute A1 references, plus print-area and print-title names |
| Document properties | Shipped: core/app strings, custom values, and creation timestamps; themes and macros remain |
| Themes | Theme part and theme color indexes |
| VBA | Macro project parts and content types |
| Memory modes | Streaming row/cell sink instead of retained `BTreeMap` |
| Deflate | Shipped: method 8, levels 1–9, streamed local headers, ZIP64 mode and end records; per-entry strategy remains |
| Workbook behavior | Shipped: `BrtCalcProp`, `BrtFileSharing`, and `BrtBookView.iTabRatio` |
| Worksheet protection | Shipped: `BrtSheetProtection`, `BrtRangeProtection`, legacy password verifier, permission options |

## 6. Roadmap

Priority order favors features that make the writer useful for ordinary
spreadsheet generation, then analysis features, then presentation and rich
objects, then workbook metadata, then performance.

### P0 — Spreadsheet completeness (recommended next)

**Goal:** cover the common “write a real worksheet” path before adding visual
objects.

| Item | Status | Why | Size |
|---|---|---|---|
| `IntoExcelData for Result<T, E>` | ✅ P0a | Common fallible-data pattern; cheap parity win | S |
| `merge_range` | ✅ | Very common layout feature; also unblocks styled reports | M |
| `write_url` + formatted variants | ✅ | Common in reports and dashboards | M |
| Row/column hidden | ✅ P0a | Basic worksheet state; low record complexity | S |
| Default row height, sheet visibility, tab color | Open | Basic worksheet state | S–M |
| `set_freeze_panes`, `set_selection`, `set_top_left_cell` | ✅ | Basic view usability | S–M |
| `set_cell_format`, `set_range_format`, `clear_cell`, `clear_cell_format` | ✅ P0a | Format-only edits without rewriting values | S–M |
| `write_formula` + `set_formula_result` | Open | Biggest functional gap; enables computed reports | L |
| `write_array_formula` / dynamic formulas | Open | Common Excel 365 reporting | L |

**P0a status:** shipped `Result<T,E>`, formatted row/column writes, row/column
matrices, cell/range format edits, clear operations, hidden rows/columns, and
the format-only parity setters. **Merge status:** `merge_range` and the three
MS-XLSB merge records are shipped. **View status:** freeze panes, frozen-pane top cell, selection,
top-left cell, zoom, display flags, pixel-unit layout setters, and the
`WSVIEWS2` records are shipped. **P0b** remains the
formula engine. Hyperlinks and worksheet state are shipped.

**Exit criteria:** public API exists; calamine reads values and formulas’
cached results where supported; record-level tests assert formula/merge/link
records; LibreOffice renders merged ranges and links; no stubs.

### P1 — Analysis features

| Item | Why | Size |
|---|---|---|
| `autofilter` ✅ | Range and filter-column records | M |
| `filter_column` ✅ | List/custom filters, blanks, AND/OR, automatic row hiding | M |
| `add_table` ✅ | Header rows, custom columns, built-in styles, and table AutoFilters | L |
| `add_conditional_format` ✅ Partial | Reporting and highlighting; cell-value, top/bottom, average, duplicate/unique, 2-/3-color scales, basic data bars, standard icon sets, text, blank, error rules | L |
| `add_data_validation` ✅ | List/number/date/time/text-length rules, prompt/error options | M |
| `unprotect_range` ✅ | Shared-workbook safety | M |
| `ignore_error` ✅ | Suppress worksheet error indicators | S |
| `set_formula_result` variants | Finishing touches for computed sheets | S–M |

**Exit criteria:** filters and validation survive calamine where readable;
basic tables are covered by record and package tests; conditional formats
cover the non-extension rule families including 2-/3-color scales, basic data
bars, standard icon sets, text, blank, and error rules, while date, formula,
x14 icons, data-bar extensions, and multi-range rules remain open; malformed
ranges rejected before output.

### P2 — Print and view

| Item | Status | Why | Size |
|---|---|---|---|
| Text headers/footers | ✅ | Reporting | M |
| Header/footer images | Open | Reporting | M |
| Print area, repeat rows/columns | Open | Multi-page reports | M |
| Manual page breaks | ✅ | Multi-page reports | S |
| Group rows/columns, outline symbols | ✅ | Collapsible reports | M |
| Full autofit | ✅ | Approximate Calibri 11 metrics, row/width limits | M |

**Exit criteria:** PDF/print preview through LibreOffice shows the expected
page layout; record-level tests cover print/view records.

### P3 — Rich objects

| Item | Why | Size |
|---|---|---|
| Images: insert, embed, offset, fit-to-cell | Common report requirement | L–XL |
| Notes/comments | Annotation | M–L |
| Shapes/textboxes, buttons, checkboxes | Form-like sheets | L |
| Charts | Dashboards | XL |
| Sparklines | Compact trend display | M |
| Background image | Presentation | M |

**Exit criteria:** each object opens in LibreOffice/Excel; relationships and
media parts are present; no orphaned parts; visual render reviewed.

### P4 — Workbook metadata and packaging

| Item | Why | Size |
|---|---|---|
| ✅ Defined names | Literals and absolute A1 cell/range references; built-in print names | — |
| ✅ Document properties | Core/app/custom properties; chrono, jiff, and `ExcelDateTime` creation times | — |
| Themes and theme colors | Visual parity | M–L |
| Chartsheets | Dashboard structure | L |
| VBA project/signature | Macro-enabled workbooks | XL |
| ✅ Calculation mode, read-only recommended, tab ratio | Workbook behavior | S |
| ✅ Deflate compression controls | Levels 1–9 and ZIP64/large-file mode | S |

**Exit criteria:** workbook-level parts and relationships are complete;
calamine/Excel opens metadata; macro-enabled output is tested only with a
known-good sample.

### P5 — Performance and data ecosystem

| Item | Why | Size |
|---|---|---|
| Constant-memory writer | Large worksheets | L |
| Low-memory writer | Reduced allocation | L |
| Temp-dir control and streaming row sink | Large-file operation | M |
| Serde serialization/deserialization | Structured Rust data | L |
| Polars adapter | Data-pipeline use | L |
| jiff and `rust_decimal` support | Optional type coverage | S–M |
| ✅ Matrix writes | Shipped in P0a | — |
| Compression/large-file tuning | Throughput and size | M |

**Exit criteria:** peak memory is measured against a large fixture; output is
still deterministic; no cell data loss under streaming.

## 7. What Not To Do

- Do not add public methods that return `Unsupported` or silently write a
  string instead of a formula. Unsupported features stay absent until their
  records exist.
- Do not copy XLSX packaging XML into XLSB. Each feature needs its MS-XLSB
  record/part equivalent.
- Do not claim full `rust_xlsxwriter` source compatibility. The current
  contract is a source-compatible subset plus additive date/time helpers.
- Do not prioritize charts/images before formulas, merges, links, and basic
  worksheet state. The latter block more real reports.
- Do not add a second owner for worksheet/package logic; `rust-xlsb-writer`
  remains the only public writer.

## 8. Immediate Recommendation

**P0a is shipped.** The next milestone should be **P0b**, the formula engine,
because formulas are the largest remaining functional gap and need a dedicated
spec:

1. Formula tokenizer/encoder.
2. `write_formula` and `write_formula_with_format`.
3. Cached `set_formula_result` values.
4. Array and dynamic-array formulas.

Hyperlinks, worksheet state, page setup, text headers/footers, manual page
breaks, outline groups, print areas, repeat titles, and approximate autofit are
shipped.

P0a was the best compatibility test: it exercised the existing style,
row/column, SST, and package pipelines without requiring new drawing or chart
parts.

## 9. Evidence And Limits

- Writer API inventory: `rust-xlsb-writer/src/{workbook,worksheet,format,data}.rs`
  and its public re-exports.
- Reference inventory: `thirdparty/rust_xlsxwriter/src/{workbook,worksheet,format,datetime}.rs`
  and the feature list in `thirdparty/rust_xlsxwriter/src/lib.rs`.
- Test evidence: 269 core + 304 writer unit tests and 7 doctests in default
  builds, plus 4 feature-gated writer tests for `jiff` and `rust_decimal`;
  calamine round-trips; LibreOffice render and XLSX round-trip of
  `rust-xlsb-writer/examples/showcase.rs`.
  LibreOffice opens the view fixtures but does not preserve XLSB panes on
  conversion, so view records are proven by record-level decode tests, not a
  rendered freeze. It preserves orientation, page order, margins, print
  gridlines/headings/centering, fit-to-page, first page number, text
  headers/footers, manual page breaks, and hyperlinks; it normalizes paper size
  and drops black-and-white and draft state. It also drops the custom tab ratio,
  manual calculation mode, read-only recommendation, protected ranges, and
  ignored worksheet errors on XLSB import, so those records are proven by decode
  tests, not an XLSX round-trip. LibreOffice preserves the core/app document
  properties, including custom properties and creation timestamps, but drops
  the worksheet `AutoFilter` on XLSB import, so the
  autofilter record is proven by package decode and an independent SheetJS
  read; SheetJS reports `!autofilter.ref`, the filter-database name, and the
  expected hidden rows for list plus custom filters. LibreOffice also preserves
  those hidden rows when importing the XLSB, though it drops the filter
  controls. LibreOffice reads the data-validation records on XLSB import and
  writes the expected list source, prompt/error text, error style, allow-blank,
  dropdown, operator, and numeric/date bounds on XLSX export; the records remain
  proven by package decode tests. It also drops all defined-name formulas on
  XLSB import, including print areas. It drops XLSB tables on import, so the
  Table part, `LISTPARTS`, relationships, and content types are proven by
  record and package decode tests rather than an XLSX round-trip. LibreOffice
  preserves XLSB rich-string runs on XLSX export, including UTF-16 offsets,
  bold/italic runs, and run colors. Calamine does not expose
  `BrtCellRString`, so rich text is proven by package decode plus the
  LibreOffice shared-string round-trip, not calamine.
  The XLSX round-trip keeps both an
  external relationship and an internal `location`. A generated 60-row workbook with breaks at
  rows 20 and 40 renders as three PDF pages, with `Report 1 of 3` through
  `Report 3 of 3` in the header. Grouped rows preserve outline levels and
  hidden rows on XLSX round-trip; collapsed rows disappear from the PDF. Real Excel fixtures decode through
  `BrtBeginHeaderFooter`: `Simple.xlsb` has six null strings and flags
  `0x000c`; `testVarious.xlsb` has text and flags `0x000f`.
- No benchmark measures peak memory or throughput. Memory-mode gaps are
  architectural, not benchmarked.
- No Microsoft Excel validation was available. LibreOffice is the visual oracle.
- XLSB record coverage is not the same as XLSX feature coverage; “missing”
  means “not implemented in this writer”, not “impossible in XLSB”.
