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

mod data;
#[cfg(test)]
mod e2e;
mod error;
mod format;
mod package;
mod sheet;
mod url;
mod workbook;
mod worksheet;

#[doc(hidden)]
pub use data::CellData;
pub use data::{IntoExcelData, IntoExcelDateTime};
pub use error::{Error, Result, XlsbError};
pub use format::{
    Color, FontScheme, Format, FormatAlign, FormatBorder, FormatDiagonalBorder, FormatPattern,
    FormatScript, FormatUnderline,
};
pub use url::Url;
pub use workbook::{CalculationMode, Workbook};
pub use worksheet::{ColNum, RowNum, Worksheet};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
