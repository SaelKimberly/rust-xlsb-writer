//! Writer-level errors.

use thiserror::Error;

/// Errors produced while building an XLSB package.
#[derive(Error, Debug)]
pub enum Error {
    /// A low-level record, style, or shared-string operation failed.
    #[error(transparent)]
    Core(#[from] rust_xlsb_core::Error),

    /// A shared-string table operation failed.
    #[error(transparent)]
    Sst(#[from] rust_xlsb_core::SSTHolderError),

    /// An I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A worksheet name violates Excel's naming rules.
    #[error("invalid sheet name: {0}")]
    InvalidSheetName(String),

    /// Two worksheets use the same name.
    #[error("sheet name already used: {0}")]
    SheetNameReused(String),

    /// A row or column index is outside the worksheet limits.
    #[error("row or column index is out of range")]
    RowColumnLimit,

    /// A column range has its first column after its last column.
    #[error("first column is greater than last column")]
    RowColumnOrder,

    /// A worksheet index is outside the workbook.
    #[error("worksheet index out of range: {0}")]
    WorksheetIndex(usize),

    /// A cell string exceeds Excel's 32,767 UTF-16 code-unit limit.
    #[error("string exceeds 32,767 UTF-16 code units")]
    StringTooLong,

    /// A value cannot be represented exactly in the target encoding.
    #[error("value cannot be represented exactly: {0}")]
    ValueNotRepresentable(&'static str),
}

/// Writer result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Compatibility alias for callers that prefer an explicit XLSB error name.
pub type XlsbError = Error;
