//! Worksheet protection options and password verifier.

use rust_xlsb_core::prelude::{
    BrtRangeProtection, BrtSheetProtection, RangeProtectionTitleSDRel, UncheckedRfX,
    UncheckedSqRfX, XlStringExt, XlWideString,
};

/// Protection options for [`Worksheet::protect_with_options`](crate::Worksheet::protect_with_options).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the fields mirror the independent Excel protection checkboxes"
)]
pub struct ProtectionOptions {
    /// Allows selecting locked cells.
    pub select_locked_cells: bool,
    /// Allows selecting unlocked cells.
    pub select_unlocked_cells: bool,
    /// Allows formatting cells.
    pub format_cells: bool,
    /// Allows formatting columns.
    pub format_columns: bool,
    /// Allows formatting rows.
    pub format_rows: bool,
    /// Allows inserting columns.
    pub insert_columns: bool,
    /// Allows inserting rows.
    pub insert_rows: bool,
    /// Allows inserting hyperlinks.
    pub insert_links: bool,
    /// Allows deleting columns.
    pub delete_columns: bool,
    /// Allows deleting rows.
    pub delete_rows: bool,
    /// Allows sorting.
    pub sort: bool,
    /// Allows using autofilters.
    pub use_autofilter: bool,
    /// Allows using pivot tables.
    pub use_pivot_tables: bool,
    /// Allows editing scenarios.
    pub edit_scenarios: bool,
    /// Allows editing objects.
    pub edit_objects: bool,
    /// Allows editing chartsheet contents. Chartsheets are not supported.
    pub contents: bool,
}

impl Default for ProtectionOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtectionOptions {
    /// Creates the default protected-worksheet options.
    ///
    /// Selection is allowed; editing and formatting are not.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            select_locked_cells: true,
            select_unlocked_cells: true,
            format_cells: false,
            format_columns: false,
            format_rows: false,
            insert_columns: false,
            insert_rows: false,
            insert_links: false,
            delete_columns: false,
            delete_rows: false,
            sort: false,
            use_autofilter: false,
            use_pivot_tables: false,
            edit_scenarios: false,
            edit_objects: false,
            contents: true,
        }
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the worksheet model and sheet serializer"
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SheetProtection {
    pub(crate) options: ProtectionOptions,
    pub(crate) password_hash: u16,
}

impl SheetProtection {
    pub(crate) const fn new(options: ProtectionOptions) -> Self {
        Self {
            options,
            password_hash: 0,
        }
    }

    pub(crate) const fn record(self) -> BrtSheetProtection {
        let options = self.options;
        BrtSheetProtection {
            protpwd: self.password_hash,
            fLocked: 1,
            fObjects: options.edit_objects as u32,
            fScenarios: options.edit_scenarios as u32,
            fFormatCells: options.format_cells as u32,
            fFormatColumns: options.format_columns as u32,
            fFormatRows: options.format_rows as u32,
            fInsertColumns: options.insert_columns as u32,
            fInsertRows: options.insert_rows as u32,
            fInsertHyperlinks: options.insert_links as u32,
            fDeleteColumns: options.delete_columns as u32,
            fDeleteRows: options.delete_rows as u32,
            fSelLockedCells: options.select_locked_cells as u32,
            fSort: options.sort as u32,
            fAutoFilter: options.use_autofilter as u32,
            fPivotTables: options.use_pivot_tables as u32,
            fSelUnlockedCells: options.select_unlocked_cells as u32,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the worksheet model and sheet serializer"
)]
pub(crate) struct UnprotectedRange {
    pub(crate) range: UncheckedRfX,
    pub(crate) name: String,
    pub(crate) password_hash: u16,
}

impl UnprotectedRange {
    pub(crate) fn record(&self) -> BrtRangeProtection {
        BrtRangeProtection {
            protPwd: self.password_hash,
            sqRfX: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![self.range],
            },
            RangeProtectionTitleSDRel: RangeProtectionTitleSDRel {
                rgchTitle: XlWideString::construct(Some(self.name.as_str())),
                dwsdRel: 0,
                pSDRel: Vec::new(),
            },
        }
    }
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the worksheet model"
)]
pub(crate) fn hash_password(password: &str) -> u16 {
    if password.is_empty() {
        return 0;
    }

    let mut hash = 0_u16;
    for byte in password.as_bytes().iter().rev() {
        hash = ((hash >> 14) & 0x0001) | ((hash << 1) & 0x7FFF);
        hash ^= u16::from(*byte);
    }
    hash = ((hash >> 14) & 0x0001) | ((hash << 1) & 0x7FFF);
    hash ^= u16::try_from(password.len() & 0xFFFF).expect("masked to 16 bits");
    hash ^ 0xCE4B
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_verifier_matches_excel() {
        assert_eq!(hash_password(""), 0x0000);
        assert_eq!(hash_password("password"), 0x83AF);
        assert_eq!(hash_password("abc123"), 0xC58F);
    }

    #[test]
    fn default_record_locks_cells_and_allows_selection() {
        let record = SheetProtection::new(ProtectionOptions::new()).record();

        assert_eq!(record.protpwd, 0);
        assert_eq!(record.fLocked, 1);
        assert_eq!(record.fSelLockedCells, 1);
        assert_eq!(record.fSelUnlockedCells, 1);
        assert_eq!(record.fObjects, 0);
        assert_eq!(record.fFormatCells, 0);
    }
}
