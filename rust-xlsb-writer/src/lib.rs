/*
 Copyright 2024 Sael Kimberly

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

 http://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/

//! Optional input adapters:
//!
//! - `jiff`: writes borrowed `jiff::civil::Date`, `Time`, and `DateTime`
//!   values.
//! - `rust_decimal`: writes `rust_decimal::Decimal` values as Excel numbers.
//!
//! Both features are disabled by default. Native [`ExcelDateTime`] values and
//! `chrono` temporal values are always available.

mod conditional_format;
mod data;
mod data_validation;
mod datetime;
pub(crate) mod defined_name;
pub(crate) mod drawing;
#[cfg(test)]
mod e2e;
mod error;
mod filter;
mod format;
mod formula;
mod image;
mod note;
mod package;
mod properties;
mod protection;
mod range_list;
mod sheet;
pub(crate) mod sparkline;
pub(crate) mod table;
mod theme;
mod url;
mod workbook;
mod worksheet;

pub use conditional_format::{
    ConditionalFormat, ConditionalFormat2ColorScale, ConditionalFormat3ColorScale,
    ConditionalFormatAverage, ConditionalFormatAverageRule, ConditionalFormatBlank,
    ConditionalFormatCell, ConditionalFormatCellRule, ConditionalFormatCustomIcon,
    ConditionalFormatDataBar, ConditionalFormatDate, ConditionalFormatDateRule,
    ConditionalFormatDuplicate, ConditionalFormatError, ConditionalFormatFormula,
    ConditionalFormatIconSet, ConditionalFormatIconType, ConditionalFormatText,
    ConditionalFormatTextRule, ConditionalFormatTop, ConditionalFormatTopRule,
    ConditionalFormatType, ConditionalFormatValue, IntoConditionalFormatValue,
};
#[doc(hidden)]
pub use data::CellData;
pub use data::{IntoExcelData, IntoExcelDateTime};
pub use data_validation::{
    DataValidation, DataValidationErrorStyle, DataValidationRule, IntoDataValidationValue,
};
pub use datetime::ExcelDateTime;
pub use error::{Error, Result, XlsbError};
pub use filter::{FilterCondition, FilterCriteria, FilterData, IntoFilterData};
pub use format::{
    Color, FontScheme, Format, FormatAlign, FormatBorder, FormatDiagonalBorder, FormatPattern,
    FormatScript, FormatUnderline,
};
pub use formula::Formula;
pub use image::Image;
pub use note::Note;
pub use properties::{CustomProperty, DocProperties, IntoCustomDateTime, IntoCustomProperty};
pub use protection::ProtectionOptions;
pub use sparkline::{ChartEmptyCells, Sparkline, SparklineRange, SparklineType};
pub use table::{Table, TableColumn, TableFunction, TableStyle};
pub use url::Url;
pub use workbook::{CalculationMode, Workbook};
pub use worksheet::{ColNum, HeaderImagePosition, IgnoreError, RowNum, Worksheet};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
