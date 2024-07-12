mod biff_id;
mod biff_record;
mod biff_scanner;
mod biff_size;

pub(crate) type BiffId = biff_id::BiffId;
pub(crate) type BiffSize = biff_size::BiffSize;
pub(self) use biff_scanner::BiffScanner;
pub(crate) type BiffRecord = biff_record::BiffRecord;

pub(crate) use biff_record::{try_to_sized, BiffError, FixSizeBiffRecord, VarSizeBiffRecord};

mod records;
