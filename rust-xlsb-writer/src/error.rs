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

    /// A row/column range has its first index after its last index.
    #[error("range start is after range end")]
    RowColumnOrder,

    /// A method argument is invalid for the requested operation.
    #[error("invalid parameter: {0}")]
    Parameter(String),

    /// A worksheet index is outside the workbook.
    #[error("worksheet index out of range: {0}")]
    WorksheetIndex(usize),

    /// A worksheet name does not exist in the workbook.
    #[error("unknown worksheet name: {0}")]
    UnknownWorksheetNameOrIndex(String),

    /// A worksheet has more than 1023 horizontal or vertical page breaks.
    #[error("a worksheet allows at most 1023 page breaks")]
    TooManyPageBreaks,

    /// Grouping would exceed Excel's seven outline levels.
    #[error("outline level exceeds Excel's seven-level limit")]
    MaxGroupLevelExceeded,

    /// A merge range contains only one cell.
    #[error("a merged range must contain more than one cell")]
    MergeRangeSingleCell,

    /// A merge range intersects an existing merged range.
    #[error("merged ranges overlap")]
    MergeRangeOverlaps,

    /// A cell string exceeds Excel's 32,767 UTF-16 code-unit limit.
    #[error("string exceeds 32,767 UTF-16 code units")]
    StringTooLong,

    /// A URL or internal location exceeds Excel's 2,080 UTF-16 code-unit limit.
    #[error("URL exceeds 2,080 UTF-16 code units")]
    MaxUrlLengthExceeded,

    /// A URL type is not a supported URL, file, mailto, or internal link.
    #[error("unknown URL type: {0}")]
    UnknownUrlType(String),

    /// A hyperlink tooltip exceeds Excel's 255 UTF-16 code-unit limit.
    #[error("hyperlink tooltip exceeds 255 UTF-16 code units")]
    TooltipTooLong,

    /// A value cannot be represented exactly in the target encoding.
    #[error("value cannot be represented exactly: {0}")]
    ValueNotRepresentable(&'static str),

    /// A date or time component is outside Excel's supported range.
    #[error("date/time value out of range: {0}")]
    DateTimeRangeError(String),

    /// A string could not be parsed as a date or time.
    #[error("cannot parse date/time: {0}")]
    DateTimeParseError(String),
}

/// Writer result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Compatibility alias for callers that prefer an explicit XLSB error name.
pub type XlsbError = Error;
