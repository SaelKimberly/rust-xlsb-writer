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
#[cfg(test)]
mod e2e;
mod error;
mod filter;
mod format;
mod package;
mod properties;
mod protection;
mod sheet;
pub(crate) mod table;
mod url;
mod workbook;
mod worksheet;

pub use conditional_format::{
    ConditionalFormat, ConditionalFormat2ColorScale, ConditionalFormat3ColorScale,
    ConditionalFormatAverage, ConditionalFormatAverageRule, ConditionalFormatBlank,
    ConditionalFormatCell, ConditionalFormatCellRule, ConditionalFormatCustomIcon,
    ConditionalFormatDataBar, ConditionalFormatDuplicate, ConditionalFormatError,
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
pub use properties::{CustomProperty, DocProperties, IntoCustomDateTime, IntoCustomProperty};
pub use protection::ProtectionOptions;
pub use table::{Table, TableColumn, TableStyle};
pub use url::Url;
pub use workbook::{CalculationMode, Workbook};
pub use worksheet::{ColNum, IgnoreError, RowNum, Worksheet};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
