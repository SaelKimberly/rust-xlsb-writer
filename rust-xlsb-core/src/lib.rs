#![deny(dead_code)]
#![allow(internal_features)]
#![feature(maybe_uninit_write_slice)]
#![feature(core_intrinsics)]
#![feature(str_from_utf16_endian)]

mod biff_head;
mod biff_id;
mod check;
mod err;
pub mod prelude;

pub use biff_head::BiffHead;
pub use biff_id::KnownID;
pub use check::{Checked, CheckedBiff, Unchecked};
pub use err::{Error, Result};

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
