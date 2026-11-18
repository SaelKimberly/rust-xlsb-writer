/*
 Copyright 2024 Sael Kimberly <sael.kimberly@yandex.ru>

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

#![deny(dead_code)]
mod biff;
mod biff_head;
mod biff_id;
mod cell_value;
mod check;
mod context;
mod err;
pub mod prelude;
mod sst_sqlite;
mod workbook;

#[cfg(test)]
mod e2e;

pub use biff::{BiffWrite, write_as_biff, write_as_empty_biff};
pub use biff_head::BiffHead;
pub use biff_id::{KnownID, RawBiffLiteral};
pub use cell_value::{CellValue, ToCellValue};
pub use check::{Checked, CheckedBiff, Unchecked};
pub use context::{NumberEncoding, NumberPolicy, SharedString, WbContext};
pub use err::{Error, Result};
pub use sst_sqlite::{SSTHolder, SSTHolderError};
pub use workbook::{Workbook, Worksheet};

#[allow(clippy::crate_in_macro_def)]
#[macro_export]
macro_rules! assign_id {
    ($name: ident) => {
        impl crate::CheckedBiff for $name {
            fn id(&self) -> crate::KnownID {
                crate::KnownID::$name
            }
        }
    };
}
